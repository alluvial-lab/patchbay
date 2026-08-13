---
id: research-handoff-spawn-crash-external-effect-evidence-contract
kind: story
stage: review
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-claim-registry-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-13
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; selected by the autopilot caller for the security-critical exact-claim external-effect boundary.
- Review weight: `thorough` from the caller; implementation stops at `review` for independent adjudication of the evidence/correlation invariants.
- Files changed: `contracts/proto/patchbay/{adapter_control,common,sessions}.proto` plus generated Rust/TypeScript contracts; claim projection and sibling durable-log folds under `core/src/`; `core/tests/spawn_claim_registry.rs`; adapter ingress and tests under `server/src/adapter_service*`; `docs/{PROTOCOL,SECURITY}.md`.
- Tests added/updated: closed phase/disposition table; all three no-effect proof variants; valid typed release and poison folds; delivered cancellation/expiry/outcome-unknown poison; wrong event kind; another claim; pre-acceptance LSN; stale/wrong adapter; silence-as-proof; phase/proof mismatch; exact external-runtime correlation; continuation prior-N liveness; authenticated ingress canonicalization and rejected wrong-claim append.
- Simplification: moved no-effect proof provenance into the typed `SpawnExecutionEvidence` event and made `SpawnClaimNoEffectRelease` reference that event once, eliminating duplicated/circular event ids in individual proof variants. Sibling projections explicitly ignore the new evidence discriminator; no second state source was added.
- Design rationale: the durable evidence repeats the complete `SpawnGenerationClaim` rather than only a command id, and records the latest durable attachment event/generation. A producer enum distinguishes core-only never-offered decisions from current-adapter reports, preventing authenticated adapter ingress from manufacturing core proof. Full claim, phase/disposition, source attachment, proof, failure, and optional runtime are revalidated during the later disposition fold. This is source/correlation evidence, not cryptographic proof of adapter honesty.
- Promotion boundary: `SpawnPromotionCommitted` remains rejected by `reject_unavailable_typed_evidence(...)`; Leaf 5 evidence cannot promote a claim.
- Discrepancies from design: added explicit producer authority and attachment-event provenance because the resolved current-authenticated-adapter trust boundary cannot be replay-validated from adapter id/generation alone. No product behavior beyond the resolved evidence boundary was added.
- Review-blocker fixes: `CorePreDeliveryTerminalProof` now references the exact durable `CommandTransition`; replay verifies exact claim command correlation, accepted pre-state, a safe non-ambiguous terminal/failure pair, and absence of delivery before the decision. Every `PROVED_NONE` path rejects `execution_outcome_unknown`.
- Post-poison safety: each claim records its latest disposition LSN; poisoned release proofs must be later than that decision, and release validation scans the complete later prefix for contradictory delivery/running, launch-attempted, identified-runtime, execution-failure, ambiguity, or poison evidence.
- Mutation evidence: `missing_evidence_event_id_is_silence_not_proof`, `evidence_source_adapter_must_match_claim_adapter`, `fresh_claim_cannot_use_prior_runtime_phase_evidence`, and `live_evidence_must_match_exact_prior_n_identity` each failed under its named guard-removal/admission mutant before the mutant was reverted.
- Review regressions: `execution_outcome_unknown_is_never_a_core_no_effect_proof`, `obsolete_refusal_proof_cannot_release_after_later_ambiguity_poison`, and `obsolete_core_proof_cannot_release_after_later_delivery` preserve both blocker fixes.
- Adjacent issues parked: none.
