---
id: logical-target-registration-rereview3-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-logical-target-registration
created: 2026-08-15
updated: 2026-08-15
---

# Thorough rereview pass 4 — Unit 3 bounded adapter point lookup

**Verdict: NITs.** Commits `0420025` and `0871d13` close pass 3's material finding. The exact authenticated retry now performs bounded attachment point lookups, an indexed durable reconciliation, and an immediate return under `CoreDecisionGate`; it does not clone or iterate the adapter registry, materialize an adapter record, recover sessions, rebuild claims, or read the full authority log. The new clone seam kills the exact pass-3 regression. One deliberately uninstrumented internal `HashMap` copy survived, so the seam is honestly shape-specific rather than a universal detector of all possible future registry-wide work; no such bypass exists in production.

Review mode: fresh-context delegated story rereview, effective weight `thorough`, pass 4 over `d57f890..0871d13` plus the complete Unit 3 path and all three prior reviews.

## Findings

### NIT — The bounded-work seam detects registered materialization routes, not every conceivable whole-registry traversal

**Location:** `core/src/adapter/mod.rs:47-61`; `server/src/adapter_service.rs:376-401`; `server/src/adapter_service/tests.rs:2167-2217`

`ADAPTER_REGISTRY_CLONES` observes every call to `AdapterRegistry::clone`, including the original `adapters.clone()` regression, and `ADAPTER_PROJECTION_MATERIALIZATIONS` observes the deferred `point_record()` route. Both are non-vacuous: restoring the pass-3 tuple clone failed on the clone delta, and forcing `point_record()` before the index failed on the materialization delta.

A review-only method that directly copied private `records` with `Self { records: self.records.clone() }`, then `black_box`ed that copy under the gate, bypassed both counters and left the focused oracle green. The same limitation applies to a future internal iterator that clones every record without calling either registered route. This is a seam-scope note, not a current defect: `records` is private, production exposes no such helper/iterator, and the reviewed path contains only keyed `get` operations plus an `Arc` clone before the indexed return.

## Bounded-path and classification analysis

The exact-retry path is:

1. acquire `CoreDecisionGate`;
2. authenticate by keyed credential/token lookup;
3. validate request domain and keyed current attachment;
4. validate source-cursor framing;
5. read the authenticated adapter record by key to extract only generation and attachment event id, while cloning only the registry `Arc` into `AdapterRegistryLookup`;
6. select the non-empty claim Operation correlation from the report;
7. call `reconcile_spawn_successor_staged_retry`, which uses the unique `(authority_domain_id, claim_operation_id)` index and joins one `events` row by primary key;
8. validate/decode that one durable staged envelope, compare the complete report and source attachment, and return its original event id.

The exact hit returns before `point_record`, `AdapterRegistry::from_single`, session checkpoint/recovery, full claim replay, classification, quarantine, or any append. No remaining work grows with authority-log length, session projection size, or adapter-registry cardinality.

`AdapterRegistry::from_single` is classification-equivalent on the indexed-miss path. `classify_session_report` accesses `adapters` only through `source_matches_current_attachment`; that helper performs exactly one `get(source.adapter_id)`. In this service path the report adapter is canonicalized to the authenticated adapter and the single entry is the same current authenticated record. Every other classification input comes from the report, source attachment, claims, logical targets, or sessions. Therefore removing unrelated adapter records cannot change any disposition.

The manual `Clone` preserves the prior `records.clone()` semantics. Its counter and increment compile only with `conformance-fault-injection`; a normal `patchbay-core` cfg probe reported the feature **OFF**, while the explicit test feature reported **ON**. The server's feature-enabling edge is a dev-dependency. `from_single` has no durable, wire, validation, or side-effect behavior; it constructs one private registry view after an indexed miss. No `.proto`, generated contract, or public protocol behavior changed.

## Checklist disposition

