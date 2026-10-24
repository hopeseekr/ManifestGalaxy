# PR6 universe generation and new-game setup

Status: implemented. The generation arithmetic is statically derived from the
decompiled Stars! 2.60j RC3 source (`stars-decompile/create.c`), which is a later
build than the supplied `stars26i.exe`. The January 1998 help supplies the
player-facing option inventory and the documented density relationship. No
original game was executed, so no observed result is claimed.

Rule identifiers referenced below are in [`rule-ledger.md`](rule-ledger.md);
unresolved items remain in [`gap-ledger.md`](gap-ledger.md).

## Evidence

| Fact | Evidence | Class |
|---|---|---|
| Size values Tiny 400, Small 800, Medium 1,200, Large 1,600, Huge 2,000 ly | Help topics 227-231 | documented-only |
| Density normal ~1/5,000 ly², sparse ~1/6,500, dense ~1/4,000, packed +30% over dense except Huge | Help topic 236 | documented-only |
| Player positions affect starting separation | Help topic 237; dialog `#390` controls 1009-1012 | documented-only |
| Galaxy Clumping clusters star systems | Help topic 244; dialog `#390` control 1050 | documented-only |
| Six advanced option bits | Help topics 238-243; dialog `#390` controls 1016-1021 | documented-only |
| Seven victory-condition controls and must-meet count | Help topics 246, 25, 24; dialog `#392` controls 291-297 | documented-only |
| `dGal = mdSize * 400 + 400`; `cPlanMax = dGal²/5000`, density adjustment, cap 999 | `create.c` `GenerateWorld()` | statically derived |
| 12-unit minimum star separation; 1,000-unit border inset | `create.c`; `globals.c` `dGalMinDist = 12`, `dGalOff = 1000` | statically derived |
| Candidate count `cPlanMax + cPlanMax/7`, then overlap removal and random culling | `create.c` `GenerateWorld()` | statically derived |
| Clumping attraction thresholds 12, 18, 25 and 40 units | `create.c` `GenerateWorld()` | statically derived |
| Environment and mineral rolls | `create.c` `GenerateWorld()` | statically derived |
| Homeworld placement, bounds and separation bands | `create.c` `GenerateWorld()` | statically derived |
| Starting technology for every primary trait | `create.c` `InitNewGamePlr()` | statically derived |
| Starting population, factories, mines, defenses and leftover conversions | `create.c` `InitNewGamePlr()` | statically derived |
| Turindrones race fields for four proficiency levels | `create.c` `vrgplrComp[1][0..3]` | statically derived |
| Turindrones AI policy and memory | not recovered | unknown |

The decompiled source lives outside the committed tree at
`stars-decompile/create.c`; the repository tracks the reproducible rule notes,
not the original payload or extracted resources.

## Recovered wizard inventory

The new-game form exposes the complete recovered advanced setup rather than a
subset:

- game name;
- galaxy size (five values);
- density (Sparse, Normal, Dense, Packed);
- player positions (Close, Moderate, Farther, Distant);
- Galaxy Clumping;
- Beginner: Maximum Minerals;
- Slower Tech Advances;
- Accelerated BBS Play;
- No Random Events;
- Computer Players Form Alliances;
- Public Player Scores;
- Turindrones opponent count (the product has no other selectable AI race);
- Easy, Standard, Tough or Expert proficiency;
- seven victory conditions with documented ranges and a must-meet count;
- minimum game years;
- explicit seed entry and display;
- human race from the race builder or a versioned race JSON file. The form
  discovers `.json` files under `races/`, allows an explicit path, and validates
  the file again before it can be used.

Victory thresholds are presented as sliders. Planet percentage, exceeding the
nearest rival and capital-ship count move in five-point increments; the score
threshold moves in 1,000-point increments; production capacity is displayed in
thousands and moves in 10-thousand steps; the year threshold moves in ten-year
steps. These are interface increments, not changes to the documented ranges.

The original basic dialog also exposes predefined races, the tutorial and the
player race dropdown. Predefined-race presets and the tutorial are excluded by
the product plan; the custom-race builder supplies the human race.

## Planet count

The recovered count is:

```text
span = 400 + 400 * size_index
base = span * span / 5000
sparse = base - base / 4
normal = base
dense  = base + base / 4
packed = dense + base / 4
count  = min(result, 999)
```

This gives:

| Size | Sparse | Normal | Dense | Packed |
|---|---:|---:|---:|---:|
| Tiny (400) | 24 | 32 | 40 | 48 |
| Small (800) | 96 | 128 | 160 | 192 |
| Medium (1,200) | 216 | 288 | 360 | 432 |
| Large (1,600) | 384 | 512 | 640 | 768 |
| Huge (2,000) | 600 | 800 | 999 | 999 |

The candidate count is `count + count/7`, capped at 999. Candidates closer than
12 units to an already accepted candidate are removed, then random candidates
are culled until the target count is reached. Because overlap removal can exceed
the culling budget, the final count can be slightly below the table; Huge Dense
and Huge Packed use the same candidate stream and therefore the same count.

**Documented difference:** help topic 236 says Packed adds 30% over Dense. The
decompiled formula adds a second quarter of the base count, which is 20% over
Dense, and the 999 cap makes Huge Packed equal Huge Dense as the help states.
PR6 follows the recovered formula and records `GAP-UNIVERSE-PACKED-001` rather
than silently adopting the help's 30% wording.

## Coordinates and clumping

Authoritative coordinates are integers in a 2,000-unit inset frame. Each
candidate is generated at:

```text
x = 1010 + Random(span - 19)
y = 1010 + Random(span - 19)
```

Candidates are sorted by x. Clumping repeatedly selects a random star, finds its
nearest neighbour, and moves it toward that neighbour by one of four weighted
fractions depending on squared separation:

| Squared distance | New position |
|---|---|
| > 1,600 | `(old + 2*nearest) / 3` |
| > 625 | `(old + nearest) / 2` |
| > 324 | `(2*old + nearest) / 3` |
| > 144 | `(4*old + nearest) / 5` |
| <= 144 | unchanged |

The list is sorted by x again afterwards. Clumping changes the distribution but
not the target count.

## Environments and minerals

Every planet receives two gravity and two temperature rolls and one radiation
roll, matching the help's slight central weighting. Minerals use two independent
45-value rolls plus 31, with the recovered high-radiation and Accelerated BBS
adjustments. At least one mineral is forced low on a random subset using the
recovered `Random(27)` branch. Maximum Minerals sets all concentrations to 100
and skips that forcing branch.

Planet 0 supplies the common homeworld surface-mineral template. Every homeworld
copies that template and the planet-0 concentrations, with concentrations
clamped to at least 30. This is how the help's statement that all home worlds
start with the same surface mineral content is implemented; racial leftover
spending can then adjust an individual homeworld.

## Homeworld placement and separation

Player 0 is placed near the centre quarter of the galaxy using up to 50 random
candidates. Remaining players are placed inside a galaxy ring whose margins
depend on player count:

| Players | Lower/upper bound |
|---|---|
| 2 | 3/20 and 17/20 of the span |
| 3-4 | 1/10 and 9/10 of the span |
| 5+ | 1/20 and 19/20 of the span |

The minimum and maximum squared distances derive from the recovered formula
`span² / player_count - span * 6`, scaled by player-position choice. A candidate
must be far enough from every earlier homeworld and within the maximum distance
of at least one. If random search fails, a deterministic sequential search widens
the bands. A final distinct-planet fallback prevents duplicate ownership; it is
a safety net and is not expected for supported settings.

## Starting conditions

All games start in year 2400 with turn 0. Every homeworld starts with one
starbase flag, 10 factories, 10 mines, 10 defenses, the common surface minerals
and concentrations, and an environment at the race's ideal point (or a random
value on an immune axis). The starting population is 25,000 colonists, or 17,500
with Low Starting Population. Accelerated BBS scales population by
`(2 * growth + 10) / 10`, where Hyper-Expansion doubles the growth term.

Starting technology is recovered per primary trait:

