# Implementation status

## Current

PR2 (documented recovered rules and reference scenarios) is complete on branch
`pr2-rules-specification`, created from the merged `trunk`. The scoped signed
commit uses this exact message:
`Documented recovered rules and reference scenarios.`

PR1 (binary evidence inventory and extraction tools) is complete and previously
integrated. A private local PDF converter is also available:
`tools/analysis/hlp_to_pdf.py`. It generates
`analysis/help/Stars-Player-Guide-1998.pdf` with 419 topics and 128 decoded
bitmap images. The PDF and extracted images are ignored, private artifacts and
must not be redistributed without permission.

PR2 has not been pushed or merged. Its prerequisite, PR1 integration, was
already present on `trunk`.

## Scope

- In scope (implemented): `tools/analysis/`, `tools/analysis/tests/`,
  `tools/analysis/validate_scenarios.py`, `docs/evidence/inventory.md`,
  `docs/evidence/scenarios/`, `docs/rules/`, `.gitignore` entries for private
  extraction output and original game payloads, this status file, and the PR1
  and PR2 checklist entries in `PLAN.md`.
- Out of scope: Rust workspace, runtime gameplay code, original-program
  execution, disassembly conclusions, Git initialization, push, or merge.

## Checklist

- [x] NE parser with checked offsets (DOS pointer, header, segments, resources,
      resident/nonresident names, module references, imports, entry table)
- [x] Dialog-template inventory and bounded selected-string references
- [x] `inventory.py` CLI and deterministic, atomic JSON report
- [x] HLP container parser: B+ tree directory, `|SYSTEM` metadata/date,
      `|TOPIC` decode, private decoded output
- [x] Evidence index and prioritized PR2 questions
- [x] Synthetic NE and HLP fixtures; malformed/truncated/oversized cases
- [x] `python3 -m unittest discover -s tools/analysis/tests -v`
- [x] `python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json`
- [x] `python3 tools/analysis/help.py 'Stars! - RIP/stars!.hlp' --output analysis/help/inventory.json --text-output analysis/help/topics.txt`
- [x] PLAN.md PR1 checkbox, after acceptance
- [x] PR2 rule index with stable IDs, statuses and source hashes
- [x] PR2 rule ledger covering all 10 primary and 14 lesser traits
- [x] PR2 ledger entries for race builder, universe setup, technologies,
      hulls/components, planets, production, research, fleets, scanning,
      combat, Turindrones/AI, victory and turn structure
- [x] PR2 turn-order dependency specification with unresolved edges explicit
- [x] PR2 gap ledger with downstream gates
- [x] PR2 59 JSON controlled scenarios; no fabricated observed results
- [x] PR2 scenario validator and focused tests
- [x] PLAN.md PR2 checkbox, after acceptance

## Validation

Commands run:

```sh
python3 -m unittest discover -s tools/analysis/tests -v
python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json
python3 tools/analysis/help.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/inventory.json \
  --text-output analysis/help/topics.txt
python3 tools/analysis/hlp_to_pdf.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/Stars-Player-Guide-1998.pdf \
  --image-dir analysis/help/images
python3 -m py_compile tools/analysis/ne.py tools/analysis/inventory.py \
  tools/analysis/help.py tools/analysis/hlp_to_pdf.py \
  tools/analysis/tests/test_ne.py tools/analysis/tests/test_inventory.py \
  tools/analysis/tests/test_help.py tools/analysis/tests/test_hlp_to_pdf.py
sha256sum stars26i.exe 'Stars! - RIP/stars!.hlp'
python3 -m unittest discover -s tools/analysis/tests -v
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
python3 -m py_compile tools/analysis/validate_scenarios.py \
  tools/analysis/tests/test_validate_scenarios.py
```

Results:

- 69 unittest tests passed (31 NE/inventory, 28 help, 10 PDF converter).
- Executable SHA-256 matches `PLAN.md`:
  `5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e`.
- Help SHA-256:
  `9c5cb10fd5e5d201813fcf6f470c495f290b322ee21e1795418a8412393270d1`.
- Two inventory runs were byte-identical; two help runs were byte-identical.
- Malformed synthetic fixtures fail with actionable offsets and no output.
- Reports contain no license-code contents and no bulk binary payloads.
- No original program was executed and no disassembly was performed.
- PR2: 86 unittest tests passed (69 PR1 tests plus 17 validator tests).
- PR2: scenario validator accepted 59 scenarios against 83 rules.
- PR2: 10 primary and 14 lesser traits each have a ledger entry and a scenario.
- PR2: 5 unknown-status scenarios and 2 statically-derived scenarios; the
  remainder are documented-only. No scenario contains an observed result.

