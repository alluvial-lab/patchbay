---
id: spawn-delivery-atomic-claim-idempotency-generation
kind: story
stage: implementing
tags: [adapter, protocol, security, verification]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution, research-handoff-spawn-generation-monotonicity-tombstoning]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Atomic generation claim before spawn delivery

## Checkpoint

Make the durable accepted spawn record itself the exclusive claim on a logical target's next generation. Boundary idempotency alone prevents only an exact retry from creating a second command; it does not prevent two distinct continuation Operations from concurrently claiming the same current generation. The shared decision gate must reconcile the durable claim projection, select at most one claim, append acceptance, and expose only the winning claim to delivery.

This strengthens Mission Control's compare-and-swap task claim with Patchbay's caller idempotency and target-generation fence. It directly addresses the spawn review's BLOCKER 4 and the outpost_pi field failure where a non-exclusive marker let multiple runtimes consume one request.

## Design

**Files**
- `contracts/proto/patchbay/operations.proto` — persist `SpawnGenerationClaim` inside `AcceptedOperation` and carry it on adapter `Delivery`.
- `core/src/session/logical_target.rs` — fold accepted spawn claims, their terminal command state, and committed generation advances into `SpawnClaimRegistry`.
- `core/src/acceptance/pipeline.rs` — accept only a claim prepared against the reconciled current generation; exact retry returns the existing persisted claim.
- `server/src/state.rs` and `server/src/service.rs` — hold the composition-root `CoreDecisionGate` across projection catch-up, claim check, and accepted append.
- `server/src/adapter_service.rs` — deliver the persisted claim rather than reconstructing or allocating a generation.
- `core/tests/acceptance_pipeline.rs`, `server/tests/grpc_smoke.rs`, and `server/tests/spawn_completion.rs` — barrier-controlled competing-claim and crash-prefix tests.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnClaimRecord {
    pub logical_target_id: LogicalTargetId,
    pub expected: Option<RuntimeGenerationRef>,
    pub claimed_generation: Generation,
    pub spawn_operation_id: CommandId,
    pub accepted_lsn: u64,
    pub disposition: SpawnClaimDisposition,
}

pub trait SpawnClaimLookup: Send + Sync {
    fn claimable(
        &self,
        domain: &AuthorityDomainId,
        candidate: &SpawnGenerationClaim,
    ) -> Result<(), SpawnClaimConflict>;
}
```

The claim is not a second persistence source: it is a projection of the accepted Operation envelope in the durable log. For a continuation, only one nonterminal accepted Operation may claim `(authority_domain_id, logical_target_id, expected_generation)`. A later intentional attempt is allowed only after the prior claimant reached a durable non-success terminal without advancing the generation; it uses a new command id/key and creates a new claim record. A successful claim remains permanently consumed by the generation-advance event.

No code path increments a generation from cached state, reconnect, timeout, or process launch. The adapter receives `claimed_generation` and must report that exact value; it never chooses `current + 1` independently.

## Acceptance evidence

- [ ] Two concurrent distinct continuation Operations against the same expected generation produce at most one accepted active claim and one adapter delivery.
- [ ] An exact retry with the same command id/key/payload returns the existing claim and command state.
- [ ] A different payload under the same key rejects without changing the claim projection.
- [ ] A failed/cancelled/expired continuation may be intentionally retried with a new command/key only after its terminal event is durable; a successful generation cannot be reclaimed.
- [ ] Crash after acceptance and before delivery reconstructs the same exclusive claim from replay.
- [ ] Delivery carries the persisted claim; absence, mismatch, or a reconstructed next generation fails closed.
- [ ] A mutation that removes the exclusive-claim check fails the competing-continuation test/vector.

## Ordering constraint

Depends on the logical-target registry and generation transition semantics. Duplicate/external-execution policy and Pi restart orchestration consume this claim.
