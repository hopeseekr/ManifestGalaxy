"""Tests for the bounded HLP inspector (tools/analysis/help.py).

All fixtures are synthetic and built in memory.  No original game payload is
read or committed.
"""

from __future__ import annotations

import json
import os
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import help  # noqa: E402


# ---------------------------------------------------------------------------
# Synthetic HLP builder
# ---------------------------------------------------------------------------


def _file_header(data: bytes) -> bytes:
    used = len(data)
    return struct.pack("<IIB", used + 9, used, 0) + data


def _system_data(
    title: str = "Synthetic Help",
    copyright_text: str = "Copyright 1998 Example",
    gen_date: int = 884558967,
    *,
    minor: int = 21,
    major: int = 1,
    flags: int = 0,
    raw_records: bool = False,
) -> bytes:
    header = struct.pack("<HHHIH", 0x036C, minor, major, gen_date, flags)
    if minor <= 16:
        return header + title.encode("cp1252") + b"\x00"
    if raw_records:
        return header
    records = bytearray()
    payload = title.encode("cp1252") + b"\x00"
    records += struct.pack("<HH", 1, len(payload)) + payload
    payload = copyright_text.encode("cp1252") + b"\x00"
    records += struct.pack("<HH", 2, len(payload)) + payload
    records += struct.pack("<HH", 3, 4) + struct.pack("<I", 0)
    return header + bytes(records)


def _lz77_literals(data: bytes) -> bytes:
    """LZ77 encode every byte as a literal (control byte 0 per 8 bytes)."""
    out = bytearray()
    for index in range(0, len(data), 8):
        out.append(0)
        out.extend(data[index : index + 8])
    return bytes(out)


def _topic_link_bytes(
    link_data1: bytes,
    link_data2_raw: bytes,
    data_len2: int,
    record_type: int,
    position: int,
    previous: int,
    next_position: int,
) -> bytes:
    data_len1 = 21 + len(link_data1)
    block_size = data_len1 + len(link_data2_raw)
    header = struct.pack(
        "<IIiiIB",
        block_size,
        data_len2,
        previous,
        next_position,
        data_len1,
        record_type,
    )
    return header + link_data1 + link_data2_raw


def _topic_data(
    links: list[dict],
    *,
    compressed: bool = False,
    block_size: int = 4096,
) -> bytes:
    """Build one |TOPIC physical block from link specifications.

    Each link dict has ``ld1`` (bytes), ``ld2`` (stored bytes), ``len2``
    (expanded length), ``type`` and optionally ``next_override``.
    """
    positions: list[int] = []
    cursor = 12
    for spec in links:
        positions.append(cursor)
        cursor += 21 + len(spec["ld1"]) + len(spec["ld2"])
    records = bytearray()
    last_header = 0
    for index, spec in enumerate(links):
        previous = positions[index - 1] if index else -1
        if "next_override" in spec:
            next_position = spec["next_override"]
        else:
            next_position = positions[index + 1] if index + 1 < len(links) else -1
        if spec["type"] == 0x02:
            last_header = positions[index]
        records += _topic_link_bytes(
            spec["ld1"],
            spec["ld2"],
            spec["len2"],
            spec["type"],
            positions[index],
            previous,
            next_position,
        )
    payload = bytes(records)
    if compressed:
        payload = _lz77_literals(payload)
    block_header = struct.pack("<III", positions[-1] if positions else -1, 12, last_header)
    return block_header + payload


def _title_link(title: str, **overrides) -> dict:
    return {
        "ld1": b"\x00" * 28,
        "ld2": title.encode("cp1252"),
        "len2": len(title.encode("cp1252")),
        "type": 0x02,
        **overrides,
    }


def _display_link(text: str) -> dict:
    # compressed long 0, compressed word 0, paragraph header 4 bytes, flags 0,
    # end-of-paragraph, end-of-character-formatting.
    link_data1 = b"\x80\x00\x00\x00\x00\x00\x00\x00\x00\x82\xff"
    link_data2 = text.encode("cp1252") + b"\x00\x00"
    return {
        "ld1": link_data1,
        "ld2": link_data2,
        "len2": len(link_data2),
        "type": 0x20,
    }


