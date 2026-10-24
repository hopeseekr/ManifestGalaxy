# PR2 rule ledger

This ledger records evidence for the rules needed by the remake. It separates
what the supplied executable shows, what the January 1998 help claims, and what
is still unknown. It does not implement or rebalance gameplay.

Statuses and citation conventions are defined in [`README.md`](README.md).
Every rule below has a machine-readable entry in [`rule-index.json`](rule-index.json)
and at least one controlled scenario under
[`../evidence/scenarios/`](../evidence/scenarios/). No scenario is marked
observed: PR2 did not run the original game.

The help is titled `Stars! Player's Guide (Jan 10, 1998)`. Its applicability to
`stars26i.exe` is unresolved. Treat every help statement as documented-only,
even where the executable contains a matching name or dialog control.

## 1. Custom-race builder

### 1.1 Wizard pages and fields

The help describes six steps (documented-only). The executable contains six
contiguous candidate dialogs with wizard navigation at file offsets `0x2fa480`
through `0x2facc0`, plus the New Game dialog `#209` at `0x2f9800`. The mapping
of exported names `RACEWIZARDDLG1`-`RACEWIZARDDLG6` to `#146`-`#151` is still a
lead.

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-RACE-WIZARD-DIALOGS-001` | Candidate wizard dialog layout and control styles | statically-derived | Dialogs `#146`-`#151` decoded from the executable; see `analysis/inventory.json` | PR5 | `SCN-RACE-WIZARD-001` |
| `RULE-RACE-PRT-SELECTION-001` | At most one primary trait in the radio group; exactly one is the product requirement | statically-derived | `#149` control 271 style `0x50020009` starts a group; 272-280 style `0x50000009` are auto-radios in that group. Required selection/default state is not in the template. | PR5 | `SCN-RACE-WIZARD-001` |
| `RULE-RACE-LRT-SELECTION-001` | Zero or more lesser traits | statically-derived | `#150` has 14 auto-checkbox controls (style `0x50010003`) | PR5 | `SCN-RACE-WIZARD-001` |
| `RULE-RACE-WIZARD-STEPS-001` | Six-step wizard and finish/save flow | documented-only | Help topic 247, TOPICPOS 1690878, HLP 428787 | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-NAME-001` | Race name, plural name, password and emblem | documented-only | Help topics 248, 249, 256; dialog `#146` controls 268/2075/269 | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-IMMUNITY-001` | Optional immunity to one environmental factor | documented-only | Help topics 283/284; dialog `#147` has three auto-checkboxes | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-POP-EFF-001` | Colonist, factory, mine and defense efficiency controls | documented-only | Help topics 286; exact ranges are not decoded | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-PREDEFINED-001` | Predefined races preset steps 2-6 in the original help | documented-only | Help topic 250, TOPICPOS 1707608, HLP 433229 | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-FINISH-001` | Finish validates, re-prompts for password and saves | documented-only | Help topic 288, TOPICPOS 1884268, HLP 474721 | PR5 | `SCN-RACE-WIZARD-002` |
| `RULE-RACE-POINTS-GATE-001` | Balance must be zero or greater before Finish | documented-only | Help topics 247 and 268 | PR5 | `SCN-RACE-POINTS-001` |
| `RULE-RACE-POINTS-COST-001` | Exact trait and field costs/formula | statically-derived | No numeric point table in the decoded help; formula traced to `stars-decompile/race.c` `CAdvantagePoints()` (NB09 symbols from `stars26jrc3.exe`). See [`advantage-points.md`](advantage-points.md). Two decompilation defects are corrected as designed. | PR5 | `SCN-RACE-POINTS-COST-001` |
| `RULE-RACE-TRAIT-EXCLUSIONS-001` | Trait exclusions and forced combinations | unknown | Not stated in decoded help; runtime behavior not observed | PR5, PR13 | `SCN-RACE-POINTS-COST-001` |
| `RULE-RACE-COMPAT-001` | Valid lesser-trait combinations and enforcement | unknown | 14 checkboxes show independent UI controls, but valid/invalid combinations are not recovered | PR5, PR13 | `SCN-RACE-POINTS-COST-001` |
| `RULE-RACE-LEFTOVER-001` | Leftover points convert to minerals, mines, factories, defenses or concentration | documented-only | Help topics 251-255 | PR5, PR6 | `SCN-RACE-LEFTOVER-001` |
| `RULE-RACE-GROWTH-001` | Habitable range, growth rate and maximum population | documented-only | Help topics 283-285, 42, 349 | PR5, PR8 | `SCN-RACE-GROWTH-001` |
| `RULE-RACE-RESEARCH-COST-001` | Research cost choices and all-fields starting tech | documented-only | Help topic 287; dialog `#151` six three-choice radio groups | PR5, PR9 | `SCN-RACE-RESEARCH-001` |
| `RULE-RACE-TURINDRONES-NAMES-001` | Opponent-name sequence contains Turindrones | statically-derived | Segment 35 relative `0xa06`, file `0x55846` | PR14 | `SCN-TURINDRONES-LIST-001` |
| `RULE-RACE-TURINDRONES-CONFIG-001` | Turindrones race and AI configuration | unknown | Name at segment 35 relative `0xa06`; four race records recovered from `stars-decompile/create.c` `vrgplrComp[1]` (PR6), AI policy still unrecovered | PR6, PR14 | `SCN-TURINDRONES-001` |

