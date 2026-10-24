# PR5 custom-race builder implementation

Status: implemented for the builder, the recovered advantage-point formula and
field validation. No runtime trait effect is claimed before PR8-PR13.

## Scope and sources

The builder is owned by `crates/stars-core/src/race/` and
`crates/stars-desktop/src/ui/race_builder.rs`. All arithmetic and validation are
in core; the egui view only edits a draft and renders core results.

| Source | What it supplies | Status |
|---|---|---|
| `stars-decompile/race.c` `CAdvantagePoints()` | Accumulator, trait tables, growth/efficiency/research curves, order of terms | statically derived from an unsupplied later build; see [`advantage-points.md`](advantage-points.md) |
| `stars-decompile/race.c` `LInnateRaceHabitability()` | Three-pass habitability sampling | statically derived |
| `stars-decompile/planet.c` `PctPlanetDesirability()` reconstruction | Sample desirability used by the reference builder model | statically derived reconstruction; PR8 owns the authoritative economy |
| `stars-decompile/enums.h`, `types.h`, `race.c` range tables | Trait indices, flags, attribute ranges, name/emblem widths | statically derived |
| January 1998 help topics 247-288 | Wizard pages, Step 5 wording, growth range, finish gate | documented-only |
| `stars26i.exe` dialogs `#146`-`#151` | Six wizard pages, ten primary radios, fourteen lesser checkboxes | statically derived; runtime mapping unresolved |

No original game was executed. The Humanoids anchor is reproduced from the
decompilation's own default record, not from an observation.

## Field inventory

| Wizard input | `RaceSpec` field | Evidence and bounds |
|---|---|---|
| Race name | `name` | Step 1; empty rejected, maximum 31 bytes from the 32-byte `PLAYER` name field |
| Plural name | `plural_name` | Step 1; same width limit |
| Race-file password | `password` | Step 1; local-only field, never logged, not a security boundary |
| Emblem | `emblem` | Step 1 portrait/emblem; `0..=31` from five-bit `iPlrBmp` |
| Leftover target | `leftover` | Step 1; five documented choices, conversion owned by PR6 |
| Primary trait | `primary` | Step 2; exactly one, `RaceAttribute` order |
| Lesser traits | `lesser` | Step 3; zero or more, recovered bit order |
| Gravity/temperature/radiation band | `habitability[0..2]` | Step 4; `0..=100`, `min <= max`, at least 5 points wide, optional immunity |
| Ideal environment point | derived midpoint | `BoundsCheckPlayer()` forces the midpoint; the wizard exposes the band and immunity, not a separate ideal control |
| Growth rate | `growth_rate` | Step 4; help documents `1..=20` |
| Colonists per resource | `resource_generation` | `rsResGen`, `7..=25` |
| Factory production/cost/operation | `factory_production`, `factory_cost`, `factory_operation` | `5..=15`, `5..=25`, `5..=25` |
| Mine production/cost/operation | `mine_production`, `mine_cost`, `mine_operation` | `5..=25`, `2..=15`, `5..=25` |
| Cheap factory materials | `cheap_factories` | recovered `ibitRaceCheapFact` option flag, priced at `-175` raw |
| Six research choices | `research.costs[0..6]` | Step 6; cheaper/standard/75%-extra |
| All fields start at tech 3 | `research.tech3` | recovered `ibitRaceTech3`, priced at `-180` raw |

The help's Step 5 text names colonists, mines and factories; it does not
recover a separate defense-efficiency race field, and the `RaceStat` table has
none. No defense field was invented.

## Validation contract

`validate_race` returns `Vec<RaceValidationError>` with a `RaceField` for every
failure:

- exactly one primary trait is structurally required; an editable `RaceDraft`
  may hold `None` and reports `MissingPrimaryTrait`;
- the primary-trait page blocks Next, PageDown and Alt+Right until a trait is
  selected; Back remains available;
- names are non-empty and at most 31 bytes;
- emblem is `0..=31`;
- growth is `1..=20`;
- each environmental bound is `0..=100` and `min <= max`;
- the seven efficiency attributes are inside their recovered ranges;
- the displayed advantage-point balance must be zero or greater.

Trait exclusions remain unknown (`GAP-EXCLUSIONS-001`). PR5 therefore rejects no
priced lesser-trait combination. The count and mixture penalties still apply, so
many traits become progressively more expensive.

## Point evaluation

`evaluate_race` is pure and takes a `HabitabilityModel`. It returns a
`RaceEvaluation` with named `PointTerm` rows, the raw third-point total and the
displayed total (`raw / 3`, truncating toward zero).

