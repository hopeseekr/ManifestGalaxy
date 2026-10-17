#!/usr/bin/env python3
"""Bounded, reproducible inspector for 16/32-bit Windows Help (HLP) files.

The parser is deliberately strict about offsets, lengths and page links.  It
does not execute any original software.  Format notes used:

  https://github.com/joncampbell123/helpdeco/blob/master/doc/html/Windows%20Help%20File%20Format.htm

Evidence-specific corrections to those notes are recorded in the output:

* The supplied Stars! help file stores ``GenDate`` as a Unix ``time_t``
  (seconds since 1970-01-01 UTC), not seconds since 1980-01-01.  Decoding
  884558967 with the documented 1980 base gives 2008, while the Unix base
  gives 1998-01-11T22:49:27Z, matching the file's title and file date.
* For ``Minor > 16`` the TOPICPOS page space is 16 KiB per physical topic
  block, including uncompressed help files.  The physical block stride is
  still 2 KiB or 4 KiB and each block carries a 12-byte header.

Output contains metadata, decoded help text and offsets only.  It never dumps
bulk binary payloads.
"""

from __future__ import annotations

import argparse
import datetime as _datetime
import hashlib
import json
import os
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Optional

HLP_MAGIC = 0x00035F3F
BTREE_MAGIC = 0x293B
SYSTEM_MAGIC = 0x036C
UNIX_EPOCH = _datetime.datetime(1970, 1, 1, tzinfo=_datetime.timezone.utc)
MAX_ENTRIES = 1_000_000
MAX_LINKS = 1_000_000


class HLPFormatError(Exception):
    """A bounded-read or format error with file offset and action detail."""

    def __init__(self, path: os.PathLike[str] | str, offset: Optional[int], message: str):
        self.path = str(path)
        self.offset = offset
        self.message = message
        super().__init__(str(self))

    def __str__(self) -> str:
        if self.offset is None:
            return f"{self.path}: {self.message}"
        return f"{self.path}:0x{self.offset:x}: {self.message}"


class BoundedReader:
    """Small struct reader that never reads outside the input buffer."""

    def __init__(self, data: bytes, path: os.PathLike[str] | str):
        self.data = data
        self.path = str(path)

    def require(self, offset: int, length: int, what: str) -> None:
        if offset < 0 or length < 0:
            raise HLPFormatError(self.path, offset, f"invalid {what} range ({offset}, {length})")
        end = offset + length
        if end > len(self.data):
            raise HLPFormatError(
                self.path,
                offset,
                f"{what} needs {length} bytes but only {max(0, len(self.data) - offset)} remain",
            )

    def read(self, offset: int, length: int, what: str) -> bytes:
        self.require(offset, length, what)
        return self.data[offset : offset + length]

    def u8(self, offset: int, what: str) -> int:
        self.require(offset, 1, what)
        return self.data[offset]

    def u16(self, offset: int, what: str) -> int:
        self.require(offset, 2, what)
        return struct.unpack_from("<H", self.data, offset)[0]

    def i16(self, offset: int, what: str) -> int:
        self.require(offset, 2, what)
        return struct.unpack_from("<h", self.data, offset)[0]

    def u32(self, offset: int, what: str) -> int:
        self.require(offset, 4, what)
        return struct.unpack_from("<I", self.data, offset)[0]

    def i32(self, offset: int, what: str) -> int:
        self.require(offset, 4, what)
        return struct.unpack_from("<i", self.data, offset)[0]

    def cstring(self, offset: int, limit: int, what: str) -> tuple[str, int]:
        """Read a NUL-terminated single-byte string inside ``limit`` bytes."""
        if limit <= 0:
            raise HLPFormatError(self.path, offset, f"{what} has no room for a string")
        self.require(offset, limit, what)
        end = self.data.find(b"\0", offset, offset + limit)
        if end < 0:
            raise HLPFormatError(
                self.path, offset, f"{what} string is not NUL-terminated within {limit} bytes"
            )
        raw = self.data[offset:end]
        try:
            value = raw.decode("cp1252")
        except UnicodeDecodeError:
            value = raw.decode("latin-1")
        return value, end + 1


def _decode_cp1252(raw: bytes) -> str:
    try:
        return raw.decode("cp1252")
    except UnicodeDecodeError:
        return raw.decode("latin-1")


def gen_date_to_iso(seconds: int) -> str:
    """Convert the observed Unix ``GenDate`` value to ISO-8601 UTC.

    This is a module-level function so the epoch can be tested independently
    of any real help file.
    """
    if seconds < 0:
        raise ValueError("generation date seconds must not be negative")
    moment = UNIX_EPOCH + _datetime.timedelta(seconds=seconds)
    return moment.strftime("%Y-%m-%dT%H:%M:%SZ")


@dataclass(frozen=True)
class FileHeader:
    offset: int
    reserved: int
    used: int
    flags: int

    @property
    def data_start(self) -> int:
        return self.offset + 9

    @property
    def end(self) -> int:
        return self.data_start + self.used


@dataclass(frozen=True)
class FileInfo:
    name: str
    header: FileHeader


@dataclass(frozen=True)
class HLPHeader:
    magic: int
    directory_start: int
    first_free_block: int
    declared_file_size: int
    actual_file_size: int

    @property
    def trailing_bytes(self) -> int:
        return self.actual_file_size - self.declared_file_size


@dataclass
class BTreeInfo:
    offset: int
    flags: int
    page_size: int
    structure: str
    must_be_zero: int
    page_splits: int
    root_page: int
    total_pages: int
    n_levels: int
    total_entries: int
    leaf_pages: int = 0
    index_pages: int = 0
    entries_read: int = 0


@dataclass
class DirectoryInfo:
    file_header: FileHeader
    btree: BTreeInfo
    files: dict[str, FileInfo]
    order: list[str] = field(default_factory=list)


@dataclass
class SystemRecord:
    record_type: int
    data_size: int
    offset: int
    kind: str
    value: object = None


@dataclass
class SystemInfo:
    file: FileInfo
    magic: int
    minor: int
    major: int
    gen_date_raw: int
    generated_utc: str
    flags: int
    compressed: bool
    topic_block_size: int
    topic_page_size: int
    title: Optional[str]
    copyright: Optional[str]
    contents_topic: Optional[int]
    config: list[str]
    records: list[SystemRecord]

    @property
    def compression(self) -> str:
        if self.minor <= 16:
            return "none (HC30, 2 KiB blocks)"
        if self.compressed:
            return f"LZ77 (block size {self.topic_block_size})"
        return f"none (block size {self.topic_block_size})"


@dataclass
class TopicIndexEntry:
    topic_offset: int
    topic_block: int
    topic_block_offset: int
    file_offset: int
    title: str
    text: str
    body_status: str


@dataclass
class TopicInfo:
    count: int
    entries: list[TopicIndexEntry]
    link_count: int
    record_counts: dict[str, int]
    limitations: list[str]
    status: str


@dataclass
class PhraseTable:
    style: str
    phrases: list[bytes]


