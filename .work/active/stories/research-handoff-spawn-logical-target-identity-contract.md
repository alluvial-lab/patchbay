---
id: research-handoff-spawn-logical-target-identity-contract
kind: story
stage: implementing
tags: [protocol, verification]
parent: research-handoff-spawn
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Logical-target and external-runtime identity contract

## Checkpoint

Define the shared identity/projection leaf before any spawn operation consumes it. A `LogicalTargetId` is stable across replacement. An `ExternalRuntimeRef` is the exact authority-domain-scoped adapter/deployment/runtime/generation incarnation. Project, cwd, Pi session paths, and labels are metadata, never identity.

Fresh managed generations are positive and begin at `1`. A logical target has at most one current generation, may have one staged claimed successor, and retains tombstones. The projection owns a reverse index proving that one exact external-runtime reference belongs to at most one logical target; a second owner is `duplicate-native-reference` and cannot stage or promote.

## Design

**Files**
- `contracts/proto/patchbay/common.proto` — generated `LogicalTargetId`, `ExternalRuntimeRef`, and `RuntimeGenerationRef`.
- `contracts/proto/patchbay/sessions.proto` — identity-bearing logical-target records/checkpoint shapes without spawn-operation behavior.
- New `core/src/session/logical_target.rs` — validated keys, current/staged/tombstone records, and reverse index.
- `core/src/session/{registry,replay}.rs`, `server/src/{checkpoint,snapshot}.rs` — fold/checkpoint exact identity state.
- Contract, replay, checkpoint, and property tests.

```rust
pub struct LogicalTargetRecord {
    pub logical_target_id: LogicalTargetId,
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub current: Option<RuntimeGenerationRef>,
    // Identity reservation only; downstream evidence contracts add provenance.
    pub reserved_candidate: Option<ExternalRuntimeRef>,
    pub tombstones: BTreeMap<ExternalRuntimeRef, Tombstone>,
}

pub trait ExternalRuntimeOwnership {
    fn owner_of(&self, external: &ExternalRuntimeRef) -> Option<&LogicalTargetId>;
}
```

V1 rejects cross-adapter/deployment migration for one logical target. Pre-provisioned/discovered sessions use an explicit logical-target assignment path and the same reverse-index constraint; they cannot collide with managed spawn identity.

## Acceptance evidence

- [ ] Generated Rust/TypeScript contracts preserve distinct logical and external-runtime id spaces.
- [ ] Generation `0`, empty ids, malformed scopes, cross-domain/cross-adapter target mutation, and runtime-ref mismatch reject before projection mutation.
- [ ] One exact external runtime cannot be owned or staged by two logical targets; `duplicate-native-reference` survives replay and checkpoint recovery.
- [ ] Current/reserved-candidate/tombstone identity slots are mutually constrained and replay-identical; this leaf imports no downstream claim/evidence type.
- [ ] Project/cwd/name/model changes cannot alter the identity tuple.
- [ ] Mutation removing any reverse-index dimension (adapter, deployment, runtime id, generation, domain) fails.

## Ordering constraint

First contract leaf. Continuation, claims, cursor scope, target resolution, staging, and promotion consume this identity; none may define a competing shape.
