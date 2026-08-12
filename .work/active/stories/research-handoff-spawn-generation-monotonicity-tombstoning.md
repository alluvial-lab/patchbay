---
id: research-handoff-spawn-generation-monotonicity-tombstoning
kind: story
stage: implementing
tags: [protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Generation monotonicity and atomic tombstoning

## Checkpoint

Make replacement a single durable transition: the exact current runtime generation is tombstoned and the next claimed generation becomes current in the same log event/fold. Restart may change `runtime_session_id`; logical-target identity is the stable slot. There is never a state where both generations are live, and reconnect/crash cannot allocate a generation.

Initial spawned generation is `1`. A managed continuation must advance by exactly one (`N → N+1`) and match a durable accepted claim. Equal reports are source-order updates/no-ops; lower reports are stale; an unclaimed greater report for a spawn-managed target is rejected. Adapter-discovered targets retain an explicit authenticated replacement path, but the core still never invents their generation.

## Design

**Files**
- `contracts/proto/patchbay/sessions.proto` — replace the same-runtime-id-only bump shape with `LogicalTargetGenerationAdvanced { logical_target_id, from, to, spawn_operation_id, continuation_status, ... }`.
- `core/src/session/events.rs` and `core/src/session/ingest.rs` — derive one event from the accepted claim and authenticated report.
- `core/src/session/registry.rs` and `core/src/session/logical_target.rs` — validate exact pre-state, install tombstone + next current record atomically, retain reverse-index history.
- `core/src/acceptance/index.rs` — identify nonterminal commands bound to the retired runtime generation.
- `server/src/adapter_service.rs` — under `CoreDecisionGate`, append the generation event and required old-generation command/Elicitation effects as one audited decision batch before publishing the new live generation.
- `specs/seed/session_generation.qnt` — model attempted claims/reports independently and promote genuine monotonic/exclusivity/stale properties through the v1 assurance lane.
- `core/tests/sessions_registry.rs`, `core/tests/sessions_ingest.rs`, and `core/tests/sessions_proptest.rs` — traces with changing runtime ids, gaps, replay, and failed pre-state.

```rust
pub struct GenerationAdvance {
    pub logical_target_id: LogicalTargetId,
    pub from: RuntimeGenerationRef,
    pub to: RuntimeGenerationRef,
    pub spawn_operation_id: CommandId,
    pub continuation_status: ContinuationStatus,
}

pub fn plan_generation_advance(
    registry: &SessionRegistry,
    claims: &SpawnClaimRegistry,
    report: &SessionReport,
) -> Result<GenerationAdvance, SessionError>;
```

When the old generation is retired, accepted-but-undelivered commands become `superseded`; delivered/running commands become `failed(execution_outcome_unknown)`; pending Elicitations bound to it become `stale`. Those effects are durable and ordered with the replacement so delivery cannot leak across generations.

## Acceptance evidence

- [ ] Fresh managed registration accepts only generation `1`; managed continuation accepts only exact `N → N+1` from its claim.
- [ ] Runtime-session id may change while logical-target id remains stable.
- [ ] Tombstone and next-current installation are atomic and replay-identical; no observer under the decision gate sees two live generations.
- [ ] Equal/lower/unclaimed-greater reports do not advance the live generation and emit bounded stale/mismatch evidence.
- [ ] Old accepted commands are superseded, old delivered/running commands fail with outcome ambiguity, and old Elicitations become stale before new-generation control is exposed.
- [ ] Overflow, missing pre-state, duplicate transition, and conflicting replay leave the complete projection unchanged.
- [ ] The formal property and executable vector fail when strict advance, exact pre-state, or atomic tombstone installation is mutated away.

## Ordering constraint

Depends on stable logical-target registration. Stale-event ingress and continuation claiming rely on this transition.