@dataclass
class HelpDocument:
    path: str
    data: bytes
    sha256: str
    header: HLPHeader
    directory: DirectoryInfo
    system: SystemInfo
    topics: TopicInfo
    phrase_style: Optional[str]
    limitations: list[str]


@dataclass
class CliResult:
    report: dict
    text: str
    document: HelpDocument


def parse_header(reader: BoundedReader) -> HLPHeader:
    if len(reader.data) < 16:
        raise HLPFormatError(
            reader.path, 0, f"truncated 16-byte HLP header (file has {len(reader.data)} bytes)"
        )
    magic = reader.u32(0, "HLP magic")
    if magic != HLP_MAGIC:
        raise HLPFormatError(
            reader.path,
            0,
            f"bad HLP magic 0x{magic:08x}; expected 0x{HLP_MAGIC:08x}",
        )
    directory_start = reader.u32(4, "directory start")
    first_free_block = reader.u32(8, "first free block")
    declared_size = reader.u32(12, "declared file size")
    actual_size = len(reader.data)
    if declared_size < 16:
        raise HLPFormatError(
            reader.path, 12, f"declared file size {declared_size} is smaller than the 16-byte header"
        )
    if declared_size > actual_size:
        raise HLPFormatError(
            reader.path,
            12,
            f"declared file size {declared_size} exceeds actual size {actual_size} (truncated input)",
        )
    if directory_start < 16 or directory_start >= declared_size:
        raise HLPFormatError(
            reader.path,
            4,
            f"directory start 0x{directory_start:x} is outside the declared file (size 0x{declared_size:x})",
        )
    if first_free_block != 0xFFFFFFFF and first_free_block >= declared_size:
        raise HLPFormatError(
            reader.path,
            8,
            f"first free block 0x{first_free_block:x} is outside the declared file",
        )
    return HLPHeader(magic, directory_start, first_free_block, declared_size, actual_size)


def read_file_header(reader: BoundedReader, offset: int, declared_size: int, label: str) -> FileHeader:
    if offset < 16 or offset + 9 > declared_size:
        raise HLPFormatError(
            reader.path, offset, f"{label} FILEHEADER at 0x{offset:x} is outside the declared file"
        )
    reserved = reader.u32(offset, f"{label} reserved space")
    used = reader.u32(offset + 4, f"{label} used space")
    flags = reader.u8(offset + 8, f"{label} file flags")
    if used > declared_size - (offset + 9):
        raise HLPFormatError(
            reader.path,
            offset + 4,
            f"{label} used size {used} runs past the declared file end (offset 0x{offset:x})",
        )
    if reserved < used:
        raise HLPFormatError(
            reader.path,
            offset,
            f"{label} reserved size {reserved} is smaller than used size {used}",
        )
    return FileHeader(offset, reserved, used, flags)


def _validate_page_number(page: int, total_pages: int, path: str, offset: int, what: str) -> None:
    if page < 0 or page >= total_pages:
        raise HLPFormatError(
            path,
            offset,
            f"{what} page number {page} is outside 0..{total_pages - 1}",
        )


def _parse_btree_page(
    reader: BoundedReader,
    page: int,
    level: int,
    btree: BTreeInfo,
    pages_base: int,
    used_end: int,
) -> tuple[list[tuple[str, int]], int, int, int]:
    """Return entries, previous page, next page and consumed bytes."""
    page_offset = pages_base + page * btree.page_size
    page_end = page_offset + btree.page_size
    if page_offset < pages_base or page_end > used_end:
        raise HLPFormatError(
            reader.path,
            page_offset,
            f"B+ tree page {page} at 0x{page_offset:x} is outside the directory file",
        )
    unused = reader.u16(page_offset, f"B+ tree page {page} unused count")
    n_entries = reader.i16(page_offset + 2, f"B+ tree page {page} entry count")
    previous = reader.i16(page_offset + 4, f"B+ tree page {page} previous page")
    if n_entries < 0:
        raise HLPFormatError(reader.path, page_offset + 2, f"B+ tree page {page} has negative entry count")
    if n_entries > btree.page_size:
        raise HLPFormatError(
            reader.path,
            page_offset + 2,
            f"B+ tree page {page} claims {n_entries} entries, which cannot fit in {btree.page_size} bytes",
        )
    header_size = 8 if level == 0 else 6
    if header_size + n_entries * 5 > btree.page_size:
        raise HLPFormatError(
            reader.path,
            page_offset + 2,
            f"B+ tree page {page} claims too many entries ({n_entries}) for page size {btree.page_size}",
        )
    cursor = page_offset + header_size
    entries: list[tuple[str, int]] = []
    for index in range(n_entries):
        name, cursor = reader.cstring(cursor, page_end - cursor, f"B+ tree page {page} entry {index} name")
        if level == 0:
            if cursor + 4 > page_end:
                raise HLPFormatError(
                    reader.path, cursor, f"B+ tree page {page} entry {index} offset is truncated"
                )
            value = reader.u32(cursor, f"B+ tree page {page} entry {index} offset")
            cursor += 4
        else:
            if cursor + 2 > page_end:
                raise HLPFormatError(
                    reader.path, cursor, f"B+ tree page {page} entry {index} page is truncated"
                )
            value = reader.i16(cursor, f"B+ tree page {page} entry {index} page")
            cursor += 2
        if not name:
            raise HLPFormatError(reader.path, cursor, f"B+ tree page {page} entry {index} has an empty name")
        entries.append((name, value))
    consumed = cursor - page_offset
    if consumed + unused != btree.page_size:
        raise HLPFormatError(
            reader.path,
            page_offset,
            f"B+ tree page {page} has inconsistent sizes: header+entries={consumed}, unused={unused}, page={btree.page_size}",
        )
    next_page = reader.i16(page_offset + 6, f"B+ tree page {page} next page") if level == 0 else -1
    return entries, previous, next_page, consumed


