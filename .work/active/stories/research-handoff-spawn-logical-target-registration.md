---
id: research-handoff-spawn-logical-target-registration
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [spawn-delivery-atomic-claim-idempotency-generation]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Claimed-successor staging and external-runtime reservation

## Redesign disposition

Rewritten. The old direct “report registers generation 1/current” behavior is superseded. Managed fresh/continuation reports stage candidate evidence only; promotion owns the first live/current mutation.

## Checkpoint

Classify the first exact managed successor report through the shared runtime-generation classifier. A legitimate fresh generation-1 or exact `N+1` report returns `ClaimedSuccessor` when its current authenticated attachment, spawn Operation id, durable claim, logical target, expected prior, adapter/deployment, runtime id, and claimed generation all agree.

The report then reserves the exact external-runtime reverse-index key and appends `SpawnSuccessorEvidenceStaged`. It does not register the successor as current, tombstone N, issue authority, or complete the command.

## Design

**Files**
- `core/src/session/{logical_target,spawn_claim,ingest}.rs` — exact classifier, ownership reservation, staged-evidence plan/fold.
- `server/src/adapter_service.rs` — authenticate attachment, catch up under gate, append staged evidence.
- Session ingress/replay/reverse-index tests.

```rust
pub fn classify_runtime_candidate(
    targets: &LogicalTargetRegistry,
    claims: &SpawnClaimRegistry,
    candidate: &RuntimeEvidenceCandidate,
) -> Result<RuntimeGenerationDisposition, SessionError>;
```

`ClaimedSuccessor` is not available to ordinary output/status/transcript/ack/Elicitation ingress. Only a SessionReport correlated to the creating claim can route to staging. Pre-provisioned discovery remains an explicit non-managed registration path using the identity contract and cannot consume a managed claim.

## Acceptance evidence

- [ ] A first fresh generation-1 report and legitimate exact N+1 report stage successfully rather than classifying `Unknown`.
- [ ] Wrong Operation, prior, generation, logical target, adapter/deployment, attachment, or claim fails without staging/current mutation.
- [ ] Staging reserves one exact external-runtime key; a second logical target receives `duplicate-native-reference` and cannot stage.
- [ ] Staged report remains non-live/non-current through hot fold, replay, snapshot, and core restart.
- [ ] Result-first and report-first orders converge to the same staged readiness facts.
- [ ] No SessionReport-specific bypass avoids the shared classifier.

## Ordering constraint

Depends on an atomically accepted exclusive claim/fence. Duplicate reconciliation and promotion folds consume staged evidence.
