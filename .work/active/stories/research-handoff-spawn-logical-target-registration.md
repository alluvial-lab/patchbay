---
id: research-handoff-spawn-logical-target-registration
kind: story
stage: review
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [spawn-delivery-atomic-claim-idempotency-generation]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-14
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

- [x] A first fresh generation-1 report and legitimate exact N+1 report stage successfully rather than classifying `Unknown`.
- [x] Wrong Operation, prior, generation, logical target, adapter/deployment, attachment, or claim fails without staging/current mutation.
- [x] Staging reserves one exact external-runtime key; a second logical target receives `duplicate-native-reference` and cannot stage.
- [x] Staged report remains non-live/non-current through hot fold, replay, snapshot, and core restart.
- [x] Result-first and report-first orders converge to the same staged readiness facts.
- [x] No SessionReport-specific bypass avoids the shared classifier.

## Ordering constraint

Depends on an atomically accepted exclusive claim/fence. Duplicate reconciliation and promotion folds consume staged evidence.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; selected by the autopilot caller for the security-critical staging/reservation boundary. Review weight: `thorough` from the caller; implementation stops at `stage: review` for independent review.
- Files changed: `core/src/session/registry.rs`, `core/src/storage/{port,rusqlite}.rs`, `core/tests/runtime_evidence_promotion.rs`, and `server/src/{adapter_service,service}.rs` plus `server/src/adapter_service/tests.rs`.
- Mechanism: authenticated reports always use the shared classifier. Only its exact `ClaimedSuccessor` value is carried into `SpawnSuccessorEvidenceStaged` and sent through `append_spawn_successor_staged_idempotent`; the managed branch never invokes ordinary session-report ingestion. An immutable exact staged-envelope retry after poison/promotion is read-only reconciliation to the original event id, not new staging admission. Fresh target creation plus reservation folds on a private projection before publication, so a duplicate cannot leak an empty target into hot state.
- External identity: the dedicated storage transaction maps reverse-index collisions to a typed `duplicate-native-reference` conflict and the server returns `FAILED_PRECONDITION`; the first logical owner remains unchanged. Existing current/candidate/tombstone checkpoint machinery retains the authority-domain-qualified reverse key across hot fold, replay, checkpoint recovery, and late correlation.
- Tests added/expanded: fresh generation-1 staging without pre-created target; exact N+1 staging while N remains current; no `SessionRegistered`/`SessionGenerationBumped` publication; staged-only hot/replay/checkpoint/core-restart equality; one-owner duplicate rejection and exact outcome; atomic fresh-fold rejection; and independent classifier mutations for Operation, adapter, deployment, runtime framing, attachment, prior, and generation. Existing result-first/report-first promotion-producer tests remain green.
- Controlled mutations, all reverted with `git restore`: replacing managed staging with ordinary report ingestion failed `exact_continuation_report_stages_n_plus_one_without_publishing_it`; removing reverse-index insertion failed `duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold`; dropping prior ownership on tombstoning failed `slot_transitions_are_exact_and_tombstones_retain_ownership`; weakening the classifier generation equality failed `classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation`.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (54 vectors, 17 promoted, 22 implementation checks, 38 mutation witnesses).
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (23/23 tests).
- Verification group 4 — `cd pi-adapter && npm test`: **PASS** (38/38 tests, including the real-core loop).
- Simplification: removed server-side reconstruction of a second `ClaimedSuccessor` disposition and the poisoned/promoted claim-state staging branch; the generated disposition returned by the classifier is now the only new-stage authority. No `.proto` or generated-contract edits were made.
- Discrepancies from design: the classifier/validation contracts were already landed in `runtime_evidence.rs`, identity/claim contracts were already complete in `logical_target.rs` and `spawn_claim.rs`, and ordinary-ingress exclusion was already complete in `ingest.rs`. The remaining production integration owners were the session registry fold, dedicated storage boundary, and adapter service; no duplicate implementation was added to the named landed files.
- Adjacent issues parked: none.