def _read_btree_header(reader: BoundedReader, base: int, used_end: int) -> tuple[BTreeInfo, int, int]:
    if base + 38 > used_end:
        raise HLPFormatError(reader.path, base, "truncated B+ tree header")
    magic = reader.u16(base, "B+ tree magic")
    if magic != BTREE_MAGIC:
        raise HLPFormatError(
            reader.path,
            base,
            f"bad B+ tree magic 0x{magic:04x}; expected 0x{BTREE_MAGIC:04x}",
        )
    flags = reader.u16(base + 2, "B+ tree flags")
    page_size = reader.u16(base + 4, "B+ tree page size")
    if page_size not in (1024, 2048, 4096):
        raise HLPFormatError(
            reader.path,
            base + 4,
            f"unsupported B+ tree page size {page_size} (expected 1024, 2048 or 4096)",
        )
    structure, _ = reader.cstring(base + 6, 16, "B+ tree structure string")
    must_be_zero = reader.u16(base + 22, "B+ tree must-be-zero")
    page_splits = reader.u16(base + 24, "B+ tree page splits")
    root_page = reader.i16(base + 26, "B+ tree root page")
    must_be_neg_one = reader.u16(base + 28, "B+ tree sentinel")
    total_pages = reader.i16(base + 30, "B+ tree total pages")
    n_levels = reader.i16(base + 32, "B+ tree level count")
    total_entries = reader.u32(base + 34, "B+ tree entry count")
    if must_be_zero != 0:
        raise HLPFormatError(reader.path, base + 22, f"B+ tree must-be-zero field is {must_be_zero}")
    if must_be_neg_one != 0xFFFF:
        raise HLPFormatError(
            reader.path, base + 28, f"B+ tree sentinel is 0x{must_be_neg_one:04x}, expected 0xffff"
        )
    if total_pages <= 0:
        raise HLPFormatError(reader.path, base + 30, f"B+ tree total page count is {total_pages}")
    if n_levels <= 0:
        raise HLPFormatError(reader.path, base + 32, f"B+ tree level count is {n_levels}")
    if total_entries > MAX_ENTRIES:
        raise HLPFormatError(
            reader.path, base + 34, f"B+ tree declares too many entries ({total_entries})"
        )
    if root_page < 0 or root_page >= total_pages:
        raise HLPFormatError(
            reader.path,
            base + 26,
            f"B+ tree root page {root_page} is outside 0..{total_pages - 1}",
        )
    pages_base = base + 38
    pages_end = pages_base + total_pages * page_size
    if pages_end > used_end:
        raise HLPFormatError(
            reader.path,
            pages_base,
            f"B+ tree pages end at 0x{pages_end:x}, past directory end 0x{used_end:x}",
        )
    info = BTreeInfo(
        offset=base,
        flags=flags,
        page_size=page_size,
        structure=structure,
        must_be_zero=must_be_zero,
        page_splits=page_splits,
        root_page=root_page,
        total_pages=total_pages,
        n_levels=n_levels,
        total_entries=total_entries,
    )
    return info, pages_base, pages_end


def parse_directory(reader: BoundedReader, header: HLPHeader) -> DirectoryInfo:
    file_header = read_file_header(reader, header.directory_start, header.declared_file_size, "directory")
    used_end = file_header.end
    btree, pages_base, _pages_end = _read_btree_header(reader, file_header.data_start, used_end)
    index_seen: set[int] = set()
    leaf_from_tree: set[int] = set()

    def walk_index(page: int, level: int) -> None:
        if page in index_seen:
            raise HLPFormatError(reader.path, pages_base + page * btree.page_size, f"B+ tree index cycle at page {page}")
        index_seen.add(page)
        btree.index_pages += 1
        entries, _previous, _next, _consumed = _parse_btree_page(
            reader, page, level, btree, pages_base, used_end
        )
        if not entries:
            raise HLPFormatError(reader.path, pages_base + page * btree.page_size, f"B+ tree index page {page} has no entries")
        for _name, child in entries:
            _validate_page_number(child, btree.total_pages, reader.path, pages_base + page * btree.page_size, "B+ tree child")
            if level - 1 == 0:
                leaf_from_tree.add(child)
            else:
                walk_index(child, level - 1)

    if btree.n_levels > 1:
        walk_index(btree.root_page, btree.n_levels - 1)
    elif btree.n_levels == 1:
        leaf_from_tree.add(btree.root_page)
    else:
        raise HLPFormatError(reader.path, btree.offset + 32, "B+ tree has no levels")

    def descend_to_leaf(level: int, page: int, seen: set[int]) -> int:
        if page in seen:
            raise HLPFormatError(reader.path, pages_base + page * btree.page_size, f"B+ tree index cycle at page {page}")
        seen.add(page)
        if level == 0:
            return page
        entries, _previous, _next, _consumed = _parse_btree_page(
            reader, page, level, btree, pages_base, used_end
        )
        if not entries:
            raise HLPFormatError(reader.path, pages_base + page * btree.page_size, f"B+ tree index page {page} has no entries")
        child = entries[0][1]
        _validate_page_number(child, btree.total_pages, reader.path, pages_base + page * btree.page_size, "B+ tree child")
        return descend_to_leaf(level - 1, child, seen)

    any_leaf = descend_to_leaf(btree.n_levels - 1, btree.root_page, set())
    # Index pages may point at a later leaf.  Walk the leaf previous links
    # backwards to reach the first leaf in the sorted chain.
    first_leaf = any_leaf
    previous_seen: set[int] = set()
    while True:
        if first_leaf in previous_seen:
            raise HLPFormatError(
                reader.path,
                pages_base + first_leaf * btree.page_size,
                f"B+ tree leaf previous-link cycle at page {first_leaf}",
            )
        previous_seen.add(first_leaf)
        _entries, previous, _next, _consumed = _parse_btree_page(
            reader, first_leaf, 0, btree, pages_base, used_end
        )
        if previous == -1:
            break
        _validate_page_number(
            previous,
            btree.total_pages,
            reader.path,
            pages_base + first_leaf * btree.page_size + 4,
            "B+ tree previous leaf",
        )
        first_leaf = previous
    files: dict[str, FileInfo] = {}
    order: list[str] = []
    seen_leaves: set[int] = set()
    expected_previous = -1
    page = first_leaf
    while page != -1:
        if page in seen_leaves:
            raise HLPFormatError(reader.path, pages_base + page * btree.page_size, f"B+ tree leaf cycle at page {page}")
        _validate_page_number(page, btree.total_pages, reader.path, pages_base, "B+ tree leaf")
        seen_leaves.add(page)
        btree.leaf_pages += 1
        entries, previous, next_page, _consumed = _parse_btree_page(
            reader, page, 0, btree, pages_base, used_end
        )
        if previous != expected_previous:
            raise HLPFormatError(
                reader.path,
                pages_base + page * btree.page_size + 4,
                f"B+ tree leaf page {page} previous link is {previous}, expected {expected_previous}",
            )
        for name, offset in entries:
            if name in files:
                raise HLPFormatError(
                    reader.path,
                    pages_base + page * btree.page_size,
                    f"duplicate internal file name {name!r}",
                )
            file_info = FileInfo(name, read_file_header(reader, offset, header.declared_file_size, f"internal file {name!r}"))
            files[name] = file_info
            order.append(name)
            btree.entries_read += 1
        if next_page >= 0:
            _validate_page_number(next_page, btree.total_pages, reader.path, pages_base + page * btree.page_size + 6, "B+ tree next")
        expected_previous = page
        page = next_page

    if btree.entries_read != btree.total_entries:
        raise HLPFormatError(
            reader.path,
            btree.offset + 34,
            f"B+ tree declares {btree.total_entries} entries but leaf pages contain {btree.entries_read}",
        )
    missing_leaf = leaf_from_tree - seen_leaves
    if missing_leaf:
        raise HLPFormatError(
            reader.path,
            btree.offset,
            f"B+ tree index points to leaf page(s) not in the linked leaf list: {sorted(missing_leaf)}",
        )
    if not files:
        raise HLPFormatError(reader.path, btree.offset, "directory contains no internal files")
    return DirectoryInfo(file_header, btree, files, order)


