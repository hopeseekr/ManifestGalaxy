"""Tests for the deterministic inventory CLI (tools/analysis/inventory.py)."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

TESTS_DIR = Path(__file__).resolve().parent
ANALYSIS_DIR = TESTS_DIR.parent
INVENTORY = ANALYSIS_DIR / "inventory.py"
FIXTURES = TESTS_DIR / "fixtures" / "ne"

sys.path.insert(0, str(ANALYSIS_DIR))
sys.path.insert(0, str(TESTS_DIR))

import ne  # noqa: E402
from test_ne import build_minimal_ne, build_rich_ne  # noqa: E402


class InventoryCliTests(unittest.TestCase):
    def _run(
        self, input_path: Path, output_path: Path, *extra: str
    ) -> subprocess.CompletedProcess:
        return subprocess.run(
            [
                sys.executable,
                str(INVENTORY),
                str(input_path),
                "--output",
                str(output_path),
                *extra,
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_cli_smoke_and_determinism(self) -> None:
        data, _layout = build_rich_ne()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "fixture.exe"
            input_path.write_bytes(data)
            first = root / "first.json"
            second = root / "second.json"

            result = self._run(input_path, first)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(first.exists())
            result = self._run(input_path, second)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(first.read_bytes(), second.read_bytes())

            text = first.read_text(encoding="utf-8")
            self.assertTrue(text.endswith("\n"))
            report = json.loads(text)
            self.assertEqual(report["schema_version"], 1)
            self.assertEqual(report["input"]["size"], len(data))
            self.assertEqual(report["input"]["sha256"], ne.file_sha256(data))
            self.assertEqual(report["resources"]["total_count"], 7)
            self.assertEqual(report["dialogs"]["count"], 4)
            self.assertEqual(
                report["resources"]["counts_by_type"]["DIALOG"], 4
            )
            self.assertFalse(
                any(key in text for key in ("generated_at", "timestamp"))
            )

    def test_changed_input_changes_report(self) -> None:
        data, _layout = build_rich_ne()
        changed = bytearray(data)
        changed[-1] ^= 0xFF
        changed.extend(b"\x00")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first_input = root / "one.exe"
            second_input = root / "two.exe"
            first_input.write_bytes(data)
            second_input.write_bytes(bytes(changed))
            first = root / "one.json"
            second = root / "two.json"
            self.assertEqual(self._run(first_input, first).returncode, 0)
            self.assertEqual(self._run(second_input, second).returncode, 0)
            one = json.loads(first.read_text(encoding="utf-8"))
            two = json.loads(second.read_text(encoding="utf-8"))
            self.assertNotEqual(one["input"]["sha256"], two["input"]["sha256"])
            self.assertNotEqual(one["input"]["size"], two["input"]["size"])
            self.assertNotEqual(one, two)

    def test_malformed_input_exits_nonzero_without_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "bad.exe"
            input_path.write_bytes(b"not an NE file")
            output = root / "report.json"
            result = self._run(input_path, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("error:", result.stderr)
            self.assertIn("no output written", result.stderr)
            self.assertFalse(output.exists())

    def test_truncated_ne_exits_nonzero_without_output(self) -> None:
        data, layout = build_minimal_ne()
        truncated = data[: layout["ne_offset"] + 8]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "truncated.exe"
            input_path.write_bytes(truncated)
            output = root / "report.json"
            result = self._run(input_path, output)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse(output.exists())

    def test_optional_help_file_metadata(self) -> None:
        data, _layout = build_minimal_ne()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "fixture.exe"
            help_path = root / "fixture.hlp"
            input_path.write_bytes(data)
            help_path.write_bytes(b"help fixture bytes")
            output = root / "report.json"
            result = self._run(input_path, output, "--help-file", str(help_path))
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertIn("help_file", report)
            self.assertEqual(report["help_file"]["size"], len(b"help fixture bytes"))
            self.assertEqual(
                report["help_file"]["sha256"],
                ne.file_sha256(b"help fixture bytes"),
            )

    def test_committed_synthetic_fixtures_parse(self) -> None:
        # These fixtures are synthetic, generated by the test builder and
        # committed only to prove that parsing does not need the game binary.
        for name in ("minimal.exe", "rich.exe"):
            path = FIXTURES / name
            self.assertTrue(path.is_file(), f"missing synthetic fixture {path}")
            with path.open("rb") as handle:
                data = handle.read()
            report = ne.build_report(data, input_path=str(path))
            self.assertEqual(report["dos_header"]["magic"], "MZ")
            self.assertEqual(report["ne_header"]["signature"], "NE")
            self.assertEqual(report["input"]["sha256"], ne.file_sha256(data))


if __name__ == "__main__":
    unittest.main()
