"""Tests for the bounded NE parser (tools/analysis/ne.py)."""

from __future__ import annotations

import os
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import ne  # noqa: E402


# ---------------------------------------------------------------------------
# Synthetic NE builder
# ---------------------------------------------------------------------------


def _align(value: int, alignment: int) -> int:
    return (value + alignment - 1) & ~(alignment - 1)


def _pascal(text: str) -> bytes:
    raw = text.encode("latin-1")
    if len(raw) > 255:
        raise ValueError("fixture name too long")
    return bytes([len(raw)]) + raw


def _dialog_template(
    title: str,
    controls: list[tuple[str, int, int]],
    *,
    style: int = 0x00C800C0,
    font: tuple[int, str] = (8, "MS Sans Serif"),
) -> bytes:
    """Build a classic 16-bit dialog template from (text, id, class) controls."""

    out = bytearray()
    out += struct.pack("<I", style)
    out += bytes([len(controls)])
    out += struct.pack("<HHHH", 10, 10, 200, 120)
    out += b"\x00"  # no menu
    out += b"\x00"  # default dialog class
    out += title.encode("latin-1") + b"\x00"
    if style & 0x40:
        out += struct.pack("<H", font[0])
        out += font[1].encode("latin-1") + b"\x00"
    for index, (text, control_id, class_byte) in enumerate(controls):
        out += struct.pack("<HHHHHI", 4, 4 + index * 12, 80, 10, control_id, 0x50010000)
        out += bytes([class_byte])
        out += text.encode("latin-1") + b"\x00"
        out += b"\x00"  # no extra data
    return bytes(out)