The applied order is the recovered order: starting balance, growth bonus and
spread, innate habitability, ideal displacement and immunity penalty,
factory/growth interaction, resource generation, factory and mine blocks (or the
Alternate Reality replacement), primary trait, lesser traits and penalties, No
Advanced Scanners adjustment, research curve, Tech 3, Alternate Reality research
adjustment, then one final division.

Implementation rule identifiers map every term to the umbrella evidence rule
`RULE-RACE-POINTS-COST-001`:

| Identifier | Term |
|---|---|
| `RULE-RACE-POINTS-START-001` | Starting 550-point balance |
| `RULE-RACE-POINTS-GROWTH-001` | Growth bonus and spread remap |
| `RULE-RACE-POINTS-HABITABILITY-001` | Innate habitability |
| `RULE-RACE-POINTS-ENVIRONMENT-001` | Ideal displacement and immunity penalty |
| `RULE-RACE-POINTS-FACTORY-GROWTH-001` | Factory/growth interaction |
| `RULE-RACE-POINTS-RESOURCE-001` | Resource generation |
| `RULE-RACE-POINTS-FACTORY-001` | Factory block |
| `RULE-RACE-POINTS-MINE-001` | Mine block |
| `RULE-RACE-POINTS-AR-001` | Alternate Reality efficiency replacement |
| `RULE-RACE-POINTS-PRIMARY-001` | Primary trait cost |
| `RULE-RACE-POINTS-LESSER-001` | Lesser trait costs |
| `RULE-RACE-POINTS-LESSER-PENALTY-001` | Lesser count and mixture penalties |
| `RULE-RACE-POINTS-NAS-001` | No Advanced Scanners adjustment |
| `RULE-RACE-POINTS-RESEARCH-001` | Research curve |
| `RULE-RACE-POINTS-TECH3-001` | Tech 3 checkbox |
| `RULE-RACE-POINTS-AR-RESEARCH-001` | Alternate Reality research adjustment |
| `RULE-RACE-POINTS-FINAL-001` | Final truncating division |

### Step 6 research-cost direction

Dialog `#151` maps control 271 ("Costs 75% extra") to stored value `0`, standard
to `1` and "Costs 50% less" to `2`. The builder radios follow that original
order. The recovered `cCur = sum(value - 1)` makes a 75%-extra field a
disadvantage that adds advantage points, while a 50%-less field is an advantage
that subtracts them. The Alternate Reality research adjustment applies when the
first field is "Costs 50% less" (stored value `2`).

### Alternate Reality Step 5

Help topics 266 and 5751 document that Alternate Reality races cannot build
planetary installations and that their resources are:

```text
Resources = Habitability_Value * sqrt(Population * Energy_Tech_Level / Efficiency_Coefficient)
```

Help topic 5751 states that the Efficiency Coefficient is specified in the
Custom Race wizard. The decompiled `CResourcesAtPlanet()` uses `rsResGen` as
that coefficient. Step 5 therefore shows only the resource-generation slider
for Alternate Reality; the factory, mine and CheapFact controls are hidden.
The advantage-point calculation still prices `rsResGen` normally and replaces
the factory and mine blocks with the recovered Alternate Reality adjustment.

### Deliberate decisions

- The mine block keeps the recovered `+80` raw at neutral settings. It is not
  normalised away.
- Ultimate Recycling uses the unexplained index-5 value `-240` and is labelled
  `inferred`, not recovered.
- The decompilation's out-of-bounds lesser-trait loop is not reproduced. Only
  the fourteen defined traits are priced; `cheap_factories` and `tech3` are
  priced by their own terms.
- The alternate growth remap keeps the 19-to-20 discontinuity: spread 39 then
  45.
- The reference Humanoids record produces exactly 25 displayed points with the
  reference habitability model, matching the decompilation's own test anchor.

## Habitability injection

`HabitabilityModel` lets PR5 price a race before PR8 finalises the planet
economy. `ReferenceHabitability` implements the recovered sampling loop with a
reconstruction of `PctPlanetDesirability()`; `FixedHabitability` isolates formula
terms in tests. The reference desirability reconstruction is not an observed
original result and must not be described as final planet-economy fidelity.

## Trait-to-effect checklist

Selections are builder-only in PR5. Every row records the later PRs that own the
runtime effect in `PRIMARY_EFFECT_COVERAGE` and `LESSER_EFFECT_COVERAGE`; a test
asserts that all 24 rows exist and that none claims a runtime effect yet.

