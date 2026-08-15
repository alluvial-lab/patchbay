---
id: logical-target-registration-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-logical-target-registration
created: 2026-08-15
updated: 2026-08-15
---

# Thorough rereview pass 3 — Unit 3 early indexed retry exit

**Verdict: MATERIAL.** Commit `014f3a5` closes both pass-2 defects: the exact authenticated retry reaches the production indexed storage port before session recovery or claim replay, and the two production-path bounded-work oracles kill the prior surviving full-scan fallback and a reinserted pre-lookup claim rebuild. Exactness, original-id return, promotion/tombstone/poison reconciliation, staging exclusivity, reservation, and replay behavior remain correct. However, the exact-retry path still clones the complete in-memory adapter projection while holding `CoreDecisionGate` before it reaches the index. That is growing, unnecessary work in the same global critical section and meets this review's explicit material definition of unbounded work under the gate.

Review mode: fresh-context delegated story rereview, effective weight `thorough`, pass 3 over `bdc1b31..014f3a5` plus the original Unit 3 implementation and both prior reviews.

## Findings

### MATERIAL — Exact retries clone the complete adapter registry under the global decision gate before indexed reconciliation

**Location:** `server/src/adapter_service.rs:1066-1086,1103-1118`

`ingest_observation` correctly authenticates and enters `CoreDecisionGate`, but the tuple used to obtain the current attachment also executes `adapters.clone()` at line 1084. `AdapterRegistry` is a `HashMap<AdapterId, AdapterRecord>`; each record includes the complete validated capability manifest. The clone therefore grows with every registered adapter (and its declared resource kinds) and happens before `existing_staged_successor_retry` can return the original staged id. Repeated exact retries still serialize unrelated core decisions behind O(current adapter registry) copying. The new gate oracle counts full durable-log reads, so it remains green while this separate pre-lookup unbounded work is present.

The clone is not needed to authenticate or reconcile an exact retry. The indexed port needs only the authenticated adapter id, current generation, and attachment event id. The full `adapter_projection` is consumed only by classification after an indexed miss.

**Required direction:** extract only `(current_adapter_generation, source_attachment)` from the point lookup, perform the indexed retry reconciliation, and clone/read the full adapter projection only after that lookup returns `None`. Keep the decision gate and current attachment/source equality unchanged. Add a regression seam that makes any whole-registry materialization before the exact-return point observable, rather than treating the existing `read_after(..., Lsn { value: 0 })` counter as proof that all gate-held work is bounded.

## Checklist disposition

| Requirement | Result |
|---|---|
| Indexed exit precedes session recovery and claim replay | **PASS** — exact retries traverse the production reconciliation method and return before either durable rebuild. |
| Attachment authentication and exact source binding | **PASS** — token authentication, payload adapter binding, current attachment lookup, source-cursor framing, and exact source-attachment equality remain ahead of success. |
| Zero unbounded work under the gate | **FAIL / MATERIAL** — no unbounded durable read remains, but the whole `AdapterRegistry` clone is growing pre-lookup work under the same global gate. |
| Production storage oracle is non-vacuous | **PASS** — reinserting `authoritative_staged_successor_reconciliations(&db)` into the production method is interrupted by the progress fuse. |
| Gate-held ingress oracle is non-vacuous | **PASS** — reinserting `rebuild_spawn_claims_from_log` before lookup is rejected by the production-route storage wrapper. |
| Non-exact/conflicting reports retain the full fail-closed path | **PASS** — indexed report/source mismatch returns `None`; the unchanged classifier, quarantine, or dedicated append conflict path then runs. |
| Post-promotion/tombstone retry | **PASS** — the shared gate orders promotion and lookup, the index is retained, the exact retry returns the original stage id, and no session/claim/authority mutation occurs. The real completion-path retry/restart test passes. |
| Post-poison/abandon retry | **PASS** — current claim disposition is not needed to acknowledge the already-committed immutable stage. Exact envelope and current source-attachment equality return historical identity only; a miss still rebuilds and validates current claim state. |
| Attachment replacement between stage and retry | **PASS** — attachment event/generation inequality prevents the early return and routes through stale-attachment classification. |
| Pass-1/2 staging, reservation, tombstone, classifier, index, generic-route, and exactness protections | **PASS** — every rerun mutation below was killed. |

