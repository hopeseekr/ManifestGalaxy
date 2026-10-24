# Advantage-point calculation

This document specifies the custom-race advantage-point calculation that PR5
must implement. It replaces the `unknown` status of `RULE-RACE-POINTS-COST-001`
with a traced formula.

## Status and source

| Field | Value |
|---|---|
| Status | `statically-derived` for structure, table values and branch order; `designed` where this document corrects a decompilation defect |
| Primary source | `stars-decompile/race.c` function `CAdvantagePoints()` (lines 61-365) and `LInnateRaceHabitability()` (lines 492-735) |
| Source provenance | README of `stars-decompile`: types, signatures and structs extracted from **NB09 CodeView symbols appended to `stars26jrc3.exe`** (Stars! 2.6j RC3) |
| Supporting source | `stars-decompile/enums.h` (`RaceAttribute`, `RaceGrbit`, `RaceStat`), `stars-decompile/create.c` (leftover cap, line 1284) |
| Cross-check | January 1998 help, topics 247, 251-255, 268, 277-287; used only for sign convention, ranges and semantics |
| Product decision | The remake targets a **functional game**, not byte compatibility with any original build. See `GAP-VERSION-001`: no 2.6i/2.6j parity is claimed or required. |

### Why this is not `observed`

No original game was executed. This is a source-reading derivation from a
decompilation, so it carries the decompilation's own risk of translation error.
Two defects were found in the source and are corrected here; each correction is
labelled **designed** and must not be described as original behaviour.

The help supplies no numeric table (`RULE-RACE-POINTS-COST-001` remains
unsatisfiable from help alone). The decompilation supplies the numbers.

## Accounting model

The calculation runs in internal **third-point units**. The starting balance is:

```text
raw = 550 * 3              // 1650 third-points, i.e. 550 displayed AP
```

The accumulator stays integral throughout. The displayed balance is produced by
**one** integer division at the very end:

```text
displayed = trunc(raw / 3)
```

C integer division truncates toward zero, so a negative raw value rounds toward
zero, not toward negative infinity. Implementations must match that (Rust `/`
on integers does the same).

Terms are applied in this order (`race.c:74-364`):

1. starting raw
2. growth-rate bonus and remapping
3. innate habitability term
4. ideal-point displacement and multiple-immunity penalty
5. factory/growth/immunity interaction
6. resource generation
7. factory block (or the Alternate Reality replacement)
8. mine block (same replacement)
9. primary trait
10. lesser traits and their count/mixture penalties
11. No Advanced Scanners primary-trait adjustment
12. research-cost net calculation
13. Tech 3 checkbox
14. Alternate Reality research adjustment
15. final division

The order matters because every term mutates the same accumulator, and the
repair terms at steps 3-5 depend on values set by earlier steps.

## Inputs

The race is a `PLAYER`-shaped record. Inputs used by the calculation:

| Input | Range | Notes |
|---|---|---|
| `rgEnvVar[3]` | `-1` or `0..100` | Ideal point per gravity, temperature, radiation. `-1` means immune. |
| `rgEnvVarMin[3]`, `rgEnvVarMax[3]` | `-1` or `0..100` | Habitable band. Forced to `-1` when immune. |
| `pctIdealGrowth` | `1..20` | Percent per year at 100% planet value. |
| `rgAttr[rsResGen]` | `7..25` | Colonists per resource point. |
| `rgAttr[rsFactProd]` | `5..15` | Factories produced per 10 000 colonists. |
| `rgAttr[rsFactBuild]` | `5..25` | Factory cost. |
| `rgAttr[rsFactOperate]` | `5..25` | Factories operated per 10 000 colonists. |
| `rgAttr[rsMineProd]` | `5..25` | Mines produced per 10 000 colonists. |
| `rgAttr[rsMineBuild]` | `2..15` | Mine cost. |
| `rgAttr[rsMineOperate]` | `5..25` | Mines operated per 10 000 colonists. |
| `rgAttr[rsTechBonus1..6]` | `0..2` | `0` = 75% extra, `1` = standard, `2` = 50% less. |
| `rgAttr[rsMajorAdv]` | `0..9` | Primary trait index (see below). |
| `grbitAttr` | bit set | Lesser traits and option flags (see below). |