Step 1 fields visible in decoded dialog `#146`: race name, plural name,
password, portrait/emblem buttons, and a "Spend up to 50 leftover advantage
points on:" caption. The 50 is a template caption, not a recovered hard cap.
Step 4 help states a growth-rate range of 1%-20%. Step 5 control ranges are not
decoded. Step 6 choices are Costs 75% extra, standard and Costs 50% less for
each of six fields, plus an all-fields starting-tech checkbox.

The at-most-one-primary and zero-or-more-lesser selection model is supported by
static Win16 control styles, not by an observed run. The product requires
exactly one primary; whether the original runtime enforced a required selection
is unresolved. The runtime mapping of the 14
lesser checkboxes to trait IDs is unresolved: the executable has no literal
ASCII lesser-trait names, while dialog `#150` has 14 empty-caption buttons.

### 1.2 Primary racial traits (10)

These are documented-only mechanics. The executable confirms the names in
dialog `#149` and the radio selection model. It does not confirm point costs,
exclusions or numeric effects. All help topics are in the January 1998 guide.

| Rule ID | Trait | Static lead | Help topic | Documented effects (summary) | Downstream | Scenario |
|---|---|---|---|---|---|---|
| `RULE-RACE-PRT-HE` | Hyper-Expansion | `#149` control 271, file `0x2fa900` | 258, TOPICPOS 1736867, HLP 437912 | Double growth; armed scout + three mini-colony ships; exclusive hulls/engine/component; no stargates; half population cap | PR5, PR6, PR8, PR9, PR10, PR13 | `SCN-PRT-HE-001` |
| `RULE-RACE-PRT-SS` | Super-Stealth | `#149` control 272 | 259, TOPICPOS 1738413, HLP 439458 | Electronics 5; stealth hulls/components; inherent 75% cloak; faster minefield travel; spy research | PR5, PR6, PR9, PR10, PR11, PR12, PR13 | `SCN-PRT-SS-001` |
| `RULE-RACE-PRT-WM` | War Monger | `#149` control 273 | 260, TOPICPOS 1740663, HLP 441708 | Weapons 5; battle hulls; half-square battle bonus; 25% cheaper weapons; design theft; no mine layers; limited defenses | PR5, PR6, PR9, PR10, PR12, PR13 | `SCN-PRT-WM-001` |
| `RULE-RACE-PRT-CA` | Claim Adjuster | `#149` control 274 | 261, TOPICPOS 1754710, HLP 443467 | Biotech 6; orbital terraformer; retro bombs; free temporary terraforming that reverts | PR5, PR6, PR8, PR9, PR13 | `SCN-PRT-CA-001` |
| `RULE-RACE-PRT-IS` | Inner-Strength | `#149` control 275 | 262, TOPICPOS 1756075, HLP 444832 | Better defense/healing; cheaper defenses; freighter reproduction; 25% costlier weapons; bomb exclusions | PR5, PR6, PR9, PR10, PR12, PR13 | `SCN-PRT-IS-001` |
| `RULE-RACE-PRT-SD` | Space Demolition | `#149` control 276 | 263, TOPICPOS 1770601, HLP 447070 | Mine layers; minefield scanners; faster minefield travel; remote detonation; 1%/4% decay; design learning | PR5, PR6, PR9, PR10, PR11, PR12, PR13 | `SCN-PRT-SD-001` |
| `RULE-RACE-PRT-PP` | Packet Physics | `#149` control 277 | 264, TOPICPOS 1772802, HLP 449271 | Energy 4; mass drivers; cheaper packets with scanners; packet sensing; receiver terraforming chances | PR5, PR6, PR8, PR9, PR10, PR11, PR12, PR13 | `SCN-PRT-PP-001` |
| `RULE-RACE-PRT-IT` | Interstellar Traveler | `#149` control 278 caption `Inter-stellar Traveler` | 265, TOPICPOS 1786810, HLP 450991 | Starting gates; unlimited gates; cargo through gates; safer overload; 25% cheaper gates; weaker mass drivers | PR5, PR6, PR9, PR10, PR11, PR12, PR13 | `SCN-PRT-IT-001` |
| `RULE-RACE-PRT-AR` | Alternate Reality | `#149` control 279 | 266, TOPICPOS 1788931, HLP 453112 | Starbase life; Death Star; orbital construction; natural mining/scans; Energy-based resources; no planetary installations; 3% travel loss | PR5, PR6, PR8, PR9, PR10, PR11, PR13 | `SCN-PRT-AR-001` |
| `RULE-RACE-PRT-JOAT` | Jack of All Trades | `#149` control 280 | 267, TOPICPOS 1802982, HLP 454875 | Tech 3 in all fields; starting fleet; built-in hull scanners; all-Costs-75%-extra fields start at tech 4 | PR5, PR6, PR8, PR9, PR11, PR13 | `SCN-PRT-JOAT-001` |