def _build_btree(entries: list[tuple[str, int]], *, page_size: int = 1024, leaf_capacity: int | None = None) -> bytes:
    ordered = sorted(entries, key=lambda item: item[0])
    if leaf_capacity is None:
        groups = [ordered]
    else:
        groups = [ordered[index : index + leaf_capacity] for index in range(0, len(ordered), leaf_capacity)]
    nodes: list[dict] = []
    pages: dict[int, bytes] = {}
    next_page = 0
    for index, group in enumerate(groups):
        nodes.append(
            {
                "page": next_page,
                "entries": group,
                "contains_pages": False,
                "prev": index - 1 if index > 0 else -1,
                "next": index + 1 if index + 1 < len(groups) else -1,
            }
        )
        next_page += 1
    levels: list[list[dict]] = [nodes]
    while len(levels[-1]) > 1:
        children = levels[-1]
        parents: list[dict] = []
        for index in range(0, len(children), 2):
            chunk = children[index : index + 2]
            parents.append(
                {
                    "page": next_page,
                    "entries": [(child["entries"][0][0], child["page"]) for child in chunk],
                    "contains_pages": True,
                    "prev": len(parents) - 1 if parents else -1,
                    "next": -1,
                }
            )
            next_page += 1
        levels.append(parents)
    all_nodes = [node for level in levels for node in level]
    for node in all_nodes:
        header = bytearray()
        body = bytearray()
        for name, value in node["entries"]:
            encoded = name.encode("cp1252")
            body += encoded + b"\x00"
            body += struct.pack("<h" if node["contains_pages"] else "<I", value)
        header_size = 6 if node["contains_pages"] else 8
        used = header_size + len(body)
        if used > page_size:
            raise AssertionError("test B+ tree page overflow")
        unused = page_size - used
        header += struct.pack("<Hh", unused, len(node["entries"]))
        header += struct.pack("<h", node["prev"])
        if not node["contains_pages"]:
            header += struct.pack("<h", node["next"])
        page = bytes(header) + bytes(body) + b"\x00" * unused
        if len(page) != page_size:
            raise AssertionError("test B+ tree page size mismatch")
        pages[node["page"]] = page
    root = levels[-1][0]["page"]
    total_pages = len(pages)
    btree_header = struct.pack(
        "<HHH16sHHhHhhI",
        0x293B,
        0x0402,
        page_size,
        b"z4\x00" + b"\x00" * 13,
        0,
        1,
        root,
        0xFFFF,
        total_pages,
        len(levels),
        len(ordered),
    )
    return btree_header + b"".join(pages[index] for index in range(total_pages))


def build_hlp(
    internal_files: list[tuple[str, bytes]],
    *,
    btree_page_size: int = 1024,
    leaf_capacity: int | None = None,
) -> bytes:
    output = bytearray(b"\x00" * 16)
    offsets: list[tuple[str, int]] = []
    for name, data in internal_files:
        offsets.append((name, len(output)))
        output += _file_header(data)
    directory = _build_btree(offsets, page_size=btree_page_size, leaf_capacity=leaf_capacity)
    directory_offset = len(output)
    output += _file_header(directory)
    struct.pack_into("<IIII", output, 0, 0x00035F3F, directory_offset, 0xFFFFFFFF, len(output))
    return bytes(output)


def base_hlp(
    *,
    title: str = "Synthetic Help",
    gen_date: int = 884558967,
    minor: int = 21,
    system_flags: int = 0,
    topic_links: list[dict] | None = None,
    compressed_topic: bool = False,
    extra_files: list[tuple[str, bytes]] | None = None,
    leaf_capacity: int | None = None,
    raw_system: bytes | None = None,
) -> bytes:
    if topic_links is None:
        topic_links = [_title_link("Test Topic"), _display_link("Hello synthetic world")]
    system = raw_system if raw_system is not None else _system_data(
        title=title, gen_date=gen_date, minor=minor, flags=system_flags
    )
    topic = _topic_data(topic_links, compressed=compressed_topic)
    files = [("|SYSTEM", system), ("|TOPIC", topic)]
    if extra_files:
        files.extend(extra_files)
    return build_hlp(files, leaf_capacity=leaf_capacity)


def _btree_offsets(data: bytes) -> tuple[int, int, int]:
    directory_start = struct.unpack_from("<I", data, 4)[0]
    btree = directory_start + 9
    page_size = struct.unpack_from("<H", data, btree + 4)[0]
    pages = btree + 38
    return btree, pages, page_size