Ranges are from `rgRaceStatMin[]` / `rgRaceStatMax[]` at `race.c:16-17`.

### Primary trait indices

`RaceAttribute` in `enums.h` is the authoritative order and matches
`rgRacePrimaryTrait[10]` position for position:

| Index | Enum | Trait | `rgRacePrimaryTrait` |
|---:|---|---|---:|
| 0 | `raCheapCol` | Hyper-Expansion | 40 |
| 1 | `raStealth` | Super-Stealth | 95 |
| 2 | `raAttack` | War Monger | 45 |
| 3 | `raTerra` | Claim Adjuster | 10 |
| 4 | `raDefend` | Inner-Strength | -100 |
| 5 | `raMines` | Space Demolition | -150 |
| 6 | `raMassAccel` | Packet Physics | 120 |
| 7 | `raStargate` | Interstellar Traveler | 180 |
| 8 | `raMacintosh` | Alternate Reality | 90 |
| 9 | `raNone` | Jack of All Trades | -66 |

**There is no "no primary trait" value.** `raNone` is Jack of All Trades, not an
absence: `create.c:1184-1188` gives `raNone` tech 3 in all six fields and
`create.c:1563` gives it its own starting ships. The plan's requirement of
exactly one primary trait is therefore also what the data model enforces.

Semantics: `raw -= rgRacePrimaryTrait[major]`. A positive entry is a **cost**; a
negative entry is a **gain**.

### Lesser trait bits

| Bit | Enum | Trait | `rgRaceAdvDisPts` entry |
|---:|---|---|---:|
| 0 | `ibitRaceIFE` | Improved Fuel Efficiency | -235 |
| 1 | `ibitRaceTT` | Total Terraforming | -25 |
| 2 | `ibitRaceARM` | Advanced Remote Mining | -159 |
| 3 | `ibitRaceISB` | Improved Starbases | -201 |
| 4 | `ibitRaceGeneralizedResearch` | Generalized Research | +40 |
| 6 | `ibitRaceMineralAlchemy` | Mineral Alchemy | -155 |
| 7 | `ibitRaceNoRamscoops` | No Ramscoop Engines | +160 |
| 8 | `ibitRaceCheapEngines` | Cheap Engines | +240 |
| 9 | `ibitRaceOBRM` | Only Basic Remote Mining | +255 |
| 10 | `ibitRaceNoAdvScanner` | No Advanced Scanners | +325 |
| 11 | `ibitRaceLowStartingPop` | Low Starting Population | +180 |
| 12 | `ibitRaceBleedingEdgeTech` | Bleeding Edge Technology | +70 |
| 13 | `ibitRaceRegeneratingShields` | Regenerating Shields | +30 |

The full array (`race.c:13`) is
`{-235, -25, -159, -201, 40, -240, -155, 160, 240, 255, 325, 180, 70, 30}`,
which has **14 entries indexed 0..13**.

Bit 5 is not declared in `RaceGrbit` in this decompilation. The array carries a
value at index 5 (`-240`) where the enum has a gap, so index 5 is either Ultimate
Recycling or a positional artefact of the decompiled table. **This document
treats index 5 as unavailable** (see Corrections). Every mapped trait keeps its
own index, so the mapping above is unaffected by that gap.

Sign convention: `raw += rgRaceAdvDisPts[bit]` for every selected lesser trait.
Negative entries are costs, positive entries are gains.

The remake treats Cheap Engines as a **gain of 80 AP** (raw +240) because that is
what the table says. The help (topic 277) confirms only the semantic side: engines
cost 50% less, with a 10% chance of refusing to engage above warp 6. No
independent confirmation of the sign exists.

## Term-by-term specification

All arithmetic is integer on 32-bit signed values unless stated otherwise.

### 1. Growth bonus and remapping

Let `g = clamp(pctIdealGrowth, 1, 20)`. If the stored value was outside that
range, the source forces it to 1 and marks the race as tampered; the remake
rejects out-of-range input during validation instead (`race.c:89-93`).

A direct bonus is added **before** remapping (`race.c:107-128`):

