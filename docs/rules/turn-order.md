# Turn-order dependency specification

Status: **global order unknown**. This document records the ordering
constraints the decoded help supports, the relationships that remain
unresolved, and the controlled measurements needed before core may claim
fidelity.

Rule IDs: `RULE-TURN-CYCLE-001`, `RULE-TURN-ORDER-001`,
`RULE-TURN-CONSTRAINTS-001`, `RULE-RNG-001`.

No original game was executed. The help is dated January 1998; its
applicability to `stars26i.exe` is unverified. Nothing below is an observed
turn-generation pipeline.

## 1. What is established

| Constraint | Status | Evidence |
|---|---|---|
| In single-player the computer acts as host and generates a new year after the player's turn | documented-only | Help topic 2, TOPICPOS 932, HLP 4505 |
| All games start in year 2400 | documented-only | Help topic 314, TOPICPOS 2050103, HLP 517676 |
| Minesweeping happens before movement, so a fleet sweeps in the year it leaves a field | documented-only | Help topic 131, TOPICPOS 1117052, HLP 285041 |
| Stargate travel is fuel-free and completes in the same year | documented-only | Help topic 50, TOPICPOS 429930, HLP 114015 |
| A packet travels half the normal distance in its first year and can decay in launch and arrival years | documented-only | Help topic 122, TOPICPOS 1033026, HLP 262455 |
| A battle lasts at most 16 rounds; a round is each token moving and firing, divided into token phases | documented-only | Help topic 376, TOPICPOS 2361467, HLP 595568 |
| Production is processed top-down and halts if the planet runs out of minerals | documented-only | Help topic 59, TOPICPOS 558530, HLP 144311 |
| Research takes resources off the top unless Contribute Only Leftover Resources is checked | documented-only | Help topic 73, TOPICPOS 657715, HLP 169768 |
| Combat reports and battle VCR describe the previous turn's battles | documented-only | Help topic 124, TOPICPOS 1051535, HLP 268676 |

These are local constraints. They do not say, for example, whether production
completes before movement, whether a newly built starbase can catch a packet in
its completion year, or whether AI orders are planned before or after the human
player's orders.

## 2. Unresolved phase relationships

The following phase categories are known to exist. Their relative order is not
recovered:

1. Player order intake and validation.
2. AI planning and order generation.
3. Production-queue progress and resource allocation.
4. Research progress and technology-level completion.
5. Construction completion (ships, mines, factories, defenses, scanners,
   starbases, stargates, mass drivers).
6. Terraforming and planetary environmental change.
7. Population growth, deaths and capacity changes.
8. Mining and mineral-concentration depletion.
9. Mine laying, decay and sweeping.
10. Fleet movement, including stargate and wormhole travel.
11. Waypoint tasks (colonize, transport, remote mine, patrol, scrap, lay mines,
    merge, transfer).
12. Packet launch, travel, catch, impact and decay.
13. Scanning and visibility updates.
14. Fleet combat, starbase combat, bombing and ground combat.
15. Resource/mineral credit from scrapping or recycling.
16. Random events and artifact/trader effects.
17. Victory evaluation and notifications.
18. Snapshot, event filtering and player-view update.

The help's local statements constrain only some edges:

```text
minesweeping ──before──> movement
stargate travel ──completes in──> same year
packet launch ──first year at half distance──> packet travel
production queue ──top-to-bottom──> production items
research ──off-the-top unless leftover-only──> production/research split
combat round ──token movement then firing──> next token/round
```

The following edges are **unknown** and must not be guessed in core:

```text
production completion  vs  movement
research completion    vs  production/design availability
construction completion vs  combat and packet catch
population growth      vs  production and combat
mining                 vs  production consumption
fleet movement         vs  combat initiation and colonization
bombing/ground combat  vs  capture and production next year
scrapping credit       vs  next-year resource pool
scanning update        vs  combat targeting and mine detection
AI planning            vs  human order intake
victory evaluation     vs  end-of-year messages
```

## 3. RNG-dependent ordering

The RNG algorithm, seed and consumption order are unknown
(`RULE-RNG-001`). Any ordering claim that depends on a random draw is therefore
conditional. In particular:

- Do not fabricate an original seed or a sequence of random events.
- Do not use wall-clock time, thread scheduling, hash iteration order or frame
  rate to choose a phase order.
- A deterministic remake may choose and document a reproducible RNG and phase
  order, but must label it as a designed implementation choice, not recovered
  original behavior, until a controlled comparison exists.
- Where randomness affects an outcome, use invariant or statistical scenarios
  rather than exact fixtures that pretend to know the original draw.

## 4. Measurement plan

The operator scenarios are:

- `SCN-TURN-ORDER-001` — global cross-phase order and RNG consumption.
- `SCN-TURN-CONSTRAINTS-001` — documented local constraints.
- `SCN-TURN-CYCLE-001` — submission and host generation.

Each discriminating trial should change one interaction and record:

1. executable hash and help hash;
2. game settings, seed if visible, difficulty, player count and turn number;
3. complete starting state relevant to the phase pair;
4. the order(s) submitted;
5. end state, messages, battle report and any visible intermediate state;
6. at least three trials, or five where randomness is suspected;
7. the conclusion and whether it is invariant, range-based or unknown.

Priority pairs for PR4/PR8-PR12:

| Trial | Question | Downstream |
|---|---|---|
| Fleet arrives at a planet the same year a starbase completes | Does the new starbase block bombing/invasion and participate in combat that year? | PR8, PR12 |
| Fleet arrives the same year a mass driver completes | Can the new driver catch a packet that year? | PR8, PR10 |
| Colony ship arrives the same year population growth is computed | Is growth computed before or after colonization? | PR8, PR10 |
| Production builds a ship the same year a fleet leaves | Does the new ship exist in orbit or in the departing fleet? | PR8, PR10 |
| Research completes the same year a design is queued | Can the new component be built that year? | PR8, PR9 |
| Bombing reduces population to zero while an invasion lands | Which resolution order determines capture? | PR12 |
| Scrapping at a starbase and spending the credit | Is the credit available in the same year or the next? | PR9, PR10 |
| Mine decay and sweeping occur on the same field | Are decay and sweep additive, and in which order? | PR10, PR12 |
| Packet arrival and fleet combat at the same planet | Does the packet hit before or after combat? | PR10, PR12 |
| AI and human orders interact at the same location | Are AI orders resolved after human orders, per player, or in a global batch? | PR14, PR15 |

## 5. Contract consequences for PR4

Until the measurements exist, PR4 must expose an explicit, versioned phase
pipeline and make the choice visible. The pipeline must:

- run on the desktop-owned worker with request IDs and ordered batches;
- apply results atomically and ignore stale responses;
- keep the last committed state on failure;
- use stable entity IDs and explicit arithmetic;
- keep RNG state versioned and separate from UI timing;
- allow a phase order to be replaced when evidence arrives without changing
  unrelated rules;
- record the phase order and RNG version in serialized state.

That is a software contract, not a claim about the original game. A dependency
graph or phase list chosen before the operator measurements must be labelled
`designed`, not `verified original`.

## 6. Current completion status

- Local ordering constraints: documented-only, with citations.
- Global phase order: unknown.
- RNG algorithm/seed/consumption: unknown.
- Original-game observation: none.
- Required resolution gate: PR4 for the deterministic pipeline and PR8-PR12 for
  the affected mechanics. A dependent feature cannot claim original fidelity
  while its required order is unresolved.
