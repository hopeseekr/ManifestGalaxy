# Implementation status

## Current

PR4 (deterministic game state and turn-processing contracts) is implemented on
branch `pr4-simulation-contracts`, created from `trunk` after PR3 was integrated
in merge commit `4fbdfc4` (`Merge pull request #3 from hopeseekr/pr3-rust-foundation`).

PR1 (binary evidence inventory and extraction tools), PR2 (documented recovered
rules and reference scenarios) and PR3 (Rust workspace and desktop foundation)
are integrated on `trunk`.

PR4 adds:

- typed `u64` identifiers for players, planets, fleets, designs, batches and
  commands, plus monotonic serialized allocators;
- integer-only authoritative positions and ordered `BTreeMap` collections;
- `GameState`, `OrderBatch`, `PlayerView`, `TurnOutcome`, typed order/view/state
  errors and canonical little-endian state encoding with version checks;
- validation before mutation, whole-batch rollback and atomic commit;
- a versioned designed phase plan with explicit `Implemented` versus
  `NotImplemented` capability for every phase;
- a versioned designed RNG with independent streams keyed by player and
  subsystem, serialized draw counters and no clock/hash/platform seed;
- player-filtered views and events with synthetic visibility fixtures;
- a desktop turn worker that owns the authoritative session, reports ordered
  phase progress and returns one completed snapshot or one typed failure;
- a headless `TurnSessionModel` that ignores stale responses and retains the
  last committed snapshot and view after a failure.

Only administrative `SetReady` and `RenameFleet` commands are implemented.
Economy, research, construction, movement, scanning, combat, packets, victory,
AI policy, persistence and all gameplay formulas remain owned by later PRs.
Every gameplay phase is an explicit not-implemented no-op, not a silent success.
The phase order and RNG are designed remake choices, not recovered original
behaviour; `GAP-TURN-001` and `GAP-RNG-001` remain open.

GitHub Actions is intentionally disabled until the final PR, per the project
owner; all validation is currently local.

## Scope

- In scope (implemented): `crates/stars-core/src/{model,orders,turn,view,rng}.rs`,
  `crates/stars-desktop/src/turn_worker.rs`, the named contract test targets,
  `docs/contracts/simulation-contracts.md`, updated evidence limitation notes,
  `README.md`, this status file, and the PR4 checklist entry in `PLAN.md`.
- Out of scope: gameplay rules and formulas, universe generation, custom-race
  builder, Turindrones planning, persistence, network or browser targets,
  execution of the original game, push, publish or merge.

## Checklist

- [x] Stable typed IDs; ordered authoritative collections; explicit integer
      widths; display projection kept out of authoritative positions
- [x] `GameState`, `OrderBatch`, `PlayerView` and `TurnOutcome` contracts
- [x] Batch validation rejects unauthorized, stale, duplicate and invalid
      requests before mutation
- [x] Atomic commits retain the previous state, events and RNG on failure
- [x] Versioned RNG state with independent player/subsystem streams
- [x] Canonical state serialization records phase-plan and RNG versions
- [x] Versioned phase dispatcher with explicit capability status and no-op
      scaffolding for unimplemented gameplay
- [x] Player-filtered views and events omit hidden entities and hidden events
- [x] Desktop turn worker with request IDs, ordered progress, completed
      snapshots, typed failures and stale-request rejection
- [x] Headless UI model ignores delayed/stale completions and retains the last
      committed snapshot on failure
- [x] Empty-game and dangling-reference view checks
- [x] `cargo test -p stars-core --test simulation_contracts` (13 tests)
- [x] `cargo test -p stars-desktop --test turn_worker` (6 tests)
- [x] PLAN.md PR4 checkbox, after acceptance
- [ ] Original global turn order and RNG remain unresolved (tracked gaps)

## Validation

Commands run locally:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-desktop --test turn_worker
cargo test --workspace --all-targets
```

Results:

- Formatting, compilation, Clippy, both named test targets and the full
  workspace test set pass.
- The core target covers identical serialized state and event order for
  identical inputs, all validation rejections, whole-batch rollback, phase
  progress and capability labels, RNG continuation after serialization,
  canonical decoding failures, allocator continuation, and player-view filtering
  including empty games and dangling references.
- The desktop target uses the real worker thread to cover ordered progress, a
  committed snapshot and filtered view, failure retention, non-monotonic request
  rejection and delayed-completion handling.
- No original game was executed. Windows and macOS builds and native window
  smoke checks were not executed in this PR; GitHub Actions remains parked.

## Remaining blockers

- `GAP-TURN-001`: the original global cross-phase order is unknown. The PR4
  pipeline is explicitly labelled designed and can be replaced when evidence
  arrives.
- `GAP-RNG-001`: the original RNG algorithm, seed and consumption order are
  unknown. The PR4 RNG is explicitly labelled designed and cannot be claimed as
  original-fidelity replay.
- Gameplay required by later PRs remains blocked or uncertain exactly as
  recorded in [docs/rules/gap-ledger.md](docs/rules/gap-ledger.md); PR4 did not
  invent any of those rules.
