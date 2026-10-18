# Implementation status

## Current

PR3 (Rust workspace and desktop application foundation) is implemented on
branch `pr3-rust-foundation`, created from `trunk` after PR2 was integrated in
merge commit `bb6dc56` (`Merge pull request #2 from hopeseekr/pr2-rules-specification`).

PR1 (binary evidence inventory and extraction tools) and PR2 (documented
recovered rules and reference scenarios) are integrated on `trunk`.

PR3 adds four Rust packages, a pinned Rust 1.96.0 toolchain, `Cargo.lock`, a
format/lint/test/cross-platform CI workflow, and a runnable egui/eframe desktop
shell. The shell has a menu, resizable map area, inspector, status area,
configurable UI scale, keyboard focus, procedural placeholder rendering, and a
desktop-owned background worker with request IDs and typed errors. It contains
no gameplay, economy, AI policy, persistence or original assets.

## Scope

- In scope (implemented): root Cargo configuration, `rust-toolchain.toml`,
  `Cargo.lock`, `crates/stars-core`, `crates/stars-ai`, `crates/stars-desktop`,
  `crates/stars-tools`, `.github/workflows/ci.yml`, `README.md`, this status
  file, and the PR3 checklist entry in `PLAN.md`.
- Out of scope: gameplay rules, universe generation, custom-race builder,
  Turindrones planning, persistence, network or browser targets, execution of
  the original game, push, publish or merge.

## Checklist

- [x] Four packages with Rust 2024 edition and pinned stable Rust 1.96.0
- [x] `Cargo.lock` committed; chosen dependency versions recorded in README
- [x] `stars-core` has no dependencies; AI depends on core; desktop on core/AI;
      tools on core/AI
- [x] egui/eframe 0.36.2 with the wgpu renderer
- [x] Desktop shell with menu, resizable map area, inspector and status area
- [x] Scalable text, UI scale presets (100/125/150/200%) and continuous slider
- [x] Keyboard focus, `F5` synthetic scan and `Ctrl+Q` exit shortcuts
- [x] Original procedural placeholder map; no extracted game art
- [x] Background worker with request IDs, progress, completed snapshot and
      typed failure messages
- [x] Atomic one-shot result application; stale results ignored; last accepted
      snapshot retained on failure
- [x] Worker shutdown and join when the handle is dropped
- [x] Headless shell-model and worker tests (`cargo test -p stars-desktop --test shell`)
- [x] Headless layout exercise at 100/150/200% UI scale and multiple aspect ratios
- [x] Linux, Windows and macOS build jobs; Linux format, lint and test job
- [x] CI dependency-boundary check
- [x] README build, run, test, dependency and platform-status documentation
- [x] PLAN.md PR3 checkbox, after acceptance

## Validation

Commands run locally:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-desktop --test shell
cargo test --workspace --all-targets
cargo run -p stars-tools
timeout 8s cargo run -p stars-desktop
```

Results:

- Formatting, compilation, Clippy, workspace tests and the named shell test
  target pass.
- The desktop process started with the wgpu/Vulkan backend on Linux and stayed
  running until an external timeout stopped it. This confirms startup, not an
  in-window close or a manual scaling check.
- Windows and macOS builds are defined in CI but were not executed locally.
  A local `cargo check --target x86_64-pc-windows-gnu` attempt could not start
  because the pinned toolchain does not have that target's `core`/`std`
  installed; this was not a source-code failure.
- Manual operating-system HiDPI scaling, mixed-DPI movement and window-close
  smoke checks were not executed locally. The headless layout test covers
  100/150/200% UI scale and arbitrary window proportions; worker shutdown and
  the failure-retains-snapshot behavior are covered by Rust tests.

## Remaining blockers

No known blocker for the PR3 foundation scope. Gameplay required by later PRs
remains blocked or uncertain exactly as recorded in
[docs/rules/gap-ledger.md](docs/rules/gap-ledger.md); PR3 did not invent any of
those rules.