The summary is not a numeric specification. Each scenario records the full
documented claim and the unresolved questions. Exact point costs are unknown for
all ten. The runtime path from `#149` to the race model and the exact
hull/component catalog are also unresolved.

### 1.3 Lesser racial traits (14)

| Rule ID | Trait | Static lead | Help topic | Documented effects (summary) | Downstream | Scenario |
|---|---|---|---|---|---|---|
| `RULE-RACE-LRT-IFE` | Improved Fuel Efficiency | `#150` candidate control 291 | 269, TOPICPOS 1805926, HLP 457819 | 15% less fuel; two extra engines; +1 starting Propulsion | PR5, PR6, PR9, PR10, PR13 | `SCN-LRT-IFE-001` |
| `RULE-RACE-LRT-TT` | Total Terraforming | `#150` candidate control 292 | 270, TOPICPOS 1818636, HLP 458241 | Start at 3%; reach 30%; 30% fewer resources; Biotech-only research | PR5, PR6, PR8, PR13 | `SCN-LRT-TT-001` |
| `RULE-RACE-LRT-ARM` | Advanced Remote Mining | `#150` candidate control 293 | 271, TOPICPOS 1819107, HLP 458712 | Three mining hulls, two robots; two starting Midget Miners | PR5, PR6, PR8, PR9, PR13 | `SCN-LRT-ARM-001` |
| `RULE-RACE-LRT-ISB` | Improved Starbases | `#150` candidate control 294 | 272, TOPICPOS 1819388, HLP 458993 | Two new starbase designs; Space Dock shipyard; 20% cloak; 20% cheaper | PR5, PR6, PR9, PR12, PR13 | `SCN-LRT-ISB-001` |
| `RULE-RACE-LRT-GR` | Generalized Research | `#150` candidate control 295 | 273, TOPICPOS 1819839, HLP 459444 | Half to current field; 15% of total to each of the five other fields; totals 125% as the help states | PR5, PR6, PR9, PR13 | `SCN-LRT-GR-001` |
| `RULE-RACE-LRT-UR` | Ultimate Recycling | `#150` candidate control 296 | 274, TOPICPOS 1820250, HLP 459855 | 90%/70% at starbase; 45%/35% at planet; next-year resource credit formula | PR5, PR6, PR9, PR10, PR13 | `SCN-LRT-UR-001` |
| `RULE-RACE-LRT-MA` | Mineral Alchemy | `#150` candidate control 297 | 275, TOPICPOS 1820929, HLP 460534 | 25 resources per kT of each mineral versus 100 without the trait | PR5, PR6, PR8, PR13 | `SCN-LRT-MA-001` |
| `RULE-RACE-LRT-NRSE` | No Ramscoop Engines | `#150` candidate control 298 | 276, TOPICPOS 1821323, HLP 460928 | Six ramscoop engines unavailable; Interspace-10 available | PR5, PR6, PR9, PR10, PR13 | `SCN-LRT-NRSE-001` |
| `RULE-RACE-LRT-CE` | Cheap Engines | `#150` candidate control 299 | 277, TOPICPOS 1821781, HLP 461386 | Engines 50% cheaper; 10% refusal above warp 6 | PR5, PR6, PR9, PR10, PR13 | `SCN-LRT-CE-001` |
| `RULE-RACE-LRT-OBRM` | Only Basic Remote Mining | `#150` candidate control 300 | 278, TOPICPOS 1822177, HLP 461782 | Three advanced robots and Maxi-Miner hull unavailable | PR5, PR6, PR8, PR9, PR13 | `SCN-LRT-OBRM-001` |
| `RULE-RACE-LRT-NAS` | No Advanced Scanners | `#150` candidate control 301 | 279, TOPICPOS 1822437, HLP 462042 | No standard penetrating scanners; conventional ranges doubled | PR5, PR6, PR9, PR11, PR13 | `SCN-LRT-NAS-001` |
| `RULE-RACE-LRT-LSP` | Low Starting Population | `#150` candidate control 302 | 280, TOPICPOS 1835113, HLP 462430 | 17,500 instead of 25,000 colonists | PR5, PR6, PR8, PR13 | `SCN-LRT-LSP-001` |
| `RULE-RACE-LRT-BET` | Bleeding Edge Technology | `#150` candidate control 303 | 281, TOPICPOS 1835483, HLP 462800 | New tech costs 2x; drops after requirements exceeded; 5%/80% miniaturization | PR5, PR6, PR9, PR13 | `SCN-LRT-BET-001` |
| `RULE-RACE-LRT-RS` | Regenerating Shields | `#150` candidate control 304 | 282, TOPICPOS 1835968, HLP 463285 | Shields +40%, regenerate 10% per round; armor at 50% | PR5, PR6, PR9, PR12, PR13 | `SCN-LRT-RS-001` |

