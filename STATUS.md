# Implementation status

## Current

PR5 (custom race builder and point calculations) is implemented on branch
`pr5-custom-race-builder`, created from `trunk` after PR4 was integrated in
merge commit `d6b9c02` (`Merge pull request #4 from hopeseekr/pr4-simulation-contracts`).

PR1 (binary evidence inventory and extraction tools), PR2 (documented recovered
rules and reference scenarios), PR3 (Rust workspace and desktop foundation) and
PR4 (deterministic state and turn-processing contracts) are integrated on
`trunk`.

PR5 adds:

- typed `PrimaryTrait` and `LesserTrait` inventories for all 10 primary and 14
  lesser traits, with recovered point values and a trait-to-effect checklist;
- `RaceSpec`, an editable `RaceDraft`, all recovered habitability, growth,
  efficiency and research fields, and the Step 1 name, emblem, password and
  leftover target;
- pure `evaluate_race` with named point terms, a raw third-point accumulator and
  one final truncating division, plus injected habitability;
- pure `validate_race` with structured field errors, recovered field ranges,
  exactly one primary trait and the non-negative balance gate;
- the recovered `LInnateRaceHabitability` sampling and a documented reference
  desirability reconstruction that reproduces the decompilation's Humanoids
  anchor of 25 displayed points;
- an egui custom-race wizard with six recovered steps, a review page, live point
  totals, tooltips, keyboard navigation, reset, a point breakdown and a final
  summary;
- the validated race in the optional `Player::race` field and canonical state
  encoding, with `CORE_API_VERSION` and `STATE_FORMAT_VERSION` bumped to 2;
- Finish-time versioned JSON race saving with an editable, suggested file name;
  the decoder validates the race again on load.

Runtime trait effects, universe generation, new-game setup and the full
save/recovery system remain owned by PR6-PR16. No runtime trait effect is
claimed.

## Scope

- In scope (implemented): `crates/stars-core/src/race/`,
  `crates/stars-desktop/src/ui/race_builder.rs`, the `Player::race` contract,
  `crates/stars-core/tests/race_builder.rs`,
  `crates/stars-desktop/tests/race_editor.rs`, updated evidence/limitation docs,
  this status file and the PR5 checklist entry in `PLAN.md`.
- Out of scope: runtime trait mechanics, universe generation, AI behaviour,
  persistence, original save compatibility, network or browser targets,
  execution of the original game, push, publish or merge.

## Checklist

- [x] All 10 primary and 14 lesser traits represented and selectable
- [x] Exactly one primary trait; incomplete drafts allowed but cannot finish
- [x] Habitability bands, immunity, ideal midpoint, growth and immunity penalty
- [x] Seven recovered efficiency attributes, CheapFact and Tech 3 option flags
- [x] Six research-cost choices and leftover target
- [x] Pure point evaluation with named terms and rule identifiers
- [x] Reference Humanoids anchor: 75 raw = 25 displayed points
- [x] Structured field validation and non-negative balance gate
- [x] Total Terraforming, Ultimate Recycling and Mineral Alchemy selectable and
      priced separately from research purchases
- [x] Trait-to-effect checklist covers all 24 rows and claims no runtime effect
- [x] Wizard pages, live totals, tooltips, keyboard navigation, reset and summary
- [x] Editing, backward navigation and reset preserve or clear the right state
- [x] Validated race passed to the new-game contract through `Player::race`
- [x] `cargo test -p stars-core --test race_builder` (19 tests)
- [x] `cargo test -p stars-desktop --test race_editor` (11 tests)
- [x] PLAN.md PR5 checkbox, after acceptance
- [ ] Runtime trait effects remain scheduled for PR6-PR13 (tracked checklist)

## Validation

Commands run locally:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-core --test race_builder
cargo test -p stars-desktop --test race_editor
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-desktop --test turn_worker
cargo test -p stars-desktop --test shell
cargo test --workspace --all-targets
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
python3 -m unittest discover -s tools/analysis/tests
```

Results:

- Formatting, compilation, Clippy, both named PR5 targets, the directly
  affected PR4 targets and the full workspace test set pass.
- The scenario validator and the Python analysis tests pass.
- The core target covers all trait selections, the special-trait combinations,
  the growth/factory/mine/research/count curves, the Humanoids anchor, field
  boundaries, overspending and canonical state round-tripping with a race.
- The desktop target covers page traversal, keyboard navigation, reset, editing,
  finish rejection and acceptance, multi-scale layout and core/UI point
  agreement.
- No original game was executed. Windows and macOS builds and native window
  smoke checks were not executed in this PR; GitHub Actions remains parked.

## Remaining blockers and limitations

- `GAP-EXCLUSIONS-001`: the exclusion matrix is unknown. PR5 accepts every
  priced combination and does not guess a rejection rule.
- `GAP-LESSER-MAP-001`: the fourteen lesser checkbox controls still have no
  proven runtime mapping. PR5 follows the canonical plan order and recovered
  cost indices.
- `GAP-VM-001` and `GAP-VERSION-001`: no original-game observation exists and
  the January 1998 help's applicability remains unresolved.
- Ultimate Recycling's `-240` price is inferred from the table's unexplained
  index 5.
- The reference planet-desirability reconstruction is a builder convenience.
  PR8 owns the authoritative planet economy and may change the habitability
  component.
- The point formula comes from a decompilation of a later build, not from
  running the supplied executable.
