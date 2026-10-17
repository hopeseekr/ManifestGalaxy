# PR4: Added deterministic game state and turn-processing contracts.

- Branch name: `pr4-simulation-contracts`
- Branch merge commit message: `Added deterministic game state and turn-processing contracts.`
- GitHub PR title: `Added deterministic game state and turn-processing contracts.`
- Git commit message: `Added deterministic game state and turn-processing contracts.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR3](PLAN-pr3-description.md). Start only after its authorized integration.

Define the deterministic state machine and contracts shared by every later subsystem.

PR3 workspace and PR2 scenario schema exist. Resolve turn-phase relationships needed now; unknown phases remain explicitly unimplemented.

## Owned paths and contracts

Own crates/stars-core/src/{model,orders,turn,view,rng}.rs, core contract tests, and the desktop worker adapter integration.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Define stable typed IDs for players, planets, fleets and designs; ordered collections for authoritative processing; explicit units and integer widths; typed command-validation errors. Keep display coordinates separate from authoritative positions.

2. Define GameState, OrderBatch, PlayerView and TurnOutcome. Validate a batch before mutation; reject unauthorized, stale, duplicate or invalid requests explicitly. Commit a turn result atomically so failures retain the previous state.

3. Specify seed/state serialization and an explicit versioned RNG algorithm. Separate independent random streams by stable player/subsystem identifiers; never seed from a clock or depend on hash-map iteration.

4. Implement the turn-phase dispatcher and event structure according to recovered ordering. Add no-op phase scaffolding only with explicit capability status; it must not imply that economy or combat already works.

5. Connect worker requests and completed snapshots, ignoring stale request IDs. Provide player-filtered views and events as contracts, initially with synthetic visibility fixtures. Record arithmetic, overflow and rounding policies without inventing gameplay constants.

## Acceptance scenarios and validation

- Identical state, seed and orders yield identical serialized state and event order across repeat runs.
- Invalid orders and worker failure leave authoritative state unchanged; delayed responses cannot overwrite a newer game.
- Player views omit hidden entities and events; test empty games and destroyed/stale entity references.

Implement and execute these checks:

```sh
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-desktop --test turn_worker
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No actual economic/combat formulas, AI choices, persistence files, or game rules in UI code.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
