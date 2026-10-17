# PR9: Implemented research and ship design.

- Branch name: `pr9-research-and-designs`
- Branch merge commit message: `Implemented research and ship design.`
- GitHub PR title: `Implemented research and ship design.`
- Git commit message: `Implemented research and ship design.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR8](PLAN-pr8-description.md). Start only after its authorized integration.

Implement technology progression and validated ship/starbase designs using an evidence-backed catalog.

PR8 economy and PR2 technology/component inventory must exist; recover costs, unlock rules, slot restrictions and racial access.

## Owned paths and contracts

Own crates/stars-core/src/{research,design}/, data/catalog/, desktop research/designer panels, and catalog validation tests.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Create data-driven hull, component and technology definitions with stable IDs, evidence references, costs, slots, prerequisites and racial restrictions. Do not transcribe unsupported values from unrelated game versions.

2. Implement research allocation, progress, advancement and race modifiers with explicit rounding and phase timing. Expose unlock changes through turn events and player views.

3. Implement DesignSpec validation, derived cost/mass/capacity and performance summaries in core. Support ship and starbase designs with recoverable slot semantics and immutable design revisions referenced by built units.

4. Integrate validated design IDs with production queues. Editing a design must not silently rewrite existing ships or completed production.

5. Build research allocation controls and the ship/starbase designer with slot feedback, component filtering and core-computed totals; produce original procedural icons.

## Acceptance scenarios and validation

- Known designs match reference costs and derived statistics; illegal slots and inaccessible components are rejected.
- Research boundary advances, overflow and racial modifiers match fixtures.
- Editing/replacing a design preserves existing unit identity and queue semantics; catalog references and prerequisites are valid.
- Verify Generalized Research, Bleeding Edge Technology research cost and miniaturization, and Improved Starbases cost, hull, and access behavior, including all trait-specific hull and component access; recover applicability and effects and record them in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage).

Implement and execute these checks:

```sh
cargo test -p stars-core --test research_and_designs
cargo test -p stars-core --test catalog_integrity
cargo test -p stars-desktop --test design_editor
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No guessed catalog values, battle resolution, AI research policy, or direct UI arithmetic for design statistics.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
