---
id: research-handoff-spawn-runtime-evidence-promotion-contract
kind: story
stage: review
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract, research-handoff-spawn-claim-registry-contract, research-handoff-spawn-crash-external-effect-evidence-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-14
---

# Runtime evidence quarantine and atomic promotion contract

## Checkpoint

Define the durable replay envelopes that make claimed-successor staging, stale-evidence quarantine, and authority-bearing promotion structural. This is the last contract leaf; no spawn operation begins until it and the cursor leaf are complete.

`ClaimedSuccessor` validates exact Operation provenance + expected prior + claimed generation. It routes only a SessionReport to staged evidence; it does not make the successor current. Stale/unknown/mismatched candidates persist, when useful, only inside `QuarantinedRuntimeEvidence`, never as raw authoritative Observations.

`SpawnPromotionCommitted` is one semantic event consumed by session, claim, authority, and command projections. It contains or references complete success evidence and the descendant Grant. Its source and audit commit atomically; replay cannot observe a candidate-live/grantless prefix.

## Design

**Files**
- `contracts/proto/patchbay/{common,observations,sessions,authority}.proto` — classifier and self-contained event envelopes; generated Rust/TypeScript artifacts follow from the schema.
- `contracts/proto/patchbay/common.proto` stored-event registry — dedicated staged/quarantined/promotion kinds.
- `core/src/storage/port.rs` plus backend contract tests — atomic audited promotion append that stamps exact event/audit linkage in one transaction.
- Replay-dispatch and generated contract tests.

```rust
pub enum RuntimeGenerationDisposition {
    Current,
    ClaimedSuccessor {
        claim_operation_id: CommandId,
        expected_prior: Option<RuntimeGenerationRef>,
        claimed_generation: Generation,
    },
    Tombstoned { superseded_at_lsn: u64 },
    Unknown,
    IdentityMismatch,
}
```

The quarantine envelope uses a generated `oneof` for each admitted candidate family (Observation, SessionReport, delivery acknowledgement, transcript/status evidence, Elicitation mutation), plus authenticated source binding, candidate target, classifier context, and canonical reason. It is not an arbitrary payload/schema escape hatch. Normal projections switch on the outer stored kind and cannot unwrap quarantine as normal authoritative ingress.

Promotion readiness requires accepted compound provenance, delivered/running lifecycle, successful Result, staged successor, exact external-runtime uniqueness reservation, exact generation transition, live-at-promotion exact-prior replacement Grant for continuation, completion audit linkage, and descendant Grant. Promotion installs authority before session current/live and command completed within the event fold order.

## Acceptance evidence

- [x] Legitimate first fresh/N+1 reports classify `ClaimedSuccessor` only on exact durable claim provenance.
- [x] Wrong Operation, expected prior, adapter/deployment, logical id, runtime id, or generation never stages.
- [x] Every quarantined candidate is carried by an admitted generated `oneof`; unknown/untyped candidates reject, and nested Observation/report/transcript/ack/Elicitation evidence cannot mutate any normal projection on hot fold or replay.
- [x] Promotion source is a self-contained replay unit carrying the complete descendant Grant; its distinct audit record commits in the same storage transaction, and no crash prefix publishes N+1 without authority.
- [x] Every consuming projection validates the same promotion event and exact pre-state; disagreement fails closed.
- [x] Revoked/expired exact-prior replacement authority before promotion suppresses promotion.
- [x] Mutations dispatching quarantine as raw evidence or exposing staged successor as current fail.

## Ordering constraint

