#!/usr/bin/env python3
"""Render a private local PDF from a user-supplied Stars! WinHelp file.

This converter reads the supplied HLP, decodes its topic text and external
``|bmN`` pictures, and writes a PDF plus extracted PNGs under ``analysis/``.
It does not execute the original program and it never embeds the original HLP
or the generated PDF in Git.

The generated document is a local format-shift of copyrighted help content.
Keep it private unless the rights holder grants redistribution permission.
"""

from __future__ import annotations

import argparse
import io
import re
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

# The converter lives beside help.py so both invoked-as-script and package
# imports work.
sys.path.insert(0, str(Path(__file__).resolve().parent))

import help as hlp  # noqa: E402

try:
    from PIL import Image
except ImportError as exc:  # pragma: no cover - dependency boundary
    raise SystemExit(
        "Pillow is required for image extraction: python3 -m pip install Pillow"
    ) from exc

try:
    from reportlab.lib import colors
    from reportlab.lib.enums import TA_CENTER
    from reportlab.lib.pagesizes import A4
    from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
    from reportlab.lib.units import mm
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.platypus import (
        BaseDocTemplate,
        CondPageBreak,
        Frame,
        Image as RLImage,
        NextPageTemplate,
        PageBreak,
        PageTemplate,
        Paragraph,
        Spacer,
        Table,
        TableStyle,
    )
    from reportlab.platypus.tableofcontents import TableOfContents
except ImportError as exc:  # pragma: no cover - dependency boundary
    raise SystemExit(
        "ReportLab is required for PDF generation: python3 -m pip install reportlab"
    ) from exc


IMAGE_MARKER_RE = re.compile(
    r"\[\[IMAGE (?:\|bm(?P<number>\d+)|(?P<embedded>embedded)) "
    r"align=(?P<align>left|center|right)\]\]"
)
CONTROL_CHARS_RE = re.compile(r"[\x00-\x08\x0b\x0c\x0e-\x1f]")


class ConversionError(Exception):
    """A bounded conversion failure with an actionable message."""


@dataclass
class PictureRecord:
    picture_type: int
    packing: int
    width: int
    height: int
    bit_count: int
    colors: int
    important: int
    compressed_size: int
    compressed_offset: int
    header_end: int
    payload: bytes
    content: bytes

    @property
    def label(self) -> str:
        return f"type {self.picture_type}, {self.width}x{self.height}, {self.bit_count}bpp"


@dataclass
class BitmapAsset:
    name: str
    path: Path
    width: int
    height: int
    source: str


# ---------------------------------------------------------------------------
# Bitmap / MRB decoding
# ---------------------------------------------------------------------------


def _u16(data: bytes, offset: int, what: str) -> int:
    if offset < 0 or offset + 2 > len(data):
        raise ConversionError(f"truncated {what}")
    return struct.unpack_from("<H", data, offset)[0]


def _u32(data: bytes, offset: int, what: str) -> int:
    if offset < 0 or offset + 4 > len(data):
        raise ConversionError(f"truncated {what}")
    return struct.unpack_from("<I", data, offset)[0]


def _compressed_u16(data: bytes, cursor: int, what: str) -> tuple[int, int]:
    if cursor >= len(data):
        raise ConversionError(f"truncated {what}")
    if data[cursor] & 1:
        if cursor + 2 > len(data):
            raise ConversionError(f"truncated {what}")
        return struct.unpack_from("<H", data, cursor)[0] >> 1, cursor + 2
    return data[cursor] >> 1, cursor + 1


def _compressed_u32(data: bytes, cursor: int, what: str) -> tuple[int, int]:
    if cursor >= len(data):
        raise ConversionError(f"truncated {what}")
    if data[cursor] & 1:
        if cursor + 4 > len(data):
            raise ConversionError(f"truncated {what}")
        return struct.unpack_from("<I", data, cursor)[0] >> 1, cursor + 4
    if cursor + 2 > len(data):
        raise ConversionError(f"truncated {what}")
    return struct.unpack_from("<H", data, cursor)[0] >> 1, cursor + 2


