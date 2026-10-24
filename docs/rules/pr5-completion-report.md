# PR5 completion report

Branch: `pr5-custom-race-builder`, created from merged `trunk` at `d6b9c02`
(`Merge pull request #4 from hopeseekr/pr4-simulation-contracts`).
Scoped commit message: `Implemented the custom race builder and point calculations.`

PR5 adds the custom race builder and the recovered advantage-point calculation.
It did not execute the original game, push, publish or merge. Runtime trait
effects, universe generation, new-game setup and the full save/recovery system
remain with later PRs. PR5 now includes a focused validated-race JSON export.

## Deliverables

| Path | Purpose |
|---|---|
| `crates/stars-core/src/race/traits.rs` | All 10 primary and 14 lesser traits, costs, evidence labels and the 24-row effect checklist |
| `crates/stars-core/src/race/spec.rs` | Typed `RaceSpec`, editable `RaceDraft`, recovered field ranges and builder defaults |
| `crates/stars-core/src/race/points.rs` | Pure point evaluation, named terms, rule identifiers and the final truncating division |
| `crates/stars-core/src/race/habitability.rs` | Injected habitability model, recovered three-pass sampling and reference desirability reconstruction |
| `crates/stars-core/src/race/validate.rs` | Structured field errors, field ranges, one-primary check and non-negative balance gate |
| `crates/stars-core/src/race/mod.rs` | Public exports, canonical state encoding for a race, trait coverage iterator |
| `crates/stars-core/src/model.rs` | Optional validated `Player::race`; state format version 2 |
| `crates/stars-desktop/src/ui/race_builder.rs` | Six-step wizard, review page, live totals, tooltips, keyboard navigation, reset and summary |
| `crates/stars-desktop/src/race_file.rs` | Versioned validated-race JSON encode/decode, save and load |
| `crates/stars-desktop/src/app.rs` | Menu/window integration and validated-race display |
| `crates/stars-core/tests/race_builder.rs` | 19 focused core acceptance tests |
| `crates/stars-desktop/tests/race_editor.rs` | 11 focused editor acceptance tests |
| `docs/rules/race-builder.md` | Field inventory, formula rule IDs, deliberate decisions, checklist and limitations |
| `docs/rules/advantage-points.md` | PR5 implementation notes and reference-anchor result |
| `docs/rules/rule-ledger.md`, `docs/rules/gap-ledger.md` | Updated implementation status and open limitations |
| `docs/contracts/simulation-contracts.md` | Version-2 player/race contract note |
| `STATUS.md`, `PLAN.md` | Current status and PR5 checklist update |

## Acceptance evidence

- All 24 canonical traits are represented and selectable. A test asserts ten
  primary and fourteen lesser IDs.
- Exactly one primary trait is required structurally; an incomplete draft
  reports `MissingPrimaryTrait` and cannot finish.
- Total Terraforming, Ultimate Recycling and Mineral Alchemy are individually
  selectable, priced separately from research purchases, and tested alone and
  together.
- The Humanoids reference record evaluates to 75 raw third-points and 25
  displayed points, matching the decompilation's own anchor.
- Growth, resource-generation, factory, mine, research and lesser-trait count
  curves are pinned by focused tests.
- Boundary values succeed; inverted bands, out-of-range attributes, long names,
  bad emblems and negative balances return field-specific errors.
- The editor preserves unrelated selections when stepping backward and clears
  the draft on reset. It rejects overspent and invalid races without losing the
  draft.
- UI evaluation is compared directly with `evaluate_race`; both agree.
- The editor renders every page at 100%, 150% and 200% zoom with different
  window proportions and accepts PageUp/PageDown and Ctrl+R.
- A validated race round-trips through canonical state in the new optional
  `Player::race` field.
- Finish saves a validated race as versioned JSON using an editable file name
  prefilled from the race name; the decoder validates it again on load.

## Commands actually run

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

- Formatting, compilation and Clippy pass.
- `race_builder`: 19 passed.
- `race_editor`: 11 passed.
- `simulation_contracts`: 17 passed.
- `turn_worker`: 6 passed.
- `shell`: 8 passed.
- Full workspace tests: all targets passed.
- Scenario validator: OK against the existing 59 scenarios and rule index.
- Python analysis tests: 93 passed.

No Windows or macOS build and no native window smoke test were executed. No
original game was executed, so no observed result is claimed.

## Remaining limitations

- `GAP-EXCLUSIONS-001` remains open: no guessed exclusion rule is enforced.
- `GAP-LESSER-MAP-001` remains open: the lesser-checkbox runtime mapping is
  unproven; PR5 follows the canonical plan order and recovered cost indices.
- `GAP-VM-001` and `GAP-VERSION-001` remain open.
- Ultimate Recycling's price is inferred.
- The reference planet-desirability reconstruction is a builder convenience
  until PR8 owns the authoritative planet economy.
- The formula is statically derived from a decompilation of a later build, not
  observed against `stars26i.exe`.
