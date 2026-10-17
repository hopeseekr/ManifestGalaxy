# PR6: Implemented universe generation and new-game setup.

- Branch name: `pr6-universe-setup`
- Branch merge commit message: `Implemented universe generation and new-game setup.`
- GitHub PR title: `Implemented universe generation and new-game setup.`
- Git commit message: `Implemented universe generation and new-game setup.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR5](PLAN-pr5-description.md). Start only after its authorized integration.

Generate deterministic starting worlds for one custom human player and one or more Turindrones opponents.

PR5 validated races and PR2 universe/Turindrones configuration evidence are required. Recover the actual opponent configuration rather than inferring it from its name.

## Owned paths and contracts

Own crates/stars-core/src/universe/, crates/stars-desktop/src/ui/new_game.rs, and universe setup tests.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Implement typed UniverseSettings for the complete recovered game-creation wizard, explicitly including galaxy size, density, star clumping/distribution, starting conditions, AI opponent count and victory options. Inventory any additional original setup controls rather than silently omitting them. Recover supported values and the actual effect of clumping from evidence; reject invalid combinations before generation.

2. Generate coordinates, environments, minerals and starting ownership from the explicit seed and verified distribution rules. Guarantee valid IDs, bounded positions, nonoverlapping ownership and usable starts according to documented constraints.

3. Store Turindrones as the sole built-in AI configuration. Create distinct opponent identities and seed streams for multiple instances; AI decision policies remain deferred to PR14.

4. Build a new-game form integrating RaceSpec validation, seed entry/display and setup options. Generate on the worker, return progress, and retain the old game if generation fails.

5. Expose a reproducible generated fixture for PR7. Document any generator differences from the original; unsupported original distributions must not be described as recovered merely because they produce plausible maps.

## Acceptance scenarios and validation

- Same seed and settings reproduce the world; changed seeds produce different supported worlds.
- Minimum and maximum supported settings produce valid starts and no illegal overlaps or ownership. Test every recovered galaxy-size, density and clumping option, including their interactions; verify spatial distributions against the reference evidence rather than just checking that the controls exist.
- Invalid custom races and unsupported settings fail before replacement of the active game; multiple opponents have distinct identities.
- Verify starting conditions for every recovered custom race PRT plus the Low Starting Population lesser trait against the [trait coverage matrix](PLAN.md#custom-race-trait-coverage); recover applicability and effects rather than assuming an ordinary baseline.

Implement and execute these checks:

```sh
cargo test -p stars-core --test universe_generation
cargo test -p stars-desktop --test new_game
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No playable AI, later-turn economic simulation, original universe-file compatibility, or extra opponent race presets.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
