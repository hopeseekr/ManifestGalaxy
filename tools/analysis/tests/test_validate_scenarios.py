"""Tests for tools/analysis/validate_scenarios.py."""

from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ANALYSIS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = ANALYSIS_DIR.parents[1]
sys.path.insert(0, str(ANALYSIS_DIR))

import validate_scenarios as vs  # noqa: E402


def rule_index() -> dict:
    return {
        "schema_version": 1,
        "source_hashes": {
            vs.EXE_NAME: vs.EXE_SHA256,
            vs.HELP_NAME: vs.HELP_SHA256,
        },
        "rules": [
            {
                "id": "RULE-RACE-PRT-HE",
                "title": "Hyper-Expansion",
                "kind": "trait",
                "trait": {"type": "primary", "name": "Hyper-Expansion", "abbreviation": "HE"},
                "status": "documented-only",
                "downstream_prs": ["PR5", "PR13"],
                "scenario_ids": ["SCN-TEST-001"],
            },
            {
                "id": "RULE-TEST-001",
                "title": "Test subsystem",
                "kind": "subsystem",
                "status": "documented-only",
                "downstream_prs": ["PR5"],
                "scenario_ids": ["SCN-TEST-001"],
            },
        ],
    }


def scenario() -> dict:
    return {
        "schema_version": 1,
        "id": "SCN-TEST-001",
        "title": "Test scenario",
        "rule_ids": ["RULE-RACE-PRT-HE", "RULE-TEST-001"],
        "status": "documented-only",
        "confidence": "help-only",
        "provenance": [
            {
                "source": vs.HELP_NAME,
                "sha256": vs.HELP_SHA256,
                "kind": "help-topic",
                "location": (
                    "TOPICPOS 1736867 (0x001a80a3), "
                    "HLP file offset 437912 (0x0006ae98), topic 258"
                ),
                "note": "synthetic test citation",
            }
        ],
        "setup": {
            "baseline": "neutral race",
            "variable": "select Hyper-Expansion",
        },
        "procedure": ["Record the baseline balance.", "Select the trait and record the result."],
        "expected": {"documented": ["The trait exists in the help."]},
        "observed": None,
        "unresolved": ["Exact point cost is unknown."],
        "downstream_prs": ["PR5", "PR13"],
    }


def write_case(base: Path, index: dict, scenarios: dict[str, dict]) -> tuple[Path, Path]:
    index_path = base / "rule-index.json"
    scenario_dir = base / "scenarios"
    scenario_dir.mkdir()
    index_path.write_text(json.dumps(index), encoding="utf-8")
    for filename, data in scenarios.items():
        (scenario_dir / filename).write_text(json.dumps(data), encoding="utf-8")
    return scenario_dir, index_path


