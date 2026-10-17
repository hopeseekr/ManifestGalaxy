# PR7: Added the star map and empire management interface.

- Branch name: `pr7-map-and-reports`
- Branch merge commit message: `Added the star map and empire management interface.`
- GitHub PR title: `Added the star map and empire management interface.`
- Git commit message: `Added the star map and empire management interface.`

## Objective and prerequisites

Read [PLAN.md](PLAN.md), [AGENTS.md](AGENTS.md), and [PR6](PLAN-pr6-description.md). Start only after its authorized integration.

Create a usable scalable map and management UI over player-visible state.

PR6 generated worlds and PR4 view contracts exist. Use clearly labeled fixtures for mechanics not implemented yet.

## Owned paths and contracts

Own crates/stars-desktop/src/ui/{map,inspector,reports,orders}.rs and UI-model tests. Extend core view DTOs only where presentation needs an existing fact.

Preserve concurrent and unrelated work. Restrict changes to this responsibility and the minimum shared interfaces needed for integration; explain any contract change.

## Implementation tasks

1. Render planets, fleets and selection overlays with egui painting in logical coordinates. Implement cursor-centered zoom, pan, selection and fit-to-view independently of simulation coordinates.

2. Add planet/fleet inspectors, sortable reports, search, and linked selection. Use stable IDs rather than row indices; retain selection through sorting and handle entities that disappear after a turn.

3. Create resizable panels with minimum usable dimensions, scrolling, keyboard navigation and configurable text scale. Provide narrow-window behavior without a forced aspect ratio.

4. Introduce draft-order editing through the command interface with core-supplied validation errors. Disable unavailable actions explicitly; do not implement economy, fuel or combat calculations in widgets.

5. Render only PlayerView information. Put future mechanic demonstrations in a labeled fixture/demo mode that cannot be mistaken for a functioning gameplay feature.

## Acceptance scenarios and validation

- Map hit testing remains accurate after zoom, pan and DPI changes; offscreen objects do not cause unbounded rendering work.
- Sorting and filtering preserve correct selected IDs; unknown information is consistently hidden.
- Test 1280×720 and 1920×1080 windows, 100/150/200% scaling, keyboard navigation and mixed-DPI monitor movement where hardware is available.
- Verify player-view and UI information for each applicable custom race trait, including unusual habitation from Alternate Reality, through core-supplied views without putting trait calculations in the UI; track coverage in the [trait coverage matrix](PLAN.md#custom-race-trait-coverage).

Implement and execute these checks:

```sh
cargo test -p stars-desktop --test map_navigation
cargo test -p stars-desktop --test management_views
cargo run -p stars-desktop
```

Also run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`. Create the named integration test targets in this PR. Run all newly added and directly affected test selectors; list additional selectors in the handoff rather than substituting a global test run.

## Evidence gate and exclusions

No new simulation rules, omniscient UI projections, persistent layouts yet, or extracted original artwork requirement.

For behavior claimed to reproduce the original, link rule IDs to source evidence and fixtures. Missing required evidence is an investigation task or a reported blocker, not permission to fabricate a formula. Document intentional modern interface behavior separately from original gameplay. PR2 may finish with a gap ledger; a dependent feature cannot claim verified completion with required rules still unknown. The disclosed AI-policy exception is described in PR14.

## Deliverables and completion

Deliver the scoped implementation or research artifacts, focused tests, updated evidence/limitations, and a completion report with commands actually run and remaining blockers. Update the corresponding master-plan checklist only when acceptance passes. Use the exact Git metadata above for scoped signed commits. Discover the actual integration branch; do not assume its name, initialize a repository, push, or merge without authorization. If Git is unavailable, retain the files and report that limitation.
