---
id: story-fix-sessions-ingest-correctness
kind: story
stage: implementing
tags: [protocol, bug, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-ingest, story-v0-core-sessions-registry]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Fix sessions ingest correctness (3 blockers from feature review)

## Scope

Three correctness blockers found in the feature-level deep review of `feature-v0-core-sessions`. All three touch `core/src/session/ingest.rs`, the `SessionGenerationBumped` proto, and `core/src/session/registry.rs`. Fix them together — they share the proto + ingest surface.

## Blockers

### B1 — Ingestion writes events that replay cannot rebuild (empty identity fields)

`ingest_session_report` validates only `authority_domain_id` (non-empty) before writing a `SessionRegistered` event. It does NOT validate `adapter_id`, `deployment_scope`, or `runtime_session_id` for emptiness. But `registry.rs::mutation_identity` (the replay path) rejects those as `CorruptRecord`. **Result: the writer can durably append an event that `rebuild_from_log` will reject — an unreplayable log.**

**Location**: `core/src/session/ingest.rs` `validate_authority_domain` (~line 260) + the first-registration branch (~line 116-142).

**Fix**: Extend validation to cover ALL identity fields before any `storage.append`. Add a `validate_report(report)` that rejects empty `adapter_id`, `deployment_scope`, `runtime_session_id` (mirroring `mutation_identity` in registry.rs). Assert storage is unchanged on rejection.

### B2 — Generation bump discards the new generation's reported state and metadata

When `report.session_generation > current`, the writer emits a `SessionGenerationBumped` event carrying only `from_generation`/`to_generation` and returns `GenerationBumped` — it discards the report's `connectivity`, `activity`, `project`, `cwd`, `name`. The projection (`observe_generation_bumped`) clones the prior record's state and metadata into the new generation. **Result: a session replacement (e.g. Pi's `session_new`) reports the new generation's state, but it is silently lost — the new generation inherits the old generation's stale connectivity/activity/metadata.** This violates the protocol's "the new generation becomes the live target" with the adapter's reported state.

**Location**: `contracts/proto/patchbay/sessions.proto` `SessionGenerationBumped` (only `from_generation`/`to_generation`); `core/src/session/ingest.rs` generation-bump branch (~line 150-170); `core/src/session/registry.rs::observe_generation_bumped` (`let mut next = current.clone()` ~line 326).

**Fix**: Extend the `SessionGenerationBumped` proto to carry the new generation's `initial_state` (SessionState) and metadata (project/cwd/name) — mirroring `SessionRegistered`. Regenerate contracts. Update `ingest_session_report` to populate them from the report. Update `observe_generation_bumped` to apply them to the new record instead of cloning the old state. This is a contract change: regenerate Rust (cargo build) + TS (buf generate) per the established regen steps.

**Design note**: this is not a 50/50 — inheriting a replaced generation's stale state defeats the purpose of session replacement. The adapter reports the new generation's current state; the event must carry it. (If a future adapter reports a bump with no state, `initial_state` can be optional and default to Unknown/Unspecified.)

### B3 — A multi-field report is truncated (only the first changed delta is persisted)

In the equal-generation branch, `ingest_session_report` checks connectivity → activity → metadata in sequence, and each changed branch does `storage.append` + `return Ok(...)`. If a single report changes multiple fields (e.g. connectivity AND activity AND labels), only the first changed delta is persisted; the rest are silently dropped. **Result: a report describing a multi-axis change is only partially recorded.**

**Location**: `core/src/session/ingest.rs` equal-generation branch (~line 173-251), early returns at lines 197/224/248.

**Fix**: Process all changed deltas in one ingestion (either append all deltas in a defined order, or loop until the report is fully represented). Define partial-failure semantics if a later append fails after an earlier one succeeded. Test a report that changes connectivity + activity + metadata simultaneously.

## Acceptance Criteria

- [ ] B1: a report with empty adapter_id/deployment_scope/runtime_session_id is rejected before any `storage.append`; `rebuild_from_log` succeeds after any accepted report
- [ ] B2: a generation bump where the report carries new state/metadata results in the new generation holding the REPORTED state/metadata, not the prior generation's
- [ ] B2: `SessionGenerationBumped` proto carries `initial_state` + project/cwd/name; contracts regenerated (Rust + TS)
- [ ] B3: a single report changing connectivity + activity + metadata persists ALL changes (replay reconstructs all three)
- [ ] Existing tests still pass; new tests cover each blocker's reproduction
- [ ] `cargo build`, `cargo test -p patchbay-core`, `cargo clippy --all-targets` clean

## Notes

- These are correctness bugs found at feature review, not new features. The 5 original sessions stories stay at `stage: review`; this story is a new child that must land before the feature can advance to `done`.
- B2 involves a proto change — follow the regen steps: edit `sessions.proto` → `cargo build -p patchbay-contracts` (Rust) → `buf generate` from `contracts/` (TS) → `git checkout contracts/rust/src/gen` → `cargo build` to restore committed Rust format. `CARGO_HOME=/tmp/cargo-home` for all cargo commands.
- Found by deep review Phase 1 (completeness) + Phase 2 (adversarial), cross-model (openai-codex/gpt-5.6-sol).
