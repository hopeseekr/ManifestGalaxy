# PR8: Implemented planetary population and economic production.

- Branch name: `pr8-planet-economy`
- Branch merge commit message: `Implemented planetary population and economic production.`
- GitHub PR title: `Implemented planetary population and economic production.`
- Git commit message: `Implemented planetary population and economic production.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR7](PLAN-pr7-description.md). Start only after its authorized integration.

Implement population, mining, industrial output and production queues as authoritative turn phases.

PR7 UI and PR2 economic rules are required, including racial modifiers and phase ordering. Unresolved point-impacting or resource formulas are blockers.

## Owned paths and contracts

Own crates/stars-core/src/economy/, economy fixtures, and the desktop planet/production editors.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Implement population growth/limits and environmental effects, resource generation, mine output/concentration changes, factory/mine costs and operation, and applicable racial variants using evidence-linked calculations.

2. Implement ordered production queues, finite/repeating orders and capacity/resource constraints only where recovered. Carry forward partial work and report completion, shortages and unusable orders according to the rule specification.

3. Apply phases in the recovered sequence; avoid counting newly completed infrastructure too early. Track mineral and population transfers explicitly so tests can check accounting.

4. Expose economy summaries and validated queue-edit commands to the inspector. Display predicted outcomes only through a pure core projection API and identify unknown future conditions.

5. Record all custom-race economic variants in the trait coverage matrix, including nonstandard population/production models rather than assuming all races use factories identically.

## Acceptance scenarios and validation

- Implement ordinary mineral alchemy and the Mineral Alchemy trait's improvement as separate, evidence-backed production rules. Test resource consumption, output of each mineral, partial/automatic queue behavior where supported, and enabled/disabled trait comparisons. Recover costs and rounding rather than assuming the trait is required to perform alchemy at all.
- Boundary populations, zero resources, exhausted concentrations, hostile environments and immunity cases match evidence.
- Queue completion and partial progress preserve accounting; invalid orders never create free minerals or resources.
- Reference multi-turn colonies reproduce observed growth/output, including rounding and special race variants.
- Verify trait-specific economy, population and growth behavior for Hyper-Expansion, Alternate Reality, Jack of All Trades, Inner-Strength, and Only Basic Remote Mining; recover applicability and effects against the ordinary baseline and record each result in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage). Mineral Alchemy remains covered by the existing scenario above.

Implement and execute these checks:

```sh
cargo test -p stars-core --test planet_economy
cargo test -p stars-desktop --test production_editor
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No ship component catalog shortcuts, combat damage, AI planning, or UI-owned production formulas.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
