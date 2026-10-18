# PR2 completion report

Branch: `pr2-rules-specification`, created from the merged `trunk`.
Scoped commit message: `Documented recovered rules and reference scenarios.`

PR2 is complete as a research/specification stage. It did not add runtime
gameplay code, execute the original game, disassemble the executable, push, or
merge.

## Deliverables

| Path | Purpose |
|---|---|
| `docs/rules/rule-index.json` | 83 machine-readable rules with stable IDs, status, downstream PRs and scenarios |
| `docs/rules/rule-ledger.md` | Narrative evidence ledger, including all 10 primary and 14 lesser traits |
| `docs/rules/turn-order.md` | Known intra-phase constraints, unresolved cross-phase edges, RNG caveats and measurement matrix |
| `docs/rules/gap-ledger.md` | 21 tracked gaps with impact, resolution and feature gate |
| `docs/rules/README.md` | Status/citation conventions and reproducible static checks |
| `docs/evidence/scenarios/*.json` | 59 controlled scenario records |
| `docs/evidence/scenarios/README.md` | Scenario schema and operator rules |
| `tools/analysis/validate_scenarios.py` | Standard-library validator for IDs, provenance, hashes, status consistency and coverage |
| `tools/analysis/tests/test_validate_scenarios.py` | 17 focused validator tests |

## Acceptance evidence

- 10 primary traits and 14 lesser traits each have a rule entry, a documented
  help citation and a controlled scenario.
- The one-primary selection model is statically derived from dialog `#149`:
  ten `BS_AUTORADIOBUTTON` controls in one radio group. The template proves at
  most one selection; exactly one is the PR5 product requirement and original
  runtime enforcement remains a gap.
- The lesser-trait selection model is statically derived from dialog `#150`:
  fourteen `BS_AUTOCHECKBOX` controls. The checkbox-to-trait mapping remains
  unresolved because the captions are empty and no literal lesser-trait ASCII
  names were found in the executable.
- Total Terraforming, Ultimate Recycling and Mineral Alchemy are inventoried
  with their documented effects and separated from ordinary terraforming,
  scrapping and mineral conversion. Exact point costs and several rounding
  rules remain gaps.
- Every game subsystem named by PR2 has a ledger entry and a downstream PR.
- No scenario claims an observed result. `observed` is `null` in all 59 records.
- Documented-only, statically-derived and unknown records cannot masquerade as
  observed; the validator rejects contradictory status/evidence combinations.
- Missing provenance, duplicate IDs, unknown rule references, missing trait
  coverage and hash mismatches are rejected by tests.

## Commands run

```sh
python3 -m unittest discover -s tools/analysis/tests -v
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
python3 -m py_compile tools/analysis/validate_scenarios.py \
  tools/analysis/tests/test_validate_scenarios.py
git diff --check
```

Results:

- `unittest`: 86 tests passed (69 existing PR1 tests plus 17 validator tests).
- Validator: 59 scenarios accepted against 83 rules.
- `py_compile`: success.
- `git diff --check`: no whitespace errors.
- Status counts: 52 documented-only, 2 statically-derived, 5 unknown, 0
  observed.

## Remaining blockers carried forward

The tracked gaps in `docs/rules/gap-ledger.md` are not hidden. The most
important are:

- no advantage-point cost table, exclusion matrix or full valid-combination set
  (`GAP-POINTS-001`, `GAP-EXCLUSIONS-001`);
- no original-game observation of the wizard, so the one-primary runtime rule
  and lesser checkbox mapping are static/design claims (`GAP-PRIMARY-001`,
  `GAP-LESSER-MAP-001`);
- no complete technology/hull/component catalog; the help table topics decode
  no rows (`GAP-CATALOG-001`);
- no global turn order or RNG algorithm (`GAP-TURN-001`, `GAP-RNG-001`);
- no combat damage arithmetic (`GAP-COMBAT-001`);
- no Turindrones race configuration or AI policy (`GAP-TURIN-001`,
  `GAP-AI-001`);
- the January 1998 help's applicability to `stars26i.exe` is unresolved
  (`GAP-VERSION-001`).

PR5 and later features may proceed with explicit designed/unknown labels where
allowed, but cannot claim original fidelity while their required gaps remain
open. PR13 is the final trait coverage gate and PR17 is the fidelity gate.

## Git state

The scoped files are committed locally with the exact prompt message. No push,
GitHub PR, or merge was performed. The branch remains based on `trunk` at the
PR1 merge commit unless an authorized actor integrates it.
