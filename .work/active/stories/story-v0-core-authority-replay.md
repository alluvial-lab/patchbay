---
id: story-v0-core-authority-replay
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-grant-check, story-v0-core-authority-ingest]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Replay and module wiring

## Scope
Implement Unit 5 of `feature-v0-core-authority`: `rebuild_from_log` (rebuild the registry from the log) + wire the authority module into the crate.

## Units
- `core/src/authority/replay.rs` — `rebuild_from_log`
- `core/src/authority/mod.rs` — confirm module wiring + re-exports
- `core/src/lib.rs` — confirm `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 5. `rebuild_from_log<S>(storage, authority_domain_id)` is a near-exact copy of `session::rebuild_from_log` / `elicitation::rebuild_slots_from_log`: read from LSN 0 (snapshot discriminator gap — deferred), fold via `observe`, validate LSN monotonicity + domain match.

Read `core/src/session/replay.rs` and `core/src/acceptance/elicitation.rs` (`rebuild_slots_from_log`) FIRST — they're the direct templates.

## Acceptance Criteria
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry that observed the same events
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`
- [ ] The existing `TestGrantCheck` in `core/tests/acceptance_pipeline.rs` can be replaced by a real `AuthorityRegistry` in an integration test

## Notes
- Depends on stories 1 (registry), 2 (GrantCheck), 3 (ingest).
- Add tests in `core/tests/authority_replay.rs`.