The `#150` control numbers are position-based candidates only. The dialog
controls have empty captions. No literal lesser-trait name was found in the
executable by exact-byte search, so the mapping must not be treated as proven.

### 1.4 Total Terraforming, Ultimate Recycling and Mineral Alchemy

These three traits need special care because ordinary mechanics use similar
language. The help distinguishes them; the point costs and some arithmetic do
not survive as verified rules.

**Total Terraforming (`RULE-RACE-LRT-TT`, help topics 270 and 55).**

- Ordinary terraforming without the trait is limited to 15% per environmental
  factor and needs Biotechnology plus a second field for each factor (help
  topics 54 and 55).
- The trait starts the race at 3% adjustment, allows technologies up to 30%,
  reduces terraforming resource cost by 30%, and requires only Biotechnology
  research (help topics 270 and 55).
- Scenario `SCN-LRT-TT-001` compares the trait against an otherwise identical
  non-TT race. Scenario `SCN-PLANET-TERRAFORM-001` measures ordinary
  terraforming separately.
- Unknown: point cost, the resource-cost rounding, whether 30% is a per-factor
  cap, and interaction with immunity, Claim Adjuster temporary terraforming and
  remote/orbital terraforming.

**Ultimate Recycling (`RULE-RACE-LRT-UR`, help topics 274 and 99).**

- Ordinary scrapping recovers 80% of construction minerals at a starbase, 33%
  at a planet without a starbase, and none in deep space; a colonization mission
  leaves 75% on the surface (help topic 99).
- The trait recovers 90% of minerals and 70% of resources at a starbase, and
  45%/35% at a planet (help topics 274 and 99). Resources are available the next
  year.
- The help states the resource credit is not strictly additive and gives
  `(Current_production x Extra_resources) / (Current_production + Extra_resources)`
  (help topic 274).
- Scenario `SCN-LRT-UR-001` compares ordinary and trait scrapping at both
  locations and checks the credit formula. Unknown: point cost, rounding,
  treatment of miniaturized designs, and zero-denominator behavior.

**Mineral Alchemy (`RULE-RACE-LRT-MA`, help topics 275 and 368).**

- The trait makes one alchemy instance cost 25 resources for one kT of each
  mineral; without the trait the help says 100 resources are needed for one kT
  of each mineral (help topic 275).
- Scenario `SCN-LRT-MA-001` compares trait and non-trait conversion, queue
  ordering and caps. Unknown: point cost, whether the quoted cost is per mineral
  or for all three, rounding, and whether the conversion is a production-queue
  item or a separate action.

These distinctions are documented-only. None of the three point costs or
runtime effects was observed against `stars26i.exe`.

### 1.5 Advantage points, exclusions and valid combinations

- **Specified.** The full calculation is in [`advantage-points.md`](advantage-points.md):
  a 1650 third-point accumulator, one truncating division by 3 at the end, both
  trait tables, all Step-5 curves, growth remapping, the habitability sampling
  term and the research curve. The structure and table values are
  `statically-derived` from `stars-decompile/race.c`, whose types and symbols
  come from NB09 CodeView data in `stars26jrc3.exe`.
- The wizard displays a current balance and requires it to be non-negative
  before Finish (help topics 247 and 268). This gate is documented-only and
  agrees with the calculation's sign convention.
- There is no "no primary trait" value: `raNone` is Jack of All Trades, so the
  product requirement of exactly one primary trait is also what the data model
  enforces.
- The executable dialog `#149` statically permits at most one primary trait.
  Trait exclusions remain **unknown**; neither the help nor the point
  calculation contains an exclusion matrix.
- The canonical validation contract for PR5 is: exactly one primary trait, zero
  or more lesser traits, a non-negative advantage-point balance, and only
  combinations allowed by recovered exclusions. Because the exclusion matrix is
  unknown, PR5 must not reject a combination on a guessed rule.
- The help contains no numeric table, so the help cannot independently confirm
  any cost. Cheap Engines being a net gain is what the table says; only its
  gameplay side is documented (help topic 277).

