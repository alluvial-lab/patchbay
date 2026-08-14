---
id: leaf6-runtime-evidence-rereview3-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-runtime-evidence-promotion-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Deep re-review pass 4 — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

**Verdict: BLOCKER (1), MATERIAL (0), NIT (0).** Independent fresh-context `openai-codex/gpt-5.6-sol` thorough pass 4 over `eee06b2 + cc7cbb1 + 4d18ce0 + 3154368`, in completeness → adversarial order. Round 3 genuinely fixes exact staged-SessionReport retries and byte/semantic-exact successful-Result retries on the normal production paths. One adversarial Result outcome remains authority-bearing: if a contradictory failed Result commits but its separately written command transition fails, the promotion producer ignores the failed Result and promotes the earlier success.

## Round-3 closure matrix

| Round-3 blocker | Verdict | Production-path evidence |
|---|---|---|
| 1. Exact managed SessionReport retry poisons replay | **PASS** | The authenticated server route returns the original staged event id for exact retries before and after promotion; one staged source remains durable; completion and restart stay green. The dedicated SQLite transaction reconstructs the domain prefix, validates the exact durable claim/classification/current attachment for a first append, scopes conflicts by claim **or** external runtime inside one authority domain, and requires full generated-envelope equality for a retry. Every generic production append route rejects the staged kind. |
| 2. Duplicate qualifying successful Result poisons promotion | **FAIL / partial** | Exact successful-Result retries on both sides of staging retain the earliest source and complete once; changed successful evidence fails closed. A different failed Result normally suppresses promotion through its terminal transition. But Observation and transition are separate writes, and a transition-write fault leaves the failed Result durable. `next_spawn_promotion` filters to successful Results before conflict handling, so it ignores that contradictory outcome and promotes the earlier success. |

## BLOCKER finding

### 1. A failed Result stranded by transition-write failure is ignored, so an earlier success still mints descendant authority

**Severity: BLOCKER**  
**Anchors:** `core/src/acceptance/observation.rs:210-260`; `core/src/session/runtime_evidence.rs:157-198`; `core/src/acceptance/index.rs:212-250`; `server/src/spawn_completion.rs:143-153`.

Non-deferred Result ingestion first appends the raw `Observation`, then separately appends the derived `CommandTransition` and audit. A write failure in the second transaction therefore leaves a valid durable failed Result while command replay remains delivered/running. Round 3's producer equality is inside a `failure_code == Unspecified` filter, so only successful Results participate in earliest-source/conflict reconciliation; the failed Result is silently ignored.

A fresh real-route probe used the production authenticated adapter service and file-backed SQLite:

1. deliver/running spawn and append a successful Result;
2. install a SQLite trigger that aborts the next `CommandTransition` insert;
3. submit `Result { failure_code: ExecutionFailed }` through authenticated ingress;
4. verify the failed Result is durable, the failure transition is absent, and the RPC reports the storage failure;
5. remove the trigger, stage the managed SessionReport, and bootstrap the production completion driver.

The driver committed one promotion, one completion audit, and one descendant Grant (`(1, 1, 1)`) instead of remaining fail-closed (`(0, 0, 0)`). A direct producer probe over the same durable shape also returned a promotion. This violates the requested conflicting-outcome rule and the story's no-authority-from-incomplete/crash evidence boundary.

**Concrete fix:** reconcile **every** exact, target-matched Result for the spawn while delivered/running before filtering for success. Retain one canonical Result source: exact repeats are no-ops; any semantically different Result, including a different `failure_code`, payload, or outcome, must suppress/fence promotion. Only a canonical successful Result may populate `SpawnPromotionResultEvidence`. Also make Result + derived transition/audit one atomic storage decision (or otherwise prevent authenticated ingress from admitting the source-without-transition prefix). Add the SQLite transition-failure probe above plus failure-before-success and success-before-failure order tests; neither ordering may promote when the Results conflict.