| Primary trait | Starting technology (Energy, Weapons, Propulsion, Construction, Electronics, Biotech) |
|---|---|
| Hyper-Expansion | 0, 0, 0, 0, 0, 0 |
| Super-Stealth | 0, 0, 0, 0, 5, 0 |
| War Monger | 1, 6, 1, 0, 0, 0 |
| Claim Adjuster | 1, 1, 1, 2, 0, 6 |
| Inner-Strength | 0, 0, 0, 0, 0, 0 |
| Space Demolition | 0, 0, 2, 0, 0, 2 |
| Packet Physics | 4, 0, 0, 0, 0, 0 |
| Interstellar Traveler | 0, 0, 5, 5, 0, 0 |
| Alternate Reality | 1, 0, 0, 0, 0, 0 |
| Jack of All Trades | 3, 3, 3, 3, 3, 3 |

Tech 3 raises fields whose research cost is "75% extra" to 3, or to 4 for Jack
of All Trades. Cheap Engines and Improved Fuel Efficiency each add one
Propulsion level afterwards. Alternate Reality homeworlds have no factories,
mines or defenses. Packet Physics and Interstellar Traveler receive a second
starting world when the galaxy is not Tiny; the primary home keeps 4/5 of the
population and the second world receives 2/5, matching the recovered code.

Leftover advantage points are capped at 50 and convert as recovered:

- Surface minerals: 10 kT per point; the lowest mineral receives the remainder
  plus its share, and all three receive the common share.
- Mineral concentration: `points / 2` (1 for one or two points) to the lowest
  mineral, then half again to all three.
- Mines: one per two points.
- Factories: one per five points.
- Defenses: one per ten points, rounded up.
- Tough and Expert computer players use the concentration branch regardless of
  their stored choice; Expert also gets 50 points and a 10% population bonus.

Starting ships and starbase component fitting are not part of PR6. The state
records the starbase flag, starting technology and starting world inventory;
PR9/PR12 own designs and combat components.

## Turindrones configuration

`vrgplrComp[1]` contains four Turindrones proficiency records. PR6 maps them to
the typed race model:

| Level | Growth | Habitability G/T/R | Primary | Lesser traits | Extra |
|---|---:|---|---|---|---|
| Easy | 14 | 27-89 / 7-63 / 35-95 | Super-Stealth | IFE, ARM, MA, RS | standard costs |
| Standard | 14 | 32-92 / 6-60 / 26-96 | Super-Stealth | IFE, ARM, MA, RS | Cheap Factories |
| Tough | 14 | 31-95 / 4-52 / 30-94 | Super-Stealth | IFE, ARM, MA, RS | Cheap Factories |
| Expert | 15 | 31-93 / 5-53 / immune | Super-Stealth | IFE, ARM, MA, RS | Cheap Factories, Tech 3, all costs 75% extra |

Each opponent gets a distinct player identifier and display name. Their RNG
streams are separated by player identifier. The AI decision policy and memory
remain PR14 work; PR6 claims only the recovered race configuration.

## Reproducible PR7 fixture

PR7 can reproduce a labelled fixture with:

```text
game name: PR7 Fixture
seed: 0x5052_375F_4649_5854
size: Medium
density: Normal
positions: Moderate
clumping: false
opponents: 2 (Standard)
human race: RaceSpec::humanoids_anchor()
```

The core test `pr7_fixture_is_stable` pins the planet count, player count and
homeworld identifiers for that input. It is a deterministic fixture, not an
observed original universe.

## Designed choices and limitations

- The RNG mapping is a remake choice. The original algorithm, seed handling and
  consumption order are unknown (`GAP-RNG-001`). Generation uses a separate
  deterministic stream derived from the explicit seed; the resulting state
  records the same seed for later turns.
- Planet names in the original come from an undecoded name table. PR6 generates
  `Star NNNN` labels (`GAP-UNIVERSE-NAMES-001`).
- Victory-condition defaults are remake interface defaults. The original
  checkbox labels and default set were not decoded; enforcement belongs to
  PR15.
- The decompilation is from a later build than the supplied executable. All
  statically derived claims carry that qualification (`GAP-VERSION-001`).
- No original game was executed, so the generator cannot be called
  observation-verified (`GAP-VM-001`).