def _build(
    *,
    segments: list[bytes],
    resource_types: list[dict],
    resident_names: list[str],
    module_references: list[str],
    nonresident_description: str | None,
    entry_count: int,
    shift: int = 6,
) -> tuple[bytes, dict]:
    """Assemble a small, valid NE file and return (bytes, layout)."""

    ne_offset = 0x40
    unit = 1 << shift
    module_name = resident_names[0] if resident_names else "MINI"
    resident_extra = resident_names[1:] if resident_names else []

    # Normalised resource types.  A type without an ordinal is string-named.
    normalised: list[dict] = []
    for spec in resource_types:
        resources = []
        for resource in spec["resources"]:
            resources.append(
                {
                    "name": resource.get("name"),
                    "id": resource.get("id"),
                    "data": resource["data"],
                    "flags": resource.get("flags", 0x1C30),
                }
            )
        normalised.append(
            {
                "ordinal": spec.get("ordinal"),
                "name": spec.get("name"),
                "resources": resources,
            }
        )

    # Resource name area and table size.  A leading zero keeps the first real
    # name offset non-zero, because type id 0 terminates the TYPEINFO list.
    # Name offsets are relative to the beginning of the resource table.
    names_start_relative = 2
    for spec in normalised:
        names_start_relative += 8 + 12 * len(spec["resources"])
    names_start_relative += 2
    name_area = bytearray(b"\x00")
    name_offsets: dict[tuple, int] = {}

    def add_name(key: tuple, text: str) -> None:
        if key not in name_offsets:
            name_offsets[key] = names_start_relative + len(name_area)
            name_area.extend(_pascal(text))

    for type_index, spec in enumerate(normalised):
        if spec["ordinal"] is None:
            add_name(("type", type_index), spec["name"])
        for resource_index, resource in enumerate(spec["resources"]):
            if resource["id"] is None:
                add_name(("resource", type_index, resource_index), resource["name"])

    table_size = names_start_relative + len(name_area)

    # Table placement.
    segment_table_offset = ne_offset + 64
    resource_table_offset = segment_table_offset + 8 * len(segments)
    resident_names_offset = resource_table_offset + table_size

    resident = bytearray()
    resident += _pascal(module_name)
    resident += struct.pack("<H", 0)
    for ordinal, name in enumerate(resident_extra, start=1):
        resident += _pascal(name)
        resident += struct.pack("<H", ordinal)
    resident += b"\x00"
    resident_first_string_offset = resident_names_offset

    module_ref_table_offset = resident_names_offset + len(resident)
    imported_names_offset = module_ref_table_offset + 2 * len(module_references)

    imported = bytearray(b"\x00")
    module_ref_values: list[int] = []
    for name in module_references:
        module_ref_values.append(len(imported))
        imported += _pascal(name)
    imported += b"\x00"

    entry_table_offset = imported_names_offset + len(imported)
    entry = bytearray()
    if entry_count:
        entry += bytes([entry_count, 0xFF])
        for index in range(entry_count):
            entry += bytes([0x01, 0xCD, 0x3F, 0x01])
            entry += struct.pack("<H", index * 4)
        entry += b"\x00"
    entry_table_length = len(entry)

    nonresident_names_offset = entry_table_offset + len(entry)
    nonresident = bytearray()
    if nonresident_description is not None:
        nonresident += _pascal(nonresident_description)
        nonresident += struct.pack("<H", 0)
    nonresident += b"\x00"
    nonresident_names_size = len(nonresident)

    cursor = _align(nonresident_names_offset + len(nonresident), unit)

    segment_layout: list[dict] = []
    for payload in segments:
        sector = cursor >> shift
        segment_layout.append(
            {
                "offset": cursor,
                "sector": sector,
                "length_raw": len(payload),
                "payload": payload,
            }
        )
        cursor = _align(cursor + len(payload), unit)

    resource_layout: list[dict] = []
    for type_index, spec in enumerate(normalised):
        for resource_index, resource in enumerate(spec["resources"]):
            payload = resource["data"]
            padded_length = _align(len(payload), unit)
            sector = cursor >> shift
            resource_layout.append(
                {
                    "type_index": type_index,
                    "resource_index": resource_index,
                    "offset": cursor,
                    "sector": sector,
                    "sector_length": padded_length >> shift,
                    "byte_length": padded_length,
                    "payload": payload,
                }
            )
            cursor = _align(cursor + len(payload), unit)

    total_size = cursor

    # Segment table.
    segment_table = bytearray()
    for item in segment_layout:
        segment_table += struct.pack(
            "<HHHH",
            item["sector"],
            item["length_raw"],
            0x1D50,
            len(item["payload"]),
        )

    # Resource table.
    resource_table = bytearray(struct.pack("<H", shift))
    resource_entry_offsets: list[int] = []
    for type_index, spec in enumerate(normalised):
        if spec["ordinal"] is None:
            type_id = name_offsets[("type", type_index)]
        else:
            type_id = 0x8000 | spec["ordinal"]
        resource_table += struct.pack("<HHI", type_id, len(spec["resources"]), 0)
        for resource_index, resource in enumerate(spec["resources"]):
            item = next(
                entry
                for entry in resource_layout
                if entry["type_index"] == type_index
                and entry["resource_index"] == resource_index
            )
            if resource["id"] is None:
                id_raw = name_offsets[("resource", type_index, resource_index)]
            else:
                id_raw = 0x8000 | resource["id"]
            resource_entry_offsets.append(
                resource_table_offset + len(resource_table)
            )
            resource_table += struct.pack(
                "<HHHHHH",
                item["sector"],
                item["sector_length"],
                resource["flags"],
                id_raw,
                0,
                0,
            )
    resource_table += struct.pack("<H", 0)
    assert len(resource_table) == names_start_relative, (
        len(resource_table),
        names_start_relative,
    )
    resource_table += name_area

    # NE header.
    header = bytearray(64)
    header[0:2] = b"NE"
    header[2] = 5
    header[3] = 60
    struct.pack_into("<H", header, 0x04, entry_table_offset - ne_offset)
    struct.pack_into("<H", header, 0x06, entry_table_length)
    struct.pack_into("<I", header, 0x08, 0)
    struct.pack_into("<H", header, 0x0C, 0x0302)
    struct.pack_into("<H", header, 0x0E, 1)
    struct.pack_into("<H", header, 0x10, 0x1000)
    struct.pack_into("<H", header, 0x12, 0x2000)
    struct.pack_into("<I", header, 0x14, (1 << 16) | 0x0010)
    struct.pack_into("<I", header, 0x18, (2 << 16) | 0x0100)
    struct.pack_into("<H", header, 0x1C, len(segments))
    struct.pack_into("<H", header, 0x1E, len(module_references))
    struct.pack_into("<H", header, 0x20, nonresident_names_size)
    struct.pack_into("<H", header, 0x22, segment_table_offset - ne_offset)
    struct.pack_into("<H", header, 0x24, resource_table_offset - ne_offset)
    struct.pack_into("<H", header, 0x26, resident_names_offset - ne_offset)
    struct.pack_into("<H", header, 0x28, module_ref_table_offset - ne_offset)
    struct.pack_into("<H", header, 0x2A, imported_names_offset - ne_offset)
    struct.pack_into("<I", header, 0x2C, nonresident_names_offset)
    struct.pack_into("<H", header, 0x30, entry_count)
    struct.pack_into("<H", header, 0x32, shift)
    struct.pack_into("<H", header, 0x34, 0)
    header[0x36] = 2
    header[0x37] = 0x08
    struct.pack_into("<H", header, 0x38, 0)
    struct.pack_into("<H", header, 0x3A, 0)
    struct.pack_into("<H", header, 0x3C, 0)
    struct.pack_into("<H", header, 0x3E, 0x030A)

    image = bytearray(total_size)
    image[0:2] = b"MZ"
    struct.pack_into("<I", image, 0x3C, ne_offset)
    image[ne_offset : ne_offset + 64] = header
    image[segment_table_offset : segment_table_offset + len(segment_table)] = (
        segment_table
    )
    image[resource_table_offset : resource_table_offset + len(resource_table)] = (
        resource_table
    )
    image[resident_names_offset : resident_names_offset + len(resident)] = resident
    image[module_ref_table_offset : module_ref_table_offset + len(module_ref_values) * 2] = b"".join(
        struct.pack("<H", value) for value in module_ref_values
    )
    image[imported_names_offset : imported_names_offset + len(imported)] = imported
    image[entry_table_offset : entry_table_offset + len(entry)] = entry
    image[
        nonresident_names_offset : nonresident_names_offset + len(nonresident)
    ] = nonresident
    for item in segment_layout:
        image[item["offset"] : item["offset"] + len(item["payload"])] = item["payload"]
    for item in resource_layout:
        image[item["offset"] : item["offset"] + len(item["payload"])] = item["payload"]

    layout = {
        "ne_offset": ne_offset,
        "ne_header": ne_offset,
        "segment_table_offset": segment_table_offset,
        "resource_table_offset": resource_table_offset,
        "resident_names_offset": resident_names_offset,
        "resident_first_string_offset": resident_first_string_offset,
        "module_ref_table_offset": module_ref_table_offset,
        "imported_names_offset": imported_names_offset,
        "entry_table_offset": entry_table_offset,
        "nonresident_names_offset": nonresident_names_offset,
        "segment_entries": [segment_table_offset + index * 8 for index in range(len(segments))],
        "resource_entries": resource_entry_offsets,
        "resource_data": resource_layout,
        "segment_data": segment_layout,
        "size": total_size,
    }
    return bytes(image), layout


