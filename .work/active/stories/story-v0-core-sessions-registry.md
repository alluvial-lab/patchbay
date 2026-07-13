---
id: story-v0-core-sessions-registry
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-state-machine]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Session delta events and the SessionRegistry projection

## Scope

Implement Unit 2 of `feature-v0-core-sessions`: the durable delta event shape and the in-memory projection that folds them. Mirrors acceptance's `CommandTransition` + `CommandIndex`.

## Units

- `contracts/proto/patchbay/sessions.proto` — add `SessionStateEvent` message + mutation sub-messages (NEW proto; regenerate contracts)
- `core/src/session/events.rs` — Rust helpers for constructing/encoding `SessionStateEvent` deltas
- `core/src/session/registry.rs` — `SessionRegistry` projection, `SessionRecord`, `SessionTombstone`

## Implementation

See `feature-v0-core-sessions.md` Unit 2 for exact signatures and the full proto shape. Key points:

- Add `SessionStateEvent` to `sessions.proto` with a `oneof mutation`: `SessionRegistered`, `SessionGenerationBumped`, `SessionConnectivityChanged`, `SessionActivityChanged`, `SessionRelabeled`. Regenerate `contracts/rust` and `contracts/ts`. The `StoredEventKind::SessionState = 7` discriminator already exists.
- `SessionRegistry` mirrors `ElicitationSlotLayer` structurally: `HashMap`-backed projection with `observe(&mut self, event) -> Result<(), SessionError>`. Sessions only consumes `SessionState` events (ignores others, like elicitation ignores `SessionState`).
- `SessionLiveKey` = identity minus generation (one live gen per `runtime_session_id`). A generation bump replaces the live entry and adds a tombstone.
- Tombstones retained indefinitely (never evicted in v0.1.0). Keyed by `(runtime_session_id, generation)`.
- `observe` validates state-axis transitions via `allowed_connectivity_transition` / `allowed_activity_transition` (from story 1) and returns `CorruptLog` on violation (Fail Fast).
- First-write-wins on duplicate `registered` events (idempotent replay).
- `resolve` and `get_session` / `get_tombstone` / `is_tombstoned` / `get_live_session` are read methods used by the resolver (story 4) and the writer (story 3).

## Acceptance Criteria

- [ ] `SessionStateEvent` proto added; `contracts/rust` and `contracts/ts` regenerated; bindings compile
- [ ] `SessionRegistry::observe` folds each mutation kind correctly
- [ ] A generation bump tombstones the prior generation and inserts the new live generation
- [ ] Tombstones are retained and queryable by `(runtime_session_id, generation)`
- [ ] `observe` rejects disallowed connectivity/activity transitions as `CorruptLog`
- [ ] `observe` is idempotent for re-delivered events
- [ ] `resolve` returns `None` for a tombstoned generation (stale target)

## Notes

- Depends on story 1 (state machine + identity + error type).
- The proto regeneration is part of this story — the registry cannot compile without the generated `SessionStateEvent` bindings.
- Do NOT implement the writer (ingest) or replay here — only the event shape and the projection fold.
