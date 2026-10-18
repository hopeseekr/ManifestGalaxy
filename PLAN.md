# Stars! remake — master implementation plan

## Product and delivery

Build a desktop-only, offline, turn-based Stars! reimplementation in Rust with egui/eframe for Windows, macOS, and Linux. The single human player uses the complete custom race builder; Turindrones is the sole built-in computer opponent type, with multiple independent opponents supported. Retain every trait exposed by the builder and its gameplay effects. Multiplayer, including hotseat and play-by-email, is excluded. PR6 covers the complete recovered game-creation wizard, explicitly including galaxy size, density and star clumping/distribution.

Modernize scaling, layout, and interaction without silently changing game rules. Support arbitrary window proportions, crisp HiDPI text/graphics, responsive background turn processing, and our own versioned local saves. Original save compatibility, other predefined races, network multiplayer, and browser/PWA delivery are excluded. A complete in-memory game precedes persistence; original AI parity is an investigation target, not a promise.

This package is a set of execution prompts, not an already reverse-engineered specification. Execute one PR prompt at a time. Do not implement all PRs merely because this plan is present.

## Current evidence and uncertainty

- Input: `stars26i.exe`, 3,216,832 bytes, a segmented 16-bit Windows NE executable for Windows 3.1; it is not a native DOS game.
- SHA-256: `5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e`.
- PR1 reproduced the NE header at file offset `0x400`, 35 segments, 122 resources, 36 dialog templates, the selected `Turindrones` string at segment 35 relative `0xa06` (file `0x55846`), and the resident/module/import tables. A name, caption or string is a lead, not proof of runtime behaviour or race configuration.
- `Stars! - RIP/stars!.hlp` is a WinHelp 3.1 container titled “Stars! Player's Guide (Jan 10, 1998)”. PR1 decoded its title, January 1998 generation date, 142 internal files and 419 topics. Check its applicability to this executable; neither the game's 1995 origin nor a filename establishes version-specific rules.
- PR1 also produced a private local PDF rendering of the help (`analysis/help/Stars-Player-Guide-1998.pdf`). It is an ignored convenience artifact, not committed evidence and not a source of gameplay rules. See “Local help PDF” below.
- No point formula, full catalog, turn ordering, combat rule, RNG algorithm, or AI policy has yet been recovered and verified. No original game was executed during the initial inspection or PR1.
- The workspace is a Git repository. The integration branch is `trunk`; PR1 is merged on `trunk`. PR2 is complete on `pr2-rules-specification` and is not merged by this plan.

## Architecture and interfaces

Use a Cargo workspace with these package paths:

| Package | Responsibility | Allowed internal dependencies |
|---|---|---|
| `crates/stars-core` | State, race validation, rules, orders, views, events, deterministic turn execution | None |
| `crates/stars-ai` | Turindrones planning and serializable AI memory | stars-core |
| `crates/stars-desktop` | egui/eframe UI, worker ownership, local I/O, packaging entry point | stars-core, stars-ai |
| `crates/stars-tools` | Headless scenarios, evidence conversion, comparisons and benchmarks | stars-core, stars-ai when running games |

PR1–PR2 use Python standard-library scripts in `tools/analysis` before the Rust workspace exists. The PR1 inspection/reporting path stays standard-library-only; the optional private PDF renderer uses Pillow and ReportLab. Keep these inspection tools separate from gameplay. Shared Rust rule calculations belong only in core.

Use Rust 2024 and a pinned stable toolchain selected and verified in PR3. Pin dependency resolution with Cargo.lock. Use egui/eframe with wgpu, procedural original graphics, and properly licensed bundled fonts. Check current official dependency documentation during implementation rather than freezing unverified versions here.

Core contracts are typed `GameState`, `OrderBatch`, `PlayerView`, `TurnOutcome`, validation errors, stable entity IDs, and a versioned explicit RNG state. AI consumes player-visible data and its own memory, not unrestricted host state. UI renders projections and submits commands; costs, predictions and validation come from core.

A desktop worker owns turn execution. It accepts request-ID-tagged ordered command batches and returns progress, filtered events, a completed snapshot, or a typed error. Apply a result once and atomically; retain the last committed state on failure and ignore stale responses. UI animation and worker timing must not alter rules or RNG consumption.

