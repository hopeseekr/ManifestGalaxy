# PR13: Implemented special technologies and racial abilities.

- Branch name: `pr13-special-mechanics`
- Branch merge commit message: `Implemented special technologies and racial abilities.`
- GitHub PR title: `Implemented special technologies and racial abilities.`
- Git commit message: `Implemented special technologies and racial abilities.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR12](PLAN-pr12-description.md). Start only after its authorized integration.

Close the complete custom-trait and special-technology coverage matrix.

PR12 basic systems and the full PR2 inventory are required. This is a mandatory completeness gate, not an optional expansion.

## Owned paths and contracts

Own focused special-rule modules in crates/stars-core, catalog extensions, affected desktop controls and docs/rules/trait-coverage.md.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

Close every row in the [canonical custom-race trait coverage in PLAN.md](PLAN.md#custom-race-trait-coverage). The final gate covers exactly 10 primary and 14 lesser traits, including point behavior, runtime effects, evidence-backed interactions and cross-system boundary cases; a missing recovered effect is an explicit completion blocker, never a reason to remove a trait from the human builder.

1. Audit every custom-builder trait and catalog entry against runtime effects in PR8–PR12. List each implemented, missing, unreachable or unverified effect with evidence and integration point.

2. Implement remaining recovered mechanics such as terraforming, remote mining, minefields/minelaying/sweeping, gates, mass drivers/packets, salvage and unusual racial population or production behavior where present in the inventory.

3. Integrate special movement, attacks and transfers into the verified phase order; explicitly cover multi-mechanic interactions rather than implementing each in isolation.

4. Enable UI commands and catalog components only when their runtime effects are implemented. Replace temporary unavailable states from earlier PRs with validated commands and core projections.

5. Add a per-trait regression manifest linking each selectable trait to evidence and tests. A missing required effect blocks completion; removing the trait from the builder is not an acceptable workaround.

## Acceptance scenarios and validation

- Total Terraforming: verify and implement its starting capability, research requirements, improvement limits and resource-cost effects. Test ordinary versus trait-enabled terraforming, environmental bounds, immunity interactions, and production queue integration.
- Ultimate Recycling: verify and implement trait-specific recovery from scrapped ships, including mineral/resource returns, location/starbase conditions, timing and caps where applicable. Test trait enabled/disabled and conservation of recovered materials; use the same validated scrapping-order interface and provide the corresponding fleet UI action.
- Mineral Alchemy: verify PR8's trait-specific production behavior end-to-end, including interactions with production queues and resource availability. All three named traits must have point-cost fixtures and runtime regression cases in the coverage matrix.
- Every selectable trait has verified point behavior and applicable runtime tests or a clearly reported completion blocker.
- Test gate capacity/damage, minefield travel, remote mining ownership, environmental bounds and packet interactions as applicable to recovered rules.
- Cross-system tests exercise special effects with fuel, combat, visibility and economy without violating deterministic order or accounting.

Implement and execute these checks:

```sh
cargo test -p stars-core --test special_mechanics
cargo test -p stars-core --test trait_coverage
cargo test -p stars-desktop --test special_orders
cargo test -p stars-core --test primary_traits
cargo test -p stars-core --test lesser_traits
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No removal of difficult custom traits, extra race presets, unrelated UI redesign, or unverified mechanics marked complete.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