def _system_records(
    reader: BoundedReader, start: int, end: int
) -> tuple[list[SystemRecord], Optional[str], Optional[str], Optional[int], list[str]]:
    records: list[SystemRecord] = []
    title = None
    copyright_text = None
    contents = None
    config: list[str] = []
    cursor = start
    index = 0
    while cursor < end:
        if end - cursor < 4:
            raise HLPFormatError(reader.path, cursor, "truncated SYSTEMREC header (need 4 bytes)")
        record_type = reader.u16(cursor, f"SYSTEMREC {index} type")
        data_size = reader.u16(cursor + 2, f"SYSTEMREC {index} size")
        if record_type == 0xFFFF:
            break
        data_start = cursor + 4
        if data_size > end - data_start:
            raise HLPFormatError(
                reader.path,
                cursor + 2,
                f"SYSTEMREC {index} (type {record_type}) size {data_size} runs past |SYSTEM end",
            )
        kind = "binary"
        value: object = None
        if record_type in (1, 2, 4, 8, 10, 14, 18):
            value, _after = reader.cstring(data_start, data_size, f"SYSTEMREC {index} string")
            kind = "string"
            if record_type == 1:
                title = value
            elif record_type == 2:
                copyright_text = value
            elif record_type == 4:
                config.append(value)
        elif record_type == 3:
            if data_size != 4:
                raise HLPFormatError(
                    reader.path,
                    cursor + 2,
                    f"SYSTEMREC {index} (contents) size is {data_size}, expected 4",
                )
            value = reader.u32(data_start, f"SYSTEMREC {index} contents topic")
            contents = value
            kind = "topic_offset"
        elif record_type == 11:
            if data_size != 2:
                raise HLPFormatError(
                    reader.path,
                    cursor + 2,
                    f"SYSTEMREC {index} (charset) size is {data_size}, expected 2",
                )
            value = reader.u16(data_start, f"SYSTEMREC {index} charset")
            kind = "charset"
        records.append(SystemRecord(record_type, data_size, cursor, kind, value))
        cursor = data_start + data_size
        index += 1
    return records, title, copyright_text, contents, config


def parse_system(reader: BoundedReader, file_info: FileInfo) -> SystemInfo:
    start = file_info.header.data_start
    end = file_info.header.end
    if file_info.header.used < 12:
        raise HLPFormatError(reader.path, start, "truncated |SYSTEM header (need 12 bytes)")
    magic = reader.u16(start, "|SYSTEM magic")
    if magic != SYSTEM_MAGIC:
        raise HLPFormatError(
            reader.path,
            start,
            f"bad |SYSTEM magic 0x{magic:04x}; expected 0x{SYSTEM_MAGIC:04x}",
        )
    minor = reader.u16(start + 2, "|SYSTEM minor version")
    major = reader.u16(start + 4, "|SYSTEM major version")
    gen_date = reader.u32(start + 6, "|SYSTEM generation date")
    flags = reader.u16(start + 10, "|SYSTEM flags")

    if minor <= 16:
        compressed = False
        topic_block_size = 2048
        topic_page_size = 2048
    else:
        topic_page_size = 16384
        if flags == 8:
            topic_block_size = 2048
            compressed = True
        elif flags == 4:
            topic_block_size = 4096
            compressed = True
        elif flags == 0:
            topic_block_size = 4096
            compressed = False
        else:
            raise HLPFormatError(
                reader.path,
                start + 10,
                f"unsupported |SYSTEM compression flags 0x{flags:04x} for minor version {minor}",
            )

    if minor <= 16:
        if start + 12 >= end:
            raise HLPFormatError(reader.path, start + 12, "truncated HC30 |SYSTEM title")
        title, _after = reader.cstring(start + 12, end - (start + 12), "HC30 |SYSTEM title")
        records: list[SystemRecord] = []
        copyright_text = None
        contents = None
        config: list[str] = []
    else:
        records, title, copyright_text, contents, config = _system_records(
            reader, start + 12, end
        )
    return SystemInfo(
        file=file_info,
        magic=magic,
        minor=minor,
        major=major,
        gen_date_raw=gen_date,
        generated_utc=gen_date_to_iso(gen_date),
        flags=flags,
        compressed=compressed,
        topic_block_size=topic_block_size,
        topic_page_size=topic_page_size,
        title=title,
        copyright=copyright_text,
        contents_topic=contents,
        config=config,
        records=records,
    )


def lz77_decompress(
    source: bytes,
    *,
    path: str,
    source_offset: int,
    context: str,
    max_output: int,
    expected_output: Optional[int] = None,
) -> bytes:
    """Decode the Microsoft LZ77 variant used by WinHelp.

    The caller supplies an output limit.  ``expected_output`` also enforces an
    exact size for tables such as |Phrases.  Truncated tokens, invalid
    back-references and output overruns raise :class:`HLPFormatError`.
    """
    out = bytearray()
    cursor = 0
    while cursor < len(source):
        flags = source[cursor]
        cursor += 1
        for bit in range(8):
            if cursor >= len(source):
                if flags >> bit:
                    raise HLPFormatError(
                        path,
                        source_offset + cursor - 1,
                        f"{context}: truncated LZ77 match token (control byte has set bits without tokens)",
                    )
                break
            if flags & (1 << bit):
                if cursor + 2 > len(source):
                    raise HLPFormatError(
                        path,
                        source_offset + cursor,
                        f"{context}: truncated LZ77 match token (need 2 bytes)",
                    )
                value = source[cursor] | (source[cursor + 1] << 8)
                cursor += 2
                back = value & 0x0FFF
                length = (value >> 12) + 3
                if back + 1 > len(out):
                    raise HLPFormatError(
                        path,
                        source_offset + cursor - 2,
                        f"{context}: LZ77 back-reference {back} exceeds output length {len(out)}",
                    )
                for _ in range(length):
                    if expected_output is not None and len(out) >= expected_output:
                        raise HLPFormatError(
                            path,
                            source_offset + cursor - 2,
                            f"{context}: LZ77 output exceeds expected size {expected_output}",
                        )
                    if len(out) >= max_output:
                        raise HLPFormatError(
                            path,
                            source_offset + cursor - 2,
                            f"{context}: LZ77 output exceeds limit {max_output}",
                        )
                    out.append(out[len(out) - back - 1])
            else:
                if expected_output is not None and len(out) >= expected_output:
                    raise HLPFormatError(
                        path,
                        source_offset + cursor,
                        f"{context}: LZ77 output exceeds expected size {expected_output}",
                    )
                if len(out) >= max_output:
                    raise HLPFormatError(
                        path,
                        source_offset + cursor,
                        f"{context}: LZ77 output exceeds limit {max_output}",
                    )
                out.append(source[cursor])
                cursor += 1
            if expected_output is not None and len(out) == expected_output:
                return bytes(out)
    if expected_output is not None and len(out) != expected_output:
        raise HLPFormatError(
            path,
            source_offset,
            f"{context}: LZ77 stream ended after {len(out)} bytes, expected {expected_output}",
        )
    return bytes(out)


