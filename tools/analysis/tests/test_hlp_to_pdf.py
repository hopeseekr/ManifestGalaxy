from __future__ import annotations

import struct
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import hlp_to_pdf as pdf  # noqa: E402


def _c_u16(value: int) -> bytes:
    if value < 128:
        return bytes([value << 1])
    return struct.pack("<H", value | 1)


def _c_u32(value: int) -> bytes:
    if value < 32768:
        return struct.pack("<H", value << 1)
    return struct.pack("<I", value | 1)


def _picture_header(
    *,
    width: int,
    height: int,
    bit_count: int,
    colors: int,
    compressed_size: int,
    palette_size: int,
    picture_type: int = 6,
    packing: int = 1,
) -> tuple[bytes, int]:
    fields = b""
    fields += _c_u32(96)  # X dpi
    fields += _c_u32(96)  # Y dpi
    fields += _c_u16(1)  # planes
    fields += _c_u16(bit_count)
    fields += _c_u32(width)
    fields += _c_u32(height)
    fields += _c_u32(colors)
    fields += _c_u32(0)  # important colors
    fields += _c_u32(compressed_size)
    fields += _c_u32(0)  # hotspot size
    header_end = 2 + len(fields) + 8
    compressed_offset = header_end + palette_size
    offsets = struct.pack("<II", compressed_offset, 0)
    return bytes([picture_type, packing]) + fields + offsets, compressed_offset


def _synthetic_mrb() -> tuple[bytes, bytes]:
    # 4x2 pixel, 4bpp, 16-colour palette.  BMP rows are bottom-up.
    pixels = bytes([0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF])
    packed = bytes([0x80 | len(pixels)]) + pixels
    palette = b"".join(
        bytes([index * 16, index * 16, index * 16, 0]) for index in range(16)
    )
    header, _ = _picture_header(
        width=4,
        height=2,
        bit_count=4,
        colors=16,
        compressed_size=len(packed),
        palette_size=len(palette),
    )
    picture = header + palette + packed
    content = struct.pack("<HHI", 0x706C, 1, 8) + picture
    return content, packed


def _document_for(content: bytes) -> SimpleNamespace:
    used = len(content)
    file_header = struct.pack("<IIB", used, used, 0)
    info = SimpleNamespace(
        header=SimpleNamespace(
            offset=0, reserved=used, used=used, flags=0
        )
    )
    return SimpleNamespace(
        data=file_header + content,
        directory=SimpleNamespace(files={"|bm0": info}),
    )


class RunLengthTests(unittest.TestCase):
    def test_literal_and_repeat_runs_round_trip(self) -> None:
        encoded = b"\x83ABC\x02x"
        self.assertEqual(pdf._run_length_decode(encoded), b"ABCxx")

    def test_truncated_literal_is_rejected(self) -> None:
        with self.assertRaises(pdf.ConversionError):
            pdf._run_length_decode(b"\x85ab")

    def test_truncated_repeat_is_rejected(self) -> None:
        with self.assertRaises(pdf.ConversionError):
            pdf._run_length_decode(b"\x02")


class BitmapExtractionTests(unittest.TestCase):
    def test_extracts_dib_and_writes_png(self) -> None:
        content, _ = _synthetic_mrb()
        document = _document_for(content)
        with tempfile.TemporaryDirectory() as temporary:
            asset = pdf.extract_bitmap(document, "|bm0", Path(temporary))
            self.assertIsNotNone(asset)
            assert asset is not None
            self.assertEqual((asset.width, asset.height), (4, 2))
            self.assertTrue(asset.path.is_file())
            from PIL import Image

            with Image.open(asset.path) as image:
                self.assertEqual(image.size, (4, 2))
                self.assertEqual(image.convert("RGB").getpixel((0, 0)), (0x80, 0x80, 0x80))

    def test_missing_internal_file_returns_none(self) -> None:
        document = _document_for(b"")
        self.assertIsNone(pdf.extract_bitmap(document, "|bm99", Path(".")))

    def test_metafile_is_reported_as_unavailable(self) -> None:
        content = struct.pack("<HHI", 0x706C, 1, 8)
        content += bytes([8, 1])
        content += _c_u16(8)  # mapping mode
        content += struct.pack("<HH", 10, 10)
        content += _c_u32(100) + _c_u32(10) + _c_u32(0)
        content += struct.pack("<II", 40, 0)
        content += b"\x00" * 20
        document = _document_for(content)
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(pdf.ConversionError):
                pdf.extract_bitmap(document, "|bm0", Path(temporary))

    def test_out_of_range_picture_offset_is_rejected(self) -> None:
        content = struct.pack("<HHI", 0x706C, 1, 999)
        document = _document_for(content)
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(pdf.ConversionError):
                pdf.extract_bitmap(document, "|bm0", Path(temporary))


class MarkupTests(unittest.TestCase):
    def test_image_marker_regex(self) -> None:
        match = pdf.IMAGE_MARKER_RE.search("[[IMAGE |bm12 align=center]]")
        self.assertIsNotNone(match)
        assert match is not None
        self.assertEqual(match.group("number"), "12")
        self.assertEqual(match.group("align"), "center")

    def test_paragraph_markup_escapes_html_and_preserves_breaks(self) -> None:
        markup = pdf._paragraph_markup("a < b & c\nnext\tcell")
        self.assertIn("a &lt; b &amp; c", markup)
        self.assertIn("<br/>", markup)
        self.assertIn("    cell", markup)

    def test_control_characters_are_removed(self) -> None:
        markup = pdf._paragraph_markup("hello\x00\x07world")
        self.assertEqual(markup, "helloworld")


if __name__ == "__main__":
    unittest.main()