| `g` | Raw bonus |
|---:|---:|
| 1 | 5 x 4200 = 21000 |
| 2 | 4 x 4200 = 16800 |
| 3 | 3 x 4200 = 12600 |
| 4 | 2 x 4200 = 8400 |
| 5 | 1 x 4200 = 4200 |
| 6 | 3600 |
| 7 | 2250 |
| 8 | 600 |
| 9 | 225 |
| 10-20 | 0 |

Then `g` is remapped to a spread factor `s` (`race.c:107-128`, `race.c:135`):

```text
g in 1..5:   s = g
g in 6..13:  s = 5 + 2*(g - 5)      // 7, 9, 11, 13, 15, 17, 19, 21
g in 14..19: s = 21 + 3*(g - 13)    // 24, 27, 30, 33, 36, 39
g == 20:     s = 45
```

The expression `5 + 8*2 + (g-13)*3` in the source is `21 + 3*(g-13)`. Checked
against the source for g = 14..19 this yields 24, 27, 30, 33, 36, 39. The
`g == 20` case is written separately as `5 + 16 + 18 + 6 = 45`.

The sequence is therefore not linear across the 19/20 boundary: g = 19 gives 39
and g = 20 gives 45, a jump of 6 rather than 3. That discontinuity is in the
source and is preserved deliberately; PR5 should test g = 19 and g = 20 as a
pair so the behaviour is pinned either way.

This bonus is the "slower growth pays" term: low growth rates add points
directly, on top of the habitability effect below.

### 2. Innate habitability

```text
lInnate = LInnateRaceHabitability(race) / 2000
raw -= (lInnate * s) / 24
```

