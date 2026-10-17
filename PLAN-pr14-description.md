# PR14: Implemented the Turindrones computer opponent.

- Branch name: `pr14-turindrones-ai`
- Branch merge commit message: `Implemented the Turindrones computer opponent.`
- GitHub PR title: `Implemented the Turindrones computer opponent.`
- Git commit message: `Implemented the Turindrones computer opponent.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR13](PLAN-pr13-description.md). Start only after its authorized integration.

Implement the sole opponent type using legal commands and an explicit fidelity report.

PR13 complete rule surface and PR2 Turindrones evidence are required. Recover its race configuration separately from its decision policy.

## Owned paths and contracts

Own crates/stars-ai/src/turindrones/, AI tests, docs/rules/turindrones.md and desktop opponent setup/status integration.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Verify the configuration used for Turindrones and any recovered difficulty or capability variants. Do not equate a race-name string with the underlying AI algorithm or invent alternate preset races.

2. Implement planning over PlayerView for exploration, colonization, production, research, ship design, logistics, defense and attack. Route all actions through core validation with stable priority/tie-breaking rules.

3. Give each opponent independent serializable AI memory and deterministic RNG state keyed by stable identity. Bound planning work and handle no legal action without an infinite loop.

4. Compare controlled choices against original observations. Record verified policy branches and differences; a functional newly designed policy is allowed only when explicitly identified as such in the fidelity report, never described as exact original AI.

5. Integrate worker progress and multiple Turindrones identities. Use deterministic fallback to legal no-op orders for reported planning failures while preserving a diagnostic, rather than granting hidden resources or omniscience.

## Acceptance scenarios and validation

- Identical views and AI memory yield identical legal orders; unseen enemy changes cannot affect decisions.
- Multiple opponents maintain independent memory and randomness; test constrained resources, no colonies, trapped fleets and no legal moves.
- Run seeded scenario games showing expansion, research, logistics and combat engagement; compare covered decisions against actual reference observations.

Implement and execute these checks:

```sh
cargo test -p stars-ai --test turindrones_policy
cargo test -p stars-ai --test ai_scenarios
cargo test -p stars-desktop --test ai_worker
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No extra opponent archetypes, hidden-state shortcuts, or unsupported claims of exact original AI parity.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
