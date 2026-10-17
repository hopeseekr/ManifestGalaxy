#!/usr/bin/env python3
"""Validate PR2 rule-index and controlled-scenario records.

The validator is intentionally standard-library-only.  It checks the evidence
claims that can be checked mechanically:

* stable IDs are unique and well formed;
* every scenario cites a known source hash and a byte/code or help location;
* an evidence status agrees with the evidence actually supplied;
* unknown records do not masquerade as documented or observed rules;
* every trait and rule in the index is covered by at least one scenario.

It does not decide whether a help claim is true for the supplied executable.
That is an evidence question for the operator scenarios and later PRs.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

EXE_NAME = "stars26i.exe"
HELP_NAME = "stars!.hlp"
EXE_SHA256 = "5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e"
HELP_SHA256 = "9c5cb10fd5e5d201813fcf6f470c495f290b322ee21e1795418a8412393270d1"

STATUSES = {
    "observed",
    "statically-derived",
    "documented-only",
    "inferred",
    "unknown",
}
CONFIDENCES = {"high", "medium", "low", "static-only", "help-only", "unknown"}
PROVENANCE_KINDS = {"executable", "help-topic", "observation", "operator-notes"}
PR_PATTERN = re.compile(r"^PR(?:[4-9]|1[0-8])$")
RULE_ID_PATTERN = re.compile(r"^RULE-[A-Z0-9][A-Z0-9-]*$")
SCENARIO_ID_PATTERN = re.compile(r"^SCN-[A-Z0-9][A-Z0-9-]*$")
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
TOPICPOS_PATTERN = re.compile(r"TOPICPOS\s+\d+\s+\(0x[0-9a-fA-F]+\)")
HELP_OFFSET_PATTERN = re.compile(r"HLP file offset\s+\d+\s+\(0x[0-9a-fA-F]+\)")
EXECUTABLE_LOCATION_PATTERN = re.compile(
    r"(?:0x[0-9a-fA-F]+|dialog\s+#\d+|segment\s+\d+|file offset|resource)",
    re.IGNORECASE,
)

REQUIRED_RULE_KEYS = {
    "id",
    "title",
    "kind",
    "status",
    "downstream_prs",
    "scenario_ids",
}
REQUIRED_SCENARIO_KEYS = {
    "schema_version",
    "id",
    "title",
    "rule_ids",
    "status",
    "confidence",
    "provenance",
    "setup",
    "procedure",
    "expected",
    "observed",
    "unresolved",
    "downstream_prs",
}


class ValidationReport:
    """Collect errors and warnings without raising on the first problem."""

    def __init__(self) -> None:
        self.errors: list[str] = []
        self.warnings: list[str] = []

    def error(self, where: str, message: str) -> None:
        self.errors.append(f"{where}: {message}")

    def warn(self, where: str, message: str) -> None:
        self.warnings.append(f"{where}: {message}")

    @property
    def ok(self) -> bool:
        return not self.errors


def _is_nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _is_string_list(value: Any, *, allow_empty: bool = False) -> bool:
    if not isinstance(value, list):
        return False
    if not value and not allow_empty:
        return False
    return all(_is_nonempty_string(item) for item in value)


def _check_pr_list(value: Any, where: str, report: ValidationReport) -> None:
    if not _is_string_list(value):
        report.error(where, "downstream_prs must be a non-empty list of PR names")
        return
    for pr in value:
        if not PR_PATTERN.match(pr):
            report.error(where, f"invalid downstream PR name: {pr!r}")


def load_json(path: Path, report: ValidationReport) -> Any | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        report.error(str(path), "file not found")
    except OSError as exc:
        report.error(str(path), f"cannot read file: {exc}")
    except json.JSONDecodeError as exc:
        report.error(str(path), f"invalid JSON at line {exc.lineno}, column {exc.colno}: {exc.msg}")
    return None


def validate_rule_index(
    data: Any, path: Path, report: ValidationReport
) -> dict[str, dict[str, Any]]:
    """Validate the machine-readable rule index and return rules by ID."""

    where = str(path)
    if not isinstance(data, dict):
        report.error(where, "rule index must be a JSON object")
        return {}
    if data.get("schema_version") != 1:
        report.error(where, "schema_version must be 1")

    hashes = data.get("source_hashes")
    if not isinstance(hashes, dict):
        report.error(where, "source_hashes must be an object")
    else:
        if hashes.get(EXE_NAME) != EXE_SHA256:
            report.error(where, f"source_hashes[{EXE_NAME!r}] must equal the PR1 executable SHA-256")
        if hashes.get(HELP_NAME) != HELP_SHA256:
            report.error(where, f"source_hashes[{HELP_NAME!r}] must equal the PR1 help SHA-256")

    rules = data.get("rules")
    if not isinstance(rules, list) or not rules:
        report.error(where, "rules must be a non-empty list")
        return {}

    by_id: dict[str, dict[str, Any]] = {}
    for index, rule in enumerate(rules):
        rwhere = f"{where}:rules[{index}]"
        if not isinstance(rule, dict):
            report.error(rwhere, "rule must be an object")
            continue
        missing = REQUIRED_RULE_KEYS - set(rule)
        if missing:
            report.error(rwhere, f"missing required keys: {', '.join(sorted(missing))}")
            continue

        rule_id = rule.get("id")
        if not _is_nonempty_string(rule_id) or not RULE_ID_PATTERN.match(rule_id):
            report.error(rwhere, f"invalid rule ID: {rule_id!r}")
            continue
        rwhere = f"{where}:rule {rule_id}"
        if rule_id in by_id:
            report.error(rwhere, "duplicate rule ID")
            continue
        by_id[rule_id] = rule

        if not _is_nonempty_string(rule.get("title")):
            report.error(rwhere, "title must be a non-empty string")
        if rule.get("kind") not in {"trait", "subsystem"}:
            report.error(rwhere, "kind must be 'trait' or 'subsystem'")
        if rule.get("status") not in STATUSES:
            report.error(rwhere, f"invalid status: {rule.get('status')!r}")
        _check_pr_list(rule.get("downstream_prs"), rwhere, report)

        scenario_ids = rule.get("scenario_ids")
        if not _is_string_list(scenario_ids):
            report.error(rwhere, "scenario_ids must be a non-empty list")
        else:
            seen: set[str] = set()
            for scenario_id in scenario_ids:
                if not SCENARIO_ID_PATTERN.match(scenario_id):
                    report.error(rwhere, f"invalid scenario ID: {scenario_id!r}")
                if scenario_id in seen:
                    report.error(rwhere, f"duplicate scenario ID in rule: {scenario_id!r}")
                seen.add(scenario_id)

        if rule.get("kind") == "trait":
            trait = rule.get("trait")
            if not isinstance(trait, dict):
                report.error(rwhere, "trait rules must contain a trait object")
            else:
                if trait.get("type") not in {"primary", "lesser"}:
                    report.error(rwhere, "trait.type must be 'primary' or 'lesser'")
                if not _is_nonempty_string(trait.get("name")):
                    report.error(rwhere, "trait.name must be a non-empty string")
                if not _is_nonempty_string(trait.get("abbreviation")):
                    report.error(rwhere, "trait.abbreviation must be a non-empty string")

    return by_id


def _validate_provenance(
    provenance: Any, where: str, report: ValidationReport
) -> list[dict[str, Any]]:
    if not isinstance(provenance, list) or not provenance:
        report.error(where, "provenance must be a non-empty list")
        return []

    checked: list[dict[str, Any]] = []
    for index, item in enumerate(provenance):
        pwhere = f"{where}:provenance[{index}]"
        if not isinstance(item, dict):
            report.error(pwhere, "provenance entry must be an object")
            continue
        source = item.get("source")
        sha256 = item.get("sha256")
        kind = item.get("kind")
        location = item.get("location")
        if not _is_nonempty_string(source):
            report.error(pwhere, "source must be a non-empty string")
            continue
        if not _is_nonempty_string(sha256) or not SHA256_PATTERN.match(sha256):
            report.error(pwhere, "sha256 must be 64 lowercase hexadecimal characters")
        if kind not in PROVENANCE_KINDS:
            report.error(pwhere, f"invalid provenance kind: {kind!r}")
        if not _is_nonempty_string(location):
            report.error(pwhere, "location must be a non-empty string")
            continue

        if source == EXE_NAME and sha256 != EXE_SHA256:
            report.error(pwhere, "executable SHA-256 does not match the PR1 input hash")
        if source == HELP_NAME and sha256 != HELP_SHA256:
            report.error(pwhere, "help SHA-256 does not match the PR1 input hash")

        if kind == "help-topic":
            if not TOPICPOS_PATTERN.search(location):
                report.error(pwhere, "help-topic location must include TOPICPOS and its hexadecimal value")
            if not HELP_OFFSET_PATTERN.search(location):
                report.error(pwhere, "help-topic location must include the HLP file offset")
        elif kind == "executable":
            if not EXECUTABLE_LOCATION_PATTERN.search(location):
                report.error(pwhere, "executable location must identify a file offset, segment, resource or dialog")
        elif kind == "observation":
            if source in {EXE_NAME, HELP_NAME}:
                report.error(pwhere, "observation entries must not use an input file as the source")
        checked.append(item)
    return checked


def _validate_expected(expected: Any, where: str, report: ValidationReport) -> dict[str, list[str]]:
    if not isinstance(expected, dict):
        report.error(where, "expected must be an object")
        return {}
    unknown_keys = set(expected) - {"documented", "static"}
    if unknown_keys:
        report.error(where, f"unknown expected keys: {', '.join(sorted(unknown_keys))}")
    for key in ("documented", "static"):
        if key in expected and not _is_string_list(expected[key]):
            report.error(where, f"expected.{key} must be a non-empty list of strings")
    return expected


def _validate_observed(observed: Any, where: str, report: ValidationReport) -> None:
    if observed is None:
        return
    if not isinstance(observed, dict):
        report.error(where, "observed must be null or an observation object")
        return
    for key in ("date", "environment", "result"):
        if not _is_nonempty_string(observed.get(key)):
            report.error(where, f"observed.{key} must be a non-empty string")
    trials = observed.get("trials")
    if not isinstance(trials, int) or isinstance(trials, bool) or trials < 1:
        report.error(where, "observed.trials must be a positive integer")


def validate_scenario(
    data: Any,
    path: Path,
    rule_ids: set[str],
    report: ValidationReport,
) -> tuple[str | None, list[str]]:
    """Validate one scenario and return (scenario_id, referenced_rule_ids)."""

    where = str(path)
    if not isinstance(data, dict):
        report.error(where, "scenario must be a JSON object")
        return None, []
    missing = REQUIRED_SCENARIO_KEYS - set(data)
    if missing:
        report.error(where, f"missing required keys: {', '.join(sorted(missing))}")
        return None, []

    if data.get("schema_version") != 1:
        report.error(where, "schema_version must be 1")

    scenario_id = data.get("id")
    if not _is_nonempty_string(scenario_id) or not SCENARIO_ID_PATTERN.match(scenario_id):
        report.error(where, f"invalid scenario ID: {scenario_id!r}")
        return None, []
    where = f"{where}:{scenario_id}"

    if not _is_nonempty_string(data.get("title")):
        report.error(where, "title must be a non-empty string")

    status = data.get("status")
    if status not in STATUSES:
        report.error(where, f"invalid status: {status!r}")
    if data.get("confidence") not in CONFIDENCES:
        report.error(where, f"invalid confidence: {data.get('confidence')!r}")

    scenario_rules = data.get("rule_ids")
    referenced: list[str] = []
    if not _is_string_list(scenario_rules):
        report.error(where, "rule_ids must be a non-empty list")
    else:
        seen_rules: set[str] = set()
        for rule_id in scenario_rules:
            if not RULE_ID_PATTERN.match(rule_id):
                report.error(where, f"invalid rule ID: {rule_id!r}")
                continue
            if rule_id in seen_rules:
                report.error(where, f"duplicate rule ID: {rule_id!r}")
                continue
            seen_rules.add(rule_id)
            if rule_id not in rule_ids:
                report.error(where, f"rule ID is not present in the rule index: {rule_id!r}")
            referenced.append(rule_id)

    provenance = _validate_provenance(data.get("provenance"), where, report)
    kinds = {item.get("kind") for item in provenance}
    sources = {item.get("source") for item in provenance}

    setup = data.get("setup")
    if not isinstance(setup, dict):
        report.error(where, "setup must be an object")
    else:
        for key in ("baseline", "variable"):
            if not _is_nonempty_string(setup.get(key)):
                report.error(where, f"setup.{key} must be a non-empty string")

    if not _is_string_list(data.get("procedure")):
        report.error(where, "procedure must be a non-empty list of strings")
    elif len(data["procedure"]) < 2:
        report.error(where, "procedure must contain at least two steps")

    expected = _validate_expected(data.get("expected"), where, report)
    observed = data.get("observed")
    _validate_observed(observed, where, report)
    if not isinstance(data.get("unresolved"), list) or not all(
        _is_nonempty_string(item) for item in data["unresolved"]
    ):
        report.error(where, "unresolved must be a list of strings")

    _check_pr_list(data.get("downstream_prs"), where, report)

    # Status/evidence consistency.  These checks are the core of the fixture
    # contract: a record cannot claim a stronger evidence class than it has.
    documented = expected.get("documented", [])
    static = expected.get("static", [])
    if observed is not None and status != "observed":
        report.error(where, f"status {status!r} contradicts a non-null observed result")
    if status == "observed":
        if observed is None:
            report.error(where, "observed status requires an observation record")
        if "observation" not in kinds:
            report.error(where, "observed status requires an observation provenance entry")
    elif "observation" in kinds:
        report.error(where, f"status {status!r} must not include an observation provenance entry")

    if status == "documented-only":
        if not documented:
            report.error(where, "documented-only status requires expected.documented")
        if "help-topic" not in kinds:
            report.error(where, "documented-only status requires a help-topic provenance entry")
        if static:
            report.error(where, "documented-only status must not claim expected.static")
    elif status == "statically-derived":
        if not static:
            report.error(where, "statically-derived status requires expected.static")
        if EXE_NAME not in sources or "executable" not in kinds:
            report.error(where, "statically-derived status requires executable provenance")
        if documented:
            report.error(where, "statically-derived status must not claim expected.documented")
    elif status == "unknown":
        if documented or static:
            report.error(where, "unknown status must not claim expected results")
        if not data.get("unresolved"):
            report.error(where, "unknown status requires at least one unresolved question")
    elif status == "inferred":
        if not _is_nonempty_string(data.get("inference_basis")):
            report.error(where, "inferred status requires a non-empty inference_basis")
        if not data.get("unresolved"):
            report.error(where, "inferred status requires at least one unresolved question")
    elif status == "observed":
        if documented or static:
            report.error(where, "observed status must not rely on expected.documented or expected.static")

    return scenario_id, referenced


def _coverage_check(
    rules: dict[str, dict[str, Any]],
    scenarios: dict[str, dict[str, Any]],
    scenario_rules: dict[str, list[str]],
    report: ValidationReport,
) -> None:
    for rule_id, rule in rules.items():
        expected_scenarios = set(rule.get("scenario_ids", []))
        missing = expected_scenarios - set(scenarios)
        for scenario_id in sorted(missing):
            report.error(f"rule {rule_id}", f"scenario_ids references missing scenario {scenario_id!r}")
        for scenario_id in expected_scenarios & set(scenarios):
            if rule_id not in scenario_rules.get(scenario_id, []):
                report.error(
                    f"rule {rule_id}",
                    f"scenario {scenario_id!r} exists but does not list this rule in rule_ids",
                )
        if rule.get("kind") == "trait" and not expected_scenarios:
            report.error(f"rule {rule_id}", "trait rule has no controlled scenario")

    for scenario_id, referenced in scenario_rules.items():
        for rule_id in referenced:
            rule = rules.get(rule_id)
            if rule is not None and scenario_id not in rule.get("scenario_ids", []):
                report.error(
                    f"scenario {scenario_id}",
                    f"rule {rule_id} does not list this scenario in scenario_ids",
                )


def validate(scenario_dir: Path, rule_index_path: Path) -> ValidationReport:
    """Validate a scenario directory against a rule index."""

    report = ValidationReport()
    if not scenario_dir.is_dir():
        report.error(str(scenario_dir), "scenario directory does not exist")
        return report

    index_data = load_json(rule_index_path, report)
    rules = validate_rule_index(index_data, rule_index_path, report) if index_data is not None else {}
    rule_ids = set(rules)

    scenarios: dict[str, dict[str, Any]] = {}
    scenario_rules: dict[str, list[str]] = {}
    paths = sorted(scenario_dir.glob("*.json"))
    if not paths:
        report.error(str(scenario_dir), "no JSON scenario files found")

    for path in paths:
        data = load_json(path, report)
        if data is None:
            continue
        scenario_id, referenced = validate_scenario(data, path, rule_ids, report)
        if scenario_id is None:
            continue
        if scenario_id in scenarios:
            report.error(str(path), f"duplicate scenario ID (also defined): {scenario_id!r}")
            continue
        scenarios[scenario_id] = data
        scenario_rules[scenario_id] = referenced
        if path.stem != scenario_id:
            report.warn(str(path), f"filename stem does not match scenario ID {scenario_id!r}")

    if rules and scenarios:
        _coverage_check(rules, scenarios, scenario_rules, report)

    return report


def default_rule_index() -> Path:
    return Path(__file__).resolve().parents[2] / "docs" / "rules" / "rule-index.json"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate PR2 rule-index and scenario evidence records."
    )
    parser.add_argument("scenario_dir", type=Path, help="directory containing scenario JSON files")
    parser.add_argument(
        "--rules",
        type=Path,
        default=None,
        help="rule-index JSON path (default: docs/rules/rule-index.json)",
    )
    args = parser.parse_args(argv)

    rule_index_path = args.rules if args.rules is not None else default_rule_index()
    report = validate(args.scenario_dir, rule_index_path)

    for warning in report.warnings:
        print(f"WARNING: {warning}", file=sys.stderr)
    if report.errors:
        for error in report.errors:
            print(f"ERROR: {error}", file=sys.stderr)
        print(
            f"Validation failed with {len(report.errors)} error(s).",
            file=sys.stderr,
        )
        return 1

    print(
        f"Validated scenarios in {args.scenario_dir} against {rule_index_path}: OK"
    )
    if report.warnings:
        print(f"Warnings: {len(report.warnings)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
