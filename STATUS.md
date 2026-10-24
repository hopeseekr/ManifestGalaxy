# Implementation status

## Current

PR7 (star map and empire management interface) is implemented on branch
`pr7-map-and-reports`, created from `pr6-universe-setup` because PR6 has not yet
been integrated on `trunk`.

PR1-PR5 are integrated on `trunk` (`a60a2f5`). PR6 is committed on
`pr6-universe-setup`; its named tests pass on this branch. PR7 builds on that
work without changing the PR6 generator.

PR7 adds:

- a scalable logical-coordinate star map with fit-to-view, cursor-centred zoom,
  drag panning, keyboard pan/zoom/fit, hit testing, selection overlays and
  viewport culling;
- linked planet and fleet selection by stable identifier shared by the map,
  reports and inspector, with selections cleared when an entity disappears;
- sortable planet and fleet reports with search, ownership filters and
  row-to-map focusing;
- planet, fleet and empire inspectors that display only `PlayerView` facts;
- managed report, inspector and map panels with minimum sizes, scrolling, no
  forced aspect ratio and a configurable text scale;
- draft administrative orders built as core `OrderBatch` values, validated by
  `GameSession::validate_orders` with core `OrderError` text;
- explicitly disabled movement, cargo and colonization actions with their
  owning PR identified;
- planet detail fields and a redacted own-race summary in `PlayerView`;
- `CORE_API_VERSION` bumped to 4; `STATE_FORMAT_VERSION` remains 3;
- a desktop integration that builds a `GameSession` and human `PlayerView` after
  a generated universe is committed;
- `cargo test -p stars-desktop --test map_navigation` (9 tests);
- `cargo test -p stars-desktop --test management_views` (8 tests);
- a documented UI contract and limitations in
  `docs/contracts/map-and-reports.md`.

Movement, cargo, colonization, economy, fuel, combat, scanning-derived
visibility, turn submission and layout persistence remain later PRs. No new
gameplay formula is introduced or claimed as recovered.

## Scope

- In scope (implemented): `crates/stars-desktop/src/ui/{map,inspector,reports,orders}.rs`,
  the minimal desktop app integration, the `PlayerView` presentation fields,
  `crates/stars-desktop/tests/map_navigation.rs`,
  `crates/stars-desktop/tests/management_views.rs`, UI/contract documentation,
  this status file and the PR7 checklist entry in `PLAN.md`.
- Out of scope: new simulation rules, hidden-state projections, scanning,
  movement, cargo, colonization, economy, fuel, combat, AI policy, persistence
  of layouts, extracted original artwork, network or browser targets, and
  original-game execution.

## Checklist

- [x] Planets, fleets and selection overlays painted from logical coordinates
- [x] Cursor-centred zoom, pan, fit-to-view and bounded zoom
- [x] Hit testing accurate after zoom, pan and logical viewport changes
- [x] Offscreen planets and fleets culled before painting
- [x] Planet and fleet inspectors from `PlayerView`
- [x] Sortable planet and fleet reports
- [x] Search and ownership filtering
- [x] Linked selection by stable identifier
- [x] Selection retained through sorting/filtering and cleared when hidden
- [x] Hidden planets, fleets and designs omitted by core views
- [x] Resizable panels with minimum sizes and scrolling
- [x] Keyboard map navigation and configurable text scale
- [x] Narrow and wide window layouts without a forced aspect ratio
- [x] Draft `OrderBatch` editing through the command contract
- [x] Core validation errors displayed for invalid drafts
- [x] Unavailable fleet actions disabled with the owning PR named
- [x] All 10 primary and 14 lesser traits exposed through core views
- [x] Alternate Reality planet facts pass through without UI calculations
- [x] `cargo test -p stars-desktop --test map_navigation`
- [x] `cargo test -p stars-desktop --test management_views`
- [x] PLAN.md PR7 checkbox and desktop vertical-slice milestone
- [ ] Mixed-DPI monitor movement and native window smoke checks (hardware not
      available in this environment)
- [ ] Turn submission and movement orders remain PR10 work

## Validation

Commands run locally:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p stars-desktop --test map_navigation
cargo test -p stars-desktop --test management_views
cargo test --workspace --all-targets
```

Results:

- The named PR7 targets pass: 9 map tests and 8 management tests.
- The full workspace test set passes after the view contract change.
- `cargo fmt --check`, `cargo check --workspace --all-targets` and
  `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo run -p stars-desktop` started the native process and it remained
  running until an 8-second timeout. No interactive visual inspection or
  mixed-DPI movement was performed, so this is not claimed as a platform smoke
  check.
- No original game was executed. Windows and macOS builds were not executed in
  this PR.

## Remaining blockers and limitations

- `GAP-VM-001` and `GAP-VERSION-001`: no original observation exists, and the
  January 1998 help's applicability remains unresolved.
- `GAP-VIEW-DETAIL-001`: visibility remains the PR4 synthetic fixture until
  PR11. A visible planet exposes all PR6 planet facts; there is no partial
  knowledge.
- `GAP-UI-LAYOUT-001`: panel sizes, filters and selections are not persisted;
  PR16 owns durable settings.
- `GAP-UNIVERSE-NAMES-001`: reports display synthetic `Star NNNN` names.
- `GAP-UNIVERSE-START-001`: starting ship designs and starbase components remain
  PR9/PR12 work, so fleet lists are empty for a fresh PR6 universe.
- `GAP-TURIN-001`: Turindrones ship designs and AI policy remain PR14 work.
- `GAP-RNG-001`: the universe stream remains a documented remake RNG.