Keep simulation independent of OS I/O, wall time, threads and rendering. Use explicit arithmetic and stable processing order; recover gameplay rounding rather than inheriting convenient language defaults. Native persistence in PR16 uses a validated versioned JSON envelope, separate settings, temporary-file replacement and recovery backups.

## Evidence workflow and gates

1. Inventory input hashes, bounded NE parsing, resources/imports, help provenance and available disassembly tools.
2. Create a rule ledger and controlled reference scenarios with stable IDs, source locations, setup, inputs, observed outputs, confidence and unresolved questions.
3. Decode relevant help and trace code/resources. Compare version-specific claims rather than treating all sources as equivalent.
4. Run controlled experiments in the original only when a suitable environment is available. Otherwise write precise operator steps and report pending observations. Do not claim VM access or invent its RNG seed.
5. Link verified rules to regression fixtures. Keep observed, statically derived, documented-only, inferred and unknown statuses distinct.
6. At each feature gate, resolve required unknown mechanics or report that PR incomplete. PR2 itself may complete with a gap ledger, so foundation work can proceed. A later blocked feature blocks sequential delivery; independent research can continue.

AI configuration and AI decision policy are separate investigations. PR14 may deliver a functional designed policy with a precise difference/fidelity report where original policy cannot be recovered. This exception does not permit invented race-point, economy or combat formulas to pass as verified.

Decoded assets/help and raw extraction output stay private in ignored `analysis/`. Commit tools, synthetic fixtures and concise factual evidence, not original game payloads. Never inspect or expose license-code contents.

### Local help PDF

PR1 includes `tools/analysis/hlp_to_pdf.py`, a private, local-only converter for
the supplied help file. It is not a gameplay dependency and is not a release
artifact.

```sh
python3 tools/analysis/hlp_to_pdf.py 'Stars! - RIP/stars!.hlp' \
  --output analysis/help/Stars-Player-Guide-1998.pdf \
  --image-dir analysis/help/images
```

The current source produces a 200-page A4 PDF with 419 topics and 128 decoded
DIB bitmap images. The PDF, extracted PNGs, text dump and inventory all remain
under ignored `analysis/` and must not be committed or redistributed. The help
text and images are copyright Mare Crisium, LLC; the converter only supports a
private format-shift of a user-supplied copy.

Implementation facts future agents must preserve:

- `tools/analysis/help.py` emits `[[IMAGE |bmN align=left|center|right]]`
  markers in decoded topic text for external pictures. `hlp_to_pdf.py` replaces
  those markers with extracted images. If the marker format or text renderer
  changes, update both the converter and the focused tests.
- External pictures are WinHelp MRB/SHG records in `|bm0`–`|bm132`; DIB
  pictures use the WinHelp RunLen/LZ77 packings and a synthesized BMP wrapper.
  Five records (`|bm2`, `|bm82`, `|bm83`, `|bm87`, `|bm108`) are Windows
  metafiles and are reported as unavailable rather than guessed.
- Help body text is best-effort: six topics use strings-only fallback for 18
  display/table records, and table cell geometry is approximate. All 419 topic
  titles and all 128 supported DIBs decode.
- The PDF is a convenience view. Cite the HLP source, TOPICPOS, or absolute HLP
  byte offset for evidence; PDF page numbers are layout-dependent and are not
  stable citations.
- Help images and text are reference evidence, not assets for the remake.
  Gameplay graphics remain original procedural work under the product rules.
- The converter uses Pillow and ReportLab; the core Rust packages and the
  PR1 inspection/reporting path remain independent of those optional Python
  packages.

## Custom-race trait coverage

The local bundled help index strings confirm these 24 names. Mechanics, point values, exclusions and exact interactions remain unverified against `stars26i.exe` (SHA-256 `5770dade2db0e11918631d5c128c0bf7ab48e371f7b2267a4ff02531bb8e198e`). The matrix is the canonical inventory for evidence and implementation. All 24 traits are available to a human custom race; recovered validation permits exactly one primary trait (PRT) and any valid lesser-trait (LRT) combination. Primary traits are not excluded by a predefined-race path because this product has no selectable predefined race presets.