## Evidence snapshot

- NE header at file offset `0x400`; 35 segments; 122 resources; 36 dialogs;
  imports `WIN87EM`, `KERNEL`, `GDI`, `USER`, `COMMDLG`, `TOOLHELP`.
- Candidate race-wizard dialogs `#146`–`#151` and `#209` are recorded as
  lead-only from captions; runtime links are unresolved.
- `Turindrones` occurs in segment 35 at relative offset `0xa06` (file
  `0x55846`); runtime use is unresolved.
- Help generation time decodes to January 1998 (raw `884558967`); 419 topics,
  142 internal files, all titles decoded.
- Body text is best-effort; six help topics are strings-only for 18 records.
- PR2 statically established: `#149` has ten primary-trait auto-radio buttons
  in one group (at most one selection); `#150` has 14 empty-caption
  auto-checkboxes; `#151` has six three-choice research-cost radio groups plus
  one checkbox.
- PR2 exact-byte search found the ten primary-trait names in `#149`, but no
  literal lesser-trait names in the executable. The `#150` checkbox-to-trait
  mapping remains unresolved.
- PR2 located the opponent-name sequence at file `0x55846`; Turindrones has no
  recovered race or AI configuration.

## Log

- 2026-10-17: Read `PLAN.md`, `PLAN-pr1-description.md`, and `AGENTS.md`.
  Integration branch is `trunk` at `72ba9a9`. Working branch is
  `pr1-binary-evidence`. Python 3.14.8 is available; `objdump` and
  `llvm-objdump` are installed but neither recognises the NE container.
- 2026-10-17: Implemented the bounded NE inventory parser and CLI, synthetic NE
  fixtures/tests, HLP directory/`|SYSTEM`/`|TOPIC` parser and CLI, synthetic
  HLP tests, and `.gitignore` protection for original payloads/private output.
- 2026-10-17: Ran the full test manifest (69 passed), all real-input CLIs,
  determinism comparisons, and `py_compile`. Wrote
  `docs/evidence/inventory.md` and updated the PR1 master-plan checkbox.
- 2026-10-17: Added the private local PDF converter, MRB/DIB and RunLen image
  decoding, PDF markers for external pictures, and focused converter tests.
  Generated a 200-page PDF with 419 topics and 128 images; five metafile
  pictures are reported as unavailable. The PDF and extracted images remain in
  ignored `analysis/help/`.
- 2026-10-17: Documented the private PDF pipeline in `PLAN.md`, `README.md`
  and the evidence index: marker contract, MRB/DIB decoding, 128 embedded
  images, five unavailable metafiles, best-effort body/table limits, optional
  Pillow/ReportLab dependency, and the requirement to cite HLP offsets rather
  than PDF pages. Updated the test count to 69.
- 2026-10-17: Remaining blocker: none inside PR1. PR2 remains gated on PR1
  integration by an authorized actor. No push, PR, or merge was performed.
- 2026-10-17: PR2: created `pr2-rules-specification` from merged `trunk`.
  Wrote the machine-readable rule index, narrative rule ledger, turn-order
  dependency specification, gap ledger, 59 controlled scenarios and a
  standard-library scenario validator. Added 17 validator tests. Ran the full
  manifest (86 passed), the validator (59 scenarios, 83 rules, OK) and
  `py_compile` on the new Python files. No original game was run; no runtime
  gameplay code was added.

## Known limitations carried into PR3

- Exported procedure names are not yet linked to dialog IDs.
- Control IDs are not yet mapped to the 24 trait identities.
- `Turindrones` is a located string, not a recovered opponent/AI configuration.
- The help is documentation and may not match this executable version.
- No gameplay formula, turn order, combat rule, RNG, or AI policy is claimed.
- PR2 recovered no advantage-point costs or exclusion matrix, no complete
  technology/hull/component catalog, no global turn order or RNG algorithm, no
  combat damage arithmetic and no Turindrones configuration. These are tracked
  in `docs/rules/gap-ledger.md` with downstream gates.
- The lesser-trait checkbox-to-trait mapping is unresolved. The executable has
  14 unnamed checkbox controls and no literal lesser-trait ASCII names.
- No PR2 scenario has an observed result. All 59 records are measurement plans
  until an operator runs them.
