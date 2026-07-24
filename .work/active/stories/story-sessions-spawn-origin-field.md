---
id: story-sessions-spawn-origin-field
kind: story
stage: done
tags: [protocol, foundation]
parent: feature-v0-core-sessions
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Add SessionRegistered.spawn_origin field (authority prerequisite)

## Scope
Add an optional `spawn_origin` field to the `SessionRegistered` proto message so the authority descendant-grant reactor can correlate a spawned session back to the spawn Operation that created it. This is a **prerequisite for `feature-v0-core-authority`** (the descendant-grant reactor, `story-v0-core-authority-spawn-tail`, depends on it).

## Why
The authority feature's descendant-grant reactor tails the log for completed spawns and must identify WHICH session a spawn produced. Today, `SessionRegistered` carries no correlation to the spawn command — so the reactor can't link them. The fix is one additive proto field: `spawn_origin: TypedCorrelation` referencing the spawn `CommandId`.

## The change
- `contracts/proto/patchbay/sessions.proto` — add to `SessionRegistered`:
  ```proto
  // Optional: the spawn Operation that created this session. Set when the
  // adapter reports a session resulting from a spawn. Lets the authority
  // descendant-grant reactor correlate the session to its spawn command.
  TypedCorrelation spawn_origin = 9;
  ```
- Regenerate contracts (Rust via `cargo build -p patchbay-contracts`; TS via `buf generate` from `contracts/`; then `git checkout contracts/rust/src/gen` + `cargo build` to restore committed Rust format — same regen steps as the sessions feature).
- Update `core/src/session/events.rs` and `ingest.rs` to carry `spawn_origin` through the `SessionRegistered` construction (optional; None when the session wasn't spawn-originated).
- Update `SessionReport` if needed so an adapter can report the spawn origin.

## Acceptance Criteria
- [ ] `SessionRegistered` proto has `spawn_origin: TypedCorrelation` (field 9, optional)
- [ ] Contracts regenerated (Rust + TS); gen diff is additions-only
- [ ] `ingest_session_report` carries `spawn_origin` through (None when not spawn-originated)
- [ ] Existing sessions tests still pass; `cargo build`, `cargo test -p patchbay-core`, `cargo clippy --all-targets` clean

## Notes
- This is a sessions-feature story (sessions owns its proto shape) but exists to unblock authority. Filed under `feature-v0-core-sessions` (re-opens its review surface per the substrate rule — re-review the parent when this lands).
- `CARGO_HOME=/tmp/cargo-home` for all cargo commands. `buf` at `$HOME/.npm-global/bin/buf`.
- Demanded by authority design review blocker #4 (spawn-tail can't derive the spawned session).

## Implementation notes
- Added `SessionRegistered.spawn_origin` as field 9 in the Protobuf source of truth and regenerated both Rust and TypeScript contracts. Regeneration followed the required sequence: `buf generate`, restore `contracts/rust/src/gen`, then `cargo build -p patchbay-contracts` so prost-build emitted the committed Rust format.
- Added `SessionReport.spawn_origin: Option<TypedCorrelation>` and carried it into the durable `SessionRegistered` mutation. Existing non-spawn report and registration fixtures explicitly use `None`.
- Full `buf generate` also repaired pre-existing TypeScript drift for the already-present `CommandTransition` message and `StoredEventKind::CommandTransition`; these generated changes are source-derived and additive rather than formatting churn.
- Files changed: `contracts/proto/patchbay/sessions.proto`, generated Rust/TypeScript contracts, `core/src/session/ingest.rs`, and session test fixtures.
- Tests added: none, per the story's test-integrity guidance; existing session tests remain the regression guard.
- Verification: `cargo build -p patchbay-contracts`, `cargo build -p patchbay-core`, `cargo test -p patchbay-core`, and `cargo clippy --all-targets -- -D warnings` pass with `CARGO_HOME=/tmp/cargo-home`. `buf lint` (from `contracts/proto`) and the TypeScript contract build also pass.
- Discrepancies from design: two direct `SessionRegistered` test fixtures also required `spawn_origin: None` after regeneration, in addition to the four named `SessionReport` fixtures.
- Adjacent issues parked: none.

## Review (fast lane, 2026-07-14)
Verdict: Approve - story verified by implement; fast-lane advance. Independently confirmed green: 171 tests across the full `patchbay-core` suite, clippy clean. Additive proto field + carry-through correct; regen workflow sound. Re-opens `feature-v0-core-sessions` review surface (spawn_origin child landed).
