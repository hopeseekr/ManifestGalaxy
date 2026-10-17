#!/usr/bin/env python3
"""Deterministic NE evidence inventory CLI.

Usage:
    python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json

The report is rebuilt from scratch on each run.  It contains no timestamps or
other nondeterministic fields.  Nothing is written if parsing fails.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

# Allow both "python3 tools/analysis/inventory.py" and direct module imports
# from the tests directory.
sys.path.insert(0, str(Path(__file__).resolve().parent))

import ne  # noqa: E402


def _hash_file(path: Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def _help_file_metadata(path: Path) -> dict[str, object]:
    size, digest = _hash_file(path)
    metadata: dict[str, object] = {
        "path": os.fspath(path),
        "size": size,
        "sha256": digest,
    }
    try:
        with path.open("rb") as handle:
            magic = handle.read(16)
    except OSError:
        magic = b""
    metadata["leading_bytes_hex"] = magic.hex()
    metadata["note"] = (
        "Hashes/metadata only; help decoding is handled separately and is not "
        "required for the NE inventory."
    )
    return metadata


def _write_atomic(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)
        os.replace(temporary_name, path)
    except BaseException:
        try:
            os.unlink(temporary_name)
        except OSError:
            pass
        raise


def build_inventory(
    input_path: Path, help_file: Path | None = None
) -> dict[str, object]:
    data = input_path.read_bytes()
    help_metadata = _help_file_metadata(help_file) if help_file is not None else None
    return ne.build_report(
        data,
        input_path=os.fspath(input_path),
        help_file=help_metadata,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Extract a reproducible inventory from a 16-bit Windows NE "
            "executable without executing it."
        )
    )
    parser.add_argument("input", type=Path, help="NE executable to inspect")
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="JSON report path (written only after a successful parse)",
    )
    parser.add_argument(
        "--help-file",
        type=Path,
        default=None,
        help="optional Windows help file whose hash/metadata is added",
    )
    args = parser.parse_args(argv)

    try:
        data = args.input.read_bytes()
    except OSError as error:
        print(f"error: cannot read {args.input}: {error}", file=sys.stderr)
        return 2

    help_metadata = None
    if args.help_file is not None:
        try:
            help_metadata = _help_file_metadata(args.help_file)
        except OSError as error:
            print(f"error: cannot read help file {args.help_file}: {error}", file=sys.stderr)
            return 2

    try:
        report = ne.build_report(
            data,
            input_path=os.fspath(args.input),
            help_file=help_metadata,
        )
    except ne.NEFormatError as error:
        print(f"error: {error}", file=sys.stderr)
        print("no output written", file=sys.stderr)
        return 2
    except Exception as error:  # pragma: no cover - defensive CLI boundary
        print(f"error: unexpected failure: {error}", file=sys.stderr)
        print("no output written", file=sys.stderr)
        return 2

    text = json.dumps(report, sort_keys=True, indent=2, ensure_ascii=False) + "\n"
    try:
        _write_atomic(args.output, text)
    except OSError as error:
        print(f"error: cannot write {args.output}: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
