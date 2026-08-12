---
id: fleet-spawn-target-resolution
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-cursor-authoritative-replacement-contract, research-handoff-spawn-runtime-evidence-promotion-contract]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-12
---

# Operation-aware spawn target and compound authority resolution

## Redesign disposition

Rewritten after the 2026-08-12 adversarial review. The historical `fleet` id is retained for reference stability, but this checkpoint does not select a fleet. It consumes all early contract leaves rather than defining logical-target, claim, or continuation types downstream.

## Checkpoint

Resolve a `spawn` against one canonical attached adapter. Fresh spawn selects one live adapter-scoped `spawn` Grant. Continuation additionally resolves the exact current prior generation and selects a live exact-generation `session-management` Grant for the same verified subject/endpoint/domain, using the same sampled decision time under `CoreDecisionGate`.

Both selected Grant ids and the exact prior are returned for the accepted envelope. Adapter spawn authority alone never authorizes continuation. Runtime-session/resource/fleet/authority-domain spawn targets reject before durable acceptance; broadcast remains excluded.

## Design

**Files**
- `core/src/acceptance/ports.rs` — operation-aware target/authority result port.
- `core/src/target.rs` — one explicit adapter target plus exact-prior lookup.
- `core/src/authority/check.rs` — deterministic two-Grant compound selection/rejection.
- `core/src/acceptance/pipeline.rs` — validate generated payload and persist resolver-produced provenance.
- `server/src/{state,service}.rs` — catch up target/claim/authority projections under one gate before decision.
- Acceptance/authority/resolver tests.

```rust
pub enum TargetBinding {
    SpawnAdapter {
        adapter_id: AdapterId,
        claim: SpawnGenerationClaim,
        continuation_authority: Option<ContinuationAuthorityProvenance>,
    },
    RuntimeSession { /* existing exact target */ },
    Resource(ResourceIdentity),
    AuthorityDomain(AuthorityDomainId),
}
```

For continuation, acceptance fails if the payload prior is not the exact current generation, if an active/poisoned claim already consumes N+1, or if either Grant is missing/revoked/expired/mismatched. Promotion later rechecks the exact replacement Grant's liveness; no other Grant id may silently replace accepted provenance.

## Acceptance evidence

- [ ] Fresh spawn resolves with one adapter-spawn Grant and generation-1 claim.
- [ ] Continuation resolves only with adapter-spawn + exact-prior session-management Grants for one verified subject/endpoint/domain.
- [ ] Missing/revoked/expired/wrong-generation replacement Grant rejects before accepted append/delivery.
- [ ] Runtime/resource/fleet/domain/malformed/mixed spawn targets reject before acceptance; unsupported adapter shape remains delivery-layer `unsupported_command`.
- [ ] Restart replay preserves durable adapter routing eligibility without fabricating a live attachment.
- [ ] Mutations accepting continuation on the broad spawn Grant alone or substituting another replacement Grant fail.

## Ordering constraint

Begins only after every contract leaf is available: the promotion-contract dependency brings the identity, continuation, claim, and crash-evidence leaves; the cursor leaf completes the parallel shared-contract layer.
