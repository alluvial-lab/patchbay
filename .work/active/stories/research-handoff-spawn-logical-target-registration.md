---
id: research-handoff-spawn-logical-target-registration
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Stable logical-target registration

## Checkpoint

Introduce the durable identity an operator controls across runtime replacement. A logical target is not a process, Pi session file, cwd, project, or label. It is authority-domain scoped, bound to one adapter/deployment in v1, and points to exactly one current runtime generation plus retained prior-generation facts.

Fresh spawned targets start at generation `1`; zero remains the Protobuf/unspecified sentinel and is invalid. For a fresh spawn, the core derives the typed logical-target id from the creation command identity and persists the prepared claim in the accepted Operation. The adapter must echo that claim in its correlated session report; the target becomes registered only after the report is durable.

## Design

**Files**
- `contracts/proto/patchbay/common.proto` — `LogicalTargetId` and `RuntimeGenerationRef` wrappers; add logical target to runtime-session `TargetScope`.
- `contracts/proto/patchbay/operations.proto` — `SpawnGenerationClaim` carried by `AcceptedOperation`/`SubmissionResult`.
- `contracts/proto/patchbay/sessions.proto` — logical target, creating/current spawn operation, continuation reference/status fields on report/event/snapshot records.
- `core/src/session/logical_target.rs` (new) — primary target record/key/index validation.
- `core/src/session/registry.rs` — make the logical target the stable live slot and keep an exact runtime-generation reverse index for routing and stale correlation.
- `core/src/session/ingest.rs` — correlate fresh registration with the exact accepted spawn claim before append.
- `core/src/target.rs` — resolve runtime targets by the complete logical + runtime-generation identity.
- `web-cockpit/src/domain/model.ts`, `web-cockpit/src/ui/session-list.ts`, and `cli/src/commands/sessions.ts` — carry/display logical target before intent; labels remain metadata.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalTargetRecord {
    pub logical_target_id: LogicalTargetId,
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub current: RuntimeGenerationRef,
    pub origin_spawn_operation_id: Option<CommandId>,
    pub current_spawn_operation_id: Option<CommandId>,
    pub continuation_of: Option<RuntimeGenerationRef>,
    pub continuation_status: ContinuationStatus,
    pub last_authoritative_lsn: u64,
}

impl SessionRegistry {
    pub fn get_logical_target(
        &self,
        id: &LogicalTargetId,
    ) -> Option<&LogicalTargetRecord>;

    pub fn resolve_runtime_generation(
        &self,
        reference: &RuntimeGenerationRef,
    ) -> RuntimeGenerationDisposition<'_>;
}
```

The v1 logical target cannot migrate between adapters or deployment scopes. Cross-adapter target migration is a reserved seam requiring an explicit protocol ceremony; it must not happen by metadata edits.

## Acceptance evidence

- [ ] Fresh spawn success creates one logical target with generation `1`, exact spawn provenance, and an exact current runtime reference.
- [ ] Generation `0`, empty ids, uncorrelated reports, cross-adapter/deployment claims, and duplicate logical-target registration reject before mutation.
- [ ] Exact durable replay reconstructs the same registry and reverse index; conflicting reuse of an event id or logical id fails closed.
- [ ] Pre-provisioned/discovered sessions use an explicit adapter-reported logical target registration path and cannot collide with a spawned target.
- [ ] Project/cwd/name/model updates do not change logical identity or generation.
- [ ] Session snapshots and both operator surfaces show logical target plus current runtime generation before an Operation is submitted.

## Ordering constraint

Depends on the canonical adapter-scoped spawn acceptance path. Every later lifecycle checkpoint consumes this registry.
