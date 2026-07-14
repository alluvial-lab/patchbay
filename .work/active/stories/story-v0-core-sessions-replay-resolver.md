---
id: story-v0-core-sessions-replay-resolver
kind: story
stage: review
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

## Implementation notes

- `core/src/session/replay.rs` — `rebuild_from_log` mirrors `rebuild_slots_from_log` (elicitation): read from LSN 0, fold via `observe`, validate LSN monotonicity + domain match. Local `event_identity` helper (duplicated from elicitation; small, keeps the module self-contained).
- `core/src/session/resolver.rs` — `impl TargetResolver for SessionRegistry`. Validation depth = existence + tombstone-only (Q3): tombstoned generation → `TargetNotFound` (stale target); unknown session → `TargetNotFound`; generation neither live nor tombstoned → `TargetNotFound`. Connectivity is NOT checked (offline/failed sessions resolve — they're delivery concerns, not identity).
- `core/src/session/mod.rs` — added `pub mod replay; pub mod resolver;` + `rebuild_from_log` re-export.
- Method-name collision: `SessionRegistry` has both an inherent `resolve(target_scope) -> Option<TargetBinding>` (from story 2) and the `TargetResolver::resolve` trait method (Result-returning). The trait method is fully qualified in tests as `TargetResolver::resolve(&registry, ...)`.
- `core/tests/sessions_replay_resolver.rs` — 8 integration tests covering: replay determinism (live gen 2 after bump, gen 1 tombstoned), live session binds, live-generation binds when unspecified, tombstoned generation rejected, unknown generation rejected, unknown session rejected, offline session resolves (Q3), failed session resolves (Q3).
- Verification: `cargo build`, `cargo test -p patchbay-core` (139 tests, all pass), `cargo clippy --all-targets` (clean).
- Note: the wave-4 subagent was interrupted mid-work (wrote replay.rs/resolver.rs + mod.rs, built clean, but did not write tests or commit). Orchestrator finished the story inline: wrote the test file, verified, advanced to review.
