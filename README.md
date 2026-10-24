# Stars! remake

A desktop-only Rust + egui/eframe reimplementation of Stars!, with a complete
custom-race builder and Turindrones AI opponents. The project is built in
staged PRs described in [PLAN.md](PLAN.md); gameplay rules and implementation
must be backed by evidence, not invented.

## Current status

PR1 (binary evidence), PR2 (rules specification and controlled scenarios), PR3
(the Rust workspace and desktop foundation) and PR4 (deterministic state and
turn-processing contracts) are integrated on `trunk`. PR5 adds the complete
custom-race builder and the recovered advantage-point calculation on branch
`pr5-custom-race-builder`.

Runtime trait effects, economy, AI policy, universe generation, persistence,
network code, browser targets and original game assets are still absent. The
desktop shell provides:

- a windowed egui/eframe shell with a menu, resizable map area, inspector and
  status area;
- UI scale controls at 100%, 125%, 150% and 200%, plus a continuous inspector
  slider;
- original procedural placeholder rendering (the map preview is synthetic, not
  a game universe);
- keyboard focus and shortcuts: `Tab` moves focus, `F5` starts a synthetic
  scan, `Ctrl+Q` closes the window;
- a desktop-owned background worker with request IDs, progress, completed
  snapshots and typed failures;
- a headless shell model that ignores stale results and retains the last
  accepted snapshot when a worker request fails.

PR4 provides deterministic core contracts for stable typed identifiers,
ordered `GameState`, validated `OrderBatch` commits, `PlayerView` filtering,
`TurnOutcome` events, versioned RNG streams and canonical state encoding. A
desktop turn worker owns the authoritative session and reports ordered phase
progress, one completed snapshot or one typed failure. The phase order and RNG
are designed remake choices because the original order and RNG are unrecovered;
see [docs/contracts/simulation-contracts.md](docs/contracts/simulation-contracts.md).

PR5 adds all 10 primary and 14 lesser traits, habitability, growth, the seven
recovered efficiency attributes, research-cost choices, structured validation,
a named advantage-point breakdown and a six-step race wizard. The reference
Humanoids anchor evaluates to the decompilation's documented 25 displayed
points. Trait selections are builder-only until their runtime owners in
PR6-PR13 land; the exclusion matrix and lesser-checkbox mapping remain open. See
[docs/rules/race-builder.md](docs/rules/race-builder.md).

## Prerequisites

- Rust 1.96.0. The repository pins this stable release in
  [`rust-toolchain.toml`](rust-toolchain.toml), with `rustfmt` and `clippy`.
- Python 3.11 or newer for the PR1 inspection tools (validated with Python
  3.14.8).
- The supplied `stars26i.exe` and `Stars! - RIP/stars!.hlp` in the repository
  root for evidence work. These original payloads are ignored by Git.
- For the private PDF converter only: Pillow and ReportLab
  (`python3 -m pip install Pillow reportlab`). The inspection and test tools
  otherwise use the Python standard library.

Linux window-system development packages may be needed to build the desktop
package; install `libwayland-dev` and `libxkbcommon-dev` locally. A working
Vulkan or compatible wgpu adapter is required at runtime.

## Build, run and test

```sh
cargo run -p stars-desktop
cargo run -p stars-tools
```

