---
id: research-handoff-spawn-idempotency-duplicate-handling
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration, research-handoff-spawn-crash-external-effect-evidence-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Duplicate, ambiguous-outcome, and claim reconciliation

## Redesign disposition

Rewritten. The previous “failed/cancelled/expired may retry after terminal” rule is superseded. Only durable proof of no external effect releases a claim.

## Checkpoint

Preserve exact Patchbay boundary retry while making external-effect ambiguity claim-poisoning. Exact retry returns the existing Operation, compound provenance, and claim. A distinct command/key cannot reuse an active, poisoned, promoted, or abandoned generation.

Consume typed execution/crash evidence:
- `proved_none` may durably release after proof validation;
- `may_exist` poisons and retains the delivery fence;
- `identified` reserves/reconciles the external runtime to the original claim;
- absent/contradictory evidence fails toward poison, never release.

A poisoned claim ends only through exact runtime reconciliation and promotion, later closed-vocabulary no-effect proof, or operator target abandonment. No automatic relaunch occurs.

## Design

**Files**
- `core/src/acceptance/{pipeline,index}.rs`, `core/src/session/spawn_claim.rs` — retry/claim reconciliation projection.
- `server/src/adapter_service.rs` — redelivery suppression and reconciliation ingress.
- Adapter-facing execution-evidence port; Pi implementation remains in its downstream redesign.
- Duplicate, crash, cancellation/expiry, reconciliation, and abandonment vectors/tests.

`execution_outcome_unknown`, delivered cancellation/expiry, launch-attempted loss, and unexplained stream loss transition claim state to poisoned regardless of terminal command state. Adapter `idempotency_strength` informs operator retry presentation but cannot override core claim exclusivity.

## Acceptance evidence

- [ ] Exact command/key/target/payload retry returns the original state/claim; changed payload rejects.
- [ ] Failed/cancelled/expired alone does not release; delivered cancellation/expiry and outcome unknown poison.
- [ ] Valid no-effect proof references the exact claim/phase/source and permits one durable release.
- [ ] An identified runtime reconciles only to its original logical target/claim and cannot collide in the reverse index.
- [ ] Poison survives core/adapter restart and blocks delivery/reclaim until reconciliation or abandonment.
- [ ] Journal/store unavailable or corrupt cannot silently execute/release.
- [ ] Mutations release-on-terminal, relaunch-on-unknown, or ignore reverse ownership fail.

## Ordering constraint

Consumes atomic claim, staged identity reservation, and the crash/effect evidence contract. Completion requires this reconciliation behavior.