def build_minimal_ne() -> tuple[bytes, dict]:
    return _build(
        segments=[],
        resource_types=[],
        resident_names=["MINI"],
        module_references=[],
        nonresident_description=None,
        entry_count=0,
    )


def build_rich_ne() -> tuple[bytes, dict]:
    valid_dialog = _dialog_template(
        "Custom Race Wizard",
        [
            ("Race Name:", 1001, 0x82),
            ("Primary Racial Trait", 1002, 0x82),
            ("&Customize Race...", 1003, 0x80),
        ],
    )
    second_dialog = _dialog_template(
        "Race Traits",
        [("Lesser Racial Traits", 1101, 0x82)],
    )
    malformed_dialog = struct.pack("<IBHHHH", 0x00C800C0, 3, 1, 2, 3, 4)
    malformed_dialog += b"\x00\x00Broken\x00"
    resource_types = [
        {
            "ordinal": 5,
            "resources": [
                {"id": 100, "data": valid_dialog},
                {"id": 101, "data": second_dialog},
                {"id": 102, "data": malformed_dialog},
                {
                    "name": "RACEWIZARDDLG1",
                    "data": _dialog_template(
                        "Custom Race",
                        [("Turindrones", 1201, 0x82)],
                    ),
                },
            ],
        },
        {
            "name": "CUSTOMTYPE",
            "resources": [
                {
                    "name": "CUSTOMRES",
                    "data": b"custom resource payload\x00",
                }
            ],
        },
        {
            "ordinal": 6,
            "resources": [
                {
                    "id": 7,
                    "data": b"\x00\x00Custom Race Wizard\x00Primary Racial Trait\x00",
                }
            ],
        },
        {
            "ordinal": 10000,
            "resources": [
                {"id": 1, "data": b"Turindrone custom race data\x00"},
            ],
        },
    ]
    segments = [
        b"\x90" * 16 + b"Turindrones\x00" + b"\x90" * 16,
        b"Primary Racial Trait\x00" + b"\x00" * 8,
    ]
    return _build(
        segments=segments,
        resource_types=resource_types,
        resident_names=["STARS!", "BATTLEPLANSDLG", "RACEWIZARDDLG1"],
        module_references=["KERNEL", "USER"],
        nonresident_description="Synthetic Stars! inventory fixture",
        entry_count=3,
    )


