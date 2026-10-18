# PR2 controlled scenarios

Each JSON file is a measurement plan for one rule or a small set of related
rules. None of these records is an original-game observation. `observed` is
`null` until an operator runs the procedure and records the environment, trials
and result.

Run the validator after any edit:

```sh
python3 tools/analysis/validate_scenarios.py docs/evidence/scenarios
```

## Record shape

| Field | Meaning |
|---|---|
| `id` | Stable scenario ID, `SCN-<family>-<name>-NNN` |
| `rule_ids` | Rules from `docs/rules/rule-index.json` exercised by this plan |
| `status` | Strongest evidence class the scenario currently supports |
| `confidence` | Human-readable confidence label |
| `provenance` | Input hash plus file/code or help-topic location |
| `setup.baseline` | Fixed starting conditions |
| `setup.variable` | The one input varied between trials |
| `procedure` | Reproducible operator steps |
| `expected.documented` | Documented-only claims to test |
| `expected.static` | Static findings read from the supplied bytes |
| `observed` | Null, or a real operator observation with date/environment/trials/result |
| `unresolved` | Questions the scenario does not answer |
| `downstream_prs` | Implementation PRs that consume the result |

The validator rejects:

- duplicate scenario or rule IDs;
- missing provenance or a mismatched input hash;
- help citations without `TOPICPOS` and an HLP file offset;
- executable citations without a file offset, segment, resource or dialog;
- `observed` status without an observation record;
- `documented-only` status without help provenance and expected documented text;
- `statically-derived` status without executable provenance and expected static
  text;
- `unknown` status with expected results or no unresolved questions;
- trait or rule coverage drift between the index and the scenarios.

## Operator rules

1. Record the executable and help hashes before starting.
2. Keep universe settings, difficulty and opponent setup fixed across trials.
3. Change one input at a time.
4. Record the exact game year, actions, messages and end state.
5. Repeat at least three times, or five where randomness may apply.
6. Never replace a pending observation with a value from memory, a wiki, a
   forum or another Stars! version.
7. If a run is impossible, leave `observed: null` and report the limitation.

The scenarios cover every canonical primary and lesser trait, the race wizard,
advantage points, universe setup, victory/score, turn order, technology/catalog,
population/growth, mining, factories, defenses, scanners, terraforming, remote
mining, production, starbases, stargates, mass drivers, fleet movement/fuel,
scanning/cloaking, combat, bombing/ground combat, minefields, Turindrones and
generic AI difficulty.
