---
id: story-v0-core-authority-spawn-tail
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-ingest, story-sessions-spawn-origin-field]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Descendant-grant-on-spawn log-tail reactor (order-independent, tested via replay)

## Scope
Implement Unit 4 of `feature-v0-core-authority` (revision 3): a pure fold that produces a descendant-grant issuance on observing a completed spawn + correlated `SessionRegistered.spawn_origin`. **Order-independent** (rev2 finding D). Exercised via replay/direct observe in tests — NO live consumer loop (rev2 finding E dropped).

## Units
- `core/src/authority/spawn_tail.rs` — `SpawnDescendantTail`, `DescendantGrantIssuance`

## Implementation
See `feature-v0-core-authority.md` Unit 4 for exact signatures. Key points:
- **Depends on `story-sessions-spawn-origin-field`** (the `SessionRegistered.spawn_origin` field).
- **Order-independent** (rev2 finding D): three separate maps — `spawn_ops` (command_id → SpawnOpInfo), `completed` (HashSet), `registrations` (spawn_origin command_id → RegistrationInfo). After ANY insertion, call `try_issue(command_id)`: issue iff spawn_ops.has(cid) && completed.has(cid) && registrations.has(cid) && !issued.has(cid). This handles all 6 arrival orders (e.g. SessionRegistered before Completed).
- `issued` = in-memory idempotency for the fold. Durable idempotency for tests = deterministic grant_id from `(authority_domain_id, spawn_command_id)`; the test harness calls `ingest_descendant_grant` with that ID (re-observe → same ID → no-op).
- The issuance's `spawned_session_scope` comes from the `SessionRegistered` event (NOT the spawn op's fleet target).
- `spawning_grant_id`: from the spawn op's authorization if available; may be `None` in v0.1.0 (documented — full durable acceptance-metadata is follow-on).
- NO live consumer loop, NO composition layer (rev2 finding E dropped). Pure fold; tests feed events directly.
- Read `core/src/acceptance/elicitation.rs` FIRST — `ElicitationSlotLayer` is the direct template (read-only log consumer).

## Acceptance Criteria
- [ ] A Spawn OPERATION + Completed transition + SessionRegistered(spawn_origin=that command) produces exactly one `DescendantGrantIssuance`, **regardless of arrival order** (test all 6 permutations)
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] A SessionRegistered without `spawn_origin` does NOT trigger an issuance
- [ ] Replay (re-observing events) does not produce duplicate issuances (idempotent via `issued` + deterministic grant_id)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's `spawned_session_scope` comes from the `SessionRegistered` event

## Notes
- Depends on stories 1 (registry), 3 (ingest), AND the sessions prerequisite `story-sessions-spawn-origin-field`.
- Add tests in `core/tests/authority_spawn_tail.rs`. The 6-permutation order test is key (rev2 finding D).
- No composition layer — this story is the reactor only.
