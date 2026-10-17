# PR5: Implemented the custom race builder and point calculations.

- Branch name: `pr5-custom-race-builder`
- Branch merge commit message: `Implemented the custom race builder and point calculations.`
- GitHub PR title: `Implemented the custom race builder and point calculations.`
- Git commit message: `Implemented the custom race builder and point calculations.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR4](PLAN-pr4-description.md). Start only after its authorized integration.

Implement the full human custom race builder with verified validation and point calculations.

PR4 contracts and the PR2 builder inventory are required. Resolve field ranges, point formulas, rounding and trait interactions before claiming this PR complete.

## Owned paths and contracts

Own crates/stars-core/src/race/, crates/stars-desktop/src/ui/race_builder.rs, and focused race/model/UI tests.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

Implement the typed trait IDs and builder coverage from the [canonical custom-race trait coverage in PLAN.md](PLAN.md#custom-race-trait-coverage): all 10 primary and 14 lesser traits remain selectable for human custom races. Enforce the recovered rule of exactly one primary trait plus any valid lesser-trait combination, with evidence-backed exclusions and point interactions. Do not replace unresolved costs or interactions with defaults.

1. Represent all inventoried primary and lesser traits, habitability controls, growth, economic and research parameters as typed RaceSpec data. Preserve all exposed capabilities even though preset races are omitted. Do not invent additional wizard fields.

2. Implement pure validate_race and evaluate_race functions returning structured field errors and a point breakdown. Derive numeric bounds and interactions from evidence; give every nontrivial formula a rule ID and fixture.

3. Build wizard pages from the recovered field inventory, with keyboard controls, explanatory tooltips, live point totals, reset and a final summary. Incomplete editor drafts are allowed; game creation accepts only a validated race.

4. Keep builder selections separate from runtime trait implementations scheduled for PR8–PR13. Maintain a trait-to-effect checklist so accepting a selection never conceals a permanently missing ability.

5. Pass the validated race to the new-game flow contract; standalone race-file persistence is deferred to PR16. Test editing, stepping backward and resetting without losing or corrupting unrelated selections.

## Acceptance scenarios and validation

- Total Terraforming, Ultimate Recycling, and Mineral Alchemy must be individually selectable and represented in RaceSpec, with evidence-backed point costs and validation. Test each enabled/disabled and in combinations; do not confuse these traits with technologies purchased through research.
- Reference race configurations match observed point totals, including boundary and interacting-trait cases.
- Invalid combinations and overspent budgets prevent completion with field-specific messages; legal boundary values succeed.
- Every inventoried field is represented, keyboard reachable, and readable at multiple scales; UI values and core evaluation agree.

Implement and execute these checks:

```sh
cargo test -p stars-core --test race_builder
cargo test -p stars-desktop --test race_editor
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No other preset races, invented race fields, AI behavior, or claim that runtime trait effects are complete before PR13.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
