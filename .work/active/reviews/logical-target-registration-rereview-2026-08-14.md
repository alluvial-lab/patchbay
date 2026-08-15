---
id: logical-target-registration-rereview-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-logical-target-registration
created: 2026-08-14
updated: 2026-08-14
---

# Thorough rereview — Unit 3 bounded staged-successor reconciliation

**Verdict: MATERIAL.** The schema-v6 index itself is exact, transactionally maintained, migration-safe, and used by the new storage lookup. Exact retry, reservation, tombstone, classifier, and generic-route protections hold. However, the actual late-retry RPC still performs an unbounded full authority-log claim rebuild while holding the global decision gate before it can reach that lookup, and the claimed bounded-work oracle does not exercise either the production storage method or the server route. A production-path full-scan mutation therefore survived the oracle.

Review mode: fresh-context delegated story rereview, effective weight `thorough`, pass 2 over `339c285..7a058cc` plus the original Unit 3 implementation and surrounding gate/replay paths.

## Findings

### MATERIAL — The late-retry path remains unbounded under the gate, and the bounded oracle is route-vacuous

**Location:** `server/src/adapter_service.rs:1037,1097-1101,1144-1146,1200-1213`; `core/src/session/spawn_claim.rs:577-594`; `core/src/storage/rusqlite.rs:4468-4496,4929-5004`

`ingest_observation` acquires `CoreDecisionGate` at line 1037. Before `existing_staged_successor_retry` can call the new indexed port, every SessionReport recovers the session projection and then calls `rebuild_spawn_claims_from_log`. That claim rebuild unconditionally calls `read_after(domain, Lsn { value: 0 })` and folds the complete authority log. Consequently an exact late staged-successor retry is still O(total durable history) while holding the global decision gate, serializing unrelated decisions behind repeated retry work. The schema-v6 lookup removes the second full-prefix scan that pass 1 identified, but it does not make the end-to-end retry path bounded.

The new oracle directly prepares `STAGED_SUCCESSOR_BY_*_SQL` against hand-seeded table rows and inspects only those statements' `FullscanStep` counters. It never calls `reconcile_spawn_successor_staged_retry`, never traverses `existing_staged_successor_retry`, and never acquires the decision gate. Mutation evidence confirms the gap:

- adding `authoritative_staged_successor_reconciliations(&db)?`—an unbounded `events` scan—to the production reconciliation method left the bounded oracle green;
- the exact-retry integration oracle also stayed green, proving the mutated production path compiled and executed;
- adding `NOT INDEXED` to the SQL constant was killed at 2,047 full-scan steps, so the test verifies only the chosen SQL statement's query plan, not bounded production routing.

**Required direction:** put exact indexed reconciliation before any full projection/claim rebuild, using only current authenticated attachment evidence plus the report's claim correlation and exact indexed staged envelope, or replace the gate-resident rebuild with a bounded incremental projection. Then make the large-prefix bounded oracle execute the production storage method and preferably the actual late-retry ingress path, with instrumentation that fails on any `read_after(..., Lsn { value: 0 })` or equivalent full scan before return.

## Checklist disposition

| Requirement | Result |
|---|---|
| Late retry is bounded under the decision gate | **FAIL / MATERIAL** — indexed helper is bounded, but the gate-held RPC performs a full claim-log rebuild first. |
| Bounded oracle kills a production full-scan fallback | **FAIL / MATERIAL** — production fallback survived; only `NOT INDEXED` in the shared SQL constant was killed. |
| Index maintenance is atomic with staged append | **PASS** — event and index row insert in one transaction; constraint/conflict failures roll back both. |
| v5→v6 backfill is correct and LSN-neutral | **PASS** — backfill derives exact rows from durable staged events in the schema transaction and allocates no event LSN. |
| Restart/replay equivalence and stale/orphan detection | **PASS** — v6 open validates exact log/index equality; migration reconstructs it; promotion intentionally retains the staged row for late retry. |
| Exact pre/post-promotion retry and mismatch behavior | **PASS** — exact retries reuse the original id; report/source mismatch returns no authority; exactness mutation was killed. |
| Dedicated-writer class barrier | **PASS** — generic raw/audited/batch/decision/dedup routes remain excluded; raw-route desync mutation was killed. |
| Pass-1 staging/reservation/tombstone/classifier regressions | **PASS** — all four mutations were re-run and killed. |

## Mutation matrix

All mutations were made on the main tree, run with focused tests, and immediately reverted with `git restore`. The tree was clean after every restore.

| Mutation | Result | Focused oracle |
|---|---|---|
| Add an unbounded durable-event scan to the production reconciliation method before its indexed lookup | **SURVIVED — MATERIAL**; both the bounded query-plan oracle and exact-retry integration test passed | `cargo test -p patchbay-core staged_successor_reconciliation_queries_do_no_full_scan_with_large_unrelated_prefix`; `cargo test -p patchbay-core --test runtime_evidence_promotion staged_successor_storage_reuses_exact_retry_and_rejects_changes_before_durability` |
| Force the claim SQL statement to use `NOT INDEXED` | **KILLED** — oracle observed 2,047 full-scan steps | `cargo test -p patchbay-core staged_successor_reconciliation_queries_do_no_full_scan_with_large_unrelated_prefix` |
| Drop report and source-attachment equality from indexed reconciliation | **KILLED** — changed report incorrectly reused the original id | `cargo test -p patchbay-core --test runtime_evidence_promotion staged_successor_storage_reuses_exact_retry_and_rejects_changes_before_durability` |
| Allow staged evidence through generic raw append, desynchronizing the index | **KILLED** — generic-route rejection assertion failed | same staged-successor storage oracle |
| Disable the managed claimed-successor staging branch | **KILLED** — continuation staging oracle failed | `cargo test -p patchbay-core-server exact_continuation_report_stages_n_plus_one_without_publishing_it` |
| Remove staged external-runtime reservation from the session fold | **KILLED** — duplicate-owner rejection disappeared | `cargo test -p patchbay-core --test runtime_evidence_promotion duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold` |
| Remove prior external-runtime ownership on tombstoning | **KILLED** — prior owner became absent | `cargo test -p patchbay-core --test logical_target_identity slot_transitions_are_exact_and_tombstones_retain_ownership` |
| Omit claimed-generation equality from the classifier | **KILLED** — wrong generation classified as claimed successor | `cargo test -p patchbay-core --test runtime_evidence_promotion classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation` |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests, including the real-core loop.

The worktree was clean after mutation restoration and after all full-suite commands.

## Recommendation

**Return `research-handoff-spawn-logical-target-registration` to `implementing`.** Preserve the schema-v6 index, atomic append/backfill validation, exact retry equality, and class barrier. Move the exact indexed fast path ahead of the gate-held full claim rebuild (or make that rebuild bounded), and replace the detached SQL-only bounded oracle with a production-path large-prefix oracle before another thorough pass.
