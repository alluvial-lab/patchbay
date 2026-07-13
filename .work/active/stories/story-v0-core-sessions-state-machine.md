---
id: story-v0-core-sessions-state-machine
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-sessions
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Session identity, state axes, and transition validation

## Scope

Implement Unit 1 of `feature-v0-core-sessions`: the single source of truth for session identity, the connectivity×activity state axes, and the allowed transition tables. This is the SSOT for allowed session-state transitions, derived from `docs/PROTOCOL.md` "Session state axes", not invented.

## Units

- `core/src/session/mod.rs` — module root, error enum, re-exports
- `core/src/session/state.rs` — `SessionIdentity`, `allowed_connectivity_transition`, `allowed_activity_transition`, `effective_connectivity`

## Implementation

See `feature-v0-core-sessions.md` Unit 1 for exact signatures. Key points:

- `SessionIdentity` is a Rust newtype over the four identity fields (adapter_id, deployment_scope, runtime_session_id, session_generation). NOT the full `Session` proto — identity is the tuple alone. Enforces "labels cannot override identity" at the type level.
- The transition adjacency tables are copied verbatim from `docs/PROTOCOL.md` "Session state axes". They are the SSOT.
- `Unspecified` is the initial state for both axes (pre-observation). The protocol's connectivity table starts from `unknown`; treat `Unspecified → Unknown` as the implicit first step, then apply the protocol table.
- `effective_connectivity` encodes "stale/unknown dominates" as a pure function.
- `SessionError` enum: `CorruptRecord`, `CorruptLog`, `InvalidTransition`, `StaleGeneration`, `Storage(#[from])`. Mirrors `StorageError`/`AcceptanceError`.

## Acceptance Criteria

- [x] `allowed_connectivity_transition` matches the protocol table exactly (exhaustive table test)
- [x] `allowed_activity_transition` matches the protocol table exactly (exhaustive table test)
- [x] `SessionIdentity` equality ignores project/cwd/name
- [x] `effective_connectivity` returns `Stale`/`Unknown` when connectivity is stale/unknown, regardless of activity
- [x] `Unspecified` is the only initial state for both axes
- [x] `core/src/session/mod.rs` and `core/src/session/state.rs` compile; module is exported from `core/src/lib.rs`

## Notes

- No deps. This is the foundation the registry, ingest, and replay stories build on.
- The `SessionError` enum lives in `mod.rs` so all session submodules can use it.
- Do NOT implement the registry or ingest here — only the state machine + identity + error type.

## Implementation notes

- Added the generated-contract-backed `SessionIdentity` tuple and canonical connectivity/activity adjacency functions in `core/src/session/state.rs`. Both tables are `const fn` single sources of truth and include only the specified pre-observation transitions from `Unspecified` plus the protocol adjacency.
- Added exhaustive table-cell unit tests for both axes, identity-field isolation, and effective-connectivity behavior across activity values.
- Added `SessionError` and the session module re-exports in `core/src/session/mod.rs`, then exported `session` from the crate root. No registry, ingestion, replay, or resolver modules were forward-declared.
- Mechanical implementation detail: generated prost `SessionState` exposes raw `i32` fields and no typed accessor in the checked-in binding. `effective_connectivity` converts with `SessionConnectivityState::try_from` and uses prost's `Unspecified` fallback for an unrecognized value; protocol-boundary validation remains responsible for rejecting such values.
- Mechanical deviation from the sketched error attribute: generated `Generation` implements `Debug` but not `Display`, so `StaleGeneration` formats `live` and `reported` with `:?` to keep the typed generated fields and compile cleanly.
- Verification: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core` passed; `CARGO_HOME=/tmp/cargo-home cargo test -p patchbay-core` passed with no failures.
