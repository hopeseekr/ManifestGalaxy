# Stars! remake — project agent instructions

# BEGIN subagents_configs
@/home/tsmith/.codex/SUBAGENT_ROUTING.md
# END subagents_configs

Read [PLAN.md](PLAN.md) and the active PR prompt before making changes. These instructions replace the unrelated RimWorld project configuration.

## Scope

- Build a Rust + egui/eframe desktop game for Windows, macOS and Linux.
- Support one human custom race and one or more Turindrones AI opponents. Keep the entire custom builder and all its exposed trait effects.
- Explicitly preserve all 10 primary and 14 lesser traits in the [custom-race trait coverage matrix](PLAN.md#custom-race-trait-coverage), including disadvantages. Primary racial traits are selectable custom-race capabilities, not the excluded predefined race presets. Require one primary trait and an evidence-validated combination of lesser traits; do not remove traits to simplify implementation.
- Exclude other preset races, original save compatibility, network multiplayer and browser/PWA targets.
- Modernize HiDPI scaling and layout; do not silently rebalance gameplay.
- Work only on the requested PR or documentation task. The plan is not authorization to implement all stages, publish, push or merge.

## Ownership and architecture

Use packages `stars-core`, `stars-ai`, `stars-desktop`, and `stars-tools` under `crates/`. Core depends on none of the other packages. AI depends on core; desktop depends on core/AI; tools may use core/AI for headless scenarios. PR1–PR2 Python inspection tools live separately in `tools/analysis/`.

Core owns authoritative arithmetic, validation, predictions, state transitions, views and events. Never duplicate a gameplay calculation in egui. AI receives only its authorized player view and memory. Any observed AI advantage must be an explicit documented capability, not unrestricted state access.

Turns run on a desktop-owned worker using request IDs and ordered batches. It reports progress, events, a completed snapshot or a typed error. Apply results atomically, ignore stale responses and retain the last good state on failure. No game rules may depend on frame rate, wall clock, thread scheduling, hash iteration or UI animation.

Keep entity IDs stable, arithmetic explicit, RNG algorithm/state versioned and processing order reproducible. Serialization must preserve deterministic continuation. Core has no filesystem, network or platform dependency.

## Reverse-engineering evidence

Preserve supplied binary/help/archive files byte-for-byte. Never inspect, log or commit license-code contents. Keep raw extracted resources, decoded help, disassembly and private observations in ignored analysis storage. Commit reproducible tools, synthetic fixtures and factual evidence references.

A resource name or string is a lead, not proof of runtime behavior. Cite executable hash and precise file/code location or decoded help topic. Distinguish observed, statically derived, documented-only, inferred and unknown claims. Check whether the 1998 help applies to the supplied binary.

Unknown required formulas are research tasks or blockers. Do not silently substitute invented rules or drop difficult traits. PR2 may record gaps; a dependent feature cannot claim complete fidelity while its required rules are unresolved. PR14 specifically permits disclosed designed AI policy differences; that exception does not extend to numerical gameplay rules.

Do not claim to have run the original without execution evidence. If VM access is absent, prepare controlled operator scenarios. Preserve uncertainty about original RNG and version differences.

## Implementation and verification

Use a pinned stable Rust toolchain, Cargo.lock, egui/eframe with wgpu, typed errors and explicit interfaces. Prefer original procedural graphics and licensed fonts over copied assets. Do not introduce browser or network infrastructure.

Each PR owns the paths and contracts stated in its prompt. Preserve others' work and explain any necessary shared contract change. Follow the subagent routing instructions where available; assign non-overlapping ownership and give validators exact test selectors.

Run rustfmt, compilation, clippy and the affected test manifest appropriate to the PR. The named test targets in prompts must be created in that PR. Research stages use their Python tests before Cargo exists. Do not write meaningless tests that merely repeat implementation constants; tie expectations to evidence or independently reasoned invariants.

Keep UI scaling and worker-response checks separate from core rules tests. Report commands actually run, failures and unavailable platform checks. Never report an unexecuted platform smoke check as passing.

## Git working agreement

Create the exact branch in the active prompt from the discovered integration branch after its predecessor has been merged by an authorized actor. This routine branch creation and scoped local commit are authorized by implementation of the prompt. Do not ask for repeat permission. A merge-message field does not authorize a merge.

### Commits and signing

- **Always commit your changes when you finish a task.** Do not leave unrelated work uncommitted in the tree once the task is complete.
- **Write commit messages as full, past-tense sentences.** Use the exact sentence supplied in the prompt when there is one; otherwise use a concise subject and, when helpful, a body explaining why.
- **Use pinentry for the GPG passphrase.** Commits are signed with the configured key; never disable signing or expose passphrases.
- If gpg fails with `cannot open '/dev/tty'`, re-run the commit with `git -c gpg.program=/tmp/opencode/gpg-ask commit ...`, which adds `--pinentry-mode ask` so pinentry can prompt.
- If Git/signing remains unavailable, keep the files and report the limitation instead of inventing a commit or initializing a repository.

Never stage unrelated changes, secrets, original game files, generated binary assets or build outputs. Do not push, publish, or merge unless explicitly requested. Rebase only when appropriate and authorized; back up before an authorized history rewrite and preserve authorship. Do not carry over obsolete branch names or rewrite recipes from other projects.

## Language and generated files

Use plain language. Never use “latch” as a verb; use stores, records, marks or remains set. Do not edit CHANGELOG.md unless requested; it is maintained externally. Update only completed items in PLAN.md, and report unresolved blockers honestly.
