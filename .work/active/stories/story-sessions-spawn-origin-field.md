---
id: story-sessions-spawn-origin-field
kind: story
stage: implementing
tags: [protocol, foundation]
parent: feature-v0-core-sessions
depends_on: []
release_binding: null
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
