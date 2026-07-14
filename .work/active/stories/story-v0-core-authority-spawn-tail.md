---
id: story-v0-core-authority-spawn-tail
kind: story
stage: implementing
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-ingest]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Descendant-grant-on-spawn log-tail (the reactor)

## Scope
Implement Unit 4 of `feature-v0-core-authority`: a pure log-tail that reacts to spawn completion by producing a descendant-grant issuance. Exactly the elicitation-slot pattern (tail the log, react to command transitions).

## Units
- `core/src/authority/spawn_tail.rs` — `SpawnDescendantTail`, `DescendantGrantIssuance`

## Implementation
See `feature-v0-core-authority.md` Unit 4 for exact signatures. The tail reads `OPERATION` + `COMMAND_TRANSITION` events acceptance wrote, reacts to `Spawn → Completed`.

Key points:
- `spawn_commands` tracks which command_ids are spawn operations (from `OPERATION` events where `OperationKind::Spawn`).
- `issued` tracks command_ids that already produced a descendant grant (idempotent on replay — first-Completed-wins, mirroring elicitation's first-answer-wins).
- On `COMMAND_TRANSITION` to `Completed` for a tracked spawn not yet issued: return `Some(DescendantGrantIssuance)`. Non-Completed terminals produce NO issuance.
- The issuance carries: `spawn_operation_id`, `spawning_grant_id` (`None` for operator-authorized spawns in v0.1.0 — implicit authority has no grant_id), `target_scope` (the spawned session), `subject_actor_id` (the spawner/operator), `allowed_operation_kinds` (`DESCENDANT_GRANT_ALLOWED_KINDS`).
- The tail writes NOTHING — it produces an `Issuance` that the composition layer feeds to `ingest_descendant_grant` (story 3's writer).
- Read `core/src/acceptance/elicitation.rs` FIRST — `ElicitationSlotLayer` is the direct template (a read-only log consumer that reacts to command transitions).

## Acceptance Criteria
- [ ] A spawn OPERATION + COMMAND_TRANSITION to Completed produces exactly one `DescendantGrantIssuance`
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] Replay (re-observing the same events) does not produce duplicate issuances (idempotent)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's provenance links the spawn_operation_id; `spawning_grant_id` is `None` for operator-authorized spawns

## Notes
- Depends on stories 1 (registry, `DESCENDANT_GRANT_ALLOWED_KINDS`) and 3 (the writer that consumes the issuance).
- Add tests in `core/tests/authority_spawn_tail.rs`.
- The composition layer (wiring the tail's output to the writer) is NOT in this story's scope — it's integration work for the replay/wiring story or a follow-on.