| Requirement | Result |
|---|---|
| Bounded end-to-end exact retry under the gate | **PASS** — keyed authentication/attachment reads → indexed reconciliation → return; zero registry clone/materialization and zero LSN-0 reads. |
| Clone-counter strength against pass-3 shape | **PASS** — restoring the original whole-registry tuple clone failed on `ADAPTER_REGISTRY_CLONES`. |
| Sneakier seam challenge | **SURVIVED / NIT** — a new uninstrumented direct private-`HashMap` copy was not observed; no equivalent production route exists. |
| `from_single` classification equivalence | **PASS** — the complete classifier call graph reads only the authenticated adapter record. |
| No production cost or protocol/API behavior change | **PASS** — normal cfg excludes the counter; clone semantics are unchanged; the constructor is side-effect-free and used only after a miss. |
| Prior staging/reservation/tombstone/classifier protections | **PASS** — all controlled regressions remained killed. |
| Prior index/exactness/class-barrier protections | **PASS** — `NOT INDEXED`, production full-scan fallback, changed-evidence acceptance, generic-route admission, and wrong-id mutations remained killed. |
| Prior early-exit protection | **PASS** — reinserted claim replay before lookup remained killed. |

## Mutation matrix

Every mutation was applied alone on the main tree, exercised with a focused test, and immediately reverted with `git restore`. The tree was clean before the first mutation, after every restore, and before/after the full suite.

| Mutation | Result | Focused oracle |
|---|---|---|
| Restore the original whole-`AdapterRegistry` tuple clone and remove the indexed-miss materialization block | **KILLED** — clone counter changed from 1 to 2 across the retry | `exact_late_staged_retry_exits_before_any_full_rebuild_under_the_decision_gate` |
| Force `point_record()` before indexed reconciliation | **KILLED** — materialization counter changed from 2 to 3 | same gate-held retry oracle |
| Add an uninstrumented direct `records.clone()` helper and execute it under the gate | **SURVIVED — NIT** — demonstrates the counters' route-specific scope | same gate-held retry oracle |
| Bypass the managed staging branch and let `ClaimedSuccessor` reach ordinary report ingress | **KILLED** — ordinary ingress rejected `spawn_origin` instead of staging | `exact_continuation_report_stages_n_plus_one_without_publishing_it` |
| Remove staged external-runtime reservation | **KILLED** — duplicate owner was admitted | `duplicate_staged_runtime_rejection_is_atomic_for_a_fresh_hot_fold` |
| Drop prior external-runtime ownership when promotion tombstones N | **KILLED** — prior owner lookup became `None` | `slot_transitions_are_exact_and_tombstones_retain_ownership` |
| Omit claimed-generation equality from the shared classifier | **KILLED** — wrong generation classified as claimed successor | `classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation` |
| Force claim lookup through `NOT INDEXED` | **KILLED** — production SQLite progress fuse interrupted the lookup | `staged_successor_reconciliation_queries_do_no_full_scan_with_large_unrelated_prefix` |
| Admit staged evidence through generic raw append | **KILLED** — the class-barrier assertion observed generic admission | `staged_successor_storage_reuses_exact_retry_and_rejects_changes_before_durability` |
| Drop complete report and source-attachment equality | **KILLED** — changed evidence reused the staged id | same storage exactness oracle |
| Reinsert the full `events`-table staged-reconciliation scan before indexed lookup | **KILLED** — production SQLite progress fuse interrupted the scan | production storage bounded-work oracle |
| Reinsert full claim replay before indexed reconciliation | **KILLED** — gate-held storage wrapper rejected the LSN-0 read | gate-held retry oracle |
| Return the current attachment id instead of the original staged id | **KILLED** — returned LSN 1 instead of staged LSN 4101 | gate-held retry oracle |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests, including the real-core loop.

## Recommendation

**Advance `research-handoff-spawn-logical-target-registration` to `done`.** There is no material current-cycle blocker. Retain the seam-scope limitation as a NIT; future registry-wide access routes should either reuse the instrumented `Clone`/materialization paths or extend the bounded-work instrumentation when introduced.