Final invariant contract leaf. Target resolution waits on this leaf and the parallel cursor replacement leaf, ensuring all operations consume settled shared contracts.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` — caller-selected strongest worker for the atomic promotion, authority fence, and replay-quarantine security boundary.
- Review weight: `thorough` (caller); implementation stops at `review` for independent convergence review and does not self-approve the promotion/quarantine/fence invariants.
- Files changed: generated contract sources/artifacts in `contracts/proto/patchbay/{common,observations,sessions,authority}.proto`, `contracts/{rust,ts}/src/gen/**`; runtime fence and ordered projection folds in `core/src/{session,authority,acceptance}/`; dedicated storage transaction in `core/src/storage/`; replay dispatch allowlists; focused contract tests.
- Tests added: `core/tests/runtime_evidence_promotion.rs` covers exact `ClaimedSuccessor`, wrong-operation rejection, staging-not-current, direct claim consumption, nested quarantine inertness, mandatory atomic quarantine audit, promotion id/audit stamping, and generic-promotion append rejection. The prior Leaf-6 guard test now explicitly rejects only the forbidden legacy two-event promotion shape.
- Atomic promotion mechanism: `Storage::append_spawn_promotion_audited` is fail-closed by default and implemented by the SQLite single-writer actor as one transaction. It predicts the two consecutive LSNs, stamps `promotion_event_id`, `completion_audit_event_id`, and the nested descendant Grant's `audit_id` before encoding, reserves the descendant Grant identity, inserts the one promotion source, inserts its source-linked audit, verifies both assigned ids, and commits once. Generic/unaudited promotion appends reject.
- Promotion un-guard wiring: `SpawnClaimRegistry` consumes `STORED_EVENT_KIND_SPAWN_PROMOTION_COMMITTED` directly, validates every embedded fact against the referenced durable prefix and exact active/poisoned claim, then sets `promoted` and clears the fence. The old disposition-event path cannot promote. `fold_spawn_promotion_ordered` stages clones and applies authority → session → claim → command before publishing any projection.
- Quarantine rationale: the generated admitted-family `oneof` has no bytes/Any/opaque arm; normal command, Elicitation, diagnostics, authority, session, and completion dispatch recognizes only the outer quarantine kind and never recursively applies its candidate. Un-audited quarantine append rejects.
- Simplification: reused the existing SQLite writer transaction and audit index instead of introducing a batch-prefix protocol; one semantic promotion source remains the only authoritative replay unit.
- Discrepancies from design: none. The implementation keeps the operation driver and target-selection work downstream; this leaf supplies envelopes, validation/folds, and storage atomicity only.
- Adjacent issues parked: none.

### Leaf 6 BLOCKER convergence — 2026-08-14

- BLOCKER 1: runtime classification now authenticates adapter id, adapter generation, and exact current attachment event for both `Current` and `ClaimedSuccessor`. Production managed reports route only to typed staging or quarantine; they never reach ordinary registration/generation-bump ingestion.
- BLOCKER 2: runtime-targeted stale, terminal, unknown, and mismatched Observations no longer persist as raw `Observation` events. The Pi e2e path now also proves pre-registration snapshot evidence is present only in durable quarantine.
- BLOCKER 3: quarantine has one typed audited append. All generic storage routes reject the special kind; the dedicated route validates the generated candidate, classification/reason/target framing, canonical stale audit, exact durable attachment, and recomputed disposition before committing.
- BLOCKER 4: promotion binds the deterministic descendant Grant to the exact accepted spawn sender, adapter target, promoted runtime, canonical eight-kind set, timestamp, and parent/continuation provenance. Promotion-time parent and replacement Grant kind/scope/liveness are revalidated.
- BLOCKER 5: `SpawnPromotionCommitted` is rejected from append, dedup, audited, decision-audited, and batch routes. Its dedicated SQLite transaction reconstructs and validates authority, session/target, claim, and command projections before inserting the atomic promotion+audit pair and grant-identity reservation.
- BLOCKER 6: `ProjectionState` now owns `SpawnClaimRegistry`; rebuild and catch-up route promotion through one staged authority → session → claim → command fold and publish all views together. The production completion driver derives an unstamped promotion only from exact durable claim/lifecycle/result/staged facts and calls the dedicated append. Pre-promotion durable histories retain an explicitly isolated compatibility-repair tail; histories containing `SpawnClaim` cannot enter it.
- Mutation-strength coverage now kills missing descendant authority, nested quarantine redispatch across all admitted families/projections, source/audit split rollback, aggregate fold omission/partial publication, attachment/claim/prior/deployment/generation mutations, authority laundering one dimension at a time, and every generic promotion/quarantine storage bypass.
- Added a fully authority-valid promotion fixture, continuation with both live Grants plus revocation/expiry and N→N+1 tombstoning cases, malformed-wire and non-durable-attachment quarantine cases, a real server catch-up/restart aggregate test, and a production managed-report staging test.
- Pi adapter integration orders a generation-changing Result before the ordinary N+1 report so the accepted generation-N result cannot become stale, while in-generation transcript/report tails remain ordered before terminal Result. The e2e stale-generation assertions now match the quarantine/tombstone contract.
- No protobuf or generated contract files changed in the BLOCKER convergence pass.

### Leaf 6 round-2 BLOCKER convergence — 2026-08-14

- Ordinary core session ingress now rejects every non-`None` `spawn_origin`. Authenticated server ingress classifies every SessionReport before choosing ordinary ingestion, so an omitted correlation cannot escape an active managed claim and stale producer/source/runtime evidence commits only as an atomically audited outer quarantine.
- Observation ingestion validates implied transitions before append. Successful spawn Result evidence qualifies only at its own delivered/running replay position, every promotion envelope requires lifecycle LSNs before Result LSN, and authority/session/claim/command consumers independently enforce the same ordering.
- Dedicated quarantine append reconstructs the complete canonical classification context from the durable adapter/session/logical-target/tombstone/claim prefix and requires exact equality. One-field forged owner, current, tombstone, and claim contexts reject without writes.
- Promotion ordering now uses a private authority-installed witness before session publication. Every nested quarantine family receives independently mutable projection pre-state, making recursive-dispatch mutants observable.
- The promoted `session-report-source-ordering` vector and its independent expectation checker now specify outer quarantine for delayed, old-producer, and old-runtime reports while retaining the unchanged hot/replay session snapshot.
- Injected mutations killed: ordinary managed-origin admission; omitted-origin active-claim escape; raw append before transition validation; coordinated removal of Result-time producer qualification and all consuming order gates; weakened disposition-only quarantine validation; an authority↔session source-order swap (non-compiling without the private witness) plus coordinated authority-witness omission; and exact nested-Observation redispatch through the command fold. Each mutation was reverted before completion and the restored tree remained clean.
- No protobuf or generated contract artifacts changed in the round-2 pass.

### Leaf 6 round-3 BLOCKER convergence — 2026-08-14

- Execution capability: `openai-codex/gpt-5.6-sol` — caller-selected strongest worker for the security-critical promotion/quarantine/authority boundary. Review weight remains caller-selected `thorough`; this pass advances only to `review` for the independent pass-4 deep re-review.
- BLOCKER 1 — idempotent staged successor: `Storage::append_spawn_successor_staged_idempotent` is now the sole production/storage append for `SpawnSuccessorEvidenceStaged`. The SQLite writer transaction validates the complete durable prefix, binds one authority-domain/claim-operation/external-runtime identity, returns the original event id for a byte-exact envelope retry, rejects a changed report or competing claim/runtime with `StagedSuccessorConflict` before durability, and preflights a first append through canonical claim/classification/session replay. Generic append, dedup, audited, decision-audited, and batch routes reject the staged kind. Authenticated ingress uses this method while holding the shared decision gate and also reconciles an exact retry after poisoning or promotion to the original staged event.
- BLOCKER 2 — idempotent deferred success: `next_spawn_promotion` retains the earliest qualifying successful Result plus its exact source Observation. Byte-exact successful Result retries before or after staging are deterministic no-ops for promotion evidence; a changed successful Result for the same claim position remains a fail-closed conflict. The existing command projection continues to collapse repeated qualifying success for redelivery suppression.
- Files changed: `core/src/session/runtime_evidence.rs`; `core/src/storage/{port,audited,rusqlite}.rs`; `core/tests/runtime_evidence_promotion.rs`; `server/src/{adapter_service,service,state}.rs`; `server/src/adapter_service/tests.rs`; `server/tests/spawn_completion.rs`.
- Tests added/extended: `staged_successor_storage_reuses_exact_retry_and_rejects_changes_before_durability` covers dedicated identity, all generic-route rejection, exact event-id reuse, changed-report pre-append conflict, single staged source, and restart replay; `promotion_producer_keeps_earliest_exact_success_result_retry_on_both_sides_of_staging` covers same-Result retry before staging, after staging, earliest-event selection, and changed-evidence conflict; the authenticated managed-report server test now retries the exact report and proves hot/replay state; `managed_evidence_retries_complete_once_and_restart_as_a_replayable_prefix` exercises three successful Result deliveries on both sides of staging, an exact SessionReport retry before and after promotion, completion-driver bootstrap, one promotion/audit/grant/terminal outcome, aggregate projection restart, and completion-driver restart.
- Mutation kills: forcing an exact staged retry to append a second source made the storage oracle fail on returned LSN `10` versus original LSN `9`; restoring the old duplicate-Result fence made the producer oracle fail on the retry before staging; removing Result conflict equality made the changed-evidence oracle fail; weakening staged conflict equality made the changed-report pre-append oracle fail. Every mutant failed its focused test and was reverted with `git restore`; the restored tree was clean before bookkeeping.
- Simplification: reused the existing single-writer transaction and durable-prefix reconstruction rather than adding a side table or protobuf identity field. Result retry handling stays in the promotion producer because the admitted Result records remain useful durable evidence and `CommandIndex` already supplies idempotent redelivery suppression.
- Discrepancies from design/review: none. This implements the review's preferred dedicated atomic staged append and its permitted earliest-exact-Result producer rule without changing wire contracts. No protobuf or generated artifact changed. Already-corrupt pre-fix duplicate-staged histories remain fail-closed rather than receiving a compatibility repair; this contract leaf is not released and has no verified external durable-data consumer.
- Adjacent issues parked: none.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** after correcting one pre-final `clone_on_copy` lint; the quoted command then passed in full.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 9/9 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 29/29 tests including the real core/adapter restart e2e.

## Verification evidence

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — passed 2026-08-14.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — passed; 54 vectors, 38 mutation witnesses killed, generated bindings clean.
- `cd operator-domain && npm run build && npm test` — passed; 9/9 tests.
- `cd pi-adapter && npm test` — passed; 29/29 tests including real core/adapter restart e2e.
- Round-2 re-run after the five fixes: all four groups above passed again; contract checks covered 54 vectors, 22 implementation checks, and 38 registered mutation witnesses. `git diff --check` also passed.
