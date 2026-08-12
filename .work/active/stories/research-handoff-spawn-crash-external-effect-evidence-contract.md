---
id: research-handoff-spawn-crash-external-effect-evidence-contract
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-claim-registry-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Spawn execution and crash-evidence contract

## Checkpoint

Define the only closed-vocabulary evidence allowed to release, poison, reconcile, or abandon a spawn claim after acceptance. Every record correlates to the exact claim Operation and identifies the orchestration phase. Missing evidence or delivery/launch ambiguity fails toward `effect_may_exist`, never toward safe release.

## Design

**Files**
- `contracts/proto/patchbay/adapter_control.proto` and generated spawn event contracts — `SpawnExecutionPhase`, `ExternalEffectDisposition`, `NoExternalEffectProof`, and exact claim correlation.
- Core boundary validators/folds and adapter control ingress tests.
- Failure-vocabulary/conformance documentation updates during implementation.

```proto
enum SpawnExecutionPhase {
  SPAWN_EXECUTION_PHASE_UNSPECIFIED = 0;
  SPAWN_EXECUTION_PHASE_ACCEPTED_NOT_OFFERED = 1;
  SPAWN_EXECUTION_PHASE_OFFERED = 2;
  SPAWN_EXECUTION_PHASE_QUIESCING_PRIOR = 3;
  SPAWN_EXECUTION_PHASE_PRIOR_TERMINATED = 4;
  SPAWN_EXECUTION_PHASE_LAUNCH_ATTEMPTED = 5;
  SPAWN_EXECUTION_PHASE_EXTERNAL_IDENTITY_KNOWN = 6;
  SPAWN_EXECUTION_PHASE_HANDSHAKE_RECONCILING = 7;
  SPAWN_EXECUTION_PHASE_SUCCESS_EVIDENCE_REPORTED = 8;
}

enum ExternalEffectDisposition {
  EXTERNAL_EFFECT_DISPOSITION_UNSPECIFIED = 0;
  EXTERNAL_EFFECT_DISPOSITION_PROVED_NONE = 1;
  EXTERNAL_EFFECT_DISPOSITION_MAY_EXIST = 2;
  EXTERNAL_EFFECT_DISPOSITION_IDENTIFIED = 3;
}
```

Closed no-effect proof variants are core atomic never-offered terminalization, authenticated adapter refusal-before-responsibility, and exact supervisor/journal pre-launch failure. Delivered cancellation/expiry, `execution_outcome_unknown`, unexplained stream loss, launch-attempted loss, and absence of an acknowledgement are never no-effect proof.

## Acceptance evidence

- [ ] Every non-unspecified phase maps to an allowed external-effect disposition and exact claim.
- [ ] No-effect proof variants carry enough durable provenance to be revalidated during replay.
- [ ] Delivery/launch ambiguity poisons the claim and retains the replacement fence.
- [ ] An identified external runtime can reconcile only to its original claim and external-runtime reverse reservation.
- [ ] Unknown/malformed phase, mismatched claim, stale attachment, or contradictory evidence fails without claim mutation.
- [ ] Mutation mapping delivered cancellation/expiry or outcome-unknown to safe release fails.

## Ordering constraint

Consumes the claim state machine. The runtime evidence/promotion envelope and duplicate/reconciliation operation consume this leaf.
