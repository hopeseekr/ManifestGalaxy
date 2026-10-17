# PR17: Verified gameplay fidelity and improved large-game performance.

- Branch name: `pr17-fidelity-and-performance`
- Branch merge commit message: `Verified gameplay fidelity and improved large-game performance.`
- GitHub PR title: `Verified gameplay fidelity and improved large-game performance.`
- Git commit message: `Verified gameplay fidelity and improved large-game performance.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR16](PLAN-pr16-description.md). Start only after its authorized integration.

Measure rule fidelity and large-game responsiveness, then fix demonstrated discrepancies and bottlenecks.

PR16 completed game/save workflow and the reference ledger are required. Keep unknown original behavior visible in the final report.

## Owned paths and contracts

Own docs/verification/, targeted regression fixtures, stars-tools comparison/benchmark harnesses and only profiled affected modules.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Run the accumulated reference scenarios and classify mismatches as implementation bugs, evidence errors, intended documented differences or unresolved observations. Fix verified bugs and record the source of each expectation.

2. Compare exact deterministic scenarios where initial randomness is known; use invariants/distributions and repeated observations where original RNG cannot be controlled. Never claim bit-for-bit parity from visually similar output.

3. Benchmark fixed small/medium/large supported universes in release mode with recorded hardware, seed, settings, turn counts, elapsed time, memory and UI frame responsiveness. Establish reproducible baselines before optimization.

4. Optimize measured hot paths without changing phase order, arithmetic, RNG consumption or results. Parallelize only demonstrably independent work with deterministic reduction; using every CPU core is not itself an acceptance criterion.

5. Audit DPI handling, keyboard workflows, visibility, all traits, save continuation and full games. Produce a fidelity matrix and explicit release blockers; rerun affected regression selectors after fixes.

## Acceptance scenarios and validation

- All verified reference fixtures pass or have accurately documented unresolved evidence; no bug is hidden by rewriting expected output.
- Before/after optimized state/event digests match for benchmark seeds.
- Desktop remains interactive during large turns; benchmark results are reproducible and untested platforms are clearly listed.

Implement and execute these checks:

```sh
cargo test -p stars-tools --test reference_comparisons
cargo test -p stars-tools --test performance_invariants
cargo run --release -p stars-tools -- benchmark --suite fixtures/benchmarks
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No invented universal speed claims, silent balance changes, random parallel reductions, or broad rewrites without profiling evidence.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
