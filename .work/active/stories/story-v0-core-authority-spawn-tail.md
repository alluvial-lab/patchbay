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

# Story: Descendant-grant-on-spawn log-tail reactor (vertical slice)

## Scope
Implement Unit 4 of `feature-v0-core-authority` (revision 2): a pure log-tail that reacts to spawn completion by producing a descendant-grant issuance. Vertical slice (R3a): correlates `Spawn → Completed` with a `SessionRegistered` carrying `spawn_origin`. Addresses review blockers #4 (spawn-result identity) and #5 (reactor shape).

## Units
- `core/src/authority/spawn_tail.rs` — `SpawnDescendantTail`, `DescendantGrantIssuance`

## Implementation
See `feature-v0-core-authority.md` Unit 4 for exact signatures. Key points:
- **Depends on `story-sessions-spawn-origin-field`** (the `SessionRegistered.spawn_origin` field) — sequenced first.
- The reactor correlates THREE events: (1) a Spawn `OPERATION` event (track command_id + spawner + spawning_grant_id), (2) its `COMMAND_TRANSITION` to `Completed`, (3) a `SessionRegistered` carrying `spawn_origin = that command_id`. The `SessionRegistered` provides the spawned session identity (adapter/deployment/runtime/generation) — review blocker #4.
- `issued: HashSet<CommandId>` = in-memory idempotency for the live tail. Durable idempotency = deterministic grant_id derived from `(authority_domain_id, spawn_command_id)` in the composition layer (story 5).
- The tail writes NOTHING — it produces an `Issuance` that the composition layer (story 5) feeds to `ingest_descendant_grant`.
- `spawning_grant_id`: from the spawn's `Authorized.grant_id` IF acceptance retains it (`story-acceptance-issuer-context`). If not retained, `None` — document.
- Read `core/src/acceptance/elicitation.rs` FIRST — `ElicitationSlotLayer` is the direct template (read-only log consumer reacting to command transitions).

## Acceptance Criteria
- [ ] A Spawn OPERATION + Completed transition + SessionRegistered(spawn_origin=that command) produces exactly one `DescendantGrantIssuance`
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] A SessionRegistered without `spawn_origin` does NOT trigger an issuance
- [ ] Replay (re-observing events) does not produce duplicate issuances (idempotent via `issued`)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's `spawned_session_scope` comes from the `SessionRegistered` event (not the spawn Operation's fleet target)

## Notes
- Depends on stories 1 (registry), 3 (ingest, for the writer the composition layer calls), AND the sessions prerequisite `story-sessions-spawn-origin-origin-field`.
- Add tests in `core/tests/authority_spawn_tail.rs`.
- The composition layer (wiring the tail's output to the writer) is story 5 — this story is the reactor only.
