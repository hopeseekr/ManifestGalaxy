# PR4 simulation contracts

Status: implemented for the deterministic contract. The phase order and RNG are
**designed remake choices**, not recovered original behaviour. `GAP-TURN-001`
and `GAP-RNG-001` remain open in the [gap ledger](../rules/gap-ledger.md).

## Scope

PR4 owns the deterministic state machine shared by later subsystems:

- stable typed identifiers and ordered collections;
- authoritative integer positions, with display projection kept in the desktop
  layer;
- `GameState`, `OrderBatch`, `PlayerView`, `TurnOutcome` and typed errors;
- validation before mutation and atomic turn commits;
- versioned RNG streams and canonical state serialization;
- an explicit, versioned phase dispatcher with capability labels;
- a desktop turn worker that owns the session and reports progress, one
  completed snapshot or one typed failure.

No economy, research, combat, movement, race-point arithmetic, AI-policy or
persistence rule is implemented here.

PR5 adds an optional validated race to each player slot, PR6 adds generated
planet details plus each player's starting technology, and PR7 exposes those
existing facts through `PlayerView` together with a redacted own-race summary.
`CORE_API_VERSION` is now 4; `STATE_FORMAT_VERSION` remains 3 because the
canonical authoritative state encoding is unchanged. The race contract is
documented in [`../rules/race-builder.md`](../rules/race-builder.md); the
universe contract is documented in
[`../rules/universe-generation.md`](../rules/universe-generation.md); the map and
report contract is documented in
[`map-and-reports.md`](map-and-reports.md). PR16 owns save-file persistence.

## Identifiers and arithmetic

| Item | Representation |
|---|---|
| Player, planet, fleet, design, batch and command identifiers | `u64` newtypes |
| Year and turn | `u32` |
| Ship counts | `u32` |
| Authoritative position | `i32` x, `i32` y |
| Event sequence | `u64` |
| RNG root seed and draws | `u64` |

Identifiers are allocated from serialized monotonic cursors and never from map
order, addresses or hashes. Entity collections are `BTreeMap` values keyed by
the typed identifier; views and events preserve stable key/sequence order.

Arithmetic policy in this PR:

- turn and year advances use checked addition and fail the batch before commit;
- identifier allocation fails explicitly at the integer limit;
- RNG mixing uses documented wrapping arithmetic, identical on every platform;
- no floating-point value participates in authoritative state, validation or
  RNG;
- no gameplay formula, rounding rule or gameplay constant is introduced.
  Future rules must state their rounding and overflow policy with evidence.

## Order validation and atomicity

An `OrderBatch` carries a monotonic `BatchId`, the submitting player, the
expected turn and an ordered command list. The following are rejected before
any mutation:

- a batch identifier that is not strictly newer than the last accepted batch;
- an expected turn different from the authoritative turn;
- an unknown player;
- duplicate command identifiers within the batch;
- an unknown fleet;
- a fleet owned by another player.

The session clones the state, validates the whole batch, runs the phase plan on
the candidate, validates the candidate and only then replaces the authoritative
state. Any failure therefore leaves the previous state, event log and RNG state
unchanged.

PR4 commands are administrative only: `SetReady` and `RenameFleet`. Movement,
production, research, combat, colonization and AI commands remain owned by
their later PRs rather than being accepted as silent no-ops.

## Phase plan

`PhasePlan` version 1 records its provenance (`designed`) and the exact ordered
phase list in state. Replacing the plan is supported through
`GameState::replace_phase_plan`; unrelated rules do not need to change when
evidence supplies a different order. Validation requires `OrderIntake` and a
terminal `Snapshot` phase, and requires `Snapshot` to be last, because those
implement the PR4 commit contract.

| # | Phase | PR4 capability |
|---|---|---|
| 1 | `AiPlanning` | not implemented |
| 2 | `OrderIntake` | implemented (administrative commands) |
| 3 | `Production` | not implemented |
| 4 | `Research` | not implemented |
| 5 | `Construction` | not implemented |
| 6 | `Terraforming` | not implemented |
| 7 | `PopulationGrowth` | not implemented |
| 8 | `Mining` | not implemented |
| 9 | `Minesweeping` | not implemented |
| 10 | `Movement` | not implemented |
| 11 | `WaypointTasks` | not implemented |
| 12 | `PacketTravel` | not implemented |
| 13 | `Scanning` | not implemented |
| 14 | `Combat` | not implemented |
| 15 | `Salvage` | not implemented |
| 16 | `RandomEvents` | not implemented |
| 17 | `Victory` | not implemented |
| 18 | `Snapshot` | implemented (turn/year advance and outcome) |

