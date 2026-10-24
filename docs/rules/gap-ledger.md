# PR2 gap ledger

PR2 may complete with tracked evidence gaps. This file is the authoritative list
of what is not recovered, which downstream feature is affected, and what would
close the gap. A dependent feature cannot claim verified fidelity while a
required gap remains open.

Status values are the same as the rule ledger: `observed`,
`statically-derived`, `documented-only`, `inferred`, `unknown`. All gaps below
are `unknown` unless stated otherwise.

| Gap ID | Gap | Impact | Required resolution | Gate |
|---|---|---|---|---|
| `GAP-VM-001` | No original game was executed; no VM or operator run is claimed. | No scenario has an observed result. Original behavior is unverified beyond static bytes and help. | Run the operator scenarios in a controlled environment; record executable hash, settings, seed if visible, actions, trials and outputs. | PR2 follow-up; every feature gate |
| `GAP-VERSION-001` | The help is dated January 1998; applicability to `stars26i.exe` is unresolved. | Every documented-only rule may differ from this executable. | Compare each help claim against controlled runs or version-specific binary evidence. | PR2 follow-up; PR17 |
| `GAP-POINTS-001` | **Closed 2026-10-18.** No advantage-point cost table or formula was recovered. | Resolved by [`advantage-points.md`](advantage-points.md): full `CAdvantagePoints()` specification, both trait tables, all Step-5 curves, growth remapping, habitability term and research curve, traced to `stars-decompile/race.c` (NB09 symbols from `stars26jrc3.exe`). Ultimate Recycling's price is `inferred` from the table's unexplained index 5. | n/a | PR5 |
| `GAP-EXCLUSIONS-001` | Trait exclusions and forced combinations are not recovered. | PR5 cannot reject invalid race combinations with confidence; it therefore accepts every priced combination and labels that policy designed. | Test all trait pairs/triples in the wizard and record disabled controls and costs. | PR5, PR13 |
| `GAP-PRIMARY-001` | **Closed 2026-10-18.** The static at-most-one selection is now backed by the data model: `raNone` is Jack of All Trades, not an absence, so exactly one primary is structural. | None; the product requirement and the data model agree. | n/a | PR5 |
| `GAP-LESSER-MAP-001` | The 14 lesser checkbox controls have empty captions; no literal lesser-trait ASCII names were found in the executable. | The checkbox-to-trait mapping is unproven. PR5 follows the canonical plan order and recovered cost indices, not an observed control mapping. | Use controlled UI observation or code cross-references to map each control ID. | PR5 |
| `GAP-GROWTH-001` | Exact growth curve after the 25% plateau, rounding, immunity behavior and Hyper-Expansion cap are unverified. | PR8 population model may diverge. | Measure controlled planets at many habitability/population points and with immunity/Hyper-Expansion. | PR8 |
| `GAP-POP-EFF-001` | **Closed 2026-10-18.** All seven Step-5 ranges and cost curves are specified in [`advantage-points.md`](advantage-points.md) from `rgRaceStatMin/Max` and the factory/mine blocks. | None. | n/a | PR5 |
| `GAP-LEFTOVER-001` | **Partly closed 2026-10-18.** The 50-point cap is confirmed by `create.c:1284` (`min(50, CAdvantagePoints(...))`). Per-choice integer rounding and the rarity weighting are still unspecified. PR5 records the selected target but does not convert points. | PR6 starting inventory may differ in the last unit per conversion. | Read the leftover conversion bodies in `stars-decompile/create.c`; until then implement floor division and label it designed. | PR5, PR6, PR13 |
| `GAP-TECH-MINI-001` | The help conflicts: topic 73 says 5% miniaturization per level; topics 71 and 281 say 4% normal (75% cap) and 5% with Bleeding Edge (80% cap). | PR9 production costs may be wrong. | Measure component cost at several levels above requirement with and without Bleeding Edge. | PR9, PR13 |
| `GAP-CATALOG-001` | Help table topics 317-333 decode no rows. No full technology, hull or component catalog is recovered. | PR9 cannot implement designs, costs or availability from evidence. | Extract/observe every catalog entry, its stats, research requirements and restrictions. | PR9 |
| `GAP-TURN-001` | Global cross-phase turn order is unknown. | PR4 provides a versioned designed pipeline; original cross-phase interactions in PR8-PR12 and replay fidelity remain unresolved. | Run the discriminating trials in `turn-order.md`. | PR4, PR8-PR12 |
| `GAP-RNG-001` | RNG algorithm, seed and consumption order are unknown. | PR4 provides a versioned designed streamed RNG; exact original replay and random-event fidelity cannot be claimed. | Recover or statistically characterize the RNG; otherwise use invariant/range checks and label the remake RNG as designed. | PR4, PR6, PR14 |
| `GAP-COMBAT-001` | Shield/armor damage allocation, repair timing and exact weapon/component stats are not recovered. | PR12 combat outcomes cannot be claimed original-fidelity. | Decode the combat tables/code or measure controlled battles; use ranges/invariants where randomness applies. | PR12 |
| `GAP-MINEFIELD-001` | Help statements on decay are ambiguous: 2%/1% base plus 4% per enclosed planet versus 1%/4% per enclosed planet. | PR10/PR12 minefield decay may be wrong. | Measure decay with controlled enclosed-planet counts and both races. | PR10, PR12 |
| `GAP-TURIN-001` | Only the name string is located; no Turindrones race, ship, research or policy configuration is recovered. | PR14 cannot reproduce the original opponent. | Observe the opponent in a controlled game and record race setup, technology, fleets and behavior. | PR14 |
| `GAP-AI-001` | AI policy, advantage-point totals by difficulty, and decision priorities are not recovered. | PR14 can only implement a disclosed designed policy. | Record AI configurations and behavior across difficulties; compare against a documented policy. | PR14 |
| `GAP-COMMAND-001` | `game.def` ranges and victory thresholds are documented-only and were not run. | PR6/PR15 setup validation may differ. | Generate controlled universes from `game.def` and compare visible parameters. | PR6, PR15 |
| `GAP-DIALOG-001` | Exported procedure names `RACEWIZARDDLG1`-`6` are not linked to dialogs `#146`-`#151`. | The runtime wizard mapping is a lead, not proof. | Cross-reference entry-table addresses and dialog resources, or observe the UI. | PR5 |
| `GAP-STYLE-001` | Dialog control styles are static; runtime enable/disable state and behavior are unobserved. | The one-primary/lesser-checkbox model may not match runtime validation. | Observe the wizard, including negative balances, disabled controls and keyboard operation. | PR5 |
| `GAP-SCENARIO-001` | All 59 scenarios are measurement plans; none has an observed result. | PR2 asserts no original behavior beyond static/help claims. | Execute the scenarios with an operator and replace `observed: null` only with real records. | PR2 follow-up |

## Not gaps

The following are intentionally selected product decisions or exclusions, not
missing evidence:

- Other predefined race presets are not a selectable product feature. The
  canonical 24 traits remain selectable for the custom race.
- Original save compatibility, network multiplayer, hotseat/PBeM and browser/PWA
  delivery are excluded by the product plan.
- The remake's interface scaling, layout and keyboard behavior are modern
  design choices; they are not original gameplay claims.
- A designed AI policy may be shipped where the original policy is unavailable,
  but PR14 must disclose the differences. Numerical economy, race-point or
  combat formulas still require evidence.

## Open-gap policy

1. A feature that depends on an open required gap may be implemented only with
   an explicit `unknown`/`designed` label and tests for invariants.
2. Do not replace a gap with a remembered Stars! value, a wiki value, or a
   convenient language default.
3. Update this ledger when evidence closes a gap; keep the old gap ID and mark
   it closed with the citation rather than deleting the history.
4. PR13 is the final trait coverage gate; PR17 is the fidelity/performance gate.
   Neither may pass with a required gap silently omitted.
