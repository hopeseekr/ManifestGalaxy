# PR7 map and reports contract

Status: implemented for the player-view presentation layer. No new simulation
rule, point formula, economic calculation or hidden-state projection is
introduced.

## Scope

PR7 owns:

- `crates/stars-desktop/src/ui/map.rs` — projection, cursor-centred zoom, pan,
  fit, hit testing, selection and painting;
- `crates/stars-desktop/src/ui/inspector.rs` — linked planet, fleet and empire
  details from the player view;
- `crates/stars-desktop/src/ui/reports.rs` — sortable, searchable planet and
  fleet reports with stable-identifier selection;
- `crates/stars-desktop/src/ui/orders.rs` — draft `OrderBatch` editing and
  core-supplied validation errors;
- the desktop integration that builds a `GameSession` and `PlayerView` after a
  generated universe is committed.

PR7 does not own movement, cargo, colonization, fuel, production, research,
combat, scanning effects, persistence or original artwork. Those actions are
shown disabled with their owning PR named.

## Player-view boundary

The map, reports, inspector and draft editor render only a
[`PlayerView`](../../crates/stars-core/src/view.rs). Hidden planets, fleets,
designs and events never enter these widgets. PR11 replaces the PR4 visibility
fixture with scanning-derived visibility in core; the desktop must not add its
own redaction or reveal rules.

PR7 extends the view with facts that already existed in authoritative state:

- planet environment, mineral concentration, surface minerals, population,
  factories, mines, defenses, homeworld and starbase flags;
- a redacted summary of the viewer's own race: name, plural name, primary trait,
  lesser traits and starting technology.

The race summary excludes the builder password and all editing-only fields.
`CORE_API_VERSION` is 4; `STATE_FORMAT_VERSION` remains 3 because the canonical
authoritative state encoding is unchanged.

Draft orders use the existing `OrderBatch` contract. `GameSession::validate_orders`
checks the same batch rules as `commit` without mutating state, consuming RNG or
reserving a batch identifier. A valid draft is not a committed turn; submission
remains later work.

## Map projection

Map state stores a world-space centre, a zoom in logical screen points per
authoritative world unit, and a viewport. Authoritative positions remain integer
`GalacticPosition` values in core and are never written back from the map.

- Fit-to-view preserves aspect ratio and uses fixed logical padding.
- Zoom is cursor-centred: the world point under the cursor stays fixed.
- Zoom is clamped to `0.01..=60.0` world units per point. Zero, negative and
  non-finite factors are ignored.
- Pan changes the world centre only.
- Hit testing uses projected logical coordinates, so window resizing, DPI
  movement and egui text zoom do not require an authoritative coordinate
  conversion.
- Planets and positioned fleets are culled to the viewport plus a small margin
  before painting.
- Fleets at unknown orbits have no projected position and cannot be hit-tested.

Selection is a stable typed planet or fleet identifier. It is retained through
sorting and filtering while the entity remains in the view, and is cleared when
a turn removes or hides it. Clicking a report row centres the map on the same
selected identifier.

## Interface behavior

The following are intentional modern interface behaviors, not claims about the
original game:

- resizable report, inspector and map panels with minimum usable sizes and no
  forced window aspect ratio;
- scrolling report tables and inspector content;
- pointer-centred zoom, drag panning, `F` to fit, arrow-key panning, `+`/`-`
  zoom and `Esc` to clear;
- configurable UI scale from the menu or management inspector;
- search, ownership filters and column sorting;
- linked selection between map, report and inspector;
- a synthetic preview map remains available only when no game exists, and is
  labelled as synthetic foundation data.

## Trait coverage

The management view exposes all 10 primary and 14 lesser traits through the
core race summary; the desktop does not evaluate traits. Planet facts for every
trait, including Alternate Reality worlds without ordinary planetary
installations, are copied from the core player view. PR7 tests assert that each
primary trait and every lesser trait reaches the view unchanged. Runtime trait
mechanics remain with their owning PRs in the
[trait coverage matrix](../../PLAN.md#custom-race-trait-coverage).

## Validation

Focused targets:

```sh
cargo test -p stars-desktop --test map_navigation
cargo test -p stars-desktop --test management_views
```

`map_navigation` covers fit, cursor-centred zoom, pan, high-DPI logical
viewports, orbit hit targets, culling, stable selection and invalid zoom input.
`management_views` covers sorting, search, owner filters, hidden-entity
redaction, disappearing selections, order-draft validation errors, all 24
traits through the view, panel rendering at 1280x720 and 1920x1080 with
100/150/200% scale, and desktop integration after universe generation.

## Limitations

- No original game was executed. This PR recovers no new numerical rule and
  makes no fidelity claim about the original map or report layout.
- Mixed-DPI monitor movement and native window smoke checks require hardware
  and were not executed here.
- Scanning remains the PR4 visibility fixture until PR11. A planet that is
  visible exposes all PR6 planet facts; there is no partial-knowledge model yet.
- Draft orders validate but do not submit or advance a turn. PR10 adds movement
  and logistics; PR8 adds economy commands.
- Panel sizes and filters are not persisted. PR16 owns durable settings.
- Graphics are procedural original work; no extracted game artwork is used.
- `GAP-UNIVERSE-NAMES-001` still gives synthetic planet names; report search and
  sorting use those visible names.
