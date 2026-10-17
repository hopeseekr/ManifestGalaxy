# Stars! remake

A desktop-only Rust + egui/eframe reimplementation of Stars!, with a complete
custom-race builder and Turindrones AI opponents. The project is being built in
staged PRs described in [PLAN.md](PLAN.md); gameplay rules and implementation
must be backed by evidence, not invented.

## Current status

PR1 — binary evidence inventory and extraction tools — is complete on branch
`pr1-binary-evidence`. There is no game executable and no Rust workspace yet;
the Rust foundation starts in PR3.

PR1 provides Python tools that inspect the supplied 16-bit Windows executable
and Windows help file without executing either one. See
[docs/evidence/inventory.md](docs/evidence/inventory.md) for findings,
confidence levels, limitations, and open PR2 questions.

## Prerequisites

- Python 3.11 or newer (validated with Python 3.14.8)
- The supplied `stars26i.exe` and `Stars! - RIP/stars!.hlp` in the repository
  root. These original payloads are ignored by Git.
- For the private PDF converter only: Pillow and ReportLab
  (`python3 -m pip install Pillow reportlab`). The inspection and test tools
  otherwise use the Python standard library.

## Run the tests

```sh
python3 -m unittest discover -s tools/analysis/tests -v
```

Expected result: `69 tests ... OK`.

## Inventory the executable

```sh
python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json
```

This writes `analysis/inventory.json`, which is ignored by Git. The tool does
not execute the game. It parses the DOS/NE headers, segments, resources, names,
imports, entry table, dialog templates, and a bounded set of selected strings.

## Inspect the help file

```sh
python3 tools/analysis/help.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/inventory.json \
  --text-output analysis/help/topics.txt
```

This writes:

- `analysis/help/inventory.json` — container metadata and topic index
- `analysis/help/topics.txt` — decoded help text

Both outputs are ignored by Git. The decoded help is documentation evidence,
not proof that the executable implements the same behavior.

## Generate a private PDF

```sh
python3 tools/analysis/hlp_to_pdf.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/Stars-Player-Guide-1998.pdf \
  --image-dir analysis/help/images
```

This writes a 200-page A4 PDF with 419 topics and 128 decoded bitmap images,
plus the extracted PNGs. The five Windows metafile pictures are reported as
unavailable rather than guessed. Use `--no-images` for a text-only PDF.

The PDF and extracted images are ignored by Git. The source help text and
images are copyright Mare Crisium, LLC; keep the generated PDF private and do
not redistribute it without permission. See the conversion-notes page inside
the PDF for the remaining decoding limits.

### How the PDF conversion works

- `help.py` emits markers such as `[[IMAGE |bm12 align=center]]` in decoded
  topic text.
- `hlp_to_pdf.py` resolves each `|bmN` marker to an internal WinHelp MRB/SHG
  picture, decodes the DIB/RunLen data, writes a PNG, and embeds it at that
  position in the PDF.
- The current help has 419 topics and 133 `|bmN` files: 128 supported DIBs are
  embedded, while five Windows metafiles (`|bm2`, `|bm82`, `|bm83`, `|bm87`,
  `|bm108`) are reported as unavailable.
- Body text is best-effort. Six topics use strings-only fallback for 18
  display/table records, and table cell geometry is approximate.
- The PDF is a convenience rendering, not authoritative evidence. Cite the HLP
  file, TOPICPOS, or absolute HLP byte offset rather than a PDF page number.
- Help images and text are reference material only. Do not copy them into the
  remake's assets or commit generated output.

If you change the picture marker format or the topic text renderer, update the
converter and run the full test suite. The focused converter tests cover
synthetic MRB/DIB extraction, RunLen decoding, metafile rejection, and marker
handling.

## Check the supplied files

```sh
python3 --version
sha256sum stars26i.exe 'Stars! - RIP/stars!.hlp'
```

The executable hash must match the master plan:

```text
5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e
```

## CLI options

```sh
python3 tools/analysis/inventory.py --help
python3 tools/analysis/help.py --help
python3 tools/analysis/hlp_to_pdf.py --help
```

## Repository layout

```text
tools/analysis/          PR1 Python inspection, help-decoding and PDF tools
tools/analysis/tests/    Synthetic NE/HLP/PDF tests
docs/evidence/           Committed evidence index and findings
analysis/inventory.json  Ignored executable inventory
analysis/help/           Ignored decoded help, images and private PDF
Stars! - RIP/            Ignored supplied help/archive files
stars26i.exe             Ignored supplied executable
```

Gameplay packages (`crates/stars-core`, `stars-ai`, `stars-desktop`,
`stars-tools`) do not exist yet. `cargo run` will not work until PR3 introduces
the Rust workspace.

## Notes for future agents

- PR1 findings are evidence leads, not recovered rules. Read
  [docs/evidence/inventory.md](docs/evidence/inventory.md) and keep observed,
  statically derived, documented-only, inferred and unknown claims separate.
- Regenerate derived artifacts with the commands above; never commit
  `analysis/`, the supplied executables/help files, extracted images, or the
  PDF.
- Keep the `[[IMAGE ...]]` marker contract in sync between `help.py` and
  `hlp_to_pdf.py`. The PDF converter should continue to fail loudly or report
  unavailable pictures rather than invent image bytes.
- The help is documentation, not proof that `stars26i.exe` implements the same
  numbers. Version-specific rules still need PR2 evidence and controlled
  scenarios.