**PR5 implementation.** The typed builder, field ranges, validation, point
breakdown and all 24 trait selections are implemented in
`crates/stars-core/src/race/` and `crates/stars-desktop/src/ui/race_builder.rs`.
The term-by-term rule identifiers, deliberate decisions, reference Humanoids
anchor and remaining limitations are recorded in
[`race-builder.md`](race-builder.md). The exclusion matrix is still unknown, so
PR5 accepts every priced combination rather than guessing a rejection rule.

### 1.6 Leftover advantage points

Documented conversions (help topics 251-255):

| Choice | Conversion |
|---|---|
| Surface minerals | 10 kT per leftover point, weighted toward rarer minerals |
| Mines | One mine per two points |
| Factories | One factory per five points |
| Defenses | One defense per ten points |
| Mineral concentration | +1% to the poorest home-world mineral per three points |

The exact rounding at boundaries and the mineral-distribution weighting are
unknown. The `#146` caption mentions up to 50 points, but whether that is a hard
cap or only a template label is unknown.

### 1.7 Growth, efficiency and research costs

- Growth rate is selectable from 1% to 20% per year in the help. Planet value
  scales the annual growth, and growth plateaus after 25% capacity (help topics
  42, 285 and 349). Maximum population at 100% value is 1,000,000. Negative
  habitability kills colonists at habitability/10 percent per year (help topic
  42). Exact growth-curve arithmetic and rounding are not recovered.
- Step 5 efficiency controls adjust colonists-per-resource, factories, mines and
  defenses. The help gives example values but not the complete control ranges or
  point-cost curve (help topic 286).
- Step 6 sets one of three research-cost choices for each of six fields, plus an
  all-fields starting-tech option. All-Costs-75%-Extra fields start at tech 3,
  or tech 4 with Jack of All Trades (help topic 287). The exact point costs are
  unknown.
- Research miniaturization has an internal help conflict: topic 73 says 5% per
  level, while topics 71 and 281 say 4% per level for normal races (75% cap) and
  5% for Bleeding Edge (80% cap). The version-specific value must be measured;
  see `GAP-TECH-MINI-001`.

## 2. Universe creation, game setup and victory

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-UNIVERSE-SIZE-001` | Tiny 400 ly through Huge 2000 ly | documented-only | Help topics 227-231 | PR6 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-DENSITY-001` | Sparse/Normal/Dense/Packed planet density | documented-only | Help topic 236 | PR6 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-POSITIONS-001` | Close/Moderate/Farther/Distant player positions | documented-only | Help topic 237; dialog `#390` controls 1009-1012 | PR6 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-CLUMPING-001` | Galaxy Clumping option | documented-only | Help topic 244; dialog `#390` control 1050 | PR6 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-OPTIONS-001` | Maximum minerals, slow tech, BBS play, random events, alliances, public scores | documented-only | Help topics 238-243; dialog `#390` controls 1016-1021 | PR6 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-PLAYERS-001` | Player-slot types and computer opponents | documented-only | Help topic 245 | PR6, PR14 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-START-001` | Year 2400 start and common home-world inventory | documented-only | Help topics 314, 41; game starts in 2400 | PR6, PR15 | `SCN-UNIVERSE-001` |
| `RULE-UNIVERSE-DIFFICULTY-001` | Difficulty changes opponent count/skill | documented-only | Help topic 226 | PR6, PR14 | `SCN-UNIVERSE-001` |

PR6 implementation evidence: the generation arithmetic for these rules is
statically derived from `stars-decompile/create.c` (`GenerateWorld()` and
`InitNewGamePlr()`), a later 2.60j RC3 build. The declared statuses above stay
`documented-only` because the source is not the supplied `stars26i.exe` and no
original execution exists. [`universe-generation.md`](universe-generation.md)
records every formula, the help/decompilation Packed-count difference and the
remaining gaps. The four Turindrones race records are recovered from
`vrgplrComp[1]`; the AI policy remains unknown.

Documented setup facts:

- Universe spans are 400, 800, 1,200, 1,600 and 2,000 light years for Tiny
  through Huge. Huge can have up to 1,000 planets (help topics 227-231).
- Normal density is approximately one planet per 5,000 ly^2; Sparse about one
  per 6,500; Dense about one per 4,000; Packed adds 30% over Dense except in
  Huge (help topic 236).
- Advanced options include Beginner: Maximum Minerals, Slower Tech Advances,
  Accelerated BBS Play, No Random Events, Computer Players Form Alliances,
  Public Player Scores and Galaxy Clumping (help topics 238-244).
- Player positions affect starting separation and are modulated by player count,
  universe size and density (help topic 237). The exact geometry is unknown.
- Player slots support predefined race, custom race, computer player, no player
  and expansion player (help topic 245). This product intentionally excludes
  selectable predefined-race presets; the evidence remains useful for the
  original wizard inventory.