The `interactions / boundary cases to verify` column records evidence questions, not assumed rules or numeric formulas. `PR2` owns evidence and controlled scenarios, `PR5` owns typed builder IDs, validation and point interactions, and `PR6`–`PR13` own the runtime effect when the affected subsystem is implemented.

### Primary racial traits (10)

| ID | Name | Affected PRs | Runtime verification responsibility | Interactions / boundary cases to verify |
|---|---|---|---|---|
| `PRT.HE` | Hyper-Expansion | PR2, PR5, PR6, PR8, PR9, PR10, PR13 | Verify growth and population capacity, plus special hull/engine availability and gate restriction. | Check starting setup, growth/capacity boundaries, hull and engine catalogs, and gate validation. |
| `PRT.SS` | Super-Stealth | PR2, PR5, PR6, PR9, PR10, PR11, PR12, PR13 | Verify cloaking, scanner interaction, theft/espionage, research and minefield speed effects. | Check cloaked visibility, scanner penetration, espionage/research events, and minefield travel order. |
| `PRT.WM` | War Monger | PR2, PR5, PR6, PR9, PR10, PR12, PR13 | Verify weapon costs, battle speed, invasion, design knowledge, mines and defenses. | Check weapon catalog costs, battle order, invasion resolution, design knowledge, mine and defense interactions. |
| `PRT.CA` | Claim Adjuster | PR2, PR5, PR6, PR8, PR9, PR13 | Verify automatic temporary terraforming, orbital adjustment and retro bombs. | Check setup and colony boundaries, temporary versus lasting changes, orbital adjustments and bomb effects. |
| `PRT.IS` | Inner-Strength | PR2, PR5, PR6, PR9, PR10, PR12, PR13 | Verify fleet reproduction, healing, defenses, weapon costs and bombing restrictions. | Check fleet phase ordering, healing/reproduction limits, defense and weapon catalogs, and bomb validation. |
| `PRT.SD` | Space Demolition | PR2, PR5, PR6, PR9, PR10, PR11, PR12, PR13 | Verify mine scanning, detonation, decay and travel effects. | Check minefield visibility, detonation order, decay timing, travel resolution and scanner interactions. |
| `PRT.PP` | Packet Physics | PR2, PR5, PR6, PR8, PR9, PR10, PR11, PR12, PR13 | Verify packets, scanning, terraforming and mass-driver effects. | Check packet setup, scanner visibility, terraforming boundaries, mass-driver travel and impact order. |
| `PRT.IT` | Interstellar Traveler | PR2, PR5, PR6, PR9, PR10, PR11, PR12, PR13 | Verify gate cargo and range limits, gate scans and packet interactions. | Check setup, gate capacity/range boundaries, scan visibility, and packet penalties without assuming rates. |
| `PRT.AR` | Alternate Reality | PR2, PR5, PR6, PR8, PR9, PR10, PR11, PR13 | Verify orbital housing, energy-dependent resources, natural mining and scans, own-world remote mining, planetary-installation restriction and colonist travel loss. | Check setup and orbital capacity, energy/resource availability, mining ownership, scanner views, installation validation and colonist travel. |
| `PRT.JOAT` | Jack of All Trades | PR2, PR5, PR6, PR8, PR9, PR11, PR13 | Verify starting technology and ships, inherent scanners and population capacity. | Check setup inventory, starting catalog legality, scanner behavior and population-capacity boundaries. |

### Lesser racial traits (14)

