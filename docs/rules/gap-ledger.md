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
| `GAP-LEFTOVER-001` | **Closed 2026-10-24 for PR6.** The 50-point cap and every leftover conversion are read from `create.c` `InitNewGamePlr()`: surface-mineral rarity weighting, concentration division, mines, factories and defenses. PR6 implements the recovered integer divisions and the Tough/Expert concentration override. | Remaining economy interaction belongs to PR8; no PR6 blocker. | n/a | PR5, PR6, PR13 |
| `GAP-TECH-MINI-001` | The help conflicts: topic 73 says 5% miniaturization per level; topics 71 and 281 say 4% normal (75% cap) and 5% with Bleeding Edge (80% cap). | PR9 production costs may be wrong. | Measure component cost at several levels above requirement with and without Bleeding Edge. | PR9, PR13 |
| `GAP-CATALOG-001` | Help table topics 317-333 decode no rows. No full technology, hull or component catalog is recovered. | PR9 cannot implement designs, costs or availability from evidence. | Extract/observe every catalog entry, its stats, research requirements and restrictions. | PR9 |
| `GAP-TURN-001` | Global cross-phase turn order is unknown. | PR4 provides a versioned designed pipeline; original cross-phase interactions in PR8-PR12 and replay fidelity remain unresolved. | Run the discriminating trials in `turn-order.md`. | PR4, PR8-PR12 |
| `GAP-VIEW-DETAIL-001` | Visibility is still the PR4 synthetic fixture. PR7 exposes all PR6 planet facts for a planet that enters the view; it has no partial-knowledge representation. | A player sees full population, mineral, installation and environment detail for every visible planet. | PR11 replaces the fixture with scanning-derived visibility and defines any partial fact disclosure required by evidence. | PR11 |
| `GAP-UI-LAYOUT-001` | PR7 panel sizes, report filters, sort orders, map pan/zoom and selection are presentation state only and are not saved. | A restart returns to the default map and report layout. | PR16 adds validated, user-scoped layout settings if persistence of these choices is desired. | PR16 |
| `GAP-RNG-001` | RNG algorithm, seed and consumption order are unknown. | PR4 provides a versioned designed streamed RNG; exact original replay and random-event fidelity cannot be claimed. | Recover or statistically characterize the RNG; otherwise use invariant/range checks and label the remake RNG as designed. | PR4, PR6, PR14 |
| `GAP-COMBAT-001` | Shield/armor damage allocation, repair timing and exact weapon/component stats are not recovered. | PR12 combat outcomes cannot be claimed original-fidelity. | Decode the combat tables/code or measure controlled battles; use ranges/invariants where randomness applies. | PR12 |
| `GAP-MINEFIELD-001` | Help statements on decay are ambiguous: 2%/1% base plus 4% per enclosed planet versus 1%/4% per enclosed planet. | PR10/PR12 minefield decay may be wrong. | Measure decay with controlled enclosed-planet counts and both races. | PR10, PR12 |
| `GAP-TURIN-001` | **Partly closed 2026-10-24.** The four Turindrones race records are recovered from `stars-decompile/create.c` `vrgplrComp[1]` and implemented by PR6 (growth, habitability, Super-Stealth, IFE/ARM/MA/RS, Cheap Factories, Tech 3, research costs). Ship designs and AI policy remain unrecovered. | PR14 still cannot reproduce original decisions, fleet composition or behavior. | Observe ship choices, research and behavior in controlled games; keep PR14's policy difference report explicit. | PR14 |
| `GAP-AI-001` | AI policy, advantage-point totals by difficulty, and decision priorities are not recovered. | PR14 can only implement a disclosed designed policy. | Record AI configurations and behavior across difficulties; compare against a documented policy. | PR14 |
| `GAP-COMMAND-001` | The `game.def` command-line path was not run. PR6 uses the documented ranges and the recovered `GetVCVal()` display conversions, but the file parser is not implemented. | PR6 form validation may differ from batch command-line validation. | Generate controlled universes from `game.def` and compare visible parameters. | PR6, PR15 |
| `GAP-UNIVERSE-PACKED-001` | Help topic 236 says Packed adds 30% over Dense; `GenerateWorld()` adds a second quarter of the base count, which is 20% over Dense, and the 999 cap makes Huge Packed equal Huge Dense. | Non-Huge Packed universes contain fewer planets than the help's wording implies. | Observe planet counts for each density at a fixed seed, or recover a version-specific formula. | PR6, PR17 |
| `GAP-UNIVERSE-NAMES-001` | The original planet-name table was not decoded. PR6 uses synthetic `Star NNNN` labels. | Planet names differ from the original; positions, environments and ownership do not. | Locate and decode the name table, or observe a generated universe. | PR7, PR17 |
| `GAP-UNIVERSE-START-001` | Starting ship designs, starbase component fitting and the Packet Physics/Interstellar Traveler second-world ship are not recovered here. PR6 stores starting technology, population, installations, starbase flag and the second owned world. | PR9/PR12 must add designs and components before the starting inventory is complete. | Decode the startup ship and starbase templates in `create.c`/`shdef` data. | PR9, PR12 |
| `GAP-UNIVERSE-VICTORY-001` | The original victory-checkbox labels and default checked set were not decoded; PR6 supplies remake interface defaults and validates documented ranges. | Default victory setup may differ; runtime victory enforcement is PR15 work. | Observe the default Step 3 dialog and run threshold trials. | PR15 |
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
