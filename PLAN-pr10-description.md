# PR10: Implemented fleet movement and logistics orders.

- Branch name: `pr10-fleet-orders`
- Branch merge commit message: `Implemented fleet movement and logistics orders.`
- GitHub PR title: `Implemented fleet movement and logistics orders.`
- Git commit message: `Implemented fleet movement and logistics orders.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR9](PLAN-pr9-description.md). Start only after its authorized integration.

Implement fleet orders and deterministic movement/logistics processing.

PR9 designs and PR2 movement, fuel, cargo and waypoint evidence are required. Special-device behavior reserved for PR13 must be visibly unavailable until implemented.

## Owned paths and contracts

Own crates/stars-core/src/fleet/, fleet/waypoint desktop editors, and movement/logistics fixtures.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Implement validated waypoint orders, speed selection, travel distance, fuel usage and arrival timing from recovered rules. Define rounding at authoritative coordinates and keep animation interpolation presentation-only.

2. Implement fleet split/merge, cargo load/unload, transfer and colonization with capacity, ownership and target validation. Handle depleted fuel and missing/destroyed targets according to evidence.

3. Connect arrivals, colony creation and deliveries to the turn dispatcher at their verified phase positions. Prevent negative cargo, duplicated ships and population/mineral creation during transfers.

4. Support repeating orders only if recovered, with bounded processing so zero-distance cycles cannot hang a turn. Emit meaningful completion/failure events.

5. Build map waypoint editing, fleet manifests and previews using pure core queries. Ensure pending UI orders do not mutate the last committed game state.

## Acceptance scenarios and validation

- Reference routes match travel time and fuel at boundaries and multiple speeds.
- Split then merge conserves ships/cargo/fuel under verified semantics; overflow and unauthorized transfer orders fail safely.
- Test zero-distance orders, no fuel, full cargo, removed targets, competing arrivals and colonization ownership changes.
- Verify Improved Fuel Efficiency, No Ramscoop Engines, and Cheap Engines fuel, engine availability, and reliability behavior, plus Inner-Strength fleet population where confirmed; recover applicability and effects against the ordinary baseline and record them in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage). Interstellar Traveler gate behavior and Stargates remain covered by PR13.

Implement and execute these checks:

```sh
cargo test -p stars-core --test fleet_logistics
cargo test -p stars-desktop --test waypoint_editor
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No invented navigation shortcuts, combat, AI movement policy, or special-device mechanics deferred to PR13.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