- `game.def` documents ranges for a command-line universe: size 0-4, density
  0-3, starting distance 0-3, player counts, victory thresholds and other
  options (help topic 24). These ranges are documented-only and were not run.

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-VICTORY-CONDITIONS-001` | Configurable victory conditions and must-meet count | documented-only | Help topics 246, 25, 24; dialog `#392` has seven condition checkboxes | PR15 | `SCN-VICTORY-001` |
| `RULE-VICTORY-SCORE-001` | Score components and history graph | documented-only | Help topic 313 | PR15 | `SCN-SCORE-001` |

The command-line help lists planets, tech level/fields, score, exceeds-nearest,
production capacity, capital ships and turns as conditions, with a must-meet
range of 0-7 and a minimum-years range of 30-500 (help topic 24). Dialog `#392`
has seven condition checkboxes but empty labels, so the labels must not be
mapped by position without observation. Multiple players can win at once, and
play may continue after victory (help topics 25 and 246).

Score components are documented as: planets 1-6 points (1 per 100,000
colonists), starbases 3 points excluding Orbital Forts, unarmed ships 1/2 point
each up to planet count, escorts 2 points each up to planet count, capital ships
`(8 x ships x planets)/(ships + planets)`, tech levels by band, and resources 1
point per 30. The exact planet curve and rounding are not decoded.

## 3. Turn structure

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-TURN-CYCLE-001` | Host generates a new year after submissions | documented-only | Help topics 2, 21, 314 | PR4, PR15 | `SCN-TURN-CYCLE-001` |
| `RULE-TURN-ORDER-001` | Global cross-phase turn order | unknown | Not stated by the decoded help | PR4, PR8-PR12, PR15 | `SCN-TURN-ORDER-001` |
| `RULE-TURN-CONSTRAINTS-001` | Known intra-phase ordering constraints | documented-only | Help topics 50, 59, 73, 122, 131, 376 | PR4, PR10, PR12 | `SCN-TURN-CONSTRAINTS-001` |
| `RULE-RNG-001` | RNG algorithm, seed and consumption order | unknown | Not recovered | PR4, PR6, PR14 | `SCN-TURN-ORDER-001` |

Known constraints from help:

- The host collects turns and generates the new year; in single-player the
  computer hosts. All games start in year 2400 (help topics 2 and 314).
- Minesweeping happens before movement (help topic 131).
- Stargate travel completes in the same year (help topic 50).
- Packet first-year travel is half normal, and packets can decay in launch and
  arrival years (help topic 122).
- Battles last up to 16 rounds; a round is each token moving and firing, split
  into phases (help topic 376).
- Production is processed top-down and stops when minerals run out (help topic
  59). Research takes resources off the top unless leftover-only is checked
  (help topic 73).

These constraints do not establish a global phase order. See
[`turn-order.md`](turn-order.md) for the dependency specification and the
measurement plan.

## 4. Technology, hulls and components

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-TECH-FIELDS-001` | Six fields, levels 0-26, one current field | documented-only | Help topic 73 | PR9 | `SCN-RACE-RESEARCH-001` |
| `RULE-TECH-COST-001` | Fibonacci-like research cost plus added per-field cost | documented-only | Help topic 76 | PR9 | `SCN-RACE-RESEARCH-001` |
| `RULE-TECH-MINI-001` | Miniaturization percentage and cap | documented-only | Help topics 71, 73, 281 (conflicting 4%/5% claims) | PR9 | `SCN-RACE-RESEARCH-001` |
| `RULE-TECH-CATALOG-001` | Exact tech, hull, component and cost catalog | unknown | Help table topics 317-333 decode titles/links but no rows | PR9 | `SCN-TECH-CATALOG-001` |

The six fields are Energy, Weapons, Propulsion, Construction, Electronics and
Biotechnology (help topic 73). Research cost rises in a Fibonacci-like series
plus 10 resources per field per level already achieved (help topic 76).

The decoded help table topics contain no data rows. The table record geometry
was not reconstructed by PR1, so no armor, weapon, bomb, engine, mechanical,
mining, orbital, planetary-installation, scanner, shield, hull, starbase-hull,
terraforming or torpedo statistics are recovered here. Trait help topics list
some exclusive component names, but that is not a complete catalog and does not
provide stats. `SCN-TECH-CATALOG-001` defines the extraction/observation work
for PR9.

