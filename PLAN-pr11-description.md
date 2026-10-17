# PR11: Implemented scanning and player-specific intelligence.

- Branch name: `pr11-scanning-and-intelligence`
- Branch merge commit message: `Implemented scanning and player-specific intelligence.`
- GitHub PR title: `Implemented scanning and player-specific intelligence.`
- Git commit message: `Implemented scanning and player-specific intelligence.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR10](PLAN-pr10-description.md). Start only after its authorized integration.

Apply recovered sensor rules and enforce information boundaries in all UI and AI inputs.

PR10 moving fleets, PR9 component stats and PR2 scanner/cloaking/report rules are required.

## Owned paths and contracts

Own crates/stars-core/src/intelligence/, player view/event filtering, desktop intelligence presentation, and leakage tests.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Implement visibility updates from planetary and fleet sensors, applicable scanner types, cloaking and racial modifiers. Recover exact distance boundaries and timing relative to movement.

2. Separate current observations, last-known reports and authoritative hidden state. Track report age and fields actually observed; never fill unknown values from the host state merely for convenience.

3. Generate player-specific views and filtered event streams, including battles, production and ownership changes. Ensure logs and debug overlays used in normal play respect the same boundary.

4. Define the AI input as PlayerView plus its own authorized planning state. Leave explicitly evidenced AI advantages as separate documented capabilities, never blanket access to GameState.

5. Add report age/unknown indicators and scanner overlays derived from core queries. Invalidate stale selection details after loss of visibility without leaking current hidden data.

## Acceptance scenarios and validation

- Exact sensor edges, cloaked targets and special racial cases match observations.
- Two worlds differing only in unobservable data produce identical views and filtered events for that player.
- Last-known reports retain only previously observed facts; opponent production, cargo and hidden battles cannot leak through UI/tooltips or AI inputs.
- Verify Super-Stealth, Jack of All Trades, Packet Physics, and No Advanced Scanners visibility and scanner effects where applicable, recovering the actual behavior rather than assuming rule multipliers, and record evidence in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage).

Implement and execute these checks:

```sh
cargo test -p stars-core --test intelligence
cargo test -p stars-core --test information_boundaries
cargo test -p stars-desktop --test intelligence_views
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No omniscient AI convenience API, network security implementation, or inferred sensor formulas treated as verified.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
