# PR15: Integrated complete games and victory conditions.

- Branch name: `pr15-complete-game-loop`
- Branch merge commit message: `Integrated complete games and victory conditions.`
- GitHub PR title: `Integrated complete games and victory conditions.`
- Git commit message: `Integrated complete games and victory conditions.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR14](PLAN-pr14-description.md). Start only after its authorized integration.

Deliver a complete in-memory game from setup through victory, defeat or resignation.

PR14 opponent and PR5–PR13 gameplay features must be integrated. Resolve recovered victory criteria and elimination behavior.

## Owned paths and contracts

Own crates/stars-desktop/src/session/, core victory/session rules, stars-tools scenario runner and end-to-end fixtures.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Wire new game, human draft orders, validation, AI planning, turn execution, reports and next-turn presentation into one session lifecycle. Prevent duplicate submission and editing of committed orders while a turn is running.

2. Implement victory-condition evaluation, player elimination, resignation and end-state presentation according to selected setup rules. Resolve simultaneous wins explicitly from evidence.

3. Handle worker errors and stale responses without replacing the last committed state; allow retry where deterministic and safe. Closing a session must not deliver an old result into a new one.

4. Add a headless deterministic scenario runner in stars-tools producing final state digests and metrics. Use it for reproducible complete games and future performance/fidelity tests.

5. Exercise all earlier UI flows with actual state rather than PR7 demonstration fixtures. Clearly disclose that session persistence arrives in PR16 and an unsaved game is lost on exit.

## Acceptance scenarios and validation

- A custom human race can start, issue orders, advance turns and finish against one or multiple Turindrones.
- Seeded fixture games reach expected terminal conditions; no infinite turn loop or duplicate event application occurs.
- Elimination, resignation, simultaneous-condition cases, worker failure/retry and new-session replacement behave consistently.

Implement and execute these checks:

```sh
cargo test -p stars-core --test victory_conditions
cargo test -p stars-desktop --test game_session
cargo test -p stars-tools --test complete_games
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No original save compatibility, networking, external telemetry, or persistence claimed before PR16.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
