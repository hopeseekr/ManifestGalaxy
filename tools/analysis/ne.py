"""Bounded, read-only parsing of 16-bit Windows NE executables.

This module is evidence-extraction code.  It never executes the input and it
never exposes raw resource or segment payloads in the report it builds.  All
reads are checked against the file length before they happen.

Reference notes (format, not game behaviour):
  * DOS header: ``e_lfanew`` at file offset 0x3c points at the NE signature.
  * NE header: 64 bytes starting at the signature; all offsets except the
    nonresident-name table are relative to the start of the NE header.
  * Segment table: 8-byte entries (sector offset, length, flags, min alloc).
  * Resource table: alignment shift, TYPEINFO records, NameInfo records, then
    Pascal-string names.  Resource ``length`` fields in the supplied binary are
    in alignment units (sectors), not bytes.
  * Resident names: Pascal strings plus a 16-bit ordinal.
  * Module references: 16-bit offsets into the imported-name table.
  * Imported names: Pascal strings.
  * Entry table: bundles of fixed, movable, or constant entries.

The module deliberately reports only evidence (counts, locations, captions,
selected string windows), not claims about runtime behaviour.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from hashlib import sha256
import struct
from typing import Any, Iterable


# ---------------------------------------------------------------------------
# Errors and bounded reading
# ---------------------------------------------------------------------------


class NEFormatError(Exception):
    """A malformed NE structure, with enough detail to fix the input.

    ``path`` is a human-readable structure path, ``offset`` is an absolute
    file offset, and ``expected``/``actual`` describe the failed check.
    """

    def __init__(
        self,
        path: str,
        offset: int,
        expected: str,
        actual: str,
        *,
        detail: str | None = None,
    ) -> None:
        self.path = path
        self.offset = offset
        self.expected = expected
        self.actual = actual
        self.detail = detail
        super().__init__(self.__str__())

    def __str__(self) -> str:  # pragma: no cover - exercised through messages
        message = (
            f"{self.path}: file offset 0x{self.offset:x}: "
            f"expected {self.expected}, got {self.actual}"
        )
        if self.detail:
            message += f" ({self.detail})"
        return message


class Reader:
    """A bounded view over immutable bytes.

    Offsets passed to the methods are always absolute file offsets.  ``start``
    and ``end`` bound every read, so a malformed table can never make the
    parser inspect bytes outside the file or outside the declared table.
    """

    def __init__(
        self,
        data: bytes,
        path: str = "<input>",
        start: int = 0,
        end: int | None = None,
    ) -> None:
        if end is None:
            end = len(data)
        if start < 0 or end < start or end > len(data):
            raise ValueError(
                f"invalid reader bounds 0x{start:x}..0x{end:x} for {len(data)} bytes"
            )
        self.data = data
        self.path = path
        self.start = start
        self.end = end

    # -- primitives ---------------------------------------------------------

    def _check(self, offset: int, length: int, what: str) -> None:
        if offset < self.start or length < 0:
            raise NEFormatError(
                self.path,
                max(offset, 0),
                f"{what} inside 0x{self.start:x}..0x{self.end:x}",
                "offset before the bounded region",
            )
        if offset + length > self.end:
            available = max(0, self.end - offset)
            raise NEFormatError(
                self.path,
                offset,
                f"{what} of {length} byte(s) inside "
                f"0x{self.start:x}..0x{self.end:x}",
                f"only {available} byte(s) available",
            )

    def u8(self, offset: int, what: str = "byte") -> int:
        self._check(offset, 1, what)
        return self.data[offset]

    def u16(self, offset: int, what: str = "word") -> int:
        self._check(offset, 2, what)
        return struct.unpack_from("<H", self.data, offset)[0]

    def u32(self, offset: int, what: str = "dword") -> int:
        self._check(offset, 4, what)
        return struct.unpack_from("<I", self.data, offset)[0]

    def bytes(self, offset: int, length: int, what: str = "bytes") -> bytes:
        self._check(offset, length, what)
        return self.data[offset : offset + length]

    def view(self, offset: int, length: int, path: str) -> "Reader":
        self._check(offset, length, path)
        return Reader(self.data, path, offset, offset + length)

    # -- strings ------------------------------------------------------------

    def pascal(self, offset: int, what: str, *, limit: int | None = None) -> tuple[str, int]:
        """Read a Pascal string and return ``(text, next_offset)``.

        ``limit`` further bounds the string (for example, to a table end).
        The length byte and all characters must fit both the reader bounds and
        the optional limit.
        """

        end = self.end if limit is None else min(limit, self.end)
        if offset < self.start or offset >= end:
            raise NEFormatError(
                self.path,
                offset,
                f"{what} length byte inside 0x{self.start:x}..0x{end:x}",
                "offset outside bounded name area",
            )
        length = self.data[offset]
        if offset + 1 + length > end:
            raise NEFormatError(
                self.path,
                offset,
                f"{what} with 1 length byte + {length} character(s) "
                f"inside 0x{self.start:x}..0x{end:x}",
                f"only {max(0, end - offset - 1)} character byte(s) available",
            )
        raw = self.data[offset + 1 : offset + 1 + length]
        return raw.decode("latin-1"), offset + 1 + length

    def cstring(
        self,
        offset: int,
        what: str,
        *,
        limit: int | None = None,
        max_length: int = 4096,
    ) -> tuple[str, int]:
        """Read a NUL-terminated Latin-1 string within the bounds."""

        end = self.end if limit is None else min(limit, self.end)
        if offset < self.start or offset >= end:
            raise NEFormatError(
                self.path,
                offset,
                f"{what} inside 0x{self.start:x}..0x{end:x}",
                "offset outside bounded string area",
            )
        stop = min(end, offset + max_length)
        nul = self.data.find(b"\x00", offset, stop)
        if nul < 0:
            raise NEFormatError(
                self.path,
                offset,
                f"NUL-terminated {what} within {max_length} byte(s) and "
                f"0x{offset:x}..0x{end:x}",
                "no NUL terminator found",
            )
        return self.data[offset:nul].decode("latin-1"), nul + 1


# ---------------------------------------------------------------------------
# Constants and small helpers
# ---------------------------------------------------------------------------


NE_HEADER_SIZE = 64
SEGMENT_ENTRY_SIZE = 8
TYPEINFO_SIZE = 8
NAMEINFO_SIZE = 12

RT_NAMES = {
    1: "CURSOR",
    2: "BITMAP",
    3: "ICON",
    4: "MENU",
    5: "DIALOG",
    6: "STRING",
    7: "FONTDIR",
    8: "FONT",
    9: "ACCELERATOR",
    10: "RCDATA",
    11: "MESSAGETABLE",
    12: "GROUP_CURSOR",
    14: "GROUP_ICON",
    16: "VERSION",
    17: "DLGINCLUDE",
    19: "PLUGPLAY",
    20: "VXD",
    21: "ANICURSOR",
    22: "ANIICON",
    23: "HTML",
}

TARGET_OS_NAMES = {
    0: "unknown",
    1: "OS/2",
    2: "Windows",
    3: "European MS-DOS 4.x",
    4: "Windows 386",
    5: "BOSS",
}

SEGMENT_FLAG_NAMES = (
    (0x0001, "DATA"),
    (0x0002, "ALLOCATED"),
    (0x0004, "LOADED"),
    (0x0010, "MOVABLE"),
    (0x0020, "PURE_SHAREABLE"),
    (0x0040, "PRELOAD"),
    (0x0080, "EXECUTE_ONLY_OR_READ_ONLY"),
    (0x0100, "RELOCATIONS"),
    (0x1000, "DISCARDABLE"),
)

RESOURCE_FLAG_NAMES = (
    (0x0010, "MOVEABLE"),
    (0x0020, "PURE_SHAREABLE"),
    (0x0040, "PRELOAD"),
)

ADDITIONAL_FLAG_NAMES = (
    (0x01, "WINDOWS_2X_PROTECTED_MODE"),
    (0x02, "PROPORTIONAL_FONTS"),
    (0x08, "FAST_LOAD_AREA"),
)

DIALOG_STYLE_DS_SETFONT = 0x0040


def _flag_names(value: int, table: Iterable[tuple[int, str]]) -> list[str]:
    return [name for bit, name in table if value & bit]


def _hex(value: int | None) -> str | None:
    return None if value is None else f"0x{value:x}"


def file_sha256(data: bytes) -> str:
    return sha256(data).hexdigest()


def _resource_type_name(type_id: int, string_name: str | None) -> str:
    if string_name is not None:
        return string_name
    ordinal = type_id & 0x7FFF
    return RT_NAMES.get(ordinal, f"#{ordinal}")


def _resource_type_ordinal(type_id: int) -> int | None:
    if type_id & 0x8000:
        return type_id & 0x7FFF
    return None


def _resource_id_dict(raw: int, name: str | None) -> dict[str, Any]:
    if raw & 0x8000:
        ordinal = raw & 0x7FFF
        return {
            "kind": "integer",
            "raw": raw,
            "id": ordinal,
            "display": f"#{ordinal}",
        }
    return {
        "kind": "string",
        "raw": raw,
        "name": name,
        "display": name if name is not None else f"<string offset 0x{raw:x}>",
    }


# ---------------------------------------------------------------------------
# DOS and NE headers
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class DOSTable:
    magic: str
    e_lfanew: int
    file_size: int
    sha256: str

    def as_dict(self) -> dict[str, Any]:
        return {
            "magic": self.magic,
            "e_lfanew": self.e_lfanew,
            "e_lfanew_hex": _hex(self.e_lfanew),
            "file_size": self.file_size,
            "sha256": self.sha256,
        }


@dataclass(frozen=True)
class NEHeader:
    signature: str
    file_offset: int
    linker_version: int
    linker_revision: int
    entry_table_offset: int
    entry_table_length: int
    crc32: int
    flags: int
    automatic_data_segment: int
    heap_size: int
    stack_size: int
    cs_ip_raw: int
    ss_sp_raw: int
    segment_count: int
    module_reference_count: int
    nonresident_names_size: int
    segment_table_offset: int
    resource_table_offset: int
    resident_names_offset: int
    module_reference_table_offset: int
    imported_names_offset: int
    nonresident_names_offset: int
    movable_entry_count: int
    sector_alignment_shift: int
    resource_segment_count: int
    target_os: int
    additional_flags: int
    return_thunks_offset: int
    segment_ref_thunks_offset: int
    swap_area_size: int
    expected_windows_version_raw: int

    def file_offset_of(self, relative: int) -> int:
        return self.file_offset + relative

    @property
    def cs(self) -> int:
        return (self.cs_ip_raw >> 16) & 0xFFFF

    @property
    def ip(self) -> int:
        return self.cs_ip_raw & 0xFFFF

    @property
    def ss(self) -> int:
        return (self.ss_sp_raw >> 16) & 0xFFFF

    @property
    def sp(self) -> int:
        return self.ss_sp_raw & 0xFFFF

    def as_dict(self) -> dict[str, Any]:
        minor = self.expected_windows_version_raw & 0xFF
        major = (self.expected_windows_version_raw >> 8) & 0xFF
        return {
            "signature": self.signature,
            "file_offset": self.file_offset,
            "file_offset_hex": _hex(self.file_offset),
            "linker_version": f"{self.linker_version}.{self.linker_revision}",
            "linker_version_major": self.linker_version,
            "linker_version_minor": self.linker_revision,
            "entry_table": {
                "ne_relative_offset": self.entry_table_offset,
                "file_offset": self.file_offset_of(self.entry_table_offset),
                "length": self.entry_table_length,
            },
            "crc32": self.crc32,
            "flags": self.flags,
            "flags_hex": _hex(self.flags),
            "automatic_data_segment": self.automatic_data_segment,
            "initial_heap_size": self.heap_size,
            "initial_stack_size": self.stack_size,
            "cs_ip": {
                "raw": self.cs_ip_raw,
                "raw_hex": _hex(self.cs_ip_raw),
                "ip_offset": self.ip,
                "cs_segment_number": self.cs,
                "note": "CS is a 1-based segment index, IP is segment-relative",
            },
            "ss_sp": {
                "raw": self.ss_sp_raw,
                "raw_hex": _hex(self.ss_sp_raw),
                "sp_offset": self.sp,
                "ss_segment_number": self.ss,
                "note": "SS is a 1-based segment index, SP is segment-relative",
            },
            "segment_count": self.segment_count,
            "module_reference_count": self.module_reference_count,
            "nonresident_names_size": self.nonresident_names_size,
            "segment_table": {
                "ne_relative_offset": self.segment_table_offset,
                "file_offset": self.file_offset_of(self.segment_table_offset),
            },
            "resource_table": {
                "ne_relative_offset": self.resource_table_offset,
                "file_offset": self.file_offset_of(self.resource_table_offset),
            },
            "resident_names_table": {
                "ne_relative_offset": self.resident_names_offset,
                "file_offset": self.file_offset_of(self.resident_names_offset),
            },
            "module_reference_table": {
                "ne_relative_offset": self.module_reference_table_offset,
                "file_offset": self.file_offset_of(
                    self.module_reference_table_offset
                ),
            },
            "imported_names_table": {
                "ne_relative_offset": self.imported_names_offset,
                "file_offset": self.file_offset_of(self.imported_names_offset),
            },
            "nonresident_names_table": {
                "file_relative_offset": self.nonresident_names_offset,
                "file_offset": self.nonresident_names_offset,
                "note": "stored relative to the file, not the NE header",
            },
            "movable_entry_count": self.movable_entry_count,
            "sector_alignment_shift": self.sector_alignment_shift,
            "sector_size": 1 << self.sector_alignment_shift
            if self.sector_alignment_shift < 32
            else None,
            "resource_segment_count": self.resource_segment_count,
            "target_os": {
                "value": self.target_os,
                "name": TARGET_OS_NAMES.get(self.target_os, "unknown"),
            },
            "additional_flags": self.additional_flags,
            "additional_flags_hex": _hex(self.additional_flags),
            "additional_flag_names": _flag_names(
                self.additional_flags, ADDITIONAL_FLAG_NAMES
            ),
            "return_thunks_offset": self.return_thunks_offset,
            "segment_ref_thunks_offset": self.segment_ref_thunks_offset,
            "swap_area_size": self.swap_area_size,
            "expected_windows_version": {
                "raw": self.expected_windows_version_raw,
                "raw_hex": _hex(self.expected_windows_version_raw),
                "major": major,
                "minor": minor,
                "text": f"{major}.{minor:02d}",
                "note": "minor version byte precedes major version byte",
            },
        }


def parse_dos_header(data: bytes, path: str = "<input>") -> DOSTable:
    if len(data) < 2:
        raise NEFormatError(
            "DOS header.magic", 0, "2-byte MZ signature", f"{len(data)} byte(s)"
        )
    if len(data) < 0x40:
        raise NEFormatError(
            "DOS header",
            0,
            "at least 64 bytes so that e_lfanew at 0x3c is readable",
            f"{len(data)} byte(s)",
        )
    magic = data[:2]
    if magic != b"MZ":
        raise NEFormatError(
            "DOS header.magic", 0, "MZ signature", repr(magic), detail="not an MZ executable"
        )
    reader = Reader(data, path)
    e_lfanew = reader.u32(0x3C, "DOS header.e_lfanew")
    if e_lfanew >= len(data):
        raise NEFormatError(
            "DOS header.e_lfanew",
            0x3C,
            f"NE header offset inside the {len(data)}-byte file",
            f"0x{e_lfanew:x} (past end of file)",
        )
    if e_lfanew + 2 > len(data):
        raise NEFormatError(
            "DOS header.e_lfanew",
            0x3C,
            "at least 2 bytes for the NE signature",
            f"only {len(data) - e_lfanew} byte(s) available",
        )
    return DOSTable(
        magic=magic.decode("ascii"),
        e_lfanew=e_lfanew,
        file_size=len(data),
        sha256=file_sha256(data),
    )


def parse_ne_header(data: bytes, dos: DOSTable, path: str = "<input>") -> NEHeader:
    reader = Reader(data, path)
    offset = dos.e_lfanew
    signature = reader.bytes(offset, 2, "NE header.signature")
    if signature != b"NE":
        raise NEFormatError(
            "NE header.signature", offset, "NE signature", repr(signature)
        )
    if offset + NE_HEADER_SIZE > len(data):
        raise NEFormatError(
            "NE header",
            offset,
            f"64-byte NE header through 0x{offset + NE_HEADER_SIZE:x}",
            f"file ends at 0x{len(data):x}",
        )

    def u16(rel: int) -> int:
        return reader.u16(offset + rel, f"NE header field at +0x{rel:x}")

    def u32(rel: int) -> int:
        return reader.u32(offset + rel, f"NE header field at +0x{rel:x}")

    return NEHeader(
        signature="NE",
        file_offset=offset,
        linker_version=reader.u8(offset + 2, "NE header.linker_version"),
        linker_revision=reader.u8(offset + 3, "NE header.linker_revision"),
        entry_table_offset=u16(0x04),
        entry_table_length=u16(0x06),
        crc32=u32(0x08),
        flags=u16(0x0C),
        automatic_data_segment=u16(0x0E),
        heap_size=u16(0x10),
        stack_size=u16(0x12),
        cs_ip_raw=u32(0x14),
        ss_sp_raw=u32(0x18),
        segment_count=u16(0x1C),
        module_reference_count=u16(0x1E),
        nonresident_names_size=u16(0x20),
        segment_table_offset=u16(0x22),
        resource_table_offset=u16(0x24),
        resident_names_offset=u16(0x26),
        module_reference_table_offset=u16(0x28),
        imported_names_offset=u16(0x2A),
        nonresident_names_offset=u32(0x2C),
        movable_entry_count=u16(0x30),
        sector_alignment_shift=u16(0x32),
        resource_segment_count=u16(0x34),
        target_os=reader.u8(offset + 0x36, "NE header.target_os"),
        additional_flags=reader.u8(offset + 0x37, "NE header.additional_flags"),
        return_thunks_offset=u16(0x38),
        segment_ref_thunks_offset=u16(0x3A),
        swap_area_size=u16(0x3C),
        expected_windows_version_raw=u16(0x3E),
    )


# ---------------------------------------------------------------------------
# Segments
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Segment:
    number: int
    sector_offset: int
    file_offset: int | None
    length_raw: int
    length: int
    flags: int
    min_alloc_raw: int
    min_alloc: int

    def as_dict(self) -> dict[str, Any]:
        return {
            "number": self.number,
            "sector_offset": self.sector_offset,
            "file_offset": self.file_offset,
            "file_offset_hex": _hex(self.file_offset),
            "length_raw": self.length_raw,
            "length": self.length,
            "flags": self.flags,
            "flags_hex": _hex(self.flags),
            "flag_names": _flag_names(self.flags, SEGMENT_FLAG_NAMES),
            "min_alloc_raw": self.min_alloc_raw,
            "min_alloc": self.min_alloc,
            "address_space": "segment-relative (segment number + offset)",
        }


def parse_segments(
    data: bytes, ne: NEHeader, reader: Reader
) -> list[Segment]:
    shift = ne.sector_alignment_shift
    if shift > 31:
        raise NEFormatError(
            "NE header.sector_alignment_shift",
            ne.file_offset + 0x32,
            "shift count at most 31 (no overflowing sector arithmetic)",
            str(shift),
        )
    # A zero file-alignment shift is documented as equivalent to 9.
    effective_shift = 9 if shift == 0 else shift
    table_offset = ne.file_offset_of(ne.segment_table_offset)
    table_length = ne.segment_count * SEGMENT_ENTRY_SIZE
    if table_offset + table_length > len(data):
        raise NEFormatError(
            "segment table",
            table_offset,
            f"{ne.segment_count} 8-byte segment table entries "
            f"({table_length} bytes)",
            f"file ends at 0x{len(data):x}",
        )
    segments: list[Segment] = []
    for index in range(ne.segment_count):
        entry_offset = table_offset + index * SEGMENT_ENTRY_SIZE
        sector_offset = reader.u16(
            entry_offset, f"segment table[{index}].sector_offset"
        )
        length_raw = reader.u16(entry_offset + 2, f"segment table[{index}].length")
        flags = reader.u16(entry_offset + 4, f"segment table[{index}].flags")
        min_alloc_raw = reader.u16(
            entry_offset + 6, f"segment table[{index}].min_alloc"
        )
        if sector_offset == 0:
            file_offset = None
            length = 0
        else:
            file_offset = sector_offset << effective_shift
            length = length_raw or 0x10000
            if file_offset + length > len(data):
                raise NEFormatError(
                    f"segment table[{index}] (segment {index + 1})",
                    entry_offset,
                    f"segment data 0x{file_offset:x}..0x{file_offset + length:x} "
                    f"inside the {len(data)}-byte file",
                    "segment data extends past end of file",
                )
        segments.append(
            Segment(
                number=index + 1,
                sector_offset=sector_offset,
                file_offset=file_offset,
                length_raw=length_raw,
                length=length,
                flags=flags,
                min_alloc_raw=min_alloc_raw,
                min_alloc=min_alloc_raw or 0x10000,
            )
        )
    return segments


# ---------------------------------------------------------------------------
# Resources
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Resource:
    type_id: int
    type_ordinal: int | None
    type_name: str
    sector_offset: int
    sector_length: int
    file_offset: int
    byte_length: int
    flags: int
    id_raw: int
    id_name: str | None
    handle: int
    usage: int

    def identity(self) -> dict[str, Any]:
        return {
            "type_id": self.type_id,
            "type_id_hex": _hex(self.type_id),
            "type_ordinal": self.type_ordinal,
            "type_name": self.type_name,
            "id": _resource_id_dict(self.id_raw, self.id_name),
        }

    def as_dict(self) -> dict[str, Any]:
        result = self.identity()
        result.update(
            {
                "sector_offset": self.sector_offset,
                "sector_length": self.sector_length,
                "file_offset": self.file_offset,
                "file_offset_hex": _hex(self.file_offset),
                "byte_length": self.byte_length,
                "flags": self.flags,
                "flags_hex": _hex(self.flags),
                "flag_names": _flag_names(self.flags, RESOURCE_FLAG_NAMES),
                "handle": self.handle,
                "usage": self.usage,
                "reserved": {"handle": self.handle, "usage": self.usage},
                "length_units": "alignment sectors (sector_length << alignment shift)",
            }
        )
        return result


@dataclass
class ResourceTable:
    alignment_shift: int
    alignment_unit: int
    types: list[dict[str, Any]] = field(default_factory=list)
    resources: list[Resource] = field(default_factory=list)
    names_start: int = 0
    end_offset: int = 0

    @property
    def total_count(self) -> int:
        return len(self.resources)

    def counts_by_type(self) -> dict[str, int]:
        counts: dict[str, int] = {}
        for entry in self.types:
            counts[entry["type_name"]] = entry["count"]
        return counts


def parse_resources(
    data: bytes,
    ne: NEHeader,
    reader: Reader,
    resident_names_file_offset: int | None,
) -> ResourceTable:
    if ne.resource_table_offset == 0:
        return ResourceTable(alignment_shift=0, alignment_unit=1)
    table_offset = ne.file_offset_of(ne.resource_table_offset)
    if table_offset + 2 > len(data):
        raise NEFormatError(
            "resource table",
            table_offset,
            "2-byte resource alignment shift",
            f"file ends at 0x{len(data):x}",
        )
    alignment_shift = reader.u16(table_offset, "resource table.alignment_shift")
    if alignment_shift == 0:
        raise NEFormatError(
            "resource table.alignment_shift",
            table_offset,
            "non-zero alignment shift",
            "0",
            detail="zero alignment would make resource sectors 1 byte",
        )
    if alignment_shift >= 32:
        raise NEFormatError(
            "resource table.alignment_shift",
            table_offset,
            "alignment shift below 32 to avoid overflow",
            str(alignment_shift),
        )
    alignment_unit = 1 << alignment_shift

    # Resource names live inside the resource table, before the resident-name
    # table.  Use the nearest following known table as a conservative limit.
    if (
        resident_names_file_offset is not None
        and table_offset < resident_names_file_offset <= len(data)
    ):
        names_limit = resident_names_file_offset
    else:
        names_limit = len(data)

    table = ResourceTable(
        alignment_shift=alignment_shift, alignment_unit=alignment_unit
    )
    cursor = table_offset + 2
    types: list[dict[str, Any]] = []
    seen_terminator = False
    while True:
        if cursor + 2 > names_limit:
            raise NEFormatError(
                "resource table.TYPEINFO",
                cursor,
                "2-byte type id (zero terminates the type list) inside "
                f"0x{table_offset:x}..0x{names_limit:x}",
                "type list runs past the resource table",
            )
        type_id = reader.u16(cursor, "resource table.TYPEINFO.type_id")
        cursor += 2
        if type_id == 0:
            seen_terminator = True
            break
        if cursor + 6 > names_limit:
            raise NEFormatError(
                "resource table.TYPEINFO",
                cursor - 2,
                "type id plus count and reserved dword",
                "truncated TYPEINFO record",
            )
        count = reader.u16(cursor, "resource table.TYPEINFO.count")
        reserved = reader.u32(cursor + 2, "resource table.TYPEINFO.reserved")
        cursor += 6

        if type_id & 0x8000:
            type_ordinal: int | None = type_id & 0x7FFF
            type_string = None
        else:
            type_ordinal = None
            name_offset = table_offset + type_id
            if name_offset < table_offset or name_offset >= names_limit:
                raise NEFormatError(
                    "resource table.TYPEINFO.type_name",
                    cursor - 6,
                    f"type-name offset 0x{type_id:x} inside the resource table",
                    f"resolved file offset 0x{name_offset:x}",
                )
            type_string, _ = reader.pascal(
                name_offset,
                f"resource type name (offset 0x{type_id:x})",
                limit=names_limit,
            )
        type_name = _resource_type_name(type_id, type_string)

        if cursor + count * NAMEINFO_SIZE > names_limit:
            raise NEFormatError(
                f"resource table type {type_name}",
                cursor,
                f"{count} 12-byte NameInfo records inside the resource table",
                "resource entries run past the resource table",
            )

        type_resources: list[Resource] = []
        for index in range(count):
            entry_offset = cursor + index * NAMEINFO_SIZE
            (
                sector_offset,
                sector_length,
                flags,
                id_raw,
                handle,
                usage,
            ) = struct.unpack_from("<HHHHHH", data, entry_offset)
            byte_length = sector_length << alignment_shift
            file_offset = sector_offset << alignment_shift
            if file_offset > len(data) or file_offset + byte_length > len(data):
                raise NEFormatError(
                    f"resource table type {type_name} entry {index}",
                    entry_offset,
                    f"resource data 0x{file_offset:x}..0x{file_offset + byte_length:x} "
                    f"inside the {len(data)}-byte file",
                    "resource data extends past end of file "
                    "(possible oversized length or bad sector offset)",
                )
            if id_raw & 0x8000:
                id_name = None
            else:
                name_offset = table_offset + id_raw
                if name_offset < table_offset or name_offset >= names_limit:
                    raise NEFormatError(
                        f"resource table type {type_name} entry {index}.id",
                        entry_offset + 6,
                        f"resource-name offset 0x{id_raw:x} inside the resource table",
                        f"resolved file offset 0x{name_offset:x}",
                    )
                id_name, _ = reader.pascal(
                    name_offset,
                    f"resource name (type {type_name}, offset 0x{id_raw:x})",
                    limit=names_limit,
                )
            resource = Resource(
                type_id=type_id,
                type_ordinal=type_ordinal,
                type_name=type_name,
                sector_offset=sector_offset,
                sector_length=sector_length,
                file_offset=file_offset,
                byte_length=byte_length,
                flags=flags,
                id_raw=id_raw,
                id_name=id_name,
                handle=handle,
                usage=usage,
            )
            type_resources.append(resource)
            table.resources.append(resource)

        types.append(
            {
                "type_id": type_id,
                "type_id_hex": _hex(type_id),
                "type_ordinal": type_ordinal,
                "type_name": type_name,
                "count": count,
                "reserved": reserved,
                "resources": [resource.as_dict() for resource in type_resources],
            }
        )
        cursor += count * NAMEINFO_SIZE

    if not seen_terminator:
        raise NEFormatError(
            "resource table.TYPEINFO", cursor, "zero type terminator", "none found"
        )
    table.types = types
    table.names_start = cursor
    table.end_offset = names_limit
    return table


# ---------------------------------------------------------------------------
# Names and imports
# ---------------------------------------------------------------------------


@dataclass
class NameTable:
    entries: list[dict[str, Any]]
    module_name: str | None = None
    description: str | None = None

    def as_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"entries": self.entries}
        if self.module_name is not None:
            result["module_name"] = self.module_name
        if self.description is not None:
            result["description"] = self.description
        return result


def parse_resident_names(
    data: bytes,
    ne: NEHeader,
    reader: Reader,
    module_ref_file_offset: int | None,
) -> NameTable:
    start = ne.file_offset_of(ne.resident_names_offset)
    if start >= len(data):
        raise NEFormatError(
            "resident-name table",
            start,
            f"table start inside the {len(data)}-byte file",
            "offset past end of file",
        )
    if (
        module_ref_file_offset is not None
        and start < module_ref_file_offset <= len(data)
    ):
        limit = module_ref_file_offset
    else:
        limit = len(data)
    entries: list[dict[str, Any]] = []
    cursor = start
    while cursor < limit:
        if reader.data[cursor] == 0:
            break
        name, after_name = reader.pascal(
            cursor, f"resident-name entry {len(entries)}", limit=limit
        )
        ordinal = reader.u16(after_name, f"resident-name entry {len(entries)}.ordinal")
        entries.append({"name": name, "ordinal": ordinal})
        cursor = after_name + 2
    table = NameTable(entries=entries)
    if entries:
        table.module_name = entries[0]["name"]
    return table


def parse_nonresident_names(
    data: bytes, ne: NEHeader, reader: Reader
) -> NameTable:
    start = ne.nonresident_names_offset
    declared_size = ne.nonresident_names_size
    if start > len(data):
        raise NEFormatError(
            "nonresident-name table",
            start,
            f"table start inside the {len(data)}-byte file",
            "offset past end of file",
        )
    if start + declared_size > len(data):
        raise NEFormatError(
            "nonresident-name table",
            start,
            f"declared {declared_size}-byte table",
            f"file ends at 0x{len(data):x}",
        )
    limit = start + declared_size
    entries: list[dict[str, Any]] = []
    cursor = start
    while cursor < limit:
        if reader.data[cursor] == 0:
            break
        name, after_name = reader.pascal(
            cursor, f"nonresident-name entry {len(entries)}", limit=limit
        )
        if after_name + 2 > limit:
            raise NEFormatError(
                "nonresident-name table",
                after_name,
                "16-bit ordinal after nonresident-name string",
                "ordinal runs past declared table size",
            )
        ordinal = reader.u16(
            after_name, f"nonresident-name entry {len(entries)}.ordinal"
        )
        entries.append({"name": name, "ordinal": ordinal})
        cursor = after_name + 2
    table = NameTable(entries=entries)
    if entries:
        table.description = entries[0]["name"]
    return table


@dataclass
class ImportTable:
    imported_names_start: int
    imported_names_end: int
    imports: list[dict[str, Any]]
    module_references: list[dict[str, Any]]

    def as_dict(self) -> dict[str, Any]:
        return {
            "imported_names_range": {
                "file_start": self.imported_names_start,
                "file_start_hex": _hex(self.imported_names_start),
                "file_end": self.imported_names_end,
                "file_end_hex": _hex(self.imported_names_end),
            },
            "imports": self.imports,
            "module_references": self.module_references,
        }


def parse_imports(data: bytes, ne: NEHeader, reader: Reader) -> ImportTable:
    module_table = ne.file_offset_of(ne.module_reference_table_offset)
    module_table_bytes = ne.module_reference_count * 2
    if module_table + module_table_bytes > len(data):
        raise NEFormatError(
            "module-reference table",
            module_table,
            f"{ne.module_reference_count} 16-bit offsets",
            f"file ends at 0x{len(data):x}",
        )
    imports_start = ne.file_offset_of(ne.imported_names_offset)
    if imports_start >= len(data):
        raise NEFormatError(
            "imported-name table",
            imports_start,
            f"table start inside the {len(data)}-byte file",
            "offset past end of file",
        )
    if (
        imports_start < ne.nonresident_names_offset <= len(data)
        and ne.nonresident_names_offset > imports_start
    ):
        imports_end = ne.nonresident_names_offset
    else:
        imports_end = len(data)

    module_references: list[dict[str, Any]] = []
    referenced_names: list[dict[str, Any]] = []
    seen_offsets: set[int] = set()
    for index in range(ne.module_reference_count):
        ref_offset_position = module_table + index * 2
        name_offset = reader.u16(
            ref_offset_position, f"module-reference table[{index}]"
        )
        name_file_offset = imports_start + name_offset
        if name_file_offset < imports_start or name_file_offset >= imports_end:
            raise NEFormatError(
                f"module-reference table[{index}]",
                ref_offset_position,
                f"imported-name offset inside 0x{imports_start:x}..0x{imports_end:x}",
                f"0x{name_offset:x} resolves to file offset 0x{name_file_offset:x}",
            )
        name, after = reader.pascal(
            name_file_offset,
            f"imported module name (module reference {index})",
            limit=imports_end,
        )
        module_references.append(
            {
                "index": index,
                "name_offset": name_offset,
                "name_file_offset": name_file_offset,
                "module_name": name,
            }
        )
        if name_offset not in seen_offsets:
            seen_offsets.add(name_offset)
            referenced_names.append(
                {
                    "name_offset": name_offset,
                    "name_file_offset": name_file_offset,
                    "name": name,
                }
            )
        _ = after

    return ImportTable(
        imported_names_start=imports_start,
        imported_names_end=imports_end,
        imports=referenced_names,
        module_references=module_references,
    )


# ---------------------------------------------------------------------------
# Entry table
# ---------------------------------------------------------------------------


@dataclass
class EntryTable:
    summary: dict[str, Any]
    bundles: list[dict[str, Any]]

    def as_dict(self) -> dict[str, Any]:
        return {"summary": self.summary, "bundles": self.bundles}


def parse_entry_table(data: bytes, ne: NEHeader, reader: Reader) -> EntryTable:
    if ne.entry_table_length == 0:
        return EntryTable(
            summary={
                "bundle_count": 0,
                "entry_count": 0,
                "movable_entry_count_header": ne.movable_entry_count,
                "exported_count": 0,
                "shared_count": 0,
                "constant_count": 0,
                "movable_count": 0,
                "fixed_count": 0,
                "truncated": False,
            },
            bundles=[],
        )
    start = ne.file_offset_of(ne.entry_table_offset)
    end = start + ne.entry_table_length
    if end > len(data):
        raise NEFormatError(
            "entry table",
            start,
            f"declared {ne.entry_table_length}-byte entry table",
            f"file ends at 0x{len(data):x}",
        )
    cursor = start
    ordinal = 1
    bundles: list[dict[str, Any]] = []
    exported = 0
    shared = 0
    constants = 0
    movable_entries = 0
    fixed_entries = 0
    while cursor < end:
        entry_count = reader.u8(cursor, "entry table bundle.count")
        cursor += 1
        if entry_count == 0:
            break
        if cursor >= end:
            raise NEFormatError(
                "entry table",
                cursor,
                "bundle segment indicator",
                "entry table ends after a non-zero bundle count",
            )
        segment_indicator = reader.u8(cursor, "entry table bundle.segment")
        cursor += 1
        if segment_indicator == 0xFF:
            kind = "movable"
            entry_size = 6
        elif segment_indicator == 0xFE:
            kind = "constant"
            entry_size = 6
        else:
            kind = "fixed"
            entry_size = 3
        if cursor + entry_count * entry_size > end:
            raise NEFormatError(
                "entry table",
                cursor,
                f"{entry_count} {entry_size}-byte {kind} entries inside the "
                f"declared table",
                "bundle runs past the entry table",
            )
        entries: list[dict[str, Any]] = []
        for _ in range(entry_count):
            flags = reader.u8(cursor, "entry table entry.flags")
            if kind == "fixed":
                offset = reader.u16(cursor + 1, "entry table entry.offset")
                segment = segment_indicator
                record = {
                    "ordinal": ordinal,
                    "flags": flags,
                    "flags_hex": _hex(flags),
                    "exported": bool(flags & 0x01),
                    "shared_data": bool(flags & 0x02),
                    "ring_transition_words": (flags >> 3) & 0x1F,
                    "kind": kind,
                    "segment": segment,
                    "offset": offset,
                    "address": f"segment {segment}:offset 0x{offset:x}",
                }
                cursor += 3
            else:
                int3 = reader.u16(cursor + 1, "entry table entry.int3")
                segment = reader.u8(cursor + 3, "entry table entry.segment")
                offset = reader.u16(cursor + 4, "entry table entry.offset")
                record = {
                    "ordinal": ordinal,
                    "flags": flags,
                    "flags_hex": _hex(flags),
                    "exported": bool(flags & 0x01),
                    "shared_data": bool(flags & 0x02),
                    "ring_transition_words": (flags >> 3) & 0x1F,
                    "kind": kind,
                    "int3": int3,
                    "segment": segment,
                    "offset": offset,
                    "address": f"segment {segment}:offset 0x{offset:x}",
                }
                cursor += 6
            entries.append(record)
            ordinal += 1
            if record["exported"]:
                exported += 1
            if record["shared_data"]:
                shared += 1
            if kind == "constant":
                constants += 1
            elif kind == "movable":
                movable_entries += 1
            else:
                fixed_entries += 1
        bundles.append(
            {
                "index": len(bundles),
                "count": entry_count,
                "segment_indicator": segment_indicator,
                "segment_indicator_hex": _hex(segment_indicator),
                "kind": kind,
                "entries": entries,
            }
        )
    summary = {
        "bundle_count": len(bundles),
        "entry_count": ordinal - 1,
        "movable_entry_count_header": ne.movable_entry_count,
        "exported_count": exported,
        "shared_count": shared,
        "constant_count": constants,
        "movable_count": movable_entries,
        "fixed_count": fixed_entries,
        "truncated": cursor > end,
        "note": "entry points are reported, not disassembled",
    }
    return EntryTable(summary=summary, bundles=bundles)


# ---------------------------------------------------------------------------
# Dialog templates
# ---------------------------------------------------------------------------


@dataclass
class DialogParseResult:
    template: dict[str, Any] | None
    error: str | None
    error_offset: int | None

    def as_dict(self) -> dict[str, Any]:
        return {"template": self.template, "parse_error": self.error}


def _read_dialog_string(
    reader: Reader, offset: int, limit: int, what: str
) -> tuple[str, int]:
    """Read a NUL string or a 0xFF + ordinal reference."""

    if offset >= limit:
        raise NEFormatError(
            "dialog template",
            offset,
            f"{what} inside the dialog resource",
            "offset at or past resource end",
        )
    if reader.data[offset] == 0xFF:
        if offset + 3 > limit:
            raise NEFormatError(
                "dialog template",
                offset,
                f"0xFF ordinal {what} (3 bytes)",
                "ordinal runs past resource end",
            )
        ordinal = reader.u16(offset + 1, f"dialog {what} ordinal")
        return f"#{ordinal}", offset + 3
    return reader.cstring(offset, what, limit=limit, max_length=1024)


def parse_dialog_template(
    reader: Reader, start: int, end: int
) -> DialogParseResult:
    """Best-effort bounded parse of a Windows 1.0/2.0 dialog template."""

    try:
        if start + 13 > end:
            raise NEFormatError(
                "dialog template",
                start,
                "13-byte classic dialog header (style, item count, x, y, cx, cy)",
                "resource too short",
            )
        view = Reader(reader.data, reader.path, start, end)
        style = view.u32(start, "dialog template.style")
        item_count = view.u8(start + 4, "dialog template.item_count")
        x = view.u16(start + 5, "dialog template.x")
        y = view.u16(start + 7, "dialog template.y")
        cx = view.u16(start + 9, "dialog template.cx")
        cy = view.u16(start + 11, "dialog template.cy")
        cursor = start + 13
        menu, cursor = _read_dialog_string(view, cursor, end, "menu")
        dialog_class, cursor = _read_dialog_string(view, cursor, end, "class")
        title, cursor = _read_dialog_string(view, cursor, end, "title")
        font = None
        if style & DIALOG_STYLE_DS_SETFONT:
            if cursor + 2 > end:
                raise NEFormatError(
                    "dialog template",
                    cursor,
                    "DS_SETFONT point size",
                    "resource ends before font size",
                )
            point_size = view.u16(cursor, "dialog template.font_size")
            cursor += 2
            face, cursor = view.cstring(
                cursor, "dialog font face", limit=end, max_length=256
            )
            font = {"point_size": point_size, "face": face}

        controls: list[dict[str, Any]] = []
        for index in range(item_count):
            if cursor + 14 > end:
                raise NEFormatError(
                    f"dialog template.control[{index}]",
                    cursor,
                    "14-byte dialog item header",
                    "control runs past resource end",
                )
            ix = view.u16(cursor, f"dialog control[{index}].x")
            iy = view.u16(cursor + 2, f"dialog control[{index}].y")
            icx = view.u16(cursor + 4, f"dialog control[{index}].cx")
            icy = view.u16(cursor + 6, f"dialog control[{index}].cy")
            control_id = view.u16(cursor + 8, f"dialog control[{index}].id")
            control_style = view.u32(
                cursor + 10, f"dialog control[{index}].style"
            )
            cursor += 14
            if cursor >= end:
                raise NEFormatError(
                    f"dialog template.control[{index}].class",
                    cursor,
                    "control class",
                    "resource ends before class",
                )
            class_byte = view.u8(cursor, f"dialog control[{index}].class")
            if 0x80 <= class_byte <= 0x85:
                class_name = {
                    0x80: "button",
                    0x81: "edit",
                    0x82: "static",
                    0x83: "listbox",
                    0x84: "scrollbar",
                    0x85: "combobox",
                }[class_byte]
                cursor += 1
            else:
                class_name, cursor = view.cstring(
                    cursor,
                    f"dialog control[{index}].class",
                    limit=end,
                    max_length=256,
                )
            text, cursor = _read_dialog_string(
                view, cursor, end, f"control[{index}].text"
            )
            if cursor >= end:
                raise NEFormatError(
                    f"dialog template.control[{index}].extra",
                    cursor,
                    "extra-data byte count",
                    "resource ends before extra-data count",
                )
            extra_length = view.u8(
                cursor, f"dialog control[{index}].extra_length"
            )
            cursor += 1
            if cursor + extra_length > end:
                raise NEFormatError(
                    f"dialog template.control[{index}].extra",
                    cursor,
                    f"{extra_length} extra-data byte(s)",
                    "extra data runs past resource end",
                )
            cursor += extra_length
            controls.append(
                {
                    "index": index,
                    "id": control_id,
                    "class": class_name,
                    "text": text,
                    "style": control_style,
                    "style_hex": _hex(control_style),
                    "x": ix,
                    "y": iy,
                    "cx": icx,
                    "cy": icy,
                }
            )
        captions = [title] + [control["text"] for control in controls]
        return DialogParseResult(
            template={
                "style": style,
                "style_hex": _hex(style),
                "item_count": item_count,
                "x": x,
                "y": y,
                "cx": cx,
                "cy": cy,
                "menu": menu,
                "class": dialog_class,
                "title": title,
                "font": font,
                "controls": controls,
                "captions": captions,
                "decoded_bytes": cursor - start,
                "resource_bytes": end - start,
            },
            error=None,
            error_offset=None,
        )
    except NEFormatError as error:
        return DialogParseResult(
            template=None, error=str(error), error_offset=error.offset
        )


# ---------------------------------------------------------------------------
# String references
# ---------------------------------------------------------------------------


DEFAULT_NEEDLES = (
    "Turindrones",
    "Turindrone",
    "Race",
    "custom",
    "Primary Racial Trait",
)

MAX_MATCHES_PER_NEEDLE = 25
MAX_WINDOW_CHARS = 160


def _printable_window(payload: bytes, match_offset: int, needle_length: int) -> str:
    start = max(0, match_offset - 48)
    length = min(MAX_WINDOW_CHARS, len(payload) - start)
    raw = payload[start : start + length]
    text = raw.decode("latin-1")
    printable = "".join(ch if 32 <= ord(ch) < 127 else "." for ch in text)
    return printable[:MAX_WINDOW_CHARS]


def build_string_references(
    data: bytes,
    segments: list[Segment],
    resources: list[Resource],
    needles: Iterable[str] = DEFAULT_NEEDLES,
) -> dict[str, Any]:
    needle_list = list(needles)
    matches: list[dict[str, Any]] = []
    caps: dict[str, dict[str, Any]] = {}
    for needle in needle_list:
        encoded = needle.encode("ascii")
        lowered = encoded.lower()
        found_total = 0
        kept = 0
        for segment in segments:
            if segment.file_offset is None or segment.length == 0:
                continue
            payload = data[segment.file_offset : segment.file_offset + segment.length]
            lowered_payload = payload.lower()
            cursor = 0
            while True:
                index = lowered_payload.find(lowered, cursor)
                if index < 0:
                    break
                found_total += 1
                if kept < MAX_MATCHES_PER_NEEDLE:
                    matches.append(
                        {
                            "needle": needle,
                            "location_kind": "segment",
                            "segment_number": segment.number,
                            "segment_relative_offset": index,
                            "segment_relative_offset_hex": _hex(index),
                            "file_offset": segment.file_offset + index,
                            "file_offset_hex": _hex(segment.file_offset + index),
                            "window": _printable_window(payload, index, len(encoded)),
                            "window_chars": MAX_WINDOW_CHARS,
                        }
                    )
                    kept += 1
                cursor = index + 1
        for resource in resources:
            if resource.byte_length == 0:
                continue
            payload = data[
                resource.file_offset : resource.file_offset + resource.byte_length
            ]
            lowered_payload = payload.lower()
            cursor = 0
            while True:
                index = lowered_payload.find(lowered, cursor)
                if index < 0:
                    break
                found_total += 1
                if kept < MAX_MATCHES_PER_NEEDLE:
                    matches.append(
                        {
                            "needle": needle,
                            "location_kind": "resource",
                            "resource_type": resource.type_name,
                            "resource": _resource_id_dict(
                                resource.id_raw, resource.id_name
                            ),
                            "file_offset": resource.file_offset + index,
                            "file_offset_hex": _hex(resource.file_offset + index),
                            "resource_relative_offset": index,
                            "window": _printable_window(payload, index, len(encoded)),
                            "window_chars": MAX_WINDOW_CHARS,
                        }
                    )
                    kept += 1
                cursor = index + 1
        caps[needle] = {
            "matches_found": found_total,
            "matches_reported": kept,
            "truncated": found_total > kept,
            "match_cap": MAX_MATCHES_PER_NEEDLE,
        }

    matches.sort(
        key=lambda item: (
            needle_list.index(item["needle"]),
            item["file_offset"],
            item["location_kind"],
        )
    )
    return {
        "needles": needle_list,
        "matches": matches,
        "caps": caps,
        "window_limit_chars": MAX_WINDOW_CHARS,
        "note": "Selected ASCII evidence leads only; absence or presence does not prove runtime behaviour.",
    }


# ---------------------------------------------------------------------------
# Dialog inventory and race-wizard candidates
# ---------------------------------------------------------------------------


RACE_KEYWORDS = {
    "race": ("race", "races", "racial"),
    "trait": ("trait", "traits", "primary racial trait", "lesser trait"),
    "custom": ("custom", "customize"),
    "wizard": ("wizard",),
}

# Names listed by the project plan/help index.  Matching them in dialog text is
# still only a lead: it does not prove which dialog configures a trait.
TRAIT_NAME_HINTS = (
    "Hyper-Expansion",
    "Super-Stealth",
    "War Monger",
    "Claim Adjuster",
    "Inner-Strength",
    "Space Demolition",
    "Packet Physics",
    "Inter-stellar Traveler",
    "Alternate Reality",
    "Jack of All Trades",
    "Improved Fuel Efficiency",
    "Total Terraforming",
    "Advanced Remote Mining",
    "Improved Starbases",
    "Generalized Research",
    "Ultimate Recycling",
    "Mineral Alchemy",
    "No Ramscoop Engines",
    "Cheap Engines",
    "Only Basic Remote Mining",
    "No Advanced Scanners",
    "Low Starting Population",
    "Bleeding Edge Technology",
    "Regenerating Shields",
)

DIALOG_NAVIGATION_TEXTS = ("< &back", "&next >", "&finish")

# Wording that appears alongside race/trait choices in the supplied dialogs.
# These are search hints, not claims about which control changes a rule.
RACE_TRAIT_PHRASE_HINTS = (
    "leftover advantage points",
    "immune to",
    "factories cost",
    "costs 75% extra",
    "costs standard amount",
    "costs 50% less",
)


def _keyword_hits(text: str) -> list[str]:
    lowered = text.lower()
    hits: list[str] = []
    for label, words in RACE_KEYWORDS.items():
        for word in words:
            if word in lowered:
                hits.append(label)
                break
    return hits


def _trait_name_hits(text: str) -> list[str]:
    lowered = text.lower()
    return [name for name in TRAIT_NAME_HINTS if name.lower() in lowered]


def build_dialog_inventory(
    data: bytes, resources: list[Resource]
) -> dict[str, Any]:
    dialog_resources = [
        resource for resource in resources if resource.type_id & 0x8000 and (
            resource.type_id & 0x7FFF
        ) == 5
    ]
    entries: list[dict[str, Any]] = []
    candidates: list[dict[str, Any]] = []
    captions_total = 0
    parse_errors = 0
    for resource in dialog_resources:
        reader = Reader(
            data,
            f"dialog resource {resource.id_name or '#' + str(resource.id_raw & 0x7FFF)}",
            resource.file_offset,
            resource.file_offset + resource.byte_length,
        )
        result = parse_dialog_template(
            reader, resource.file_offset, resource.file_offset + resource.byte_length
        )
        identity = resource.identity()
        entry: dict[str, Any] = {
            "resource": identity,
            "file_offset": resource.file_offset,
            "file_offset_hex": _hex(resource.file_offset),
            "byte_length": resource.byte_length,
            "sector_length": resource.sector_length,
        }
        reasons: list[str] = []
        if result.template is None:
            parse_errors += 1
            entry["parse_error"] = result.error
            entry["parse_error_offset"] = result.error_offset
            entry["captions"] = []
            entry["title"] = None
            entry["item_count"] = None
        else:
            template = result.template
            captions_total += len(template["captions"])
            entry.update(
                {
                    "parse_error": None,
                    "title": template["title"],
                    "item_count": template["item_count"],
                    "font": template["font"],
                    "controls": template["controls"],
                    "captions": template["captions"],
                    "decoded_bytes": template["decoded_bytes"],
                    "style": template["style"],
                    "style_hex": template["style_hex"],
                }
            )
            searchable = [template["title"], *template["captions"]]
            phrase_hits: list[str] = []
            for text in searchable:
                if not text:
                    continue
                keyword_hits = _keyword_hits(text)
                if keyword_hits:
                    reasons.append(f"dialog text contains race/trait keyword: {text!r}")
                trait_hits = _trait_name_hits(text)
                if trait_hits:
                    reasons.append(
                        "dialog text contains trait name(s): "
                        + ", ".join(trait_hits)
                    )
                lowered = text.lower()
                for phrase in RACE_TRAIT_PHRASE_HINTS:
                    if phrase in lowered and phrase not in phrase_hits:
                        phrase_hits.append(phrase)
            if phrase_hits:
                reasons.append(
                    "dialog text contains race/trait wording: "
                    + ", ".join(repr(phrase) for phrase in phrase_hits)
                )
            navigation = [
                control["text"]
                for control in template["controls"]
                if control.get("text", "").strip().lower() in DIALOG_NAVIGATION_TEXTS
            ]
            if navigation and reasons:
                reasons.append(
                    "dialog has wizard-style navigation controls: "
                    + ", ".join(repr(text) for text in navigation)
                )
        name_text = identity["id"].get("name") or identity["id"].get("display", "")
        name_hits = _keyword_hits(str(name_text))
        if name_hits:
            reasons.append(f"resource name/id contains keyword(s): {', '.join(name_hits)}")
        if reasons:
            candidates.append(
                {
                    "resource": identity,
                    "file_offset": resource.file_offset,
                    "reasons": reasons,
                    "confidence": "lead_only",
                    "limitation": "Presence and captions do not establish runtime behaviour.",
                }
            )
        entries.append(entry)

    candidate_strings: list[dict[str, Any]] = []
    return {
        "dialog_type": "DIALOG (resource type ordinal 5)",
        "count": len(dialog_resources),
        "parsed_count": len(dialog_resources) - parse_errors,
        "parse_error_count": parse_errors,
        "captions_total": captions_total,
        "resources": entries,
        "race_wizard_candidates": candidates,
        "candidate_strings": candidate_strings,
        "note": "Total dialog count is measured independently of any expected count.",
    }


# ---------------------------------------------------------------------------
# Full report
# ---------------------------------------------------------------------------


def build_report(
    data: bytes,
    *,
    input_path: str = "<memory>",
    help_file: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Parse ``data`` into a deterministic, JSON-serialisable evidence report."""

    path = input_path
    dos = parse_dos_header(data, path)
    reader = Reader(data, path)
    ne = parse_ne_header(data, dos, path)

    segments = parse_segments(data, ne, reader)
    resident_names_file_offset = ne.file_offset_of(ne.resident_names_offset)
    resources_table = parse_resources(
        data, ne, reader, resident_names_file_offset
    )
    module_ref_file_offset = ne.file_offset_of(ne.module_reference_table_offset)
    resident_names = parse_resident_names(
        data, ne, reader, module_ref_file_offset
    )
    nonresident_names = parse_nonresident_names(data, ne, reader)
    imports = parse_imports(data, ne, reader)
    entry_table = parse_entry_table(data, ne, reader)

    dialogs = build_dialog_inventory(data, resources_table.resources)
    strings = build_string_references(
        data, segments, resources_table.resources, DEFAULT_NEEDLES
    )

    # Candidate strings: only selected scan windows that themselves contain a
    # race/trait/custom keyword.  These remain leads, never behaviour claims.
    candidate_strings: list[dict[str, Any]] = []
    for match in strings["matches"]:
        hits = _keyword_hits(match["window"])
        if hits:
            candidate_strings.append(
                {
                    "needle": match["needle"],
                    "matched_keywords": hits,
                    "location_kind": match["location_kind"],
                    "segment_number": match.get("segment_number"),
                    "resource_type": match.get("resource_type"),
                    "resource": match.get("resource"),
                    "file_offset": match["file_offset"],
                    "file_offset_hex": match["file_offset_hex"],
                    "window": match["window"],
                    "confidence": "lead_only",
                }
            )
    # Cap candidate strings so the report never becomes a bulk text dump.
    strings["candidate_cap"] = 50
    strings["candidate_truncated"] = len(candidate_strings) > 50
    dialogs["candidate_strings"] = candidate_strings[:50]

    report: dict[str, Any] = {
        "schema_version": 1,
        "tool": "tools/analysis/inventory.py",
        "input": {
            "path": input_path,
            "sha256": dos.sha256,
            "size": len(data),
            "detected_format": "16-bit Windows NE (New Executable)",
        },
        "dos_header": dos.as_dict(),
        "ne_header": ne.as_dict(),
        "segments": {
            "count": len(segments),
            "entries": [segment.as_dict() for segment in segments],
            "note": "Segments use a 1-based number and segment-relative offsets; file offsets are separate.",
        },
        "resources": {
            "alignment_shift": resources_table.alignment_shift,
            "alignment_unit": resources_table.alignment_unit,
            "total_count": resources_table.total_count,
            "counts_by_type": resources_table.counts_by_type(),
            "types": resources_table.types,
            "note": "Resource payloads are located and measured, never copied into the report.",
        },
        "resident_names": resident_names.as_dict(),
        "nonresident_names": nonresident_names.as_dict(),
        "imports": imports.as_dict(),
        "entry_table": entry_table.as_dict(),
        "dialogs": dialogs,
        "string_references": strings,
        "race_wizard_candidate_summary": {
            "dialog_count": len(dialogs["race_wizard_candidates"]),
            "string_count": len(candidate_strings),
            "confidence": "lead_only",
            "note": "Candidates are keyword matches for PR2 follow-up; they are not proof of race-wizard behaviour.",
        },
        "extraction_limitations": [
            "No code is executed and no disassembly is performed.",
            "Resource/segment payloads are not included in the report.",
            "Dialog templates are decoded best-effort; a malformed dialog is recorded without aborting the report.",
            "String matches are selected ASCII leads with bounded windows, not a string-table inventory.",
            "Resource length fields are treated as alignment sectors; this matches the supplied binary and synthetic fixtures.",
            "Resource type/id string offsets are resolved relative to the resource table, which is where the supplied binary stores those Pascal names (not the resident-name table).",
            "The NE format does not record original source semantics; names and captions are evidence, not behaviour.",
        ],
        "unknowns": [
            "Whether resource/dialog identifiers correspond to the race-wizard runtime path requires code cross-references.",
            "The meaning of custom resource types and their relationship to dialogs/strings is unresolved.",
            "No claim is made about original RNG, rules, or AI policy.",
        ],
    }
    if help_file is not None:
        report["help_file"] = help_file
    return report
