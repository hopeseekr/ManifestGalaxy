# PR2: Documented recovered rules and reference scenarios.

- Branch name: `pr2-rules-specification`
- Branch merge commit message: `Documented recovered rules and reference scenarios.`
- GitHub PR title: `Documented recovered rules and reference scenarios.`
- Git commit message: `Documented recovered rules and reference scenarios.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR1](PLAN-pr1-description.md). Start only after its authorized integration.

Produce the rule ledger and reproducible observation scenarios that later implementation agents can trust.

PR1 inventory must exist. Read its confidence limits and verify which bundled help claims apply to this binary.

## Owned paths and contracts

Own docs/rules/, docs/evidence/scenarios/, and tools/analysis/validate_scenarios.py with its tests. Do not add runtime game code.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

Use the canonical [custom-race trait coverage in PLAN.md](PLAN.md#custom-race-trait-coverage) as the exhaustive inventory. Add a ledger entry and controlled scenario for all 10 primary and 14 lesser traits, including the recovered one-primary and valid-lesser-combination validation rule. The bundled help index confirms the names only; cite executable or decoded-help evidence for mechanics and leave point values, exclusions, interactions and effects unknown until recovered.

1. Define stable rule IDs and statuses: observed, statically derived, documented-only, inferred, and unknown. Each record must cite an executable hash and byte/code location or decoded help topic; include applicability, inputs, expected results, and unresolved questions.

2. Inventory all race-builder pages, field ranges, trait exclusions, point interactions, racial effects, Turindrones configuration, technologies, hulls/components, the complete game-creation wizard (including galaxy size, density and star clumping/distribution), victory options, and turn phases. Map every rule family to PR5–PR15.

3. Specify controlled experiments for point costs, habitability, growth, mining, production, research, fuel, movement, sensors, combat, terraforming, special traits, and AI. Vary one input at a time and test boundaries and interactions.

4. If the original VM is accessible, record setup, actions, observations and repeated trials. Otherwise prepare operator instructions and mark observations pending. Do not claim to have run the original. Its random seed may be inaccessible: record it as unknown and use invariant/range tests instead of fabricated exact outcomes.

5. Create JSON scenario records and a validator for IDs, provenance, expected outcomes and confidence. Write a turn-order dependency specification with unresolved relationships explicit. PR2 may complete with a tracked gap ledger; required gaps must be resolved at the corresponding feature gate.

## Acceptance scenarios and validation

- Explicitly inventory Total Terraforming, Ultimate Recycling, and Mineral Alchemy as lesser racial traits. Recover their builder point costs, exclusions/interactions, and runtime effects. Distinguish ordinary terraforming, scrapping/recycling and mineral alchemy from the improvements granted by the corresponding trait; compare help claims against this executable.
- Every custom trait and game subsystem has a ledger entry and assigned downstream PR.
- Observed fixtures cite actual observations; documented-only and inferred rules cannot masquerade as observed.
- Validation rejects missing provenance, duplicate IDs and contradictory fixture status. Manual experiments are reproducible by a VM operator.

Implement and execute these checks:

```sh
python3 -m unittest discover -s tools/analysis/tests -v
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
```

Create the named analysis tools/tests in this PR. These checks intentionally do not require the future Rust workspace.

## Evidence gate and exclusions

No guessed numeric formulas, fabricated original-game seeds, or claim of complete rule recovery while gaps remain.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