def parse_phrases(reader: BoundedReader, directory: DirectoryInfo, system: SystemInfo) -> tuple[Optional[PhraseTable], list[str]]:
    limitations: list[str] = []
    if "|Phrases" in directory.files:
        file_info = directory.files["|Phrases"]
        start = file_info.header.data_start
        end = file_info.header.end
        data = reader.read(start, end - start, "|Phrases data")
        if len(data) < 4:
            raise HLPFormatError(reader.path, start, "truncated |Phrases header")
        first = struct.unpack_from("<H", data, 0)[0]
        new_style = first == 0x0800
        if new_style:
            if len(data) < 6:
                raise HLPFormatError(reader.path, start, "truncated MVB |Phrases header")
            count = struct.unpack_from("<H", data, 2)[0]
            one_hundred = struct.unpack_from("<H", data, 4)[0]
            header = 6
        else:
            count = first
            one_hundred = struct.unpack_from("<H", data, 2)[0]
            header = 4
        if one_hundred != 0x0100:
            raise HLPFormatError(
                reader.path,
                start + (4 if new_style else 2),
                f"unknown |Phrases marker 0x{one_hundred:04x}, expected 0x0100",
            )
        if count > 100_000:
            raise HLPFormatError(reader.path, start, f"|Phrases declares too many phrases ({count})")
        if new_style:
            if len(data) < 10:
                raise HLPFormatError(reader.path, start, "truncated MVB |Phrases size field")
            decompressed_size = struct.unpack_from("<I", data, 6)[0]
            header = 10 + 30
        elif system.minor > 16:
            if len(data) < 8:
                raise HLPFormatError(reader.path, start, "truncated |Phrases decompressed size")
            decompressed_size = struct.unpack_from("<I", data, 4)[0]
            header = 8
        else:
            decompressed_size = None
            header = 4
        offsets_end = header + (count + 1) * 2
        if offsets_end > len(data):
            raise HLPFormatError(reader.path, start + header, "truncated |Phrases offset table")
        base = (count + 1) * 2
        offsets = [
            struct.unpack_from("<H", data, header + index * 2)[0] - base
            for index in range(count + 1)
        ]
        compressed = data[offsets_end:]
        if decompressed_size is None:
            phrase_data = compressed
        else:
            phrase_data = lz77_decompress(
                compressed,
                path=reader.path,
                source_offset=start + offsets_end,
                context="|Phrases",
                max_output=decompressed_size,
                expected_output=decompressed_size,
            )
        if len(offsets) != count + 1:
            raise HLPFormatError(reader.path, start, "|Phrases offset count mismatch")
        if offsets[0] != 0:
            raise HLPFormatError(
                reader.path,
                start + header,
                f"|Phrases first offset is {offsets[0]}, expected 0 after header adjustment",
            )
        for index in range(count):
            if offsets[index] > offsets[index + 1]:
                raise HLPFormatError(
                    reader.path,
                    start + header + index * 2,
                    "|Phrases offsets are not nondecreasing",
                )
        if offsets[-1] > len(phrase_data):
            raise HLPFormatError(
                reader.path,
                start + offsets_end,
                f"|Phrases last offset {offsets[-1]} exceeds phrase data size {len(phrase_data)}",
            )
        phrases = [
            phrase_data[offsets[index] : offsets[index + 1]] for index in range(count)
        ]
        return PhraseTable("mvb-oldstyle" if new_style else "oldstyle", phrases), limitations

    if "|PhrIndex" in directory.files or "|PhrImage" in directory.files:
        limitations.append(
            "Hall phrase compression (|PhrIndex/|PhrImage) is present but not decoded; "
            "phrase-compressed LinkData2 would be unreadable"
        )
        return None, limitations
    return None, limitations


def expand_link_data2(
    raw: bytes,
    *,
    compressed_length: int,
    expected_length: int,
    phrases: Optional[PhraseTable],
    path: str,
    source_offset: int,
    context: str,
) -> bytes:
    if expected_length < 0 or expected_length > (1 << 24):
        raise HLPFormatError(path, source_offset, f"{context}: unreasonable LinkData2 size {expected_length}")
    if expected_length <= compressed_length:
        if expected_length > len(raw):
            raise HLPFormatError(
                path, source_offset, f"{context}: LinkData2 needs {expected_length} stored bytes, has {len(raw)}"
            )
        return raw[:expected_length]
    if phrases is None:
        raise HLPFormatError(
            path,
            source_offset,
            f"{context}: phrase-compressed LinkData2 needs {expected_length} bytes but no supported |Phrases table exists",
        )
    out = bytearray()
    cursor = 0
    while cursor < compressed_length and len(out) < expected_length:
        if cursor >= len(raw):
            raise HLPFormatError(path, source_offset + cursor, f"{context}: truncated phrase reference")
        value = raw[cursor]
        cursor += 1
        if value == 0 or value >= 16:
            out.append(value)
            continue
        if cursor >= len(raw):
            raise HLPFormatError(path, source_offset + cursor - 1, f"{context}: truncated two-byte phrase reference")
        second = raw[cursor]
        cursor += 1
        number = 256 * (value - 1) + second
        phrase_number = number // 2
        if phrase_number >= len(phrases.phrases):
            raise HLPFormatError(
                path,
                source_offset + cursor - 2,
                f"{context}: phrase number {phrase_number} outside 0..{len(phrases.phrases) - 1}",
            )
        out.extend(phrases.phrases[phrase_number])
        if number & 1:
            out.append(0x20)
        if len(out) > expected_length:
            raise HLPFormatError(
                path,
                source_offset + cursor,
                f"{context}: phrase expansion exceeds expected size {expected_length}",
            )
    if len(out) != expected_length:
        raise HLPFormatError(
            path,
            source_offset,
            f"{context}: phrase expansion produced {len(out)} bytes, expected {expected_length}",
        )
    return bytes(out)


class _LinkFormatError(Exception):
    pass


def _scan_long(data: bytes, cursor: int) -> tuple[int, int]:
    if cursor + 2 > len(data):
        raise _LinkFormatError("truncated compressed long")
    if data[cursor] & 1:
        if cursor + 4 > len(data):
            raise _LinkFormatError("truncated compressed long")
        value = struct.unpack_from("<I", data, cursor)[0]
        return (value >> 1) - 0x40000000, cursor + 4
    value = struct.unpack_from("<H", data, cursor)[0]
    return (value >> 1) - 0x4000, cursor + 2


def _scan_word(data: bytes, cursor: int) -> tuple[int, int]:
    if cursor >= len(data):
        raise _LinkFormatError("truncated compressed word")
    if data[cursor] & 1:
        if cursor + 2 > len(data):
            raise _LinkFormatError("truncated compressed word")
        return struct.unpack_from("<H", data, cursor)[0] >> 1, cursor + 2
    return data[cursor] >> 1, cursor + 1


def _scan_int(data: bytes, cursor: int) -> tuple[int, int]:
    if cursor >= len(data):
        raise _LinkFormatError("truncated compressed signed short")
    if data[cursor] & 1:
        if cursor + 2 > len(data):
            raise _LinkFormatError("truncated compressed signed short")
        return (struct.unpack_from("<H", data, cursor)[0] >> 1) - 0x4000, cursor + 2
    return (data[cursor] >> 1) - 0x40, cursor + 1