Run the required checks from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-core --test race_builder
cargo test -p stars-desktop --test race_editor
cargo test -p stars-desktop --test shell
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-desktop --test turn_worker
cargo test --workspace --all-targets
```

`stars-tools` is a headless foundation check; it prints the core contract
version and opponent name and does not simulate a game yet.

Run the PR1/PR2 Python tests with:

```sh
python3 -m unittest discover -s tools/analysis/tests -v
```

## Desktop foundation

The window starts at 1280x800 with a 640x420 minimum. It is freely resizable
and has no fixed aspect ratio. The inspector can be dragged wider or narrower;
the map consumes the remaining space. Text and controls scale with the UI scale
factor in addition to the operating system's HiDPI scale.

The synthetic worker generates a deterministic map preview from integer
coordinates. It reports progress between rows. Applying a completed result is
atomic and one-shot. A failure leaves the previously accepted preview visible,
and responses from superseded request IDs are ignored. Closing the window drops
the worker handle, which requests shutdown and joins the worker thread. A
shutdown request also interrupts the worker's between-row delay, so a long
`step_delay` cannot hold the window open.

## Workspace layout

| Package | Responsibility | Dependencies |
|---|---|---|
| `crates/stars-core` | State, race validation, rules, orders, views, events, deterministic turn execution | None |
| `crates/stars-ai` | Turindrones planning and serializable AI memory | `stars-core` |
| `crates/stars-desktop` | egui/eframe UI, worker ownership, local I/O, packaging entry point | `stars-core`, `stars-ai` |
| `crates/stars-tools` | Headless scenarios, evidence conversion, comparisons and benchmarks | `stars-core`, `stars-ai` when running games |

Core deliberately has no UI, operating-system, clock, filesystem or thread
dependencies. Verify the boundary locally with `cargo tree -p stars-core`; it
must list no dependencies. The AI and tools packages must not pull in the
desktop/UI stack.

## Chosen dependency versions

Versions are recorded here and pinned by the committed `Cargo.lock`.

| Dependency | Version | Role |
|---|---|---|
| Rust | 1.96.0 | Pinned stable toolchain |
| `eframe` | 0.36.2 | Native window, event loop and wgpu integration |
| `egui` | 0.36.2 | Immediate-mode UI and procedural painting |
| `wgpu` | 30.0.1 | Renderer selected through eframe's `wgpu` feature |
| `winit` | 0.30.13 | Native windowing backend |
| `env_logger` | 0.11.11 | Opt-in diagnostics via `RUST_LOG` |
| `log` | 0.4.34 | Logging facade |

Egui/eframe 0.36.2 requires Rust 1.95 or newer and uses Rust 2024 edition. The
font set is eframe's bundled, licensed default set; no extracted game art or
copyrighted help images are used.

## Platform checks

The following status is recorded honestly rather than inferred:

| Check | Status |
|---|---|
| Linux x86_64 build, clippy, unit and shell tests | Executed locally |
| Linux window startup with wgpu (Vulkan adapter) | Executed locally; the process was stopped by an external timeout after startup, so this is not a window-close smoke test |
| Windows native build and runtime | Not executed locally; the cross-platform workflow is parked until the final PR, and a local cross-check could not run because the pinned toolchain lacks the Windows target standard library |
| macOS native build and runtime | Not executed locally; the cross-platform workflow is parked until the final PR |
| Manual 100/150/200% OS scaling and mixed-DPI window movement | Not executed locally; headless layout tests run at 100/150/200% UI scale with several window aspect ratios |
| Manual OS window-close test | Not executed locally; worker shutdown and model behavior are covered by tests |

GitHub Actions is intentionally disabled until the final PR. The intended
Ubuntu/Windows/macOS workflow is preserved as
`.github/workflows/ci.yml.disabled`; GitHub does not run files with that
extension. All current validation is performed locally.

## PR1 analysis tools

The Python tools inspect the supplied 16-bit Windows executable and Windows
help file without executing either one. See
[docs/evidence/inventory.md](docs/evidence/inventory.md) for findings,
confidence levels, limitations and open questions.

### Inventory the executable

```sh
python3 tools/analysis/inventory.py stars26i.exe --output analysis/inventory.json
```

This writes `analysis/inventory.json`, which is ignored by Git. The tool does
not execute the game. It parses the DOS/NE headers, segments, resources, names,
imports, entry table, dialog templates, and a bounded set of selected strings.

### Inspect the help file

```sh
python3 tools/analysis/help.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/inventory.json \
  --text-output analysis/help/topics.txt
```

This writes container metadata and a topic index plus decoded help text. Both
outputs are ignored by Git. The decoded help is documentation evidence, not
proof that the executable implements the same behavior.

### Generate a private PDF

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

#### How the PDF conversion works

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

## Repository layout

```text
crates/stars-core/       Deterministic core contracts (no dependencies)
crates/stars-ai/         Turindrones opponent boundary
crates/stars-desktop/    egui/eframe shell and background worker
crates/stars-tools/      Headless workspace utility
.github/workflows/       Parked CI workflow (ci.yml.disabled, not run)
tools/analysis/          PR1 Python inspection, help-decoding and PDF tools
tools/analysis/tests/    Synthetic NE/HLP/PDF tests
docs/evidence/           Committed evidence index and findings
docs/rules/              PR2 rules ledger, gap ledger and turn-order notes
analysis/inventory.json  Ignored executable inventory
analysis/help/           Ignored decoded help, images and private PDF
Stars! - RIP/            Ignored supplied help/archive files
stars26i.exe             Ignored supplied executable
```

## Notes for future agents

- PR1 findings are evidence leads, not recovered rules. Read
  [docs/evidence/inventory.md](docs/evidence/inventory.md) and
  [docs/rules/README.md](docs/rules/README.md), and keep observed, statically
  derived, documented-only, inferred and unknown claims separate.
- The PR3 map preview and worker are foundation scaffolding only. Do not treat
  them as gameplay rules or universe generation.
- PR4 owns the deterministic game-state, order and turn-processing contracts.
  Keep worker timing and UI animation out of rules and RNG consumption.
- Regenerate derived artifacts with the commands above; never commit
  `analysis/`, the supplied executables/help files, extracted images, or the
  PDF.
- Keep the `[[IMAGE ...]]` marker contract in sync between `help.py` and
  `hlp_to_pdf.py`. The PDF converter should continue to fail loudly or report
  unavailable pictures rather than invent image bytes.
- The help is documentation, not proof that `stars26i.exe` implements the same
  numbers. Version-specific rules still need PR2 evidence and controlled
  scenarios.