def _first_leaf_offset(data: bytes) -> int:
    btree, pages, page_size = _btree_offsets(data)
    root = struct.unpack_from("<h", data, btree + 26)[0]
    levels = struct.unpack_from("<h", data, btree + 32)[0]
    page = root
    while levels > 1:
        page_offset = pages + page * page_size
        name_end = data.index(b"\x00", page_offset + 6)
        page = struct.unpack_from("<h", data, name_end + 1)[0]
        levels -= 1
    return pages + page * page_size


# ---------------------------------------------------------------------------
# Valid fixtures
# ---------------------------------------------------------------------------


class ValidParsingTests(unittest.TestCase):
    def test_header_directory_system_and_topics(self):
        data = base_hlp()
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "valid.hlp"
            path.write_bytes(data)
            document = help.parse_help(path)

        self.assertEqual(document.header.magic, help.HLP_MAGIC)
        self.assertEqual(document.header.declared_file_size, len(data))
        self.assertEqual(document.header.trailing_bytes, 0)
        self.assertEqual(document.system.title, "Synthetic Help")
        self.assertEqual(document.system.copyright, "Copyright 1998 Example")
        self.assertEqual(document.system.generated_utc, "1998-01-11T22:49:27Z")
        self.assertEqual(document.system.minor, 21)
        self.assertEqual(document.system.flags, 0)
        self.assertFalse(document.system.compressed)
        self.assertEqual(document.topics.count, 1)
        self.assertEqual(document.topics.entries[0].title, "Test Topic")
        self.assertIn("Hello synthetic world", document.topics.entries[0].text)
        self.assertEqual(set(document.directory.files), {"|SYSTEM", "|TOPIC"})
        self.assertEqual(document.directory.btree.total_entries, 2)

    def test_multi_page_index_and_leaf(self):
        extra = [(f"|dummy{index:02d}_long_name", struct.pack("<I", index)) for index in range(9)]
        data = base_hlp(leaf_capacity=3, extra_files=extra)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "multi.hlp"
            path.write_bytes(data)
            document = help.parse_help(path)
        self.assertGreaterEqual(document.directory.btree.leaf_pages, 3)
        self.assertGreaterEqual(document.directory.btree.index_pages, 2)
        self.assertGreaterEqual(document.directory.btree.n_levels, 3)
        self.assertIn("|dummy08_long_name", document.directory.files)
        self.assertEqual(document.directory.btree.entries_read, 11)

    def test_lz77_topic_block(self):
        links = [_title_link("Compressed Topic"), _display_link("LZ77 body text")]
        data = base_hlp(system_flags=4, topic_links=links, compressed_topic=True)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "compressed.hlp"
            path.write_bytes(data)
            document = help.parse_help(path)
        self.assertTrue(document.system.compressed)
        self.assertEqual(document.system.topic_block_size, 4096)
        self.assertEqual(document.topics.entries[0].title, "Compressed Topic")
        self.assertIn("LZ77 body text", document.topics.entries[0].text)

    def test_phrase_compressed_title_uses_phrases_table(self):
        phrases = [b"Phrase", b"Title"]
        offsets = [0, 6, 11]
        phrase_header = (
            struct.pack("<HH", len(phrases), 0x0100)
            + struct.pack("<I", len(b"".join(phrases)))
            + b"".join(struct.pack("<H", (len(phrases) + 1) * 2 + offset) for offset in offsets)
        )
        phrase_file = phrase_header + _lz77_literals(b"".join(phrases))
        # Phrase 0 followed by a space (odd number), phrase 1, then a literal.
        compressed_title = bytes([1, 1, 1, 2]) + b"!"
        expanded = b"Phrase Title!"
        links = [
            {
                "ld1": b"\x00" * 28,
                "ld2": compressed_title,
                "len2": len(expanded),
                "type": 0x02,
            }
        ]
        data = base_hlp(topic_links=links, extra_files=[("|Phrases", phrase_file)])
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "phrases.hlp"
            path.write_bytes(data)
            document = help.parse_help(path)
        self.assertEqual(document.phrase_style, "oldstyle")
        self.assertEqual(document.topics.entries[0].title, "Phrase Title!")

    def test_uncompressed_hc30_system(self):
        data = build_hlp(
            [
                ("|SYSTEM", _system_data(title="Old Help", gen_date=0, minor=16)),
                ("|TOPIC", _topic_data([_title_link("Old Topic")])),
            ]
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "hc30.hlp"
            path.write_bytes(data)
            document = help.parse_help(path)
        self.assertEqual(document.system.title, "Old Help")
        self.assertEqual(document.system.generated_utc, "1970-01-01T00:00:00Z")
        self.assertEqual(document.topics.entries[0].title, "Old Topic")

    def test_date_conversion_is_independent_of_real_file(self):
        self.assertEqual(help.gen_date_to_iso(0), "1970-01-01T00:00:00Z")
        self.assertEqual(help.gen_date_to_iso(884558967), "1998-01-11T22:49:27Z")
        self.assertEqual(help.gen_date_to_iso(1), "1970-01-01T00:00:01Z")

    def test_report_and_text_are_deterministic(self):
        data = base_hlp()
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "deterministic.hlp"
            path.write_bytes(data)
            first = help.build_cli_result(path)
            second = help.build_cli_result(path)
        first_json = json.dumps(first.report, sort_keys=True, indent=2)
        second_json = json.dumps(second.report, sort_keys=True, indent=2)
        self.assertEqual(first_json, second_json)
        self.assertEqual(first.text, second.text)
        self.assertNotIn("timestamp", first_json.lower())


# ---------------------------------------------------------------------------
# Malformed fixtures
# ---------------------------------------------------------------------------


class MalformedInputTests(unittest.TestCase):
    def _expect(self, data: bytes, fragment: str):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "broken.hlp"
            path.write_bytes(data)
            with self.assertRaises(help.HLPFormatError) as caught:
                help.parse_help(path)
        message = str(caught.exception)
        self.assertIn(fragment, message)
        self.assertIn("broken.hlp", message)
        return caught.exception

    def test_wrong_magic(self):
        data = bytearray(base_hlp())
        data[0:4] = b"BAD!"
        self._expect(bytes(data), "bad HLP magic")

    def test_truncated_header(self):
        self._expect(b"\x3f\x5f\x03\x00" + b"\x00" * 4, "truncated 16-byte HLP header")

    def test_directory_offset_out_of_bounds(self):
        data = bytearray(base_hlp())
        struct.pack_into("<I", data, 4, len(data) + 100)
        self._expect(bytes(data), "directory start")

    def test_truncated_fileheader(self):
        data = bytearray(base_hlp())
        directory_start = struct.unpack_from("<I", data, 4)[0]
        truncated = data[: directory_start + 4]
        struct.pack_into("<I", truncated, 12, len(truncated))
        self._expect(bytes(truncated), "FILEHEADER")

    def test_bad_btree_magic(self):
        data = bytearray(base_hlp())
        directory_start = struct.unpack_from("<I", data, 4)[0]
        struct.pack_into("<H", data, directory_start + 9, 0x1234)
        self._expect(bytes(data), "bad B+ tree magic")

    def test_bad_btree_page_size(self):
        data = bytearray(base_hlp())
        directory_start = struct.unpack_from("<I", data, 4)[0]
        struct.pack_into("<H", data, directory_start + 9 + 4, 1000)
        self._expect(bytes(data), "unsupported B+ tree page size")

    def test_bad_page_number(self):
        extra = [(f"|dummy{index:02d}", struct.pack("<I", index)) for index in range(5)]
        data = bytearray(base_hlp(leaf_capacity=2, extra_files=extra))
        btree, pages, page_size = _btree_offsets(data)
        root = struct.unpack_from("<h", data, btree + 26)[0]
        root_offset = pages + root * page_size
        name_end = data.index(b"\x00", root_offset + 6)
        struct.pack_into("<h", data, name_end + 1, 999)
        self._expect(bytes(data), "page number 999")

    def test_leaf_cycle(self):
        data = bytearray(base_hlp())
        leaf_offset = _first_leaf_offset(data)
        struct.pack_into("<h", data, leaf_offset + 6, 0)
        self._expect(bytes(data), "cycle")

    def test_index_cycle(self):
        extra = [(f"|dummy{index:02d}", struct.pack("<I", index)) for index in range(4)]
        data = bytearray(base_hlp(leaf_capacity=2, extra_files=extra))
        btree, pages, page_size = _btree_offsets(data)
        root = struct.unpack_from("<h", data, btree + 26)[0]
        root_offset = pages + root * page_size
        name_end = data.index(b"\x00", root_offset + 6)
        struct.pack_into("<h", data, name_end + 1, root)
        self._expect(bytes(data), "index cycle")

    def test_missing_nul_in_directory_string(self):
        data = bytearray(base_hlp())
        leaf_offset = _first_leaf_offset(data)
        # Keep the page header intact but replace all entry bytes with 'A'.
        page_size = 1024
        data[leaf_offset + 8 : leaf_offset + page_size] = b"A" * (page_size - 8)
        self._expect(bytes(data), "not NUL-terminated")

    def test_truncated_system(self):
        data = build_hlp(
            [
                ("|SYSTEM", b"\x6c\x03"),
                ("|TOPIC", _topic_data([_title_link("Topic")])),
            ]
        )
        self._expect(data, "truncated |SYSTEM header")

    def test_bad_system_record_length(self):
        system = struct.pack("<HHHIH", 0x036C, 21, 1, 0, 0) + struct.pack("<HH", 1, 100) + b"abc"
        data = build_hlp(
            [
                ("|SYSTEM", system),
                ("|TOPIC", _topic_data([_title_link("Topic")])),
            ]
        )
        self._expect(data, "runs past |SYSTEM end")

    def test_topic_link_cycle(self):
        links = [_title_link("Cycle", next_override=12)]
        data = base_hlp(topic_links=links)
        self._expect(data, "|TOPIC link cycle")

    def test_topic_link_out_of_bounds(self):
        links = [_title_link("Jump", next_override=0x7FFFFFFF)]
        data = base_hlp(topic_links=links)
        self._expect(data, "outside")


class LZ77Tests(unittest.TestCase):
    def test_truncated_match(self):
        with self.assertRaises(help.HLPFormatError) as caught:
            help.lz77_decompress(
                b"\x01",
                path="x.hlp",
                source_offset=0,
                context="test",
                max_output=100,
            )
        self.assertIn("truncated LZ77 match", str(caught.exception))

    def test_output_overrun(self):
        # First bit literal 'A'; second bit a match with length 18.
        source = b"\x02A\x00\xf0"
        with self.assertRaises(help.HLPFormatError) as caught:
            help.lz77_decompress(
                source,
                path="x.hlp",
                source_offset=0,
                context="test",
                max_output=5,
            )
        self.assertIn("exceeds limit", str(caught.exception))

    def test_invalid_back_reference(self):
        # First bit (and only bit reachable) is a match before any output.
        source = b"\x01\x00\x00"
        with self.assertRaises(help.HLPFormatError) as caught:
            help.lz77_decompress(
                source,
                path="x.hlp",
                source_offset=0,
                context="test",
                max_output=100,
            )
        self.assertIn("back-reference", str(caught.exception))

    def test_literal_round_trip(self):
        raw = b"Hello, synthetic LZ77!"
        self.assertEqual(
            help.lz77_decompress(
                _lz77_literals(raw),
                path="x.hlp",
                source_offset=0,
                context="test",
                max_output=len(raw),
                expected_output=len(raw),
            ),
            raw,
        )


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


class CliTests(unittest.TestCase):
    def test_cli_writes_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "input.hlp"
            source.write_bytes(base_hlp())
            report = root / "out" / "inventory.json"
            text = root / "out" / "topics.txt"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(Path(help.__file__)),
                    str(source),
                    "--output",
                    str(report),
                    "--text-output",
                    str(text),
                    "--quiet",
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report.exists())
            self.assertTrue(text.exists())
            parsed = json.loads(report.read_text(encoding="utf-8"))
            self.assertEqual(parsed["system"]["title"], "Synthetic Help")
            self.assertEqual(parsed["topics"]["count"], 1)
            self.assertIn("Hello synthetic world", text.read_text(encoding="utf-8"))

    def test_cli_malformed_input_produces_no_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "bad.hlp"
            source.write_bytes(b"not a help file")
            report = root / "inventory.json"
            text = root / "topics.txt"
            completed = subprocess.run(
                [
                    sys.executable,
                    str(Path(help.__file__)),
                    str(source),
                    "--output",
                    str(report),
                    "--text-output",
                    str(text),
                ],
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("truncated 16-byte HLP header", completed.stderr)
            self.assertFalse(report.exists())
            self.assertFalse(text.exists())


if __name__ == "__main__":
    unittest.main()
