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
Implement Unit 5 of `feature-v0-core-authority` (revision 3): `rebuild_from_log` + module wiring. **No composition layer** (rev2 finding E dropped — no live consumer loop in v0.1.0).

## Units
- `core/src/authority/replay.rs` — `rebuild_from_log`
- `core/src/authority/mod.rs` — confirm module wiring + re-exports
- `core/src/lib.rs` — confirm `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 5. `rebuild_from_log<S>(storage, authority_domain_id)` mirrors `session::rebuild_from_log` / `elicitation::rebuild_slots_from_log`: read from LSN 0 (snapshot discriminator gap — deferred), fold via `observe`, validate LSN monotonicity + domain match.

**No `AuthorityComposition`** (rev2 finding E dropped). The registry is rebuilt via `rebuild_from_log`; the spawn-tail is a separate fold exercised in tests. A live composition layer is follow-on when the ingress exists.

Read `core/src/session/replay.rs` and `core/src/acceptance/elicitation.rs` (`rebuild_slots_from_log`) FIRST.

## Acceptance Criteria
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`

## Notes
- Depends on stories 1 (registry), 2 (GrantCheck), 3 (ingest).
- Add tests in `core/tests/authority_replay.rs`.
