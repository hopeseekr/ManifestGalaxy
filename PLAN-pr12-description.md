# PR12: Implemented combat and planetary conquest.

- Branch name: `pr12-combat-and-conquest`
- Branch merge commit message: `Implemented combat and planetary conquest.`
- GitHub PR title: `Implemented combat and planetary conquest.`
- Git commit message: `Implemented combat and planetary conquest.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR11](PLAN-pr11-description.md). Start only after its authorized integration.

Resolve battles and planetary attacks deterministically and display replayable outcomes.

PR11 visibility, PR9 designs and PR2 combat/bombing/invasion evidence are required. Resolve damage, targeting, defenses, randomness and phase-order dependencies.

## Owned paths and contracts

Own crates/stars-core/src/combat/, desktop battle viewer, combat fixtures, and event visibility integration.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Implement encounter grouping, battle orders/targeting, movement within combat, weapon resolution, shields/armor, disengagement and losses according to the recovered inventory. Make random draws and stable ordering explicit.

2. Implement bombardment, planetary defenses and population invasion/ownership transfer, including racial modifiers and resource/unit aftermath. Apply only recovered supported attack types.

3. Return a structured battle trace alongside state changes, separating internal resolution from per-player reports. Rendering playback must never rerun combat or consume simulation RNG.

4. Integrate simultaneous encounters and destroyed fleet/planet references safely with later turn phases. Preserve relevant accounting, ownership and design references.

5. Build a scalable battle viewer with pause, speed and step controls plus a textual report for accessibility. Use original procedural visuals rather than requiring copied sprites.

## Acceptance scenarios and validation

- Small hand-checkable encounters match evidence; same seed yields identical outcomes and trace.
- Test empty weapons, shields, mixed fleets, retreat, mutual destruction, starbase defense, invasion thresholds and elimination.
- Replay speed and skipped animations leave outcomes unchanged; hidden participants/results are filtered according to observed visibility rules.
- Verify War Monger, Inner-Strength, and Regenerating Shields combat and defense effects, including trait-specific component access; recover applicability and effects rather than assuming rule multipliers, and record each result in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage). Improved Starbases hull costs remain covered where applicable.

Implement and execute these checks:

```sh
cargo test -p stars-core --test combat_resolution
cargo test -p stars-core --test planetary_conquest
cargo test -p stars-desktop --test battle_playback
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No alternative combat balance, fabricated exact random parity, or AI battle policy beyond legal order contracts.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