| ID | Name | Affected PRs | Runtime verification responsibility | Interactions / boundary cases to verify |
|---|---|---|---|---|
| `LRT.IFE` | Improved Fuel Efficiency | PR2, PR5, PR6, PR9, PR10, PR13 | Verify engine access, fuel use and starting technology. | Check engine catalog, starting fleet/technology, fuel accounting, range and movement order. |
| `LRT.TT` | Total Terraforming | PR2, PR5, PR6, PR8, PR13 | Verify starting capability, research requirements, improvement limits and resource costs for terraforming. | Check setup capability, research gate, environmental bounds, immunity and production integration. |
| `LRT.ARM` | Advanced Remote Mining | PR2, PR5, PR6, PR8, PR9, PR13 | Verify miner hull and equipment availability, starting fleet and remote-mining behavior. | Check starting catalog, ownership, output, range, timing and resource accounting. |
| `LRT.ISB` | Improved Starbases | PR2, PR5, PR6, PR9, PR12, PR13 | Verify starbase build cost, hull and dock access, capacity, damage and combat effects. | Check construction catalog, dock/hull legality, capacity, damage and combat ordering. |
| `LRT.GR` | Generalized Research | PR2, PR5, PR6, PR9, PR13 | Verify research distribution, costs, progression and technology availability. | Check starting research state, field allocation, technology catalog and boundary costs. |
| `LRT.UR` | Ultimate Recycling | PR2, PR5, PR6, PR9, PR10, PR13 | Verify scrapped-ship recovery, minerals or resources, location, starbase conditions, timing and caps. | Check scrapping order, design/component catalog, location and starbase conditions, and conservation. |
| `LRT.MA` | Mineral Alchemy | PR2, PR5, PR6, PR8, PR13 | Verify production conversion, queue behavior and resource availability. | Check starting economy, queue ordering, resource accounting and production limits. |
| `LRT.NRSE` | No Ramscoop Engines | PR2, PR5, PR6, PR9, PR10, PR13 | Verify engine catalog restrictions, fuel use, design validation and movement. | Check starting engines, catalog exclusion, fuel accounting and movement boundaries. |
| `LRT.CE` | Cheap Engines | PR2, PR5, PR6, PR9, PR10, PR13 | Verify engine cost, construction cost, movement and stalling behavior. | Check starting designs, engine catalog costs, construction accounting and stall/order handling. |
| `LRT.OBRM` | Only Basic Remote Mining | PR2, PR5, PR6, PR8, PR9, PR13 | Verify mining equipment restrictions and population bonus. | Check starting population, miner equipment catalog, remote-mining ownership and output boundaries. |
| `LRT.NAS` | No Advanced Scanners | PR2, PR5, PR6, PR9, PR11, PR13 | Verify advanced-scanner catalog exclusion, penetration and range, plus trait interactions. | Check starting/catalog validation, scanner penetration/range, stealth and other trait combinations. |
| `LRT.LSP` | Low Starting Population | PR2, PR5, PR6, PR8, PR13 | Verify starting population, colony setup, growth and economy. | Check setup population bounds, colony initialization, growth and production accounting. |
| `LRT.BET` | Bleeding Edge Technology | PR2, PR5, PR6, PR9, PR13 | Verify technology age, component cost and miniaturization. | Check starting technology, component/hull catalog, age boundaries, costs and design legality. |
| `LRT.RS` | Regenerating Shields | PR2, PR5, PR6, PR9, PR12, PR13 | Verify shield component access, damage, regeneration timing, combat phase ordering and repair limits. | Check starting/design catalog, shield damage and regeneration order, repair limits and combat boundaries. |

PR13 is the final coverage gate: every row must have evidence, typed selection and point behavior, a runtime implementation or an explicit completion blocker. Its focused regression targets are `cargo test -p stars-core --test primary_traits` and `cargo test -p stars-core --test lesser_traits`.

## Ordered PR checklist

The prerequisite for each PR after PR1 is its predecessor's authorized integration, plus the evidence gates in that prompt. The table describes the principal deliverable; each linked file contains exact branch name, branch merge commit message, GitHub PR title, Git commit message, owned paths, tasks and acceptance checks.