def _render_paragraph(
    link_data1: bytes,
    cursor: int,
    strings: list[bytes],
    string_index: int,
    output: list[bytes],
) -> tuple[int, int]:
    if cursor + 2 > len(link_data1):
        raise _LinkFormatError("truncated paragraph flags")
    flags = struct.unpack_from("<H", link_data1, cursor)[0]
    cursor += 2
    if flags & 0x0001:
        _value, cursor = _scan_long(link_data1, cursor)
    for mask in (0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0040):
        if flags & mask:
            _value, cursor = _scan_int(link_data1, cursor)
    if flags & 0x0100:
        cursor += 3
    if flags & 0x0200:
        tab_count, cursor = _scan_int(link_data1, cursor)
        if tab_count < 0 or tab_count > 10_000:
            raise _LinkFormatError(f"unreasonable tab count {tab_count}")
        for _ in range(tab_count):
            tab, cursor = _scan_word(link_data1, cursor)
            if tab & 0x4000:
                _tab_type, cursor = _scan_word(link_data1, cursor)
    if cursor > len(link_data1):
        raise _LinkFormatError("paragraph data runs past LinkData1")
    output.append(strings[string_index] if string_index < len(strings) else b"")
    string_index += 1
    steps = 0
    while True:
        steps += 1
        if steps > 100_000:
            raise _LinkFormatError("formatting command loop")
        if cursor >= len(link_data1):
            raise _LinkFormatError("LinkData1 ended before 0xFF formatting terminator")
        command = link_data1[cursor]
        if command == 0xFF:
            cursor += 1
            break
        if command == 0x20:
            cursor += 5
        elif command == 0x21:
            cursor += 3
        elif command == 0x80:
            cursor += 3
        elif command == 0x81:
            output.append(b"\n")
            cursor += 1
        elif command == 0x82:
            output.append(b"\n")
            cursor += 1
        elif command == 0x83:
            output.append(b"\t")
            cursor += 1
        elif command in (0x86, 0x87, 0x88):
            if cursor + 2 > len(link_data1):
                raise _LinkFormatError("truncated picture command")
            picture_type = link_data1[cursor + 1]
            cursor += 2
            picture_size, cursor = _scan_long(link_data1, cursor)
            if picture_size < 0 or cursor + picture_size > len(link_data1):
                raise _LinkFormatError("picture data runs past LinkData1")
            if picture_type == 0x22:
                _hotspots, cursor = _scan_word(link_data1, cursor)
            cursor += picture_size
        elif command == 0x89:
            cursor += 1
        elif command == 0x8B:
            output.append(b" ")
            cursor += 1
        elif command == 0x8C:
            # Non-breaking hyphen: the hyphen byte itself follows in LinkData2.
            cursor += 1
        elif command in (0xC8, 0xCC):
            if cursor + 3 > len(link_data1):
                raise _LinkFormatError("truncated macro command")
            length = struct.unpack_from("<H", link_data1, cursor + 1)[0]
            cursor += length + 3
        elif command in (0xE0, 0xE1, 0xE2, 0xE3, 0xE6, 0xE7):
            cursor += 5
        elif command in (0xEA, 0xEB, 0xEE, 0xEF):
            if cursor + 3 > len(link_data1):
                raise _LinkFormatError("truncated external jump command")
            length = struct.unpack_from("<H", link_data1, cursor + 1)[0]
            cursor += length + 3
        else:
            cursor += 1
        if cursor > len(link_data1):
            raise _LinkFormatError("formatting command runs past LinkData1")
        output.append(strings[string_index] if string_index < len(strings) else b"")
        string_index += 1
    return cursor, string_index


def _render_display_record(link_data1: bytes, link_data2: bytes) -> str:
    strings = link_data2.split(b"\0")
    cursor = 0
    _topic_size, cursor = _scan_long(link_data1, cursor)
    _topic_length, cursor = _scan_word(link_data1, cursor)
    cursor += 4
    output: list[bytes] = []
    _cursor, _index = _render_paragraph(link_data1, cursor, strings, 0, output)
    return _decode_cp1252(b"".join(output))


def _render_table_record(link_data1: bytes, link_data2: bytes) -> str:
    strings = link_data2.split(b"\0")
    cursor = 0
    _topic_size, cursor = _scan_long(link_data1, cursor)
    _topic_length, cursor = _scan_word(link_data1, cursor)
    if cursor + 2 > len(link_data1):
        raise _LinkFormatError("truncated table header")
    columns = link_data1[cursor]
    table_type = link_data1[cursor + 1]
    cursor += 2
    if table_type in (0, 2):
        cursor += 2
    if columns == 0 or columns > 256:
        raise _LinkFormatError(f"unreasonable table column count {columns}")
    cursor += 2 * columns
    output: list[bytes] = []
    string_index = 0
    rows = 0
    while cursor + 2 <= len(link_data1) and struct.unpack_from("<h", link_data1, cursor)[0] != -1:
        if cursor + 5 > len(link_data1):
            raise _LinkFormatError("truncated table column header")
        cursor += 5
        cursor, string_index = _render_paragraph(link_data1, cursor, strings, string_index, output)
        output.append(b"\t")
        rows += 1
        if rows > 10_000:
            raise _LinkFormatError("too many table rows")
    output.append(b"\n")
    return _decode_cp1252(b"".join(output))


def _fallback_strings(link_data2: bytes) -> str:
    parts = [_decode_cp1252(part) for part in link_data2.split(b"\0") if part]
    return " ".join(parts)