`LInnateRaceHabitability()` is the expensive part and is specified separately in
[Innate habitability](#innate-habitability-term). It returns an integer estimate
of habitable volume. Both divisions are truncating.

### 3. Ideal-point displacement and immunity count

For each of the three environmental axes (`race.c:142-148`):

```text
if rgEnvVar[i] >= 0:  raw += |rgEnvVar[i] - 50| * 4
                       // moving the ideal off centre pays back points
else:                 immuneCount++
```

Then (`race.c:154-155`):

```text
if immuneCount > 1:   raw -= 150
```

The help (topic 284) confirms the direction: moving a bar away from centre
increases advantage points.

### 4. Factory/growth/immunity interaction

Applies only when factory operation or factory production is above 10
(`race.c:167-181`):

```text
if rsFactOperate > 10 or rsFactProd > 10:
    op   = max(1, rsFactOperate - 9)
    prod = max(1, rsFactProd - 9)
    prod *= 3 if primary == Hyper-Expansion else 2
    raw -= op * prod * pctGrowth / (2 if immuneCount >= 2 else 9)
```

Note `pctGrowth` here is the **clamped original growth**, not the remapped `s`.

### 5. Resource generation

`R = min(rsResGen, 25)` (`race.c:191-200`):

| `R` | Raw change |
|---:|---:|
| <= 7 | -2400 |
| 8 | -1260 |
| 9 | -600 |
| 10 | 0 |
| 11-25 | +120 x (R - 10) |

### 6. Factory block

Skipped entirely for Alternate Reality (`race.c:206`). With
`a = 10 - rsFactProd`, `b = 10 - rsFactBuild`, `c = 10 - rsFactOperate`
(`race.c:213-254`):

```text
F  = a * 100            if a > 0   else a * 121
F -= b * 55             if b < 0   else F -= b*b*60
F += c * 40             if c > 0   else c * 35

if F > 700:  F = 700 + (F - 700) / 3          // compression, truncating

if c <= -7:
    if c >= -11:  F -= 30 * (-6 - c)
    elif c >= -14: F -= 75*3 + 45 * (-12 - c)
    else:          F -= 120*3

if a <= -3:  F -= (-2 - a) * 20 * 3

raw += F

if cheap factory materials flag:  raw -= 175
```

Sign convention verified by sweep at `rsFactProd = rsFactOperate = 10`
(raw points; negative is a cost):

| `rsFactBuild` | Raw |
|---:|---:|
| 5 | -1500 |
| 9 | -60 |
| 10 | 0 |
| 11 | +55 |
| 15 | +275 |
| 25 | +741 |

So cheaper factories are a cost and dearer factories are a gain, which matches
the help's statement that the balance falls when a control grants an advantage.

### 7. Mine block

Also skipped for Alternate Reality (`race.c:261-281`). With
`a = 10 - rsMineProd`, `b = 3 - rsMineBuild`, `c = 10 - rsMineOperate`:

```text
M  = a * 100            if a > 0   else a * 169
M -= (-80 + b * 65)     if b <= 0   else M -= 360
M += c * 40             if c > 0   else c * 35

raw += M
```

Neutral settings are `rsMineProd = 10`, `rsMineBuild = 3`, `rsMineOperate = 10`,
which give `a = b = c = 0` and therefore **`M = +80`**, not zero. The factory
block at neutral settings returns 0. The asymmetry is in the source (`-80 + 0`
is a gain) and is preserved here, but it means a nominally "standard" race
collects 80 raw points from the mine block. PR5 should assert this explicitly so
nobody silently "fixes" it later.

The block yields `-360` as soon as `rsMineBuild < 3`, so the crossover between
paying and gaining is at `rsMineBuild = 3`.

Sweep at `rsMineProd = rsMineOperate = 10` (raw points):

| `rsMineBuild` | 2 | 3 | 5 | 10 | 15 |
|---|---:|---:|---:|---:|---:|
| Raw | -360 | +80 | +210 | +535 | +860 |

The sign convention matches the factory block (dearer mines pay), but the
neutral case lands at +80 rather than 0. Either the original neutral mine cost is
2, not 3, or the decompiled `-80` constant absorbs a baseline. PR5 must pick one
deliberately: normalise `M` by subtracting 80 so neutral is 0, or keep the raw
offset so the 550 starting balance absorbs it. **Keeping the source value is the
recommended default**, because the product is not trying to match a specific
build and an unexplained -80 shift is less risky than inventing a correction.
Record the choice either way.


### 8. Alternate Reality replacement

When `rgAttr[rsMajorAdv] == raMacintosh`, steps 6 and 7 are replaced by a single
`raw += 210` (`race.c:282-284`). The help (topic 286) agrees that most
efficiency controls are disabled for such races.

### 9. Primary trait

```text
raw -= rgRacePrimaryTrait[major]
```

### 10. Lesser traits and count penalties

For each selected lesser trait with a mapped entry (`race.c:295-315`):

```text
raw += entry
if entry < 0: countCost++  else: countGain++

total = countCost + countGain
if total > 4:              raw -= (total * 10) * (total - 4)
if countGain - countCost > 3:  raw -= 60 * ((countGain - countCost) - 3)
if countCost - countGain > 3:  raw -= 40 * ((countCost - countGain) - 3)
```

The classification uses the **entry sign**, not a per-trait category label.

### 11. No Advanced Scanners adjustment

Only when that trait is selected (`race.c:323-330`):

| Primary | Raw adjustment |
|---|---:|
| Packet Physics | -280 |
| Super-Stealth | -200 |
| Jack of All Trades | -40 |
| any other | 0 |

### 12. Research cost

`cCur = sum(rsTechBonus1..6) - 6`, i.e. the net count of 50%-less fields minus
75%-extra fields (`race.c:333-353`). Dialog `#151` maps "Costs 75% extra" to
stored value `0`, standard to `1` and "Costs 50% less" to `2`, so an extra-cost
field contributes `-1` and a cheaper field contributes `+1`.

A more expensive field is a disadvantage, so `cCur < 0` adds advantage points.
A cheaper field is an advantage, so `cCur > 0` subtracts them.

```text
if cCur > 0:  raw -= cCur*cCur*130
              if cCur == 6:  raw += (36 - 25) * 130     // 1430 back
              if cCur == 5:  raw += (25 - 21) * 130     // 520 back
if cCur < 0:  raw += rgRaceDisEnvPts[-cCur - 1]
              if -cCur > 4 and rsResGen < 10:  raw -= 190
```

`rgRaceDisEnvPts = {150, 330, 540, 780, 1050, 1380}` (`race.c:14`).

Worked values for `cCur > 0` (net 50%-less fields, an advantage):

| `cCur` | Raw change | Derivation |
|---:|---:|---|
| 1 | -130 | 130 x 1 |
| 2 | -520 | 130 x 4 |
| 3 | -1170 | 130 x 9 |
| 4 | -2080 | 130 x 16 |
| 5 | -2730 | 3250 - 520, replacing the 130 x 25 square |
| 6 | -3250 | 4680 - 1430, replacing the 130 x 36 square |

The add-backs at 5 and 6 make those two steps *cheaper* than the plain square
would be: 3250 instead of 4680 at six fields. The curve still rises
monotonically. This is in the source and is preserved.

Worked values for `cCur < 0` (net 75%-extra fields, a disadvantage), i.e.
`k = -cCur`:

| `k` | Raw gain |
|---:|---:|
| 1 | 150 |
| 2 | 330 |
| 3 | 540 |
| 4 | 780 |
| 5 | 1050 |
| 6 | 1380 |

### 13. Tech 3 checkbox

```text
if ibitRaceTech3:  raw -= 180
```

### 14. Alternate Reality research adjustment

```text
if primary == Alternate Reality and rsTechBonus1 == 2:  raw -= 100
```

The source conditions on `rsTechBonus1` specifically, not on the sum
(`race.c:360-361`). Stored value `2` is "Costs 50% less", the cheaper choice.

### 15. Result

```text
return (int16_t)(raw / 3)     // truncating division
```

## Innate habitability term

`LInnateRaceHabitability()` (`race.c:492-735`) estimates habitable volume by
sampling the race's environmental box. It runs three passes, one per terraforming
depth, and sums a weighted volume.

### Terraforming depth per pass

| Pass | `pctTerra` without Total Terraforming | with Total Terraforming |
|---:|---:|---:|
| 0 | 0 | 0 |
| 1 | 5 | 8 |
| 2 | 15 | 17 |

This is the mechanism behind the help's statement (topics 284, 285) that immunity
can waste points spent on Total Terraforming: an immune axis has no band to
expand, so deeper passes cannot add volume there.

### Per-axis setup

For each axis:

```text
if rgEnvVar[i] < 0:                 // immune
    base = 50;  inc = 11;  steps = 1
else:
    base = max(0, rgEnvVarMin[i] - pctTerra)
    top  = min(100, rgEnvVarMax[i] + pctTerra)
    inc  = top - base
    steps = 11
```

### Sampling

For pass `iTerra`:

- Axis 0 walks `rgSteps[0]` samples, axis 1 `rgSteps[1]`, axis 2 `rgSteps[2]`.
- Sample values are linearly interpolated:
  `v = base + (k * inc) / (steps - 1)`, or `base` when `steps == 1`.
- For passes above 0 on a non-immune axis, the offset from the ideal is clamped
  into the terraformable band:

```text
delta = envVar[axis] - sample
if |delta| <= pctTerra:  delta = 0
elif delta < 0:          delta += pctTerra
else:                    delta -= pctTerra
sample = envVar[axis] - delta
```

- Each sample becomes a planet with those three environmental values and is
  scored by `PctPlanetDesirability(planet, 0)`, truncated to 16 bits.

### Weighting

```text
deltaSum = delta[0] + delta[1] + delta[2]
if deltaSum > pctTerra:
    desire = max(0, desire - (deltaSum - pctTerra))

weight = 7 if pass == 0 else (5 if pass == 1 else 6)
acc   += desire * desire * weight            // 32-bit, truncated
```

Then per axis-1 sample:

```text
if envVar[2] >= 0:  l1 = (acc * inc[2]) / 100
else:               l1 = acc * 11
iSum += l1
```

Per axis-0 sample:

```text
if envVar[1] >= 0:  iSum = iSum * inc[1] / 100
else:               iSum = iSum * 11
terraSum += iSum
```

After the axis-0 loop:

```text
if envVar[0] >= 0:  terraSum = terraSum * inc[0] / 100
else:               terraSum = terraSum * 11
lInnate += terraSum
```

The immune case multiplies by 11 because an immune axis is sampled once where a
band would be sampled eleven times, and the factor compensates.

### Return

```text
return floor(lInnate / 10.0 + 0.5)
```

The source accumulates `double` values but the inputs are small integers, so an
integer implementation is exact. PR5 may use rational arithmetic scaled by 100 to
avoid floating point entirely; that is a documented implementation choice, not a
behaviour change.

`PctPlanetDesirability()` is owned by PR8 (planet economy) and is not yet
specified. Until it exists, PR5 must take habitability as an injected function so
the builder can be tested without the economy. A placeholder that returns a
fixed value is acceptable **only** in tests, and must be labelled as such.

## Leftover points

`create.c:1284` caps the value used for leftover conversion:

```c
iT = min(50, CAdvantagePoints(pplr));
```

So the 50-point cap mentioned in dialog `#146` is real, not just a label. This
closes the uncertain part of `GAP-LEFTOVER-001`; the cap is now
`statically-derived`.

Documented conversions (help topics 251-255, confirmed verbatim):

| Choice | Conversion |
|---|---|
| Surface minerals | 10 kT per leftover point, weighted toward rarer minerals |
| Mines | 1 mine per 2 points |
| Factories | 1 factory per 5 points |
| Defenses | 1 defense per 10 points |
| Mineral concentration | +1% to the poorest home-world mineral per 3 points |

Exact integer rounding for the five conversions and the rarity weighting are
**not** specified in the decompiled source read here. PR6 owns them. Until they
are specified, implement integer division (floor) and label it **designed**.

## Exclusions and validation

The point calculation contains no exclusion logic. `CAdvantagePoints()` prices
whatever combination it is given.

`GAP-EXCLUSIONS-001` remains open: no trait-pair exclusion matrix was recovered.

PR5's validation contract is therefore:

1. exactly one primary trait (enforced by the data model: there is no "none");
2. a lesser-trait selection with a defined cost;
3. lesser-trait count/mixture penalties as specified above;
4. final balance greater than or equal to zero.

Because the exclusion matrix is unknown, PR5 must not reject a combination on a
guessed rule. The remakes's own policy of permitting any priced combination is a
**designed** decision and must stay visible in this document.

A subtle consequence worth testing: because count penalties make traits 5 and
beyond progressively more expensive, a race with many traits is heavily taxed.
That is a game-design consequence of the recovered formula, not a validation
rule.

## Corrections to the decompilation

Two defects were found in `stars-decompile/race.c`. Both are corrected in this
specification. Both corrections are **designed**: they make the game functional
and internally consistent, and they are not claims about original behaviour.

### C1. Lesser-trait loop reads past the array

The source iterates `i = 0..ibitRaceLast` where `ibitRaceLast == 32`, then
indexes the 14-entry array:

```c
for (i = 0; i <= ibitRaceLast; i++)
    if (GetRaceGrbit(pplr, i)) { ...; cPoints += rgRaceAdvDisPts[i]; }
```

Bits 14-32 are out of bounds for the array. Bits 29-31 (`ibitRaceTech3`,
`ibitRaceAIPlayer`, `ibitRaceCheapFact`) are real option flags, so any race using
Costs One Less factories or the Tech 3 checkbox triggers a read outside the
array.

**Correction:** iterate only over the lesser-trait bits that have a defined
entry, and price the option flags in their own terms (the source already handles
`Tech3` at `race.c:356` and `CheapFact` at `race.c:253`).

### C2. Bit 5 has no enum entry but the table has a value

`RaceGrbit` in `enums.h` jumps from `0x04` to `0x06`. The table holds `-240` at
index 5, which does not correspond to any selectable trait in the product.

**Correction:** index 5 is not a selectable trait. No lesser trait is priced at
-240 unless a later artifact identifies it as Ultimate Recycling, which the
help documents as a real trait with a real effect. PR5 must implement Ultimate
Recycling's **effect** in PR13 regardless; only its price is unresolved here.

### Consequence for the 14 lesser traits

The plan requires all 14 lesser traits to be selectable with evidence-backed
costs. This specification supplies costs for 13 of them. Ultimate Recycling's
effect is documented (help topic 274) but its price is not established. PR5 must
either:

- use the `-240` at index 5 as Ultimate Recycling's cost, labelled **inferred**,
  because it is the only unexplained value adjacent to the gap; or
- report Ultimate Recycling's price as a completion blocker.

The first option is recommended for a functional game, provided PR5 records it
as inferred rather than recovered.

## Reference case

The decompilation's own test asserts one anchor:

```text
vrgplrDef[0] ("Humanoids")  ->  CAdvantagePoints == 25
```

The test passes (`stars-decompile/bin/test_race`). The value's provenance is not
recorded in the repository: it was added in commit `e8641cc` ("fixed genworld to
work with race points"), not from a documented measurement. It is a useful
smoke anchor for the whole pipeline, including the habitability sampling, but it
is **one** case and does not validate the trait tables.

PR5 should reproduce this value if the Humanoids default definition is
transcribed, and should treat a mismatch as a signal about the habitability term
rather than about the trait tables.

## Implementation notes for PR5

- Use a single `i32` accumulator starting at 1650. Do not round intermediate
  values.
- Divide by 3 exactly once, at the end, with truncation toward zero.
- Inject `PctPlanetDesirability` so the builder is testable before PR8.
- Keep every constant in a named, documented location; each must trace to this
  document.
- The formula is pure: no RNG, no I/O, no clock. It is safe to call while typing
  in the builder.
- Recommended tests: growth bonus at every breakpoint (g = 1, 5, 6, 9, 10, 13,
  14, 19, 20); the growth-remap discontinuity at 19/20 (39 then 45); research
  curve at `cCur` from -6 to +6 (expecting 1380, 1050, 780, 540, 330, 150, 0,
  -130, -520, -1170, -2080, -2730, -3250); the factory sweep at
  `rsFactBuild` = 9/10/11 giving -60/0/+55; the mine sweep at
  `rsMineBuild` = 2/3/10 giving -360/+80/+535; lesser-trait count penalties at
  4/5/6 traits and at the two mixture thresholds; Alternate Reality block
  replacement; the No Advanced Scanners pairing for all three primaries; and the
  C1 overrun regression (a race with CheapFact set must not read outside the
  trait table).

## Gap-ledger updates

| Gap | Before | After |
|---|---|---|
| `GAP-POINTS-001` | unknown | **closed** for 13 lesser traits, both trait tables (primary fully), all Step-5 curves, growth and habitability; Ultimate Recycling price `inferred` |
| `GAP-LEFTOVER-001` | unverified | **partly closed**: the 50-point cap is `statically-derived`; per-conversion rounding still designed |
| `GAP-EXCLUSIONS-001` | unknown | unchanged |
| `GAP-VERSION-001` | unresolved | **not a blocker by product decision**: no original-build parity is required |
| `GAP-PRIMARY-001` | static | **closed**: exactly one primary is enforced by the data model (`raNone` is Jack of All Trades) |
| `GAP-POP-EFF-001` | not decoded | **closed**: all seven ranges and curves specified above |

## PR5 implementation notes

PR5 implements this specification in `crates/stars-core/src/race/` with the
field inventory, validation contract, trait coverage checklist and deliberate
decisions recorded in [`race-builder.md`](race-builder.md).

- `evaluate_race` takes an injected `HabitabilityModel`. `ReferenceHabitability`
  runs the recovered three-pass sampling with the integer reconstruction of
  `PctPlanetDesirability()` from `stars-decompile/planet.c`;
  `FixedHabitability` isolates formula terms in tests.
- The reference `RaceSpec::humanoids_anchor()` record reproduces the decompilation's
  own anchor of **25 displayed points** and **75 raw third-points**. That is an
  internal consistency check against the decompiled default record, not an
  observation of the original game.
- Each term carries an implementation rule identifier (`RULE-RACE-POINTS-*`)
  mapped to the umbrella evidence rule `RULE-RACE-POINTS-COST-001`. The mapping
  is listed in [`race-builder.md`](race-builder.md#point-evaluation).
- The recovered mine-block `+80` raw at neutral settings is kept, not
  normalised. Ultimate Recycling's price remains `inferred`. The C1 array
  overrun is not reproduced.
- The environment ideal point is derived as the band midpoint, matching
  `BoundsCheckPlayer()`; the builder exposes the band and immunity.
- The Step 5 efficiency inventory is the seven recovered `RaceStat` fields.
  The help's Step 5 text names colonists, mines and factories; it does not
  recover a separate defense-efficiency field, and none was invented.
- A race-file password from Step 1 is represented as a local-only field. It is
  never logged and is not a security boundary; PR16 owns persistence.

Remaining blockers are unchanged: no exclusion matrix (`GAP-EXCLUSIONS-001`),
no proven lesser-checkbox mapping (`GAP-LESSER-MAP-001`), and no original-game
execution (`GAP-VM-001`). The reference desirability reconstruction is a builder
convenience until PR8 owns the authoritative planet economy.