# ---------------------------------------------------------------------------
# Valid parsing
# ---------------------------------------------------------------------------


class ValidParsingTests(unittest.TestCase):
    def test_minimal_headers(self) -> None:
        data, layout = build_minimal_ne()
        report = ne.build_report(data, input_path="minimal.exe")
        self.assertEqual(report["dos_header"]["magic"], "MZ")
        self.assertEqual(report["dos_header"]["e_lfanew"], layout["ne_offset"])
        self.assertEqual(report["ne_header"]["signature"], "NE")
        self.assertEqual(report["ne_header"]["segment_count"], 0)
        self.assertEqual(report["ne_header"]["expected_windows_version"]["text"], "3.10")
        self.assertEqual(report["resident_names"]["module_name"], "MINI")
        self.assertEqual(report["segments"]["count"], 0)
        self.assertEqual(report["resources"]["total_count"], 0)
        self.assertEqual(report["dialogs"]["count"], 0)
        self.assertEqual(report["entry_table"]["summary"]["entry_count"], 0)

    def test_rich_segments_resources_and_names(self) -> None:
        data, layout = build_rich_ne()
        report = ne.build_report(data, input_path="rich.exe")

        self.assertEqual(report["ne_header"]["segment_count"], 2)
        segments = report["segments"]["entries"]
        self.assertEqual(segments[0]["number"], 1)
        self.assertEqual(segments[0]["file_offset"], layout["segment_data"][0]["offset"])
        self.assertEqual(segments[0]["length"], len(layout["segment_data"][0]["payload"]))
        self.assertIn("MOVABLE", segments[0]["flag_names"])
        self.assertEqual(segments[0]["sector_offset"], layout["segment_data"][0]["sector"])
        self.assertEqual(segments[0]["address_space"], "segment-relative (segment number + offset)")

        resources = report["resources"]
        self.assertEqual(resources["total_count"], 7)
        self.assertEqual(resources["alignment_shift"], 6)
        self.assertEqual(resources["alignment_unit"], 64)
        self.assertEqual(resources["counts_by_type"]["DIALOG"], 4)
        self.assertEqual(resources["counts_by_type"]["CUSTOMTYPE"], 1)
        self.assertEqual(resources["counts_by_type"]["STRING"], 1)
        self.assertEqual(resources["counts_by_type"]["#10000"], 1)

        dialogs = [r for r in resources["types"] if r["type_name"] == "DIALOG"][0]
        first = dialogs["resources"][0]
        self.assertEqual(first["id"]["kind"], "integer")
        self.assertEqual(first["id"]["id"], 100)
        self.assertGreater(first["byte_length"], 0)
        self.assertEqual(first["file_offset"], layout["resource_data"][0]["offset"])
        string_id = dialogs["resources"][3]
        self.assertEqual(string_id["id"]["kind"], "string")
        self.assertEqual(string_id["id"]["name"], "RACEWIZARDDLG1")

        named_type = [r for r in resources["types"] if r["type_name"] == "CUSTOMTYPE"][0]
        self.assertEqual(named_type["type_ordinal"], None)
        named_resource = named_type["resources"][0]
        self.assertEqual(named_resource["id"]["kind"], "string")
        self.assertEqual(named_resource["id"]["name"], "CUSTOMRES")

        self.assertEqual(report["resident_names"]["module_name"], "STARS!")
        self.assertEqual(
            [entry["name"] for entry in report["resident_names"]["entries"]],
            ["STARS!", "BATTLEPLANSDLG", "RACEWIZARDDLG1"],
        )
        self.assertEqual(
            report["nonresident_names"]["description"],
            "Synthetic Stars! inventory fixture",
        )
        self.assertEqual(
            [ref["module_name"] for ref in report["imports"]["module_references"]],
            ["KERNEL", "USER"],
        )
        self.assertEqual(
            [entry["name"] for entry in report["imports"]["imports"]],
            ["KERNEL", "USER"],
        )
        self.assertEqual(report["entry_table"]["summary"]["entry_count"], 3)
        self.assertEqual(report["entry_table"]["summary"]["movable_count"], 3)
        self.assertEqual(report["entry_table"]["summary"]["exported_count"], 3)
        self.assertFalse(report["entry_table"]["summary"]["truncated"])

    def test_zero_segment_offset_and_min_alloc(self) -> None:
        data, layout = build_rich_ne()
        patched = bytearray(data)
        struct.pack_into("<H", patched, layout["segment_entries"][1], 0)
        struct.pack_into("<H", patched, layout["segment_entries"][1] + 2, 0)
        struct.pack_into("<H", patched, layout["segment_entries"][1] + 6, 0)
        report = ne.build_report(bytes(patched), input_path="patched.exe")
        segment = report["segments"]["entries"][1]
        self.assertIsNone(segment["file_offset"])
        self.assertEqual(segment["length"], 0)
        self.assertEqual(segment["min_alloc_raw"], 0)
        self.assertEqual(segment["min_alloc"], 0x10000)

    def test_dialog_templates_and_candidates(self) -> None:
        data, _layout = build_rich_ne()
        report = ne.build_report(data, input_path="rich.exe")
        dialogs = report["dialogs"]
        self.assertEqual(dialogs["count"], 4)
        self.assertEqual(dialogs["parsed_count"], 3)
        self.assertEqual(dialogs["parse_error_count"], 1)
        titles = {
            entry["title"] for entry in dialogs["resources"] if entry["parse_error"] is None
        }
        self.assertIn("Custom Race Wizard", titles)
        self.assertIn("Race Traits", titles)
        self.assertIn("Custom Race", titles)
        valid = [
            entry
            for entry in dialogs["resources"]
            if entry.get("title") == "Custom Race Wizard"
        ][0]
        self.assertIn("Primary Racial Trait", valid["captions"])
        self.assertIn("&Customize Race...", valid["captions"])
        self.assertEqual(valid["font"]["face"], "MS Sans Serif")
        errors = [
            entry for entry in dialogs["resources"] if entry["parse_error"] is not None
        ]
        self.assertEqual(len(errors), 1)
        self.assertIn("control", errors[0]["parse_error"])
        self.assertTrue(errors[0]["parse_error_offset"] > 0)
        self.assertGreaterEqual(len(dialogs["race_wizard_candidates"]), 3)

    def test_string_references_are_bounded(self) -> None:
        data, _layout = build_rich_ne()
        report = ne.build_report(data, input_path="rich.exe")
        strings = report["string_references"]
        self.assertEqual(
            strings["needles"],
            ["Turindrones", "Turindrone", "Race", "custom", "Primary Racial Trait"],
        )
        turindrones = [
            match
            for match in strings["matches"]
            if match["needle"] == "Turindrones"
        ]
        self.assertTrue(turindrones)
        segment_match = [m for m in turindrones if m["location_kind"] == "segment"][0]
        self.assertEqual(segment_match["segment_number"], 1)
        self.assertEqual(segment_match["segment_relative_offset"], 16)
        self.assertLessEqual(len(segment_match["window"]), 160)
        resource_match = [m for m in turindrones if m["location_kind"] == "resource"][0]
        self.assertIn(
            resource_match["resource"]["display"], {"#1", "RACEWIZARDDLG1"}
        )
        primary = [
            match
            for match in strings["matches"]
            if match["needle"] == "Primary Racial Trait"
        ]
        self.assertTrue(primary)
        for match in strings["matches"]:
            self.assertLessEqual(len(match["window"]), 160)

    def test_report_has_no_timestamps(self) -> None:
        data, _layout = build_rich_ne()
        report = ne.build_report(data, input_path="rich.exe")
        text = ne.file_sha256(data)
        self.assertEqual(report["input"]["sha256"], text)
        self.assertNotIn("timestamp", repr(report).lower())
        self.assertNotIn("generated_at", repr(report).lower())