This order places the documented local constraint “minesweeping before
movement” correctly, but the global order is still unknown. Every skipped phase
emits `PhaseNotImplemented`; no skipped phase reads or writes gameplay values.
The worker reports each phase through progress messages in this order.

Event sequence numbers are deterministic and strictly increasing within a
game. The sequence never wraps: a state whose next event sequence has
exhausted its `u64` range is rejected as invalid. `TurnOutcome` includes the
complete post-commit state, the ordered events and the submitting player's
filtered view.

## RNG streams

`RngState` version 1 stores one explicit root seed and a `BTreeMap` of stream
keys to draw counters. A stream key is an optional stable `PlayerId` plus a
subsystem (`TurnSetup`, `AiPlanning`, `Movement`, `Combat`, `Mining`, `Research`,
`RandomEvents`). A draw mixes the seed, a stable integer selector for the key
and the stream's draw counter with SplitMix64-style wrapping arithmetic.

Drawing from one stream advances only that stream. No wall clock, thread
identifier, platform random source, standard-library hasher or hash-map
iteration participates. The algorithm is reproducible across supported
platforms, but it is not claimed to match the original game's unrecovered RNG.

## Canonical state encoding

`GameState::encode_canonical` and `decode_canonical` provide the determinism
contract used by replay and future persistence. The layout is:

```text
"STC4" | state format u16 | core contract u32 | year u32 | turn u32
       | phase plan      | RNG state          | id cursors
       | last batch       | event sequence
       | players          | designs            | planets | fleets
```

Version 3 adds each player's six starting technology levels and each planet's
environment, mineral concentration, surface minerals, population, factories,
mines, defenses, homeworld flag and starbase flag. Positions remain integers;
no floating-point value is stored. The generator and its evidence are in
[`../rules/universe-generation.md`](../rules/universe-generation.md).

All integers are little-endian. Collections are length-prefixed `u32` counts in
stable identifier order. Strings are length-prefixed UTF-8. The encoder and
decoder validate magic, format and contract versions, RNG and phase-plan
versions, tags, lengths, duplicate identifiers, trailing bytes, allocator
cursors and entity references. Rejection is explicit; malformed input is never
partially applied.

The canonical encoding is not the PR16 save-file envelope. PR16 adds a validated
JSON envelope, settings, temporary-file replacement and recovery backups around
this contract.

## Player views

`PlayerView` contains only the viewing player's identity and the visible
designs, planets, fleets and events. The PR4 fixture has three visibility states:

- `Public`: any player may see the entity/event;
- `OwnerOnly`: only the owner (or a listed event player) may see it;
- `Hidden`: no player view may contain it.

View construction fails on dangling player, design or planet references rather
than exposing partial state. PR11 replaces the synthetic entity visibility input
with scanning-derived visibility.

## Desktop worker protocol

`TurnWorkerHandle` owns the `GameSession` on its own thread. It accepts
request-identified batches and emits, in order:

1. one `Progress` message per phase;
2. exactly one `Completed` message containing a boxed `TurnOutcome`; or
3. exactly one `Failed` message with a typed `TurnWorkerError`.

A non-monotonic request identifier is rejected as `StaleRequest` before the
batch runs. A rejected batch leaves the worker's session unchanged, so the next
valid request continues from the last committed turn. `TurnSessionModel` ignores
messages that do not belong to the active request, commits snapshots atomically
and retains the last committed snapshot and view after a failure. Dropping the
handle requests shutdown and joins the thread.

## Verification

```sh
cargo test -p stars-core --test simulation_contracts
cargo test -p stars-desktop --test turn_worker
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

The core test target covers deterministic replay of serialized state and event
order, every validation rejection, batch rollback, phase progress, RNG stream
serialization, canonical decoding failures and view filtering including empty
games and dangling references. The desktop target covers real worker progress,
completed snapshots, failure retention, stale request IDs and delayed
completion handling.

## Remaining blockers

- `GAP-TURN-001`: global original cross-phase order is unknown. The delivered
  pipeline is labelled designed and is replaceable when evidence arrives.
- `GAP-RNG-001`: original RNG algorithm, seed and consumption order are
  unknown. The delivered RNG is labelled designed; exact original replay cannot
  be claimed.
- Later PRs must supply their own evidence-backed rules. Their phases are
  explicit no-ops in this build.
