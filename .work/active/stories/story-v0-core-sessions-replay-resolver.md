---
id: story-v0-core-sessions-replay-resolver
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-state-machine, story-v0-core-sessions-registry, story-v0-core-sessions-ingest]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Story: Replay, TargetResolver impl, and module wiring

## Scope

Implement Unit 4 of `feature-v0-core-sessions`: rebuild the registry from the log, implement the `TargetResolver` port on the registry, and wire the session module into the crate.

## Units

- `core/src/session/replay.rs` — `rebuild_from_log`
- `core/src/session/resolver.rs` — `impl TargetResolver for SessionRegistry`
- `core/src/session/mod.rs` — module wiring (re-exports)
- `core/src/lib.rs` — export `pub mod session`

## Implementation

See `feature-v0-core-sessions.md` Unit 4 for exact signatures. Key points:

- `rebuild_from_log<S>(storage, authority_domain_id)` is a near-exact copy of `rebuild_slots_from_log` (elicitation) and `rebuild_from_log` (acceptance): read from LSN 0, fold via `observe`, validate LSN monotonicity and domain match. Replays from LSN 0 because the snapshot slot has no projection discriminator (Q2=defer).
- `impl TargetResolver for SessionRegistry` — implements the port declared in `core/src/acceptance/ports.rs`. Validation depth = existence + tombstone-only (Q3=c):
  - Tombstoned generation → `TargetNotFound` (stale target; wrong generation = wrong target).
  - Offline/failed connectivity → ALLOWED (connectivity is a delivery concern; the operator may queue commands for offline sessions).
  - Requested generation must match the live generation (else stale). Unspecified generation → bind the live generation.
- The `SessionRegistry` implements `TargetResolver` directly (it holds the state; the port is a read interface), mirroring how `CommandIndex` implements `CommandStateLookup`.

## Acceptance Criteria

- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry that observed the same events
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `TargetResolver::resolve` returns `Ok(TargetBinding)` for a live session
- [ ] `TargetResolver::resolve` returns `TargetNotFound` for a tombstoned generation
- [ ] `TargetResolver::resolve` returns `TargetNotFound` for an unknown session
- [ ] `TargetResolver::resolve` returns `Ok` for an offline/failed session (connectivity not checked)
- [ ] `TargetResolver::resolve` binds the live generation when `session_generation` is unspecified
- [ ] `core/src/session/` module compiles and is exported from `core/src/lib.rs`
- [ ] The existing `TestTargetResolver` in `core/tests/acceptance_pipeline.rs` can be replaced by a real `SessionRegistry` in an integration test

## Notes

- Depends on stories 1, 2, and 3 (needs the full registry, events, and writer).
- Read `core/src/acceptance/ports.rs` for the exact `TargetResolver` / `TargetBinding` / `TargetNotFound` shapes — they already exist; this story implements them.
- Read `core/src/acceptance/elicitation.rs` `rebuild_slots_from_log` for the replay template.