class ValidatorTests(unittest.TestCase):
    def test_repository_scenarios_validate(self) -> None:
        scenario_dir = REPO_ROOT / "docs" / "evidence" / "scenarios"
        index_path = REPO_ROOT / "docs" / "rules" / "rule-index.json"
        if not scenario_dir.is_dir() or not index_path.is_file():
            self.skipTest("repository scenario evidence is not present")
        report = vs.validate(scenario_dir, index_path)
        self.assertTrue(report.ok, report.errors)

    def test_valid_index_and_scenario(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(report.ok, report.errors)

    def test_duplicate_scenario_id_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            second = scenario()
            second["title"] = "duplicate"
            scenario_dir, index_path = write_case(
                Path(raw),
                rule_index(),
                {"one.json": scenario(), "two.json": second},
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("duplicate scenario ID" in e for e in report.errors), report.errors)

    def test_missing_provenance_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["provenance"] = []
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("provenance must be a non-empty list" in e for e in report.errors))

    def test_wrong_executable_hash_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = "statically-derived"
            data["confidence"] = "static-only"
            data["expected"] = {"static": ["A static fact."]}
            data["provenance"] = [
                {
                    "source": vs.EXE_NAME,
                    "sha256": "0" * 64,
                    "kind": "executable",
                    "location": "dialog #149 at file offset 0x2fa900",
                }
            ]
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("executable SHA-256 does not match" in e for e in report.errors))

    def test_unknown_provenance_source_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["provenance"][0]["source"] = "other.hlp"
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(
                any("unsupported provenance source" in e for e in report.errors),
                report.errors,
            )

    def test_documented_only_requires_help_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["provenance"] = [
                {
                    "source": vs.EXE_NAME,
                    "sha256": vs.EXE_SHA256,
                    "kind": "executable",
                    "location": "dialog #149 at file offset 0x2fa900",
                }
            ]
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("requires a help-topic provenance" in e for e in report.errors))

    def test_help_location_requires_topicpos_and_offset(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["provenance"][0]["location"] = "topic 258"
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("must include TOPICPOS" in e for e in report.errors))
            self.assertTrue(any("must include the HLP file offset" in e for e in report.errors))

    def test_observed_status_requires_observation(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = "observed"
            data["confidence"] = "high"
            data["expected"] = {}
            data["provenance"] = [
                {
                    "source": "operator-notes",
                    "sha256": vs.HELP_SHA256,
                    "kind": "observation",
                    "location": "operator log at 2026-10-17",
                }
            ]
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("requires an observation record" in e for e in report.errors))

    def test_non_null_observation_with_documented_status_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["observed"] = {
                "date": "2026-10-17",
                "environment": "test",
                "trials": 1,
                "result": "something",
            }
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("contradicts a non-null observed result" in e for e in report.errors))

    def test_unknown_status_rejects_expected_results(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = "unknown"
            data["confidence"] = "unknown"
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("unknown status must not claim expected results" in e for e in report.errors))

    def test_unknown_status_requires_unresolved(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = "unknown"
            data["confidence"] = "unknown"
            data["expected"] = {}
            data["unresolved"] = []
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("unknown status requires at least one unresolved" in e for e in report.errors))

    def test_statically_derived_rejects_documented_expectation(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = "statically-derived"
            data["confidence"] = "static-only"
            data["provenance"].append(
                {
                    "source": vs.EXE_NAME,
                    "sha256": vs.EXE_SHA256,
                    "kind": "executable",
                    "location": "dialog #149 at file offset 0x2fa900",
                }
            )
            data["expected"]["static"] = ["A static fact."]
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("must not claim expected.documented" in e for e in report.errors))

    def test_boolean_schema_version_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["schema_version"] = True
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("schema_version must be 1" in e for e in report.errors), report.errors)

    def test_float_rule_index_schema_version_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            index = rule_index()
            index["schema_version"] = 1.0
            scenario_dir, index_path = write_case(
                Path(raw), index, {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("schema_version must be 1" in e for e in report.errors), report.errors)

    def test_unhashable_status_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["status"] = []
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("invalid status" in e for e in report.errors), report.errors)

    def test_invalid_utf8_is_reported_without_a_traceback(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            scenario_dir, index_path = write_case(Path(raw), rule_index(), {})
            (scenario_dir / "SCN-BAD-001.json").write_bytes(b"\xff\xfe")
            report = vs.validate(scenario_dir, index_path)
            self.assertFalse(report.ok)
            self.assertTrue(any("cannot decode file as UTF-8" in e for e in report.errors), report.errors)

    def test_scenario_id_with_trailing_newline_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["id"] = "SCN-TEST-001\n"
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("invalid scenario ID" in e for e in report.errors), report.errors)

    def test_rule_status_stronger_than_linked_scenario_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            index = rule_index()
            index["rules"][0]["status"] = "observed"
            scenario_dir, index_path = write_case(
                Path(raw), index, {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(
                any("stronger than every linked scenario" in e for e in report.errors),
                report.errors,
            )

    def test_missing_trait_coverage_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            index = rule_index()
            index["rules"][0]["scenario_ids"] = ["SCN-MISSING-001"]
            scenario_dir, index_path = write_case(
                Path(raw), index, {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("references missing scenario" in e for e in report.errors))

    def test_unknown_rule_reference_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            data = scenario()
            data["rule_ids"] = ["RULE-NOT-IN-INDEX"]
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": data}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("not present in the rule index" in e for e in report.errors))

    def test_duplicate_rule_id_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            index = rule_index()
            index["rules"].append(copy.deepcopy(index["rules"][0]))
            scenario_dir, index_path = write_case(
                Path(raw), index, {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("duplicate rule ID" in e for e in report.errors))

    def test_scenario_not_listed_by_rule_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            index = rule_index()
            index["rules"][1]["scenario_ids"] = ["SCN-OTHER-001"]
            scenario_dir, index_path = write_case(
                Path(raw), index, {"SCN-TEST-001.json": scenario()}
            )
            report = vs.validate(scenario_dir, index_path)
            self.assertTrue(any("does not list this scenario" in e for e in report.errors))

    def test_cli_exit_codes(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            scenario_dir, index_path = write_case(
                Path(raw), rule_index(), {"SCN-TEST-001.json": scenario()}
            )
            command = [
                sys.executable,
                str(ANALYSIS_DIR / "validate_scenarios.py"),
                str(scenario_dir),
                "--rules",
                str(index_path),
            ]
            good = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(good.returncode, 0, good.stderr)
            self.assertIn("OK", good.stdout)

            bad_data = scenario()
            bad_data["provenance"] = []
            (scenario_dir / "SCN-TEST-001.json").write_text(
                json.dumps(bad_data), encoding="utf-8"
            )
            bad = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(bad.returncode, 1)
            self.assertIn("Validation failed", bad.stderr)


if __name__ == "__main__":
    unittest.main()