def _run_length_decode(source: bytes) -> bytes:
    """Decode the WinHelp RunLen packing method."""

    output = bytearray()
    cursor = 0
    while cursor < len(source):
        control = source[cursor]
        cursor += 1
        if control & 0x80:
            count = control & 0x7F
            if cursor + count > len(source):
                raise ConversionError("truncated RunLen literal run")
            output.extend(source[cursor : cursor + count])
            cursor += count
        else:
            if cursor >= len(source):
                raise ConversionError("truncated RunLen repeat run")
            output.extend(bytes([source[cursor]]) * control)
            cursor += 1
    return bytes(output)


def _parse_picture_records(content: bytes, label: str) -> list[PictureRecord]:
    if len(content) < 8:
        raise ConversionError(f"{label}: MRB/SHG header is too short")
    magic = _u16(content, 0, f"{label} magic")
    count = _u16(content, 2, f"{label} picture count")
    if magic not in (0x506C, 0x706C):
        raise ConversionError(f"{label}: unexpected picture magic 0x{magic:04x}")
    if count < 1 or count > 64:
        raise ConversionError(f"{label}: unreasonable picture count {count}")
    if 4 + 4 * count > len(content):
        raise ConversionError(f"{label}: picture offset table is truncated")

    records: list[PictureRecord] = []
    for index in range(count):
        picture_offset = _u32(
            content, 4 + 4 * index, f"{label} picture {index} offset"
        )
        if picture_offset < 8 or picture_offset >= len(content):
            raise ConversionError(
                f"{label} picture {index}: offset {picture_offset} is outside the file"
            )
        payload = content[picture_offset:]
        if len(payload) < 2:
            raise ConversionError(f"{label} picture {index}: truncated type header")
        picture_type = payload[0]
        packing = payload[1]
        cursor = 2

        # Metafile records have a different header.  They are recorded as
        # unavailable rather than parsed with the bitmap layout.
        if picture_type == 8:
            _mapping_mode, cursor = _compressed_u16(payload, cursor, "mapping mode")
            if cursor + 4 > len(payload):
                raise ConversionError(f"{label} picture {index}: truncated metafile size")
            width = _u16(payload, cursor, "metafile width")
            height = _u16(payload, cursor + 2, "metafile height")
            cursor += 4
            _decompressed_size, cursor = _compressed_u32(
                payload, cursor, "metafile decompressed size"
            )
            compressed_size, cursor = _compressed_u32(
                payload, cursor, "metafile compressed size"
            )
            _hotspot_size, cursor = _compressed_u32(
                payload, cursor, "metafile hotspot size"
            )
            if cursor + 8 > len(payload):
                raise ConversionError(f"{label} picture {index}: truncated metafile offsets")
            compressed_offset = _u32(payload, cursor, "metafile data offset")
            cursor += 8
            records.append(
                PictureRecord(
                    picture_type=picture_type,
                    packing=packing,
                    width=width,
                    height=height,
                    bit_count=0,
                    colors=0,
                    important=0,
                    compressed_size=compressed_size,
                    compressed_offset=compressed_offset,
                    header_end=cursor,
                    payload=payload,
                    content=content,
                )
            )
            continue

        if picture_type not in (5, 6):
            records.append(
                PictureRecord(
                    picture_type=picture_type,
                    packing=packing,
                    width=0,
                    height=0,
                    bit_count=0,
                    colors=0,
                    important=0,
                    compressed_size=0,
                    compressed_offset=0,
                    header_end=cursor,
                    payload=payload,
                    content=content,
                )
            )
            continue

        _xdpi, cursor = _compressed_u32(payload, cursor, "X dpi")
        _ydpi, cursor = _compressed_u32(payload, cursor, "Y dpi")
        _planes, cursor = _compressed_u16(payload, cursor, "planes")
        bit_count, cursor = _compressed_u16(payload, cursor, "bit count")
        width, cursor = _compressed_u32(payload, cursor, "width")
        height, cursor = _compressed_u32(payload, cursor, "height")
        colors, cursor = _compressed_u32(payload, cursor, "colors used")
        important, cursor = _compressed_u32(payload, cursor, "important colors")
        compressed_size, cursor = _compressed_u32(
            payload, cursor, "compressed size"
        )
        _hotspot_size, cursor = _compressed_u32(payload, cursor, "hotspot size")
        if cursor + 8 > len(payload):
            raise ConversionError(f"{label} picture {index}: truncated offsets")
        compressed_offset = _u32(payload, cursor, "compressed-data offset")
        cursor += 4
        _hotspot_offset = _u32(payload, cursor, "hotspot offset")
        cursor += 4
        if compressed_offset >= len(payload) or compressed_size > len(payload):
            raise ConversionError(
                f"{label} picture {index}: compressed data is outside the picture"
            )
        if compressed_offset + compressed_size > len(payload):
            raise ConversionError(
                f"{label} picture {index}: compressed data is truncated"
            )
        records.append(
            PictureRecord(
                picture_type=picture_type,
                packing=packing,
                width=width,
                height=height,
                bit_count=bit_count,
                colors=colors,
                important=important,
                compressed_size=compressed_size,
                compressed_offset=compressed_offset,
                header_end=cursor,
                payload=payload,
                content=content,
            )
        )
    return records