## 5. Planets and economy

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-PLANET-POP-001` | Population capacity and annual growth | documented-only | Help topics 42, 285, 349 | PR8 | `SCN-RACE-GROWTH-001` |
| `RULE-PLANET-MINES-001` | Mine output and concentration depletion | documented-only | Help topics 43, 44, 113 | PR8 | `SCN-PLANET-MINES-001` |
| `RULE-PLANET-FACTORIES-001` | Factory output and build cost | documented-only | Help topics 45, 71 | PR8 | `SCN-PLANET-FACTORIES-001` |
| `RULE-PLANET-DEFENSES-001` | Defenses against bombs and invasion | documented-only | Help topics 46, 126, 128 | PR8, PR12 | `SCN-PLANET-DEFENSES-001` |
| `RULE-PLANET-SCANNER-001` | Planet scanner coverage and upgrades | documented-only | Help topics 47, 147 | PR11 | `SCN-PLANET-SCANNER-001` |
| `RULE-PLANET-TERRAFORM-001` | Terraforming technology and limits | documented-only | Help topics 52-56 | PR8, PR13 | `SCN-PLANET-TERRAFORM-001` |
| `RULE-PLANET-REMOTE-MINING-001` | Remote mining of uninhabited worlds | documented-only | Help topics 113, 114 | PR8, PR10 | `SCN-PLANET-REMOTE-MINING-001` |
| `RULE-PLANET-PRODUCTION-001` | Production queue order and mineral blocking | documented-only | Help topics 58-60 | PR8 | `SCN-PLANET-PRODUCTION-001` |
| `RULE-PLANET-STARBASE-001` | Starbase construction, shipyard and defense | documented-only | Help topics 49, 133 | PR9, PR12 | `SCN-PLANET-STARBASE-001` |
| `RULE-PLANET-STARGATE-001` | Stargate travel and cargo limits | documented-only | Help topics 50, 105 | PR10 | `SCN-PLANET-STARGATE-001` |
| `RULE-PLANET-MASSDRIVER-001` | Packet launch, catch and impact | documented-only | Help topics 51, 122 | PR8, PR10 | `SCN-PLANET-MASSDRIVER-001` |

Documented statements that later implementation must not silently change:

- Planet value sets maximum population: 100% supports 1,000,000; growth is value
  times the race maximum and plateaus after 25% capacity; negative value kills
  colonists at value/10 percent per year (help topic 42).
- Mining productivity is potential productivity times mineral concentration;
  concentration falls toward 1, home-world concentration never drops below 30%,
  and mine years to reduce concentration by one are approximately 12,500 /
  concentration (help topics 43, 44, 113).
- Factories produce resources and consume nothing after construction; the help
  examples give 15 resources per factory, five resources to build, and a 4 kT
  germanium cost (3 kT with Costs One Less) (help topics 45 and 71). Examples
  are not automatically defaults; race-dependent values must be read and
  measured.
- Defenses reduce bombing and invasion; the help example calls planetary
  defenses 75% effective against enemy colonist drops (help topics 46 and 128).
- Planet scanners have a fixed build cost and upgrade automatically when better
  technology is available (help topic 47).
- Terraforming without Total Terraforming improves each factor at least 3% and
  at most 15%; Total Terraforming reaches 30% and uses only Biotechnology
  (help topics 54, 55, 270).
- Production is top-down and halts when minerals run out (help topic 59).
- A starbase can be a shipyard, defensive station and platform for a stargate or
  mass driver; a space dock is required to build ships (help topic 49).
- Stargates are cargo-less, fuel-free and single-year for most races (help
  topic 50).
- Mass drivers catch safely only with an equal-or-greater driver; otherwise
  damage occurs and 2/3 of a packet's minerals are lost on an uninhabited
  impact. First-year packet travel is half normal (help topics 51 and 122).

Exact arithmetic, rounding and catalog values remain gaps. The help statements
are not runtime-verified.

## 6. Fleets, scanning and combat

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-FLEET-MOVEMENT-001` | Warp speed, distance and waypoint movement | documented-only | Help topics 90, 101 | PR10 | `SCN-FLEET-MOVEMENT-001` |
| `RULE-FLEET-FUEL-001` | Fuel capacity, use and free speed | documented-only | Help topics 90, 95 | PR10 | `SCN-FLEET-FUEL-001` |
| `RULE-SCAN-RANGE-001` | Additive scanner range and penetration | documented-only | Help topic 147 | PR11 | `SCN-SCAN-RANGE-001` |
| `RULE-SCAN-CLOAK-001` | Cloak percentage and effective range | documented-only | Help topics 150, 151 | PR11 | `SCN-SCAN-CLOAK-001` |
| `RULE-COMBAT-ROUNDS-001` | Battle rounds, movement and firing phases | documented-only | Help topics 334, 339, 376 | PR12 | `SCN-COMBAT-001` |
| `RULE-COMBAT-TARGETING-001` | Primary/secondary targeting and disengagement | documented-only | Help topics 136, 137 | PR12 | `SCN-COMBAT-001` |
| `RULE-COMBAT-DAMAGE-001` | Shield, armor, damage and repair arithmetic | unknown | Help table/combat topics do not contain recovered formulas | PR12, PR13 | `SCN-COMBAT-DAMAGE-001` |
| `RULE-COMBAT-BOMB-001` | Orbital bombing and population estimates | documented-only | Help topics 126, 127 | PR12 | `SCN-COMBAT-BOMB-001` |
| `RULE-COMBAT-GROUND-001` | Ground combat and planetary defenses | documented-only | Help topic 128 | PR12 | `SCN-COMBAT-BOMB-001` |
| `RULE-MINEFIELD-001` | Laying, sweeping, decay, visibility and detonation | documented-only | Help topics 129-132, 341 | PR10, PR12, PR13 | `SCN-MINEFIELD-001` |