| Trait | ID | Runtime effect owner status |
|---|---|---|
| Hyper-Expansion | `PRT.HE` | PR6, PR8, PR9, PR10, PR13; pending |
| Super-Stealth | `PRT.SS` | PR6, PR9, PR10, PR11, PR12, PR13; pending |
| War Monger | `PRT.WM` | PR6, PR9, PR10, PR12, PR13; pending |
| Claim Adjuster | `PRT.CA` | PR6, PR8, PR9, PR13; pending |
| Inner-Strength | `PRT.IS` | PR6, PR9, PR10, PR12, PR13; pending |
| Space Demolition | `PRT.SD` | PR6, PR9, PR10, PR11, PR12, PR13; pending |
| Packet Physics | `PRT.PP` | PR6, PR8, PR9, PR10, PR11, PR12, PR13; pending |
| Interstellar Traveler | `PRT.IT` | PR6, PR9, PR10, PR11, PR12, PR13; pending |
| Alternate Reality | `PRT.AR` | PR6, PR8, PR9, PR10, PR11, PR13; pending |
| Jack of All Trades | `PRT.JOAT` | PR6, PR8, PR9, PR11, PR13; pending |
| Improved Fuel Efficiency | `LRT.IFE` | PR6, PR9, PR10, PR13; pending |
| Total Terraforming | `LRT.TT` | PR6, PR8, PR13; pending |
| Advanced Remote Mining | `LRT.ARM` | PR6, PR8, PR9, PR13; pending |
| Improved Starbases | `LRT.ISB` | PR6, PR9, PR12, PR13; pending |
| Generalized Research | `LRT.GR` | PR6, PR9, PR13; pending |
| Ultimate Recycling | `LRT.UR` | PR6, PR9, PR10, PR13; pending |
| Mineral Alchemy | `LRT.MA` | PR6, PR8, PR13; pending |
| No Ramscoop Engines | `LRT.NRSE` | PR6, PR9, PR10, PR13; pending |
| Cheap Engines | `LRT.CE` | PR6, PR9, PR10, PR13; pending |
| Only Basic Remote Mining | `LRT.OBRM` | PR6, PR8, PR9, PR13; pending |
| No Advanced Scanners | `LRT.NAS` | PR6, PR9, PR11, PR13; pending |
| Low Starting Population | `LRT.LSP` | PR6, PR8, PR13; pending |
| Bleeding Edge Technology | `LRT.BET` | PR6, PR9, PR13; pending |
| Regenerating Shields | `LRT.RS` | PR6, PR9, PR12, PR13; pending |

## UI and state contract

The wizard has six recovered steps plus a review page. It supports Tab focus,
PageUp/PageDown or Alt+Left/Right page movement, Ctrl+R reset, live point totals,
tooltips, a point breakdown and a final summary. Tests render every page at
100%, 150% and 200% zoom and with different window proportions.

A validated `RaceSpec` is stored in the new optional `Player::race` field and in
canonical state encoding. PR5 bumped `CORE_API_VERSION` and
`STATE_FORMAT_VERSION` to 2; PR6 bumps them to 3 when it adds planet details and
starting technology. PR6 consumes the validated race in the new-game form.

The review page shows an editable **Race file name**, prefilled from the race
name as a safe slug plus `.json`. **Finish** validates the race and writes the
versioned JSON document to `races/<file name>` under the current working
directory, using a temporary file and replacement. The review page displays the
absolute destination before saving. The document contains the format marker
`stars-remake-race`, version `1`, every `RaceSpec` field and the race-file
password in plain text; it is a local race file, not a security boundary.
Loading through `race_file::load_race_json` revalidates every field and the
advantage-point balance before returning a `RaceSpec`. The broader game-save
envelope, autosave and crash recovery remain owned by PR16.

## Known limitations

- The point formula comes from a decompilation of a later build
  (`stars26jrc3.exe`), not from running `stars26i.exe`.
- `GAP-VERSION-001` remains open: the January 1998 help's applicability to the
  supplied executable is unresolved.
- The checkbox-to-lesser-trait mapping is still unproven
  (`GAP-LESSER-MAP-001`); PR5 follows the canonical plan order and recovered cost
  indices.
- The exclusion matrix is unknown (`GAP-EXCLUSIONS-001`). PR5 does not guess it.
- Ultimate Recycling's price is inferred.
- The reference planet-desirability reconstruction is a PR5 convenience for
  live totals. PR8 owns the authoritative planet economy and may change the
  habitability component.
- No original-game execution, native window smoke test or AI behavior is
  claimed.
- The race JSON file is a focused PR5 export, not the PR16 game-save format.
  It has no migration path yet because only version 1 exists.