def _decode_picture(record: PictureRecord, label: str) -> Image.Image:
    if record.picture_type != 6:
        raise ConversionError(
            f"{label}: picture type {record.picture_type} is not a DIB bitmap"
        )
    if record.bit_count not in (1, 4, 8, 24, 32):
        raise ConversionError(
            f"{label}: unsupported DIB bit count {record.bit_count}"
        )
    if not (0 < record.width <= 20_000 and 0 < record.height <= 20_000):
        raise ConversionError(
            f"{label}: unreasonable DIB dimensions {record.width}x{record.height}"
        )
    if record.colors == 0:
        palette_entries = 1 << record.bit_count if record.bit_count <= 8 else 0
    else:
        palette_entries = record.colors
    if palette_entries < 0 or palette_entries > 256:
        raise ConversionError(
            f"{label}: unreasonable palette size {palette_entries}"
        )
    palette_end = record.header_end + palette_entries * 4
    if palette_end > len(record.payload):
        raise ConversionError(f"{label}: palette is truncated")
    palette = record.payload[record.header_end : palette_end]
    compressed = record.payload[
        record.compressed_offset : record.compressed_offset + record.compressed_size
    ]
    if record.packing == 0:
        pixels = compressed
    elif record.packing == 1:
        pixels = _run_length_decode(compressed)
    elif record.packing == 2:
        pixels = hlp.lz77_decompress(
            compressed,
            path=label,
            source_offset=0,
            context=label,
            max_output=64 * 1024 * 1024,
        )
    elif record.packing == 3:
        intermediate = hlp.lz77_decompress(
            compressed,
            path=label,
            source_offset=0,
            context=label,
            max_output=64 * 1024 * 1024,
        )
        pixels = _run_length_decode(intermediate)
    else:
        raise ConversionError(f"{label}: unsupported packing method {record.packing}")

    stride = ((record.width * record.bit_count + 31) // 32) * 4
    expected = stride * record.height
    if len(pixels) != expected:
        raise ConversionError(
            f"{label}: decoded {len(pixels)} pixel bytes, expected {expected}"
        )
    header = struct.pack(
        "<IiiHHIIiiII",
        40,
        record.width,
        record.height,
        1,
        record.bit_count,
        0,
        len(pixels),
        0,
        0,
        palette_entries,
        0,
    )
    dib = header + palette + pixels
    off_bits = 14 + len(header) + len(palette)
    bmp = b"BM" + struct.pack("<IHHI", 14 + len(dib), 0, 0, off_bits) + dib
    try:
        image = Image.open(io.BytesIO(bmp))
        image.load()
        return image
    except Exception as exc:  # PIL raises several exception types
        raise ConversionError(f"{label}: PIL could not decode the DIB: {exc}") from exc


def extract_bitmap(
    document: hlp.HelpDocument, name: str, output_dir: Path
) -> Optional[BitmapAsset]:
    """Extract the best-resolution supported picture from internal file *name*."""

    info = document.directory.files.get(name)
    if info is None:
        return None
    offset = info.header.offset
    used = info.header.used
    if offset + 9 + used > len(document.data):
        raise ConversionError(f"{name}: internal file is truncated")
    content = document.data[offset + 9 : offset + 9 + used]
    records = _parse_picture_records(content, name)
    candidates: list[PictureRecord] = []
    for record in records:
        if record.picture_type != 6:
            continue
        if record.bit_count not in (1, 4, 8, 24, 32):
            continue
        candidates.append(record)
    candidates.sort(key=lambda item: item.width * item.height, reverse=True)
    errors: list[str] = []
    for record in candidates:
        try:
            image = _decode_picture(record, name)
        except ConversionError as exc:
            errors.append(str(exc))
            continue
        output_dir.mkdir(parents=True, exist_ok=True)
        path = output_dir / f"{name.lstrip('|')}.png"
        image.save(path)
        return BitmapAsset(
            name=name,
            path=path,
            width=image.width,
            height=image.height,
            source=record.label,
        )
    detail = f" ({'; '.join(errors)})" if errors else ""
    raise ConversionError(f"{name}: no supported DIB bitmap found{detail}")


# ---------------------------------------------------------------------------
# Text and PDF layout
# ---------------------------------------------------------------------------


def _escape_xml(text: str) -> str:
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\t", "    ")
    )