class TopicBlockReader:
    def __init__(self, reader: BoundedReader, file_info: FileInfo, system: SystemInfo):
        self.reader = reader
        self.file_info = file_info
        self.system = system
        self.block_count = (file_info.header.used + system.topic_block_size - 1) // system.topic_block_size
        self.cache: dict[int, bytes] = {}
        self.cache_headers: dict[int, tuple[int, int, int]] = {}

    def block(self, page: int) -> bytes:
        if page in self.cache:
            return self.cache[page]
        if page < 0 or page >= self.block_count:
            raise HLPFormatError(
                self.reader.path,
                self.file_info.header.data_start,
                f"|TOPIC block {page} is outside 0..{self.block_count - 1}",
            )
        block_start = self.file_info.header.data_start + page * self.system.topic_block_size
        block_end = min(block_start + self.system.topic_block_size, self.file_info.header.end)
        if block_start + 12 > block_end:
            raise HLPFormatError(self.reader.path, block_start, f"truncated |TOPIC block {page} header")
        last_link = self.reader.u32(block_start, f"|TOPIC block {page} last link")
        first_link = self.reader.u32(block_start + 4, f"|TOPIC block {page} first link")
        last_header = self.reader.u32(block_start + 8, f"|TOPIC block {page} last topic header")
        if first_link < 12:
            raise HLPFormatError(
                self.reader.path,
                block_start + 4,
                f"|TOPIC block {page} first link 0x{first_link:x} is below the 12-byte header",
            )
        self.cache_headers[page] = (last_link, first_link, last_header)
        raw = self.reader.read(block_start + 12, block_end - block_start - 12, f"|TOPIC block {page} data")
        if self.system.compressed:
            if not raw:
                raise HLPFormatError(self.reader.path, block_start + 12, f"empty compressed |TOPIC block {page}")
            data = lz77_decompress(
                raw,
                path=self.reader.path,
                source_offset=block_start + 12,
                context=f"|TOPIC block {page}",
                max_output=self.system.topic_page_size,
            )
        else:
            data = raw
        if len(data) > self.system.topic_page_size:
            raise HLPFormatError(
                self.reader.path,
                block_start + 12,
                f"|TOPIC block {page} expands to {len(data)} bytes, limit {self.system.topic_page_size}",
            )
        self.cache[page] = data
        return data

    def position_to_block(self, topic_pos: int) -> tuple[int, int]:
        if topic_pos < 12:
            raise HLPFormatError(self.reader.path, topic_pos, f"invalid TOPICPOS {topic_pos} (below 12)")
        offset = topic_pos - 12
        return divmod(offset, self.system.topic_page_size)

    def read(self, topic_pos: int, length: int) -> bytes:
        if length < 0:
            raise HLPFormatError(self.reader.path, topic_pos, "negative |TOPIC read length")
        output = bytearray()
        cursor = topic_pos
        loops = 0
        while length:
            loops += 1
            if loops > self.block_count + 2:
                raise HLPFormatError(self.reader.path, cursor, "|TOPIC read crossed too many blocks")
            page, offset = self.position_to_block(cursor)
            data = self.block(page)
            if offset > len(data):
                raise HLPFormatError(
                    self.reader.path,
                    self.file_info.header.data_start + page * self.system.topic_block_size + 12 + offset,
                    f"TOPICPOS 0x{cursor:x} offset {offset} is past block {page} data length {len(data)}",
                )
            if offset == len(data):
                cursor = (page + 1) * self.system.topic_page_size + 12
                continue
            take = min(length, len(data) - offset)
            output.extend(data[offset : offset + take])
            cursor += take
            length -= take
        return bytes(output)

    def absolute_offset(self, topic_pos: int) -> int:
        page, offset = self.position_to_block(topic_pos)
        return self.file_info.header.data_start + page * self.system.topic_block_size + 12 + offset


def parse_topics(
    reader: BoundedReader,
    directory: DirectoryInfo,
    system: SystemInfo,
    phrases: Optional[PhraseTable],
    initial_limitations: Iterable[str],
) -> TopicInfo:
    if "|TOPIC" not in directory.files:
        raise HLPFormatError(reader.path, directory.btree.offset, "|TOPIC internal file is missing")
    file_info = directory.files["|TOPIC"]
    topic_reader = TopicBlockReader(reader, file_info, system)
    limitations = set(initial_limitations)
    fallback_topics = 0
    fallback_records = 0
    record_counts: dict[str, int] = {}
    entries: list[TopicIndexEntry] = []
    current: Optional[TopicIndexEntry] = None
    seen: set[int] = set()
    topic_pos = 12
    link_count = 0
    while True:
        if topic_pos <= 0:
            break
        if topic_pos in seen:
            raise HLPFormatError(
                reader.path,
                topic_reader.absolute_offset(topic_pos) if topic_pos >= 12 else topic_pos,
                f"|TOPIC link cycle at TOPICPOS 0x{topic_pos:x}",
            )
        seen.add(topic_pos)
        link_count += 1
        if link_count > MAX_LINKS:
            raise HLPFormatError(reader.path, topic_pos, f"|TOPIC has more than {MAX_LINKS} links")
        absolute = topic_reader.absolute_offset(topic_pos)
        header = topic_reader.read(topic_pos, 21)
        block_size, data_len2, _previous, next_block, data_len1, record_type = struct.unpack(
            "<IIiiIB", header
        )
        if block_size < 21:
            raise HLPFormatError(reader.path, absolute, f"TOPICLINK BlockSize {block_size} is below 21")
        if data_len1 < 21 or data_len1 > block_size:
            raise HLPFormatError(
                reader.path,
                absolute,
                f"TOPICLINK DataLen1 {data_len1} is invalid for BlockSize {block_size}",
            )
        if data_len2 > block_size or data_len2 > (1 << 24):
            raise HLPFormatError(
                reader.path, absolute, f"TOPICLINK DataLen2 {data_len2} is invalid for BlockSize {block_size}"
            )
        if block_size > file_info.header.used:
            raise HLPFormatError(
                reader.path,
                absolute,
                f"TOPICLINK BlockSize {block_size} exceeds |TOPIC size {file_info.header.used}",
            )
        record = topic_reader.read(topic_pos, block_size)
        link_data1 = record[21:data_len1]
        compressed_len = block_size - data_len1
        raw_link_data2 = record[data_len1:block_size]
        try:
            link_data2 = expand_link_data2(
                raw_link_data2,
                compressed_length=compressed_len,
                expected_length=data_len2,
                phrases=phrases,
                path=reader.path,
                source_offset=absolute + data_len1,
                context=f"TOPICLINK at 0x{absolute:x}",
            )
        except HLPFormatError:
            raise
        record_counts[f"0x{record_type:02x}"] = record_counts.get(f"0x{record_type:02x}", 0) + 1
        if record_type == 0x02:
            if not link_data2:
                title = ""
            else:
                # The supplied file uses the whole DataLen2 payload for titles
                # that exactly fill it, so a final NUL is not required here.
                end = link_data2.find(b"\0")
                if end < 0:
                    end = len(link_data2)
                title = _decode_cp1252(link_data2[:end])
            page, offset = topic_reader.position_to_block(topic_pos)
            current = TopicIndexEntry(
                topic_offset=topic_pos,
                topic_block=page,
                topic_block_offset=offset,
                file_offset=absolute,
                title=title,
                text="",
                body_status="ok",
            )
            entries.append(current)
        elif record_type in (0x20, 0x23) and current is not None:
            try:
                if record_type == 0x20:
                    text = _render_display_record(link_data1, link_data2)
                else:
                    text = _render_table_record(link_data1, link_data2)
            except _LinkFormatError:
                text = _fallback_strings(link_data2)
                current.body_status = "strings-only"
                fallback_records += 1
                fallback_topics += 1
            current.text += text
        if next_block <= 0:
            break
        topic_pos = next_block

    if fallback_records:
        limitations.add(
            f"best-effort body decoding: {fallback_records} display/table record(s) used "
            "NUL-separated strings because LinkData1 formatting could not be followed"
        )
    if fallback_topics:
        unique = {entry.topic_offset for entry in entries if entry.body_status != "ok"}
        limitations.add(f"{len(unique)} topic(s) have strings-only body text")
    if phrases is not None:
        limitations.add(f"phrase table decoded using {phrases.style} |Phrases")
    status = "ok" if not fallback_records and not any(item.startswith("Hall phrase") for item in limitations) else "partial"
    return TopicInfo(
        count=len(entries),
        entries=entries,
        link_count=link_count,
        record_counts=dict(sorted(record_counts.items())),
        limitations=sorted(limitations),
        status=status,
    )