## Staged-report rule assessment

The changed-report rule is conservative and sound for this unreleased security boundary:

- while the claim is active, the first staged envelope is immutable; exact full-envelope equality returns its original id, while any changed report for the same claim or external runtime returns `StagedSuccessorConflict` before writes;
- poisoned and promoted claims use the same dedicated retry reconciliation, so exact late retries still return the original id and changed evidence conflicts;
- released/abandoned claims do not regain staging authority and fall back to ordinary mismatch/quarantine handling;
- source attachment, exact claim, classification context, report, classified target, and reservation all participate in generated-struct equality—identity-key equality alone is insufficient;
- the SQLite prefix query is authority-domain scoped, while the `command OR external-runtime` collision rule prevents reuse across claims inside that domain.

A changed authenticated report produced `FAILED_PRECONDITION`, appended nothing, left one staged source, did not disturb a later exact retry, and replayed successfully. Same-claim/different-runtime and different-claim/same-runtime probes both returned the explicit conflict before durability.

## Probe and mutant matrix

All reviewer-only probes and mutations ran in a detached temporary worktree and were removed afterwards; the target tree remained clean.

| Probe / mutant | Oracle | Result |
|---|---|---|
| Exact managed SessionReport twice over authenticated route | Returned ids equal; exactly one staged source; session replay green | **PASS** |
| Exact managed SessionReport after promotion | Returns original staged id; aggregate and completion-driver restart green | **PASS** |
| Changed authenticated SessionReport after staging | `FAILED_PRECONDITION`; zero new rows; later exact retry and replay green | **PASS** |
| Same claim with a different runtime; different claim with the reserved runtime | Dedicated append returns `StagedSuccessorConflict`; one source remains | **PASS** |
| Three byte-identical successful Results before/after staging | Earliest Result retained; completion commits once; restart quiescent | **PASS** |
| Changed successful Result (`observed_at` differs) | Completion driver returns corrupt-log failure; no promotion; aggregate replay has no partial publication | **PASS / fail-closed** |
| Different failed Result with its normal terminal transition | Completion driver remains green but commits no promotion/grant; replay green | **PASS / fail-closed** |
| Failed Result whose transition insert is faulted after source commit | Real authenticated route leaves failed Result durable; later completion promotes earlier success | **FAIL — BLOCKER 1** |
| Mutant: compare staged retry by report only, ignoring changed attachment/context | Reviewer attachment-exactness probe | **KILLED** — changed attachment was wrongly accepted as a retry |
| Mutant: scope staged collision by `claim AND external` instead of `claim OR external` | Reviewer competing claim/runtime probe | **KILLED** — explicit identity-conflict oracle failed |
| Mutant: disable earliest-Result semantic inequality | Existing changed-success producer oracle | **KILLED** — conflicting success wrongly promoted |
| Prior managed-ingress bypass, stale-report quarantine, Result ordering, context forgery, nested quarantine redispatch, authority/session ordering, generic-route exclusivity | Focused core/server regression set | **PASS** — all named oracles remain green |
| Pre-fix duplicate staged sources | Session replay, producer, and dedicated append | **Fail closed; no repair path**. No current producer can create the prefix. This leaf is after tag `v0.2.1`, absent from `origin/main`, unbound to a release, and has no verified external durable-data consumer, so compatibility repair is not justified. |

## Full verification suite

All requested baseline commands passed on the restored clean target tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

The green baseline does not exercise a contradictory failed Result stranded by the separate transition-write failure.

## Final recommendation

**Return `research-handoff-spawn-runtime-evidence-promotion-contract` to `implementing`.** Preserve the round-3 staged-report and exact-success retry fixes. Close only BLOCKER 1 by making conflicting Result outcomes producer-visible and removing or safely fencing the source-without-transition crash prefix, then rerun the thorough convergence lane. Do not advance Leaf 6 to done yet.