def _paragraph_markup(text: str) -> str:
    cleaned = CONTROL_CHARS_RE.sub("", text).replace("\r\n", "\n").replace("\r", "\n")
    cleaned = cleaned.strip("\n")
    if not cleaned:
        return ""
    return _escape_xml(cleaned).replace("\n", "<br/>")


def _register_pdf_fonts() -> tuple[str, str, str, str]:
    candidates = [
        (
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Oblique.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf",
        ),
        (
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Oblique.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-BoldOblique.ttf",
        ),
    ]
    for regular, bold, italic, bold_italic in candidates:
        if all(Path(path).is_file() for path in (regular, bold, italic, bold_italic)):
            pdfmetrics.registerFont(TTFont("GuideSans", regular))
            pdfmetrics.registerFont(TTFont("GuideSans-Bold", bold))
            pdfmetrics.registerFont(TTFont("GuideSans-Italic", italic))
            pdfmetrics.registerFont(TTFont("GuideSans-BoldItalic", bold_italic))
            pdfmetrics.registerFontFamily(
                "GuideSans",
                normal="GuideSans",
                bold="GuideSans-Bold",
                italic="GuideSans-Italic",
                boldItalic="GuideSans-BoldItalic",
            )
            return (
                "GuideSans",
                "GuideSans-Bold",
                "GuideSans-Italic",
                "GuideSans-BoldItalic",
            )
    return "Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique"


def _build_styles(fonts: tuple[str, str, str, str]) -> dict[str, ParagraphStyle]:
    regular, bold, italic, bold_italic = fonts
    styles = getSampleStyleSheet()
    return {
        "cover_title": ParagraphStyle(
            "CoverTitle",
            parent=styles["Title"],
            fontName=bold,
            fontSize=28,
            leading=34,
            alignment=TA_CENTER,
            spaceAfter=10,
        ),
        "cover_sub": ParagraphStyle(
            "CoverSub",
            parent=styles["Normal"],
            fontName=regular,
            fontSize=12,
            leading=17,
            alignment=TA_CENTER,
            textColor=colors.HexColor("#38424f"),
        ),
        "heading": ParagraphStyle(
            "TopicHeading",
            parent=styles["Heading1"],
            fontName=bold,
            fontSize=16,
            leading=20,
            spaceBefore=10,
            spaceAfter=6,
            textColor=colors.HexColor("#1f3348"),
            keepWithNext=True,
        ),
        "body": ParagraphStyle(
            "Body",
            parent=styles["BodyText"],
            fontName=regular,
            fontSize=9.4,
            leading=13.2,
            spaceAfter=4,
            textColor=colors.HexColor("#111111"),
        ),
        "small": ParagraphStyle(
            "Small",
            parent=styles["BodyText"],
            fontName=regular,
            fontSize=8,
            leading=11,
            textColor=colors.HexColor("#555555"),
        ),
        "toc": ParagraphStyle(
            "TOC",
            parent=styles["Normal"],
            fontName=regular,
            fontSize=8.4,
            leading=10.6,
            spaceAfter=0,
        ),
        "notice": ParagraphStyle(
            "Notice",
            parent=styles["BodyText"],
            fontName=regular,
            fontSize=9,
            leading=13,
            textColor=colors.HexColor("#333333"),
            backColor=colors.HexColor("#f2f4f6"),
            borderColor=colors.HexColor("#c8ced6"),
            borderWidth=0.5,
            borderPadding=8,
            spaceBefore=8,
            spaceAfter=8,
        ),
    }


