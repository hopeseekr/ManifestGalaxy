# Implementation status

## Current

PR6 (universe generation and new-game setup) is implemented on branch
`pr6-universe-setup`, created from `trunk` after PR5 was integrated in merge
commit `a60a2f5` (`Merge pull request #5 from hopeseekr/pr5-custom-race-builder`).

PR1-PR5 are integrated on `trunk`.

PR6 adds:

- typed `UniverseSettings` for the complete recovered advanced wizard: five
  galaxy sizes, four densities, four player-position choices, Galaxy Clumping,
  all six advanced options, Turindrones opponent count, four proficiency levels,
  seven victory conditions, must-meet count, minimum years and an explicit seed;
- deterministic planet generation using the recovered span, count, density,
  separation and clumping formulas from `stars-decompile/create.c`, including
  bounded integer coordinates and the 999-planet cap;
- generated planet environments, mineral concentrations, surface minerals,
  population, factories, mines, defenses, homeworld and starbase flags;
- recovered homeworld placement, environment-at-ideal, common surface-mineral
  template, 25,000/17,500 starting population, leftover conversions, and
  starting technology for all ten primary traits;
- recovered second starting worlds for Packet Physics and Interstellar Traveler
  outside Tiny galaxies, including the 4/5 and 2/5 population split;
- the four recovered Turindrones race records as the sole built-in AI
  configuration, with distinct player identities and player-keyed RNG streams;
- a new-game form with race validation, seed entry, progress, typed errors and
  atomic replacement of the active universe only on success;
- a desktop-owned universe worker with request IDs, progress checkpoints,
  completed snapshots and typed failures;
- `CORE_API_VERSION` and `STATE_FORMAT_VERSION` bumped to 3 for planet details
  and starting technology;
- a reproducible PR7 fixture and evidence/limitation documentation.

Runtime AI policy, later-turn economy, starting ship designs, starbase component
fitting and persistence remain with later PRs. No runtime AI decision effect is
claimed.

## Scope

- In scope (implemented): `crates/stars-core/src/universe/`, the
  `Planet`/`Player` generation fields, `crates/stars-desktop/src/ui/new_game.rs`,
  new-game app integration, `crates/stars-core/tests/universe_generation.rs`,
  `crates/stars-desktop/tests/new_game.rs`, evidence/limitation docs, this
  status file and the PR6 checklist entry in `PLAN.md`.
- Out of scope: AI planning, later-turn economy, ship/component catalogs,
  combat, original universe-file compatibility, original save compatibility,
  predefined-race presets, network or browser targets, execution of the original
  game, push, publish or merge.

## Checklist

- [x] Complete recovered wizard inventory with size, density, positions,
      clumping and all advanced options
- [x] Victory conditions and must-meet validation from documented ranges
- [x] Deterministic planet generation from an explicit seed
- [x] Recovered planet counts, density multipliers, bounds and clumping
- [x] Bounded, unique, non-overlapping homeworld assignment for 1-15 opponents
- [x] Generated environments, minerals, starting ownership and homeworld detail
- [x] Starting technology for all ten primary traits
- [x] Low Starting Population starts at 17,500 colonists
- [x] Alternate Reality homeworlds start without planetary installations
- [x] Packet Physics and Interstellar Traveler second start worlds
- [x] Turindrones as the sole built-in AI configuration with four proficiency
      records
- [x] Distinct opponent identities and player-keyed RNG streams
- [x] New-game form validates a `RaceSpec` before submission
- [x] Generation runs on a desktop worker and reports progress
- [x] Failed validation or generation retains the previous universe
- [x] Reproducible PR7 fixture
- [x] `cargo test -p stars-core --test universe_generation` (12 tests)
- [x] `cargo test -p stars-desktop --test new_game` (7 tests)
- [x] PLAN.md PR6 checkbox, after acceptance
- [ ] Starting ship designs and starbase components remain PR9/PR12 work
- [ ] Original AI policy remains PR14 work

## Validation

Commands run locally:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-core --test universe_generation
cargo test -p stars-desktop --test new_game
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-core --test race_builder
cargo test -p stars-desktop --test turn_worker
cargo test -p stars-desktop --test shell
cargo test -p stars-desktop --test race_editor
cargo test --workspace --all-targets
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
python3 -m unittest discover -s tools/analysis/tests
```

Results:

- The named PR6 targets pass, as do the directly affected PR4/PR5 desktop and
  core targets and the full workspace test set.
- The generator tests cover every size × density × clumping combination,
  same-seed reproduction, changed-seed divergence, density ordering, Huge
  Dense/Packed parity, clumping nearest-neighbour reduction, player-position
  separation, 16-slot maximum settings, distinct opponent streams, all primary
  starting conditions, Low Starting Population, Maximum Minerals, Accelerated
  BBS, invalid settings/races and the PR7 fixture.
- The desktop tests cover resizable multi-scale rendering of every control,
  worker progress/completion, validation retention, worker-failure retention,
  distinct opponent identities and stale request rejection.
- No original game was executed. Windows and macOS builds and native window
  smoke checks were not executed in this PR; GitHub Actions remains parked.

## Remaining blockers and limitations

- `GAP-VM-001` and `GAP-VERSION-001`: no original observation exists, and the
  January 1998 help's applicability remains unresolved.
- `GAP-RNG-001`: the universe stream is a documented remake RNG, not the
  unrecovered original algorithm.
- `GAP-UNIVERSE-PACKED-001`: the help says Packed is +30% over Dense; the
  decompiled formula yields +20%, and Huge Packed equals Huge Dense at the cap.
- `GAP-UNIVERSE-NAMES-001`: planet names use synthetic `Star NNNN` labels.
- `GAP-UNIVERSE-START-001`: starting ships and starbase component fitting remain
  PR9/PR12 work.
- `GAP-UNIVERSE-VICTORY-001`: victory defaults are remake interface defaults;
  runtime enforcement is PR15 work.
- `GAP-TURIN-001`: the race configuration is recovered, but Turindrones ship
  designs, research choices and AI policy remain PR14 work.
- `GAP-COMMAND-001`: the `game.def` command-line path was not run.
- `GAP-EXCLUSIONS-001` and `GAP-LESSER-MAP-001` remain PR5 limitations.
