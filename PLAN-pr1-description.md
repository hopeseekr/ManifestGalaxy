# PR1: Established the binary evidence inventory and extraction tools.

- Branch name: `pr1-binary-evidence`
- Branch merge commit message: `Established the binary evidence inventory and extraction tools.`
- GitHub PR title: `Established the binary evidence inventory and extraction tools.`
- Git commit message: `Established the binary evidence inventory and extraction tools.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md) and [AGENTS.md](AGENTS.md).

Extract a reproducible inventory from the supplied 16-bit Windows NE executable without executing it.

The supplied executable and bundled Windows help are the inputs; there is no Rust workspace yet.

## Owned paths and contracts

Own tools/analysis/*.py, tools/analysis/tests/, docs/evidence/inventory.md, and .gitignore entries for private extraction output. Use Python standard-library tools here; PR3 introduces Rust.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Hash every inspected input. Parse the DOS pointer at 0x3c, NE header, segment table, resource table, resident names, module references, and imports with checked offsets and lengths. Distinguish file offsets from segment-relative addresses.

2. Implement `python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json`. Emit hashes, lengths, types, locations, and selected identifiers rather than bulk asset or string dumps. Never inspect or print code.txt.

3. Inventory candidate race-wizard dialogs, Turindrones references, strings, and resource/code cross-references. Determine the dialog count and relationships from reproducible evidence rather than filtering toward an expected count. Treat identifiers as leads, not proof of behavior. Identify a compatible 16-bit disassembly workflow and record tooling availability.

4. Inspect the help container and record its January 1998 date. Document a repeatable decoding workflow, retain decoded material in ignored analysis/, and cite decoded topics or byte offsets rather than raw binary line numbers.

5. Write an evidence index with reproducible commands, extraction limitations, and prioritized questions for PR2. Create malformed and minimal synthetic NE fixtures rather than committing the original executable.

## Acceptance scenarios and validation

- The supplied binary hash matches the master plan; changed inputs yield a clearly different report.
- Truncated headers, invalid pointers, oversized resources, and segment boundaries fail with actionable errors, without reading outside the input.
- Two inventory runs produce equivalent metadata. Reports contain no license-code contents or bulk binary payloads.

Implement and execute these checks:

```sh
python3 -m unittest discover -s tools/analysis/tests -v
python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json
```

Create the named analysis tools/tests in this PR. These checks intentionally do not require the future Rust workspace.

## Evidence gate and exclusions

No game implementation, Rust workspace, asset redistribution, or claims that rules have been recovered.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