class GuideDocTemplate(BaseDocTemplate):
    """DocTemplate that records topic headings for the TOC and PDF outline."""

    def __init__(self, filename: str, **kwargs: object) -> None:
        super().__init__(filename, **kwargs)
        self._topic_keys: dict[int, str] = {}

    def afterFlowable(self, flowable: object) -> None:
        if not isinstance(flowable, Paragraph):
            return
        topic_index = getattr(flowable, "_topic_index", None)
        if topic_index is None:
            return
        title = flowable.getPlainText()
        key = f"topic-{topic_index}"
        self._topic_keys[topic_index] = key
        self.canv.bookmarkPage(key)
        self.canv.addOutlineEntry(title[:120], key, level=0, closed=True)
        self.notify("TOCEntry", (0, title, self.page, key))


def _make_doc(
    output: Path,
    fonts: tuple[str, str, str, str],
    title: str,
    source_hash: str,
) -> GuideDocTemplate:
    regular, bold, _italic, _bold_italic = fonts
    page_width, page_height = A4
    left = right = 18 * mm
    top = 17 * mm
    bottom = 18 * mm
    frame = Frame(
        left,
        bottom,
        page_width - left - right,
        page_height - top - bottom,
        id="body",
        leftPadding=0,
        rightPadding=0,
        topPadding=0,
        bottomPadding=0,
    )

    def footer(canvas, doc) -> None:
        canvas.saveState()
        canvas.setFont(regular, 7)
        canvas.setFillColor(colors.HexColor("#666666"))
        canvas.drawString(left, 10 * mm, "Stars! Player's Guide — private local copy")
        canvas.drawRightString(page_width - right, 10 * mm, f"Page {doc.page}")
        canvas.restoreState()

    def cover_page(canvas, doc) -> None:
        canvas.saveState()
        canvas.setStrokeColor(colors.HexColor("#1f3348"))
        canvas.setLineWidth(2)
        canvas.line(left, 38 * mm, page_width - right, 38 * mm)
        canvas.setFont(regular, 7)
        canvas.setFillColor(colors.HexColor("#666666"))
        canvas.drawString(left, 30 * mm, f"Source SHA-256 {source_hash}")
        canvas.restoreState()

    doc = GuideDocTemplate(
        str(output),
        pagesize=A4,
        leftMargin=left,
        rightMargin=right,
        topMargin=top,
        bottomMargin=bottom,
        title=title,
        author="Mare Crisium, LLC (source); generated locally",
        subject="Private local conversion of the Stars! Player's Guide",
        creator="tools/analysis/hlp_to_pdf.py",
    )
    # The cover is the first page and uses a separate template without the
    # ordinary footer.  The body template takes over after the cover.
    cover_frame = Frame(
        left,
        40 * mm,
        page_width - left - right,
        page_height - 40 * mm - 30 * mm,
        id="cover",
        leftPadding=0,
        rightPadding=0,
        topPadding=0,
        bottomPadding=0,
    )
    doc.addPageTemplates(
        [
            PageTemplate(id="cover", frames=[cover_frame], onPage=cover_page),
            PageTemplate(id="main", frames=[frame], onPage=footer),
        ]
    )
    return doc


def _image_flowable(
    asset: BitmapAsset,
    align: str,
    max_width: float,
) -> list[object]:
    natural_width = asset.width * 72.0 / 96.0
    natural_height = asset.height * 72.0 / 96.0
    width = min(natural_width, max_width)
    height = natural_height * (width / natural_width) if natural_width else natural_height
    # Keep very tall screenshots from taking more than a page by themselves.
    max_height = 225 * mm
    if height > max_height:
        width *= max_height / height
        height = max_height
    image = RLImage(str(asset.path), width=width, height=height)
    image.hAlign = {"left": "LEFT", "center": "CENTER", "right": "RIGHT"}[align]
    return [Spacer(1, 3), image, Spacer(1, 7)]