# ---------------------------------------------------------------------------
# Malformed inputs
# ---------------------------------------------------------------------------


class MalformedInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.data, self.layout = build_rich_ne()

    def _parse(self, data: bytes) -> None:
        ne.build_report(data, input_path="malformed.exe")

    def _assert_error(self, data: bytes, *fragments: str) -> ne.NEFormatError:
        with self.assertRaises(ne.NEFormatError) as caught:
            self._parse(data)
        message = str(caught.exception).lower()
        for fragment in fragments:
            self.assertIn(fragment.lower(), message)
        self.assertGreaterEqual(caught.exception.offset, 0)
        return caught.exception

    def test_too_short_for_mz(self) -> None:
        self._assert_error(b"", "dos header", "mz signature")
        self._assert_error(b"MZ", "dos header", "at least 64 bytes")

    def test_bad_mz(self) -> None:
        data = bytearray(self.data)
        data[0:2] = b"ZM"
        self._assert_error(bytes(data), "mz signature", "dos header")

    def test_e_lfanew_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<I", data, 0x3C, len(data) + 10)
        self._assert_error(bytes(data), "e_lfanew", "past end")

    def test_missing_or_short_ne_header(self) -> None:
        data = bytearray(self.data[: self.layout["ne_offset"] + 10])
        struct.pack_into("<I", data, 0x3C, self.layout["ne_offset"])
        data[self.layout["ne_offset"] : self.layout["ne_offset"] + 2] = b"NE"
        self._assert_error(bytes(data), "64-byte ne header", "ne header")

    def test_bad_ne_magic(self) -> None:
        data = bytearray(self.data)
        data[self.layout["ne_offset"] : self.layout["ne_offset"] + 2] = b"PE"
        self._assert_error(bytes(data), "ne signature")

    def test_segment_table_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["ne_offset"] + 0x22, 0xFFFF)
        self._assert_error(bytes(data), "segment table", "file ends")

    def test_segment_data_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["segment_entries"][0] + 2, 0xFFFF)
        self._assert_error(bytes(data), "segment", "past end")

    def test_resource_table_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["ne_offset"] + 0x24, 0xFFFF)
        self._assert_error(bytes(data), "resource table", "file ends")

    def test_bad_resource_alignment(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["resource_table_offset"], 0)
        self._assert_error(bytes(data), "alignment_shift", "non-zero")
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["resource_table_offset"], 40)
        self._assert_error(bytes(data), "alignment_shift", "below 32")

    def test_resource_data_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["resource_entries"][0], 0xFFFF)
        self._assert_error(bytes(data), "resource data", "past end")

    def test_oversized_resource_length(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["resource_entries"][0] + 2, 0xFFFF)
        self._assert_error(bytes(data), "resource data", "oversized")

    def test_resident_name_table_outside_file(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["ne_offset"] + 0x26, 0xFFFF)
        self._assert_error(bytes(data), "resident-name table", "past end")

    def test_truncated_pascal_string(self) -> None:
        data = bytearray(self.data)
        data[self.layout["resident_first_string_offset"]] = 0xFF
        self._assert_error(bytes(data), "resident-name", "character")

    def test_module_reference_outside_imported_names(self) -> None:
        data = bytearray(self.data)
        struct.pack_into("<H", data, self.layout["module_ref_table_offset"], 0xFFFF)
        self._assert_error(bytes(data), "module-reference", "imported-name")

    def test_resource_name_offset_outside_table(self) -> None:
        data = bytearray(self.data)
        # First resource is an integer id; the named type entry is later.
        named_entry = None
        for offset in self.layout["resource_entries"]:
            id_raw = struct.unpack_from("<H", data, offset + 6)[0]
            if not id_raw & 0x8000:
                named_entry = offset
                break
        self.assertIsNotNone(named_entry)
        # Corrupt the separate named resource rather than the type offset.
        struct.pack_into("<H", data, named_entry + 6, 0x7FFE)
        self._assert_error(bytes(data), "resource-name", "file offset")

    def test_rejects_non_ne_input(self) -> None:
        self._assert_error(b"MZ" + b"\x00" * 62, "ne signature")


# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------


class DeterminismTests(unittest.TestCase):
    def test_same_input_same_report(self) -> None:
        data, _layout = build_rich_ne()
        first = ne.build_report(data, input_path="fixture.exe")
        second = ne.build_report(data, input_path="fixture.exe")
        self.assertEqual(first, second)

    def test_changed_input_changes_hash_and_report(self) -> None:
        data, _layout = build_rich_ne()
        first = ne.build_report(data, input_path="fixture.exe")
        changed = bytearray(data)
        changed[-1] ^= 0xFF
        second = ne.build_report(bytes(changed), input_path="fixture.exe")
        self.assertNotEqual(first["input"]["sha256"], second["input"]["sha256"])
        self.assertNotEqual(
            ne.file_sha256(data), ne.file_sha256(bytes(changed))
        )

    def test_reader_report_excludes_payloads(self) -> None:
        data, _layout = build_rich_ne()
        report = ne.build_report(data, input_path="fixture.exe")
        # Bounded string windows are allowed, but the segment/resource payload
        # bytes must not appear as complete blocks in the report.
        for item in report["segments"]["entries"]:
            self.assertNotIn("payload", item)
        for resource_type in report["resources"]["types"]:
            for resource in resource_type["resources"]:
                self.assertNotIn("payload", resource)
                self.assertNotIn("data", resource)


if __name__ == "__main__":
    unittest.main()
