---
id: story-v0-core-authority-replay
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-grant-check, story-v0-core-authority-ingest, story-v0-core-authority-spawn-tail]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Replay, composition/wiring, and module wiring

## Scope
Implement Unit 5 of `feature-v0-core-authority` (revision 2): `rebuild_from_log` + the `AuthorityComposition` layer (owns the live reactor loop + durable descendant-grant idempotency) + module wiring. Addresses review blocker #5 (reactor wiring + durable idempotency).

## Units
- `core/src/authority/replay.rs` — `rebuild_from_log`
- `core/src/authority/composition.rs` — `AuthorityComposition`
- `core/src/authority/mod.rs` — confirm module wiring + re-exports
- `core/src/lib.rs` — confirm `pub mod authority;`

## Implementation
See `feature-v0-core-authority.md` Unit 5 for exact signatures. Key points:
- `rebuild_from_log` mirrors `session::rebuild_from_log` / `elicitation::rebuild_slots_from_log`: read from LSN 0, fold via `observe`, validate LSN monotonicity + domain match.
- `AuthorityComposition` (review blocker #5) owns the wiring: folds each event into BOTH the registry (grant state) AND the spawn tail (descendant-grant issuance). When the tail produces an issuance, the composition layer writes the descendant grant via `ingest_descendant_grant` with a **deterministic grant_id** derived from `(authority_domain_id, spawn_command_id)` — this makes issuance idempotent across replay/crash (re-observed completed spawn → same grant_id → no-op duplicate). This is the durable idempotency the review demanded.
- Catch-up after crash: replay uses `rebuild_from_log` (registry) + re-runs the composition's tail over the log to catch missed issuances.
- Read `core/src/session/replay.rs` and `core/src/acceptance/elicitation.rs` FIRST.

## Acceptance Criteria
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `AuthorityComposition::observe` folds events into the registry AND issues descendant grants on completed spawns
- [ ] A crashed-then-restarted composition does NOT issue duplicate descendant grants (deterministic grant_id)
- [ ] `core/src/authority/` module compiles and is exported from `core/src/lib.rs`

## Notes
- Depends on stories 1-4.
- Add tests in `core/tests/authority_replay.rs`. The crash-recovery-no-duplicates test is key.
- The composition layer is the R3a vertical-slice wiring the review demanded (blocker #5).