## Interleaving analysis

- **Promotion wins the gate first:** promotion retains the staged reconciliation row. The later exact retry reads that row and returns its original id without publishing or re-folding the now-current runtime.
- **Retry wins the gate first:** it performs a read-only lookup and releases the gate; promotion then validates and commits against the unchanged durable prefix.
- **A later promotion tombstones the prior generation:** the retry still names the immutable staged successor and cannot alter either current or tombstoned ownership. Tombstone reverse ownership remains retained.
- **Claim poison or target abandonment occurs after staging:** the durable stage remains a truthful prior fact. Returning its id neither reactivates the claim nor grants staging authority; any changed report/source misses and must pass the current projection/claim rebuild.
- **Attachment replacement occurs after staging:** the current `RuntimeEvidenceSourceAttachment` differs in generation and/or attachment event id, so the indexed equality test returns no authority and the current stale/conflict route runs.

## Mutation matrix

All mutations were made on the main tree, exercised with one focused test, and immediately reverted with `git restore`. The tree was clean after every restore.

| Mutation | Result | Focused oracle |
|---|---|---|
| Reinsert the pass-2 surviving full `events` scan (`authoritative_staged_successor_reconciliations`) inside the production reconciliation method | **KILLED** — SQLite progress fuse interrupted the production call | `cargo test -p patchbay-core staged_successor_reconciliation_queries_do_no_full_scan_with_large_unrelated_prefix` |
| Reinsert a full claim rebuild immediately before the indexed early exit | **KILLED** — gate-held wrapper rejected the LSN-0 authority-log read | `cargo test -p patchbay-core-server exact_late_staged_retry_exits_before_any_full_rebuild_under_the_decision_gate` |
| Fresh: return the current attachment event id instead of the original staged event id | **KILLED** — exact retry returned LSN 1 instead of staged LSN 4101 | same gate-held exact-retry oracle |
| Force the claim lookup through `NOT INDEXED` | **KILLED** — production method exceeded the SQLite work fuse | production storage bounded-work oracle |
| Drop complete report and source-attachment equality from indexed reconciliation | **KILLED** — changed report incorrectly reused the staged id | production storage bounded-work/exactness oracle |
| Allow staged evidence through generic raw append | **KILLED** — class-barrier assertion observed admission | `cargo test -p patchbay-core --test runtime_evidence_promotion staged_successor_storage_reuses_exact_retry_and_rejects_changes_before_durability` |
| Disable managed staging and let the claimed successor reach ordinary report ingress | **KILLED** — continuation report failed instead of staging | `cargo test -p patchbay-core-server exact_continuation_report_stages_n_plus_one_without_publishing_it` |
| Remove staged external-runtime reservation from the session fold | **KILLED** — duplicate owner was admitted | `cargo test -p patchbay-core --test runtime_evidence_promotion duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold` |
| Remove prior external-runtime ownership when promotion tombstones it | **KILLED** — tombstoned owner lookup became absent | `cargo test -p patchbay-core --test logical_target_identity slot_transitions_are_exact_and_tombstones_retain_ownership` |
| Omit claimed-generation equality from the shared classifier | **KILLED** — wrong generation classified as claimed successor | `cargo test -p patchbay-core --test runtime_evidence_promotion classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation` |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests, including the real-core loop.

The worktree was clean after mutation restoration and all full-suite commands.

## Recommendation

**Return `research-handoff-spawn-logical-target-registration` to `implementing`.** Preserve the early indexed port, exact envelope/source equality, schema-v6 index, and both production-path oracles. Defer whole-registry projection materialization until after an indexed miss, add a mutation-sensitive pre-return bounded-work guard for that shape, and run another thorough pass before advancing to `done`.
