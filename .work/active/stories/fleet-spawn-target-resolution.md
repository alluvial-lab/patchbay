---
id: fleet-spawn-target-resolution
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-spawn
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-12
---

# OperationKind-aware spawn target resolution

## Checkpoint

Make the acceptance boundary bind a `spawn` to one canonical attached adapter and prepare its typed lifecycle claim before durable acceptance. The historical story id says `fleet`, but the committed v1 path is **not** fleet selection: `TargetScopeKind::Adapter` is the only admitted spawn target. Fleet-supervisor and authority-domain selection stay reserved; broadcast is rejected.

The current `TargetRegistry` already dispatches by `OperationKind` and resolves an attached adapter. This checkpoint preserves that implementation, removes stale fleet wording, and extends the boundary so a typed fresh/continuation `SpawnRequest` is validated with the target rather than decoded later by delivery code.

## Design

**Files**
- `contracts/proto/patchbay/operations.proto` — generated `SpawnRequest`, `FreshSpawn`, `SpawnContinuation`, `SpawnTargetSpec`, and `SpawnGenerationClaim` wire contracts.
- `core/src/acceptance/ports.rs` — change the resolver input from detached kind/scope fields to the complete validated Operation and return the prepared spawn claim with the adapter binding.
- `core/src/target.rs` — enforce the OperationKind × TargetScopeKind matrix and delegate continuation lookup to the logical-target projection.
- `core/src/acceptance/pipeline.rs` — validate the generated spawn envelope before grant/target stateful work and persist the resolver-produced claim in `AcceptedOperation`.
- `server/src/state.rs` — catch up the target/logical-target projection while holding the shared `CoreDecisionGate` before resolving and accepting.
- `core/tests/resource_resolver.rs` and `core/tests/acceptance_pipeline.rs` — exact target matrix and pre-append rejection evidence.

```rust
pub trait TargetResolver: Send + Sync {
    fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation: &Operation,
    ) -> impl Future<Output = Result<TargetBinding, TargetNotFound>> + Send;
}

pub enum TargetBinding {
    SpawnAdapter {
        adapter_id: AdapterId,
        claim: SpawnGenerationClaim,
    },
    RuntimeSession { /* existing exact identity */ },
    Resource(ResourceIdentity),
    AuthorityDomain(AuthorityDomainId),
}
```

For a fresh spawn, the core derives a distinct typed `LogicalTargetId` from the accepted creation `CommandId` and claims generation `1`. For continuation, the payload must name the current logical target and exact prior runtime-generation reference; the resolver prepares exactly `expected_generation + 1`. The adapter-specific `target_spec.shape` remains open and adapter-enforced at delivery.

## Acceptance evidence

- [ ] A canonical attached-adapter `spawn` resolves and returns `TargetBinding::SpawnAdapter` with a prepared generation claim.
- [ ] Runtime-session, operational-resource, fleet-supervisor, authority-domain, malformed, and mixed spawn scopes reject before the accepted Operation append.
- [ ] Existing-session and resource Operations still use their existing resolvers.
- [ ] An unknown/reserved OperationKind rejects before grant evaluation; an adapter-unsupported target-spec shape remains an accepted delivery-layer `unsupported_command`.
- [ ] Restart/fresh payload malformation and generation overflow reject before durable acceptance.
- [ ] Ordinary core restart replay keeps a durably registered adapter spawn-resolvable but does not fabricate a live attachment channel.

## Ordering constraint

This is the first checkpoint. Logical-target registration and every continuation claim depend on one unambiguous adapter-scoped acceptance path.
