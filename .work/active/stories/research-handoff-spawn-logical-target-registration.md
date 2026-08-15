---
id: research-handoff-spawn-logical-target-registration
kind: story
stage: done
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [spawn-delivery-atomic-claim-idempotency-generation]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
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

### Fix round — bounded indexed late-retry reconciliation

- Execution/dispatch: retained caller-selected `openai-codex/gpt-5.6-sol` and `thorough` review weight. Direct-read only: the material finding named one storage/service path and the existing SQLite identity-index pattern answered the integration questions without exploratory fan-out.
- Mechanism: schema v6 adds `staged_successor_reconciliations`, atomically maintained by `append_spawn_successor_staged_idempotent`, with unique authority-domain-qualified claim and canonical external-runtime indexes plus the original source LSN and canonical staged bytes. A v5→v6 migration validates and backfills existing staged events without consuming an event LSN. The new storage-owned read-only reconciliation port joins the indexed row to its durable event, validates the exact index/envelope relationship, and returns the original id only for exact durable claim + report + authenticated source-attachment equality. Adapter ingress calls that bounded port while retaining the existing global decision-gate ordering; it no longer reads the authority log for late retry reconciliation.
- Behavior retained: exact pre-promotion and post-promotion retries resolve to the original staged event; changed report/source/claim evidence returns no reconciliation authority and follows the existing fail-closed conflict/quarantine path. Dedicated append conflict semantics and exact external-runtime ownership remain unchanged.
- Tests: added a 4,096-row SQLite `StatementStatus::FullscanStep == 0` oracle for both claim and external-runtime lookup indexes; expanded staged retry coverage for exact and changed evidence before promotion; added post-promotion original-id reconciliation; added v5 index-backfill/no-LSN-consumption coverage; updated legacy schema-downgrade fixtures for schema v6.
- Controlled fix-round mutations, all killed and restored with `git restore`: disabling index use produced 2,047 full-scan steps and failed the bounded-work oracle; omitting report/source exactness failed the changed-evidence retry oracle. The pass-1 mutations were re-run and remained killed: disabling managed staging failed `exact_continuation_report_stages_n_plus_one_without_publishing_it`; removing candidate reservation failed `duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold`; dropping prior ownership on tombstoning failed `slot_transitions_are_exact_and_tombstones_retain_ownership`; omitting claimed-generation equality failed `classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation`.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (including mandatory warnings-as-errors clippy).
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (54 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses; 54 model promotion blocks).
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (23/23 tests).
- Verification group 4 — `cd pi-adapter && npm test`: **PASS** (38/38 tests, including the real-core loop).
- Discrepancies from the review recommendation: none. No `.proto`, generated-contract, foundation-doc, or unrelated `.work/` edits were required.
- Adjacent issues parked: none.

### Fix round 2 — early indexed retry exit and production-path bounded oracle

- Execution/dispatch: retained caller-selected `openai-codex/gpt-5.6-sol` and caller-selected `thorough` review weight. Direct-read only: pass 2 named the exact server/storage path, and the existing authenticated-ingress, storage-port, and SQLite-index boundaries resolved the implementation without exploratory fan-out.
- Mechanism: SessionReport ingress now authenticates the current attachment, binds the domain and adapter, validates required source-cursor framing, canonicalizes the adapter id, and then performs exact indexed reconciliation before either session recovery or the full claim fold. The storage port takes the report's non-empty claim Operation correlation rather than a rebuilt `SpawnClaimRecord`; it validates the indexed durable staged envelope and returns the original event id only when the complete report and current authenticated source attachment match exactly. That equality reuses the staged envelope's original full validation, and the read-only return creates no admission, classification, projection, or authority decision, so the skipped rebuild-dependent checks are unnecessary even after claim poison/promotion. A miss or changed report/source continues through the unchanged full classifier/conflict/quarantine path.
- Simplification: removed the late-retry dependency on a fully rebuilt claim projection and narrowed the reconciliation input to evidence already present at authenticated ingress. Non-retry behavior and the schema-v6 writer/index contract are unchanged; no `.proto`, generated-contract, or foundation-doc edit was required.
- Tests: replaced the detached SQL-constant oracle with a 4,096-row test that calls the production `RusqliteStorage::reconcile_spawn_successor_staged_retry` method under a SQLite progress fuse covering every statement on that connection, and asserts changed evidence returns no authority. Added a gate-held production `ingest_observation` test over 4,096 unrelated durable events using a storage wrapper that rejects/counts every `read_after(domain, Lsn { value: 0 })`; it proves invalid attachment evidence never reaches reconciliation and an exact authenticated retry returns the original staged event with zero full reads.
- Controlled round-2 mutations, all killed and restored with `git restore`: (1) the pass-2 surviving `authoritative_staged_successor_reconciliations` full events-table scan before the indexed production lookup was interrupted by the new production-method oracle; (2) a full claim rebuild reinserted before the server's indexed early exit was rejected by the gate-held storage wrapper; (3) removing report/source exactness failed the changed-evidence assertion; (4) forcing the claim query `NOT INDEXED` exceeded the production-method progress bound; and (5) allowing staged evidence through generic raw append failed the class-barrier integration oracle. The pass-1 set remained killed: disabling the managed staging branch failed `exact_continuation_report_stages_n_plus_one_without_publishing_it`; removing staged external-runtime reservation failed `duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold`; dropping tombstone ownership failed `slot_transitions_are_exact_and_tombstones_retain_ownership`; and omitting claimed-generation equality failed `classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation`.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (including mandatory warnings-as-errors clippy and the new production storage/ingress oracles).
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (54 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses; 54 model promotion blocks).
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (23/23 tests).
- Verification group 4 — `cd pi-adapter && npm test`: **PASS** (38/38 tests, including the real-core loop).
- Discrepancies from the pass-2 direction: none. The indexed early exit remains under `CoreDecisionGate`, attachment authentication remains ahead of it, and every non-exact route retains its prior semantics.
- Adjacent issues parked: none.
