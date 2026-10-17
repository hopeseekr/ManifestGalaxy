# PR3: Added the Rust workspace and desktop application foundation.

- Branch name: `pr3-rust-foundation`
- Branch merge commit message: `Added the Rust workspace and desktop application foundation.`
- GitHub PR title: `Added the Rust workspace and desktop application foundation.`
- Git commit message: `Added the Rust workspace and desktop application foundation.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR2](PLAN-pr2-description.md). Start only after its authorized integration.

Establish the Rust workspace and a resizable desktop shell before introducing game mechanics.

PR2 ledger and scenario format exist; unknown gameplay formulas do not block scaffolding.

## Owned paths and contracts

Own root Cargo configuration, crates/stars-core, crates/stars-ai, crates/stars-desktop, crates/stars-tools, README.md, and build CI. Keep PR1 Python analysis tools available.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Create the four packages named in the master plan, with Rust 2024 edition, a pinned stable toolchain selected at implementation time, and a committed Cargo.lock. Check supported dependency versions against official documentation and record the chosen versions.

2. Use egui/eframe with the wgpu renderer. Core must have no UI, OS I/O, clock, or thread dependencies; AI depends on core, desktop on core/AI, and tools may depend on core. Add serde only where data interchange needs it.

3. Create an offline desktop shell with map area, inspector, status area, menu, scalable text, keyboard focus, and configurable UI scale. Draw original procedural placeholders; do not require extracted game art.

4. Introduce a desktop-owned background-worker adapter with request IDs and progress/result/error messages. Demonstrate responsive UI with synthetic work; preserve the last accepted UI snapshot if the worker fails.

5. Add Windows, macOS and Linux build jobs, rustfmt/clippy checks, a headless shell-model test, and documented build/run instructions. Record platform checks that cannot be executed locally.

## Acceptance scenarios and validation

- A blank shell opens and resizes at 100%, 150% and 200% scaling without fixed aspect ratio.
- Synthetic background work does not block input; closing the window terminates cleanly.
- All crates compile and the dependency graph has no core-to-UI dependency.

Implement and execute these checks:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-desktop --test shell
cargo run -p stars-desktop
```

Create the named test targets in this PR. Report unavailable platform checks explicitly; do not claim an unexecuted smoke test passed.

## Evidence gate and exclusions

No economy, AI policies, persistence, browser targets, or fabricated rule implementations.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