def parse_help(path: os.PathLike[str] | str) -> HelpDocument:
    path_string = str(path)
    try:
        data = Path(path).read_bytes()
    except OSError as exc:
        raise HLPFormatError(path_string, None, f"cannot read input: {exc}") from exc
    reader = BoundedReader(data, path_string)
    header = parse_header(reader)
    directory = parse_directory(reader, header)
    if "|SYSTEM" not in directory.files:
        raise HLPFormatError(path_string, directory.btree.offset, "|SYSTEM internal file is missing")
    system = parse_system(reader, directory.files["|SYSTEM"])
    phrases, phrase_limitations = parse_phrases(reader, directory, system)
    topics = parse_topics(reader, directory, system, phrases, phrase_limitations)
    limitations = list(topics.limitations)
    if header.trailing_bytes:
        limitations.append(
            f"{header.trailing_bytes} byte(s) follow the declared HLP size and were not parsed"
        )
    return HelpDocument(
        path=path_string,
        data=data,
        sha256=hashlib.sha256(data).hexdigest(),
        header=header,
        directory=directory,
        system=system,
        topics=topics,
        phrase_style=phrases.style if phrases else None,
        limitations=sorted(set(limitations)),
    )


def build_report(document: HelpDocument) -> dict:
    header = document.header
    directory = document.directory
    system = document.system
    topics = document.topics
    btree = directory.btree
    report = {
        "tool": "tools/analysis/help.py",
        "input": {
            "path": document.path,
            "size": len(document.data),
            "sha256": document.sha256,
        },
        "hlp_header": {
            "magic": f"0x{header.magic:08x}",
            "directory_start": header.directory_start,
            "first_free_block": -1 if header.first_free_block == 0xFFFFFFFF else header.first_free_block,
            "declared_file_size": header.declared_file_size,
            "actual_file_size": header.actual_file_size,
            "trailing_bytes": header.trailing_bytes,
        },
        "directory": {
            "file_header": {
                "offset": directory.file_header.offset,
                "reserved": directory.file_header.reserved,
                "used": directory.file_header.used,
                "flags": directory.file_header.flags,
            },
            "btree": {
                "offset": btree.offset,
                "flags": f"0x{btree.flags:04x}",
                "page_size": btree.page_size,
                "structure": btree.structure,
                "page_splits": btree.page_splits,
                "root_page": btree.root_page,
                "total_pages": btree.total_pages,
                "levels": btree.n_levels,
                "entries": btree.total_entries,
                "leaf_pages": btree.leaf_pages,
                "index_pages": btree.index_pages,
            },
            "internal_files": {
                name: {
                    "offset": info.header.offset,
                    "size": info.header.used,
                    "reserved": info.header.reserved,
                    "flags": info.header.flags,
                }
                for name, info in sorted(directory.files.items())
            },
        },
        "system": {
            "offset": system.file.header.offset,
            "size": system.file.header.used,
            "magic": f"0x{system.magic:04x}",
            "minor": system.minor,
            "major": system.major,
            "gen_date_raw": system.gen_date_raw,
            "gen_date_epoch": "unix (1970-01-01T00:00:00Z); format note claiming 1980 does not match the supplied file",
            "generated_utc": system.generated_utc,
            "flags": system.flags,
            "compression": system.compression,
            "compressed": system.compressed,
            "topic_block_size": system.topic_block_size,
            "topic_page_size": system.topic_page_size,
            "title": system.title,
            "copyright": system.copyright,
            "contents_topic": system.contents_topic,
            "config": system.config,
            "records": [
                {
                    "type": record.record_type,
                    "size": record.data_size,
                    "offset": record.offset,
                    "kind": record.kind,
                    "value": record.value,
                }
                for record in system.records
            ],
        },
        "phrases": {
            "style": document.phrase_style,
        },
        "topics": {
            "count": topics.count,
            "link_count": topics.link_count,
            "record_counts": topics.record_counts,
            "decode_status": topics.status,
            "limitations": topics.limitations,
            "index": [
                {
                    "topic_offset": entry.topic_offset,
                    "file_offset": entry.file_offset,
                    "topic_block": entry.topic_block,
                    "topic_block_offset": entry.topic_block_offset,
                    "title": entry.title,
                    "text_length": len(entry.text),
                    "body_status": entry.body_status,
                }
                for entry in topics.entries
            ],
        },
        "limitations": document.limitations,
    }
    return report


def build_text_output(document: HelpDocument) -> str:
    system = document.system
    lines = [
        f"HLP: {document.path}",
        f"SHA-256: {document.sha256}",
        f"Size: {len(document.data)} bytes (declared {document.header.declared_file_size})",
        f"System title: {system.title or ''}",
        f"Copyright: {system.copyright or ''}",
        f"Generated (UTC): {system.generated_utc}",
        f"Compression: {system.compression}",
        f"Topics: {document.topics.count}",
        "",
    ]
    for index, entry in enumerate(document.topics.entries):
        lines.append(
            f"=== Topic {index:04d} | TOPICPOS {entry.topic_offset} "
            f"(0x{entry.topic_offset:08x}) | file offset {entry.file_offset} "
            f"(0x{entry.file_offset:08x}) | body {entry.body_status} ==="
        )
        lines.append(f"Title: {entry.title}")
        lines.append(entry.text)
        lines.append("")
    return "\n".join(lines)


def build_cli_result(path: os.PathLike[str] | str) -> CliResult:
    document = parse_help(path)
    return CliResult(build_report(document), build_text_output(document), document)


def _write_atomic(path: os.PathLike[str] | str, data: bytes) -> None:
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".tmp")
    try:
        temporary.write_bytes(data)
        os.replace(temporary, destination)
    finally:
        try:
            if temporary.exists():
                temporary.unlink()
        except OSError:
            pass


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Inspect a Windows Help (HLP) file without executing it.")
    parser.add_argument("input", help="HLP file to inspect")
    parser.add_argument("--output", required=True, help="JSON inventory output path")
    parser.add_argument("--text-output", required=True, help="decoded topic text output path")
    parser.add_argument("--quiet", action="store_true", help="do not print the summary")
    args = parser.parse_args(argv)
    try:
        result = build_cli_result(args.input)
    except HLPFormatError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    report_bytes = (json.dumps(result.report, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode("utf-8")
    text_bytes = result.text.encode("utf-8")
    try:
        _write_atomic(args.output, report_bytes)
        _write_atomic(args.text_output, text_bytes)
    except OSError as exc:
        print(f"error: cannot write output: {exc}", file=sys.stderr)
        return 1
    if not args.quiet:
        system = result.document.system
        print(
            f"{result.document.path}: {system.title!r}, {system.generated_utc}, "
            f"{result.document.topics.count} topics, {len(result.document.directory.files)} internal files"
        )
        print(f"wrote {args.output} and {args.text_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
