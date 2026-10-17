# PR18: Added verified desktop release packages.

- Branch name: `pr18-desktop-releases`
- Branch merge commit message: `Added verified desktop release packages.`
- GitHub PR title: `Added verified desktop release packages.`
- Git commit message: `Added verified desktop release packages.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR17](PLAN-pr17-description.md). Start only after its authorized integration.

Produce installable offline desktop artifacts with documented platform validation.

PR17 fidelity report and all mandatory acceptance gates must pass. Resolve release-blocking omissions rather than calling an incomplete prototype finished.

## Owned paths and contracts

Own packaging/, release CI, README release instructions and docs/verification/release-checklist.md. Change runtime code only for demonstrated packaging defects.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Build versioned Windows x86_64 ZIP, macOS arm64/x86_64 app bundles, and Linux x86_64 archive on native CI runners. Record minimum supported OS/runtime versions from dependencies and test hosts; include bundled fonts/procedural assets and required notices.

2. Build with the locked toolchain/dependencies and embed an application version, ruleset version and source revision. Generate artifact checksums and document reproducible commands. Release artifacts remain untracked build outputs.

3. Test fresh extraction/install and offline launch without the original executable or help installed. Verify paths containing spaces/non-ASCII text, user-writable saves and read-only install locations.

4. Smoke-test new game, custom race, multiple Turindrones, turn processing, battle report, save/reload, export and upgrade retention on each platform. Exercise DPI scaling and monitor changes where hosts allow.

5. Use code-signing/notarization only with configured credentials and authorization; never invent signing success. Produce unsigned artifacts with explicit status if credentials are unavailable. Publishing/uploading remains a separate authorized action.

## Acceptance scenarios and validation

- Artifacts run offline on each claimed supported target and preserve saves/settings through an application upgrade.
- Checksums match packaged files; no license codes, original binary/help payloads, private paths or debug fixture games are bundled.
- Platform smoke results and unsigned/untested limitations are recorded; a cross-compile alone does not count as a successful platform run.

Implement and execute these checks:

```sh
cargo build --locked --release -p stars-desktop
cargo test -p stars-desktop --test release_assets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Create the named test targets in this PR. Report unavailable platform checks explicitly; do not claim an unexecuted smoke test passed.

## Evidence gate and exclusions

No PWA, network services, telemetry, automatic publishing or original asset redistribution.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
