# PR16: Added versioned saves and crash recovery.

- Branch name: `pr16-save-and-recovery`
- Branch merge commit message: `Added versioned saves and crash recovery.`
- GitHub PR title: `Added versioned saves and crash recovery.`
- Git commit message: `Added versioned saves and crash recovery.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR15](PLAN-pr15-description.md). Start only after its authorized integration.

Add robust local game/race persistence without coupling the engine to filesystem APIs.

PR15 full game state and AI memory contracts are stable. There is no requirement to read or write original Stars! files.

## Owned paths and contracts

Own crates/stars-desktop/src/persistence/, core/AI serialization contracts, save fixtures and persistence UI.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Define a versioned save envelope containing ruleset version, authoritative state, RNG state, AI memory and necessary session metadata. Store player draft orders explicitly if saved; keep UI preferences separate. Use serde JSON with a documented extension and bounded input validation.

2. Write via a temporary file in the destination directory and platform-tested replacement, retaining a recoverable previous save. Report I/O failures and never truncate the only good copy first.

3. Add manual save/open/save-as, export/import and post-turn autosave. Snapshot only committed state; never serialize half a turn. Run I/O off the UI thread with clear success/failure status.

4. Add standalone validated race import/export and separate UI settings storage. Reject future unsupported versions and malformed or inconsistent state with helpful messages; validate references, ownership, counts and numeric ranges.

5. Document version migration policy and implement migrations only for real supported prior schema fixtures. On startup offer recovery from a valid autosave/backup rather than silently replacing the user's selected save.

## Acceptance scenarios and validation

- Save/load then continue produces the same state/events as uninterrupted play, including every opponent's RNG/memory.
- Test truncated, oversized, future-version and invalid-reference files, permission failures, and interrupted replacement.
- Autosave never contains partial turns; failed writes preserve the last good file; race import applies builder validation.

Implement and execute these checks:

```sh
cargo test -p stars-desktop --test persistence
cargo test -p stars-desktop --test recovery
cargo test -p stars-core --test save_contract
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No legacy save parsing, cloud synchronization, fabricated historical migrations, or platform-specific paths in core.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
