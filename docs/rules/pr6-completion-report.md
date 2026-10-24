# PR6 completion report

Branch: `pr6-universe-setup`, created from `trunk` after PR5 was integrated in
merge commit `a60a2f5` (`Merge pull request #5 from hopeseekr/pr5-custom-race-builder`).
Scoped commit message: `Implemented universe generation and new-game setup.`

PR6 adds deterministic universe generation and the complete recovered new-game
setup. It did not execute the original game, push, publish or merge. Starting
ship designs, starbase component fitting, AI policy, later-turn economy and
persistence remain with later PRs.

## Deliverables

| Path | Purpose |
|---|---|
| `crates/stars-core/src/universe/settings.rs` | Typed size, density, positions, difficulty, options, victory conditions and seed validation |
| `crates/stars-core/src/universe/generate.rs` | Recovered planet counts, coordinates, clumping, environments, minerals, homeworlds, starting inventory and technologies |
| `crates/stars-core/src/universe/turindrones.rs` | Four recovered Turindrones race records as typed races |
| `crates/stars-core/src/universe/mod.rs` | Public universe API |
| `crates/stars-core/src/model.rs` | Planet environment/mineral/inventory fields and player starting technology; state format version 3 |
| `crates/stars-desktop/src/ui/new_game.rs` | Headless new-game form, saved-race loading, generation worker, progress, typed errors and atomic retention |
| `crates/stars-desktop/src/app.rs` | New-game menu, window and inspector summary |
| `crates/stars-core/tests/universe_generation.rs` | 12 focused core acceptance tests, including the PR7 fixture |
| `crates/stars-desktop/tests/new_game.rs` | 10 focused desktop acceptance tests |
| `docs/rules/universe-generation.md` | Recovered formulas, wizard inventory, starting conditions, Turindrones records, designed choices and gaps |
| `docs/rules/rule-ledger.md`, `docs/rules/gap-ledger.md` | Updated evidence notes and limitations |
| `docs/contracts/simulation-contracts.md` | Version-3 state contract note |
| `STATUS.md`, `PLAN.md` | Current status and PR6 checklist update |

## Acceptance evidence

- Same seed and settings reproduce the exact universe; a changed seed changes
  the planets.
- Every size × density × clumping combination produces bounded, usable planets
  and distinct homeworlds. Counts follow the recovered formula and density
  ordering. Huge Dense and Huge Packed match because of the 999 cap.
- Clumping reduces the mean nearest-neighbour distance across repeated seeds;
  Distant player positions separate homeworlds more than Close positions.
- A 16-slot Huge Packed game places distinct human and Turindrones identities
  with no ownership overlap.
- All ten primary traits produce their recovered starting technology, the
  recovered homeworld environment and the correct installation rules. Low
  Starting Population starts at 17,500 colonists. Packet Physics and
  Interstellar Traveler receive second worlds outside Tiny galaxies.
- Maximum Minerals sets concentration 100; Accelerated BBS scales population;
  homeworlds share the surface-mineral template before leftover spending.
- Invalid settings and races return typed errors before a state is produced.
  The desktop form retains the previously committed universe after validation,
  worker or generation failures.
- The form discovers saved race JSON files, loads and revalidates them, and uses
  an explicitly loaded file until the user clears it. A failed race-file load
  retains both the current race and the current universe.
- Victory conditions use sliders with the requested increments: five points for
  planet percentage, nearest-rival percentage and capital ships; 1,000 points
  for score; production capacity displayed in thousands with 10-thousand steps.
- The generation worker reports progress checkpoints, one completed universe or
  a typed failure, and rejects stale request identifiers.
- The PR7 fixture is pinned by seed, settings, planet count and homeworld IDs.

## Commands actually run

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

- Formatting, compilation and Clippy pass.
- `universe_generation`: 12 passed.
- `new_game`: 10 passed.
- `simulation_contracts`: 17 passed.
- `race_builder`: 21 passed.
- `turn_worker`: 6 passed.
- `shell`: 8 passed.
- `race_editor`: 14 passed.
- Full workspace tests: all targets passed.
- Scenario validator: OK.
- Python analysis tests: 93 passed.

No Windows or macOS build and no native window smoke test were executed. No
original game was executed, so no observed result is claimed.

## Remaining limitations

- `GAP-VM-001` and `GAP-VERSION-001` remain open.
- `GAP-RNG-001`: generation uses a documented remake stream, not the original
  algorithm.
- `GAP-UNIVERSE-PACKED-001`: the help says Packed is +30% over Dense; the
  recovered formula yields +20%, and Huge Packed equals Huge Dense.
- `GAP-UNIVERSE-NAMES-001`: synthetic star names.
- `GAP-UNIVERSE-START-001`: starting ships and starbase components remain
  PR9/PR12 work.
- `GAP-UNIVERSE-VICTORY-001`: victory defaults are interface defaults; runtime
  enforcement remains PR15 work.
- `GAP-TURIN-001`: race records are recovered, but ship designs, research
  choices and AI policy remain PR14 work.
- `GAP-COMMAND-001`: the `game.def` command-line path was not run.
- `GAP-EXCLUSIONS-001` and `GAP-LESSER-MAP-001` remain PR5 limitations.