| Done | PR prompt | Principal deliverable |
|---|---|---|
| [x] | [PR1: binary evidence](PLAN-pr1-description.md) | Established the binary evidence inventory and extraction tools. |
| [x] | [PR2: rules specification](PLAN-pr2-description.md) | Documented recovered rules and reference scenarios. |
| [ ] | [PR3: rust foundation](PLAN-pr3-description.md) | Added the Rust workspace and desktop application foundation. |
| [ ] | [PR4: simulation contracts](PLAN-pr4-description.md) | Added deterministic game state and turn-processing contracts. |
| [ ] | [PR5: custom race builder](PLAN-pr5-description.md) | Implemented the custom race builder and point calculations. |
| [ ] | [PR6: universe setup](PLAN-pr6-description.md) | Implemented universe generation and new-game setup. |
| [ ] | [PR7: map and reports](PLAN-pr7-description.md) | Added the star map and empire management interface. |
| [ ] | [PR8: planet economy](PLAN-pr8-description.md) | Implemented planetary population and economic production. |
| [ ] | [PR9: research and designs](PLAN-pr9-description.md) | Implemented research and ship design. |
| [ ] | [PR10: fleet orders](PLAN-pr10-description.md) | Implemented fleet movement and logistics orders. |
| [ ] | [PR11: scanning and intelligence](PLAN-pr11-description.md) | Implemented scanning and player-specific intelligence. |
| [ ] | [PR12: combat and conquest](PLAN-pr12-description.md) | Implemented combat and planetary conquest. |
| [ ] | [PR13: special mechanics](PLAN-pr13-description.md) | Implemented special technologies and racial abilities. |
| [ ] | [PR14: turindrones ai](PLAN-pr14-description.md) | Implemented the Turindrones computer opponent. |
| [ ] | [PR15: complete game loop](PLAN-pr15-description.md) | Integrated complete games and victory conditions. |
| [ ] | [PR16: save and recovery](PLAN-pr16-description.md) | Added versioned saves and crash recovery. |
| [ ] | [PR17: fidelity and performance](PLAN-pr17-description.md) | Verified gameplay fidelity and improved large-game performance. |
| [ ] | [PR18: desktop releases](PLAN-pr18-description.md) | Added verified desktop release packages. |

## Milestones

- [ ] Research readiness — PR1–PR2: reproducible inventory, rule/trait catalog, scenario format and explicit gaps.
- [ ] Desktop vertical slice — PR3–PR7: deterministic contracts, validated builder, generated universe and usable map. PR7 may use labeled fixtures for later mechanics.
- [ ] Complete playable session — PR8–PR15: economy, designs, fleets, intelligence, combat, all trait effects, Turindrones and victory. No persistence claim yet.
- [ ] Durable and assessed game — PR16–PR17: versioned saves/recovery, continuation equivalence, measured performance and fidelity report.
- [ ] Desktop delivery — PR18: actual offline smoke checks and packaged artifacts for supported targets.

## Branch and agent workflow

Read AGENTS.md and the selected PR prompt before work. Discover the integration branch from real repository metadata; never assume `main` or `trunk`. Start the exact named branch after the predecessor has been merged by an authorized actor. Executing a PR prompt authorizes routine branch creation and scoped signed local commits, not pushing, publishing, merging or initializing Git. Metadata specifies messages for an eventual authorized merge; it is not an instruction to perform one.

The same full past-tense sentence is used for each PR's GitHub title, branch merge commit message and Git commit message. Keep unrelated changes intact. If the workspace or signing setup prevents a commit, retain completed work and report the limitation.

## Validation and completion

Every PR names focused test targets to create and run, plus relevant structural checks. Add regression selectors for directly affected existing tests; do not replace meaningful test coverage with a single broad build. Research tools must reject malformed/truncated binary inputs. Core must pass deterministic replay, boundary arithmetic, trait and visibility tests. UI checks include resizable layouts, keyboard operation, 100/150/200% scaling and mixed-DPI movement where hardware permits.

Complete-game fixtures must cover one and multiple opponents, elimination/victory and worker failures. Persistence must survive invalid inputs, failed writes and interrupted updates, and preserve future turn results after reload. Compare observed original behavior with honest confidence; uncertain RNG requires invariant/statistical checks rather than fake exact fixtures.

Research tooling checks include `python3 -m unittest discover -s
tools/analysis/tests -v`. The local PDF converter is covered by synthetic MRB,
DIB, RunLen and marker tests; generating a PDF is a private manual check, not a
release gate. Do not report a generated PDF as a platform smoke test, and do not
commit its copyrighted content.

Record benchmark seed/settings/hardware and release-mode measurements. CPU utilization alone is not success. Native CI builds are necessary but do not replace actual offline platform smoke checks. Finish with an evidence coverage report, known differences, remaining limitations and platform results. Never mark an unchecked milestone complete.