Documented facts:

- Distance traveled per year is the square of warp speed; absolute maximum is
  warp 10. Most ships are safe at warp 9; warp 10 risks a 10% chance of losing
  each ship unless the engine is warp-10 capable (help topic 90).
- Fuel is shared by a fleet; capacity is summed and use is the sum of individual
  ship use. Standard engines move at warp 1 without fuel (help topics 90 and 95).
- Scanner range is the fourth root of the sum of each scanner range to the
  fourth power, for both normal and planet-penetrating scanners (help topic 147).
- Cloaks reduce effective scanner range by a percentage; the maximum is 98%;
  same-location fleets always see each other (help topics 150 and 151).
- A battle lasts at most 16 rounds. Movement is 1-8 squares per round, split
  into phases by allowable movement and ordered heaviest to lightest within a
  +/-15% margin (help topics 339 and 376). The movement formula is best-effort
  strings-only text and is not accepted as verified arithmetic.
- Targeting tries the primary target type, then secondary, then disengages
  (help topics 136 and 137).
- Bombing requires the starbase to be gone; enemy population reports are +/-20%
  estimates; each bomb slot drops that many bombs per year (help topics 126 and
  127). Ground combat uses the Transport task, and planetary defenses are 75%
  effective against enemy colonist drops (help topic 128).
- Minefields decay 2% per year normally, 1% for Space Demolition, plus 4% per
  enclosed planet, capped at 50% (help topic 130). The Space Demolition trait
  topic phrases its own decay as 1% per enclosed planet and other players'
  fields as 4% per enclosed planet (help topic 263), so the additive versus
  per-planet interpretation is unresolved. Sweeping happens before movement
  and beam sweep is power times range squared summed over the fleet (help topic
  131); unseen minefields are 75% cloaked and minefields are always 75% cloaked
  from non-penetrating scanners (help topic 129).

Combat damage, shield/armor allocation, repair timing and exact component stats
are unknown. `SCN-COMBAT-DAMAGE-001` is a measurement plan, not a formula.

## 7. Turindrones and AI

| Rule ID | Subject | Status | Evidence | Downstream | Scenario |
|---|---|---|---|---|---|
| `RULE-RACE-TURINDRONES-NAMES-001` | Opponent-name sequence includes Turindrones | statically-derived | Segment 35 relative `0xa06`, file `0x55846` | PR14 | `SCN-TURINDRONES-LIST-001` |
| `RULE-RACE-TURINDRONES-CONFIG-001` | Turindrones race/AI configuration | unknown | Name located; four race records recovered from `stars-decompile/create.c` `vrgplrComp[1]` by PR6; AI policy still unrecovered | PR6, PR14 | `SCN-TURINDRONES-001` |
| `RULE-AI-ADVANTAGE-001` | AI advantage points vary by difficulty | documented-only | Help topics 347, 348 | PR14 | `SCN-AI-001` |
| `RULE-AI-TURINDRONES-001` | Turindrones planning policy and memory | unknown | Not recovered | PR14 | `SCN-TURINDRONES-001` |

The data segment contains the ASCII sequence `Stars!.Stars!.Robotoids.
Turindrones.Automitrons.Rototills.Cybertrons.Macinti.Random` at file offset
`0x55846`. PR1 also found the string `Turindrone` as a prefix at the same
location. This does not prove that the entry is selectable, that it is a full
race preset, or that it uses a particular AI policy.

The help says Easy AIs receive substantially fewer advantage points, Expert AIs
more, and Standard/Hard in between (help topic 348). It gives no point totals or
race configurations. PR14 may disclose designed AI policy differences, but that
exception does not extend to numerical gameplay rules such as economy or combat.

## 8. Coverage and unresolved work

- All 24 canonical traits have a rule entry and a scenario. Every scenario has a
  citation, a one-variable setup, a reproducible procedure and explicit
  unresolved questions.
- All game subsystems named by PR2 have at least one ledger entry assigned to a
  downstream PR: race builder, universe setup, technologies/catalog, planets,
  production, research, fleets/fuel/movement, scanning, combat, minefields,
  Turindrones/AI, victory, turn cycle and saves-adjacent flow notes.
- No original game was run. No point cost, full catalog, combat damage formula,
  RNG policy or Turindrones configuration is claimed as recovered.
- The tracked blockers and their downstream gates are listed in
  [`gap-ledger.md`](gap-ledger.md).
