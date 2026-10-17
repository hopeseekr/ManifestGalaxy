# Implementation status

## Current

PR1 (binary evidence inventory and extraction tools) is complete and locally
validated. The scoped signed commit uses this exact message:
`Established the binary evidence inventory and extraction tools.`

A private local PDF converter is also available:
`tools/analysis/hlp_to_pdf.py`. It generates
`analysis/help/Stars-Player-Guide-1998.pdf` with 419 topics and 128 decoded
bitmap images. The PDF and extracted images are ignored, private artifacts and
must not be redistributed without permission.

PR2 has not been started. Its prerequisite is PR1's authorized integration;
the PR1 prompt does not authorize a push, merge, or PR2 work.

## Scope

- In scope (implemented): `tools/analysis/`, `tools/analysis/tests/`,
  `docs/evidence/inventory.md`, `.gitignore` entries for private extraction
  output and original game payloads, this status file, and the PR1 checklist
  entry in `PLAN.md`.
- Out of scope: Rust workspace, gameplay rules, original-program execution,
  disassembly conclusions, Git initialization, push, or merge.

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

## Known limitations carried into PR2

- Exported procedure names are not yet linked to dialog IDs.
- Control IDs are not yet mapped to the 24 trait identities.
- `Turindrones` is a located string, not a recovered opponent/AI configuration.
- The help is documentation and may not match this executable version.
- No gameplay formula, turn order, combat rule, RNG, or AI policy is claimed.