def _topic_story(
    document: hlp.HelpDocument,
    topic_index: int,
    entry: hlp.TopicIndexEntry,
    assets: dict[str, Optional[BitmapAsset]],
    styles: dict[str, ParagraphStyle],
    frame_width: float,
) -> list[object]:
    story: list[object] = [CondPageBreak(55)]
    heading_text = entry.title.strip() or f"Untitled topic {topic_index + 1}"
    heading = Paragraph(
        f'<a name="topic-{topic_index}"/>{_escape_xml(heading_text)}',
        styles["heading"],
    )
    heading._topic_index = topic_index  # type: ignore[attr-defined]
    story.append(heading)
    story.append(
        Paragraph(
            f'<font color="#6b7785" size="7.5">Topic {topic_index + 1} of '
            f'{document.topics.count} · TOPICPOS {entry.topic_offset} · '
            f'HLP offset 0x{entry.file_offset:x}</font>',
            styles["small"],
        )
    )
    story.append(Spacer(1, 5))

    text = entry.text
    cursor = 0
    for match in IMAGE_MARKER_RE.finditer(text):
        chunk = text[cursor : match.start()]
        markup = _paragraph_markup(chunk)
        if markup:
            story.append(Paragraph(markup, styles["body"]))
        cursor = match.end()
        number = match.group("number")
        align = match.group("align")
        if match.group("embedded"):
            story.append(
                Paragraph(
                    "<i>[Embedded help picture was not extracted; see the "
                    "conversion notes.]</i>",
                    styles["small"],
                )
            )
            continue
        asset = assets.get(f"|bm{number}")
        if asset is None:
            story.append(
                Paragraph(
                    f"<i>[Help image |bm{number} was not decoded; see the "
                    "conversion notes.]</i>",
                    styles["small"],
                )
            )
            continue
        story.extend(_image_flowable(asset, align, frame_width))
    tail = text[cursor:]
    markup = _paragraph_markup(tail)
    if markup:
        story.append(Paragraph(markup, styles["body"]))
    if not text.strip():
        story.append(Paragraph("<i>(No decoded body text for this topic.)</i>", styles["small"]))
    return story


def _build_pdf(
    document: hlp.HelpDocument,
    output: Path,
    image_dir: Path,
    include_images: bool,
) -> dict[str, object]:
    fonts = _register_pdf_fonts()
    styles = _build_styles(fonts)
    title = document.system.title or "Stars! Player's Guide"
    copyright_text = document.system.copyright or "Copyright status unknown"
    doc = _make_doc(output, fonts, title, document.sha256)
    frame_width = A4[0] - 36 * mm

    assets: dict[str, Optional[BitmapAsset]] = {}
    image_errors: dict[str, str] = {}
    if include_images:
        names = sorted(
            (
                name
                for name in document.directory.files
                if name.startswith("|bm") and name[3:].isdigit()
            ),
            key=lambda name: int(name[3:]),
        )
        for name in names:
            try:
                assets[name] = extract_bitmap(document, name, image_dir)
            except ConversionError as exc:
                image_errors[name] = str(exc)
                assets[name] = None

    unavailable = [name for name, asset in assets.items() if asset is None]
    story: list[object] = [NextPageTemplate("cover")]
    story.append(Spacer(1, 22 * mm))
    story.append(Paragraph(_escape_xml(title), styles["cover_title"]))
    story.append(
        Paragraph(
            "Private local PDF conversion of the supplied Windows Help file",
            styles["cover_sub"],
        )
    )
    story.append(Spacer(1, 10 * mm))
    story.append(
        Paragraph(
            f"<b>Source:</b> {_escape_xml(document.path)}<br/>"
            f"<b>Source SHA-256:</b> {document.sha256}<br/>"
            f"<b>Help generation date:</b> {document.system.generated_utc}<br/>"
            f"<b>Topics:</b> {document.topics.count}<br/>"
            f"<b>Images:</b> {sum(1 for asset in assets.values() if asset)} extracted"
            + (f"; {len(unavailable)} unavailable" if unavailable else ""),
            styles["cover_sub"],
        )
    )
    story.append(Spacer(1, 12 * mm))
    story.append(
        Paragraph(
            f"<b>{_escape_xml(copyright_text)}</b><br/><br/>"
            "This file was generated locally from a user-supplied copy for "
            "private reference. The help text and images remain the property "
            "of their copyright holder. Do not redistribute this PDF or the "
            "extracted images without permission.",
            styles["notice"],
        )
    )
    story.append(NextPageTemplate("main"))
    story.append(PageBreak())

    story.append(Paragraph("About this conversion", styles["heading"]))
    story.append(
        Paragraph(
            "The source is a WinHelp 3.1 container. Topic text, titles, and "
            "external bitmap pictures were decoded without executing the "
            "original application. Paragraph formatting, table geometry, "
            "embedded pictures, hotspots, and metafile pictures are reproduced "
            "only where the converter supports them.",
            styles["body"],
        )
    )
    story.append(
        Paragraph(
            "Known limits: six topics use strings-only fallback for 18 display/"
            "table records; table cell geometry is approximate"
            + (
                f"; {len(unavailable)} metafile picture(s) are not rendered"
                if unavailable
                else ""
            )
            + ". All topic titles and supported DIB bitmaps were decoded.",
            styles["body"],
        )
    )
    if image_errors:
        rows = [["Image", "Reason"]]
        for name, reason in sorted(image_errors.items()):
            rows.append([name, reason[:240]])
        table = Table(rows, colWidths=[25 * mm, frame_width - 25 * mm])
        table.setStyle(
            TableStyle(
                [
                    ("FONTNAME", (0, 0), (-1, -1), fonts[0]),
                    ("FONTSIZE", (0, 0), (-1, -1), 7),
                    ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#e7ebef")),
                    ("GRID", (0, 0), (-1, -1), 0.25, colors.HexColor("#b9c1ca")),
                    ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ]
            )
        )
        story.append(Spacer(1, 6))
        story.append(table)
    story.append(PageBreak())

    toc = TableOfContents()
    toc.levelStyles = [styles["toc"]]
    toc.dotsMinLevel = 0
    story.append(Paragraph("Contents", styles["heading"]))
    story.append(toc)
    story.append(PageBreak())

    for index, entry in enumerate(document.topics.entries):
        story.extend(
            _topic_story(document, index, entry, assets, styles, frame_width)
        )

    output.parent.mkdir(parents=True, exist_ok=True)
    doc.multiBuild(story)
    return {
        "output": str(output),
        "topics": document.topics.count,
        "images_extracted": sum(1 for asset in assets.values() if asset),
        "image_errors": len(image_errors),
        "size": output.stat().st_size,
    }


def convert(
    input_path: Path,
    output_path: Path,
    image_dir: Path,
    *,
    include_images: bool = True,
) -> dict[str, object]:
    document = hlp.parse_help(input_path)
    if "|TOPIC" not in document.directory.files:
        raise ConversionError("help file has no |TOPIC internal file")
    return _build_pdf(document, output_path, image_dir, include_images)


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Generate a private local PDF from a WinHelp file. The PDF and "
            "extracted images stay under analysis/ and must not be redistributed "
            "without permission."
        )
    )
    parser.add_argument("input", type=Path, help="HLP file to convert")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("analysis/help/Stars-Player-Guide-1998.pdf"),
        help="PDF output path (default: analysis/help/Stars-Player-Guide-1998.pdf)",
    )
    parser.add_argument(
        "--image-dir",
        type=Path,
        default=Path("analysis/help/images"),
        help="directory for extracted PNGs (default: analysis/help/images)",
    )
    parser.add_argument(
        "--no-images",
        action="store_true",
        help="omit images (useful for a quick text-only PDF)",
    )
    args = parser.parse_args(argv)
    try:
        result = convert(
            args.input,
            args.output,
            args.image_dir,
            include_images=not args.no_images,
        )
    except (hlp.HLPFormatError, ConversionError, OSError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    print(
        f"wrote {result['output']} ({result['size']} bytes, "
        f"{result['topics']} topics, {result['images_extracted']} images, "
        f"{result['image_errors']} unavailable)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
