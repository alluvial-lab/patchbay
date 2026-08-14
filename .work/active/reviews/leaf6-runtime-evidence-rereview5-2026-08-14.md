---
id: leaf6-runtime-evidence-rereview5-2026-08-14
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

# Deep re-review pass 6 — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

**Verdict: BLOCKER (1), MATERIAL (0), NIT (0).** Independent fresh-context `openai-codex/gpt-5.6-sol` thorough pass 6 over the cumulative implementation through `87b3882` plus non-semantic clippy allow commit `8eda91c`, in completeness → adversarial order. All three round-5 blockers are genuinely closed on their named production paths: generic transition-producing Observation routes reject, the atomic transition append validates the durable command pre-state in the writer transaction, and exact failed-Result RPC retries return the original source id without writes. The class-level sweep found one remaining special-boundary bypass: the dedicated ordinary Observation+transition append itself accepts a successful spawn Result and commits `Running → Completed`, bypassing the dedicated deferred-evidence/promotion owner.

## Round-5 closure matrix

| Round-5 blocker | Verdict | Pass-6 evidence |
|---|---|---|
| 1. Six generic source-without-transition bypasses | **PASS** | Fresh matrix over `append`, `append_audited`, `append_decision`, `append_batch_audited`, `append_decision_audited_many`, and `append_dedup` rejected canonical Status, successful Result, and `ExecutionFailed` Result shapes with zero writes. The backend guard also rejects direct generic append, and every backend writer implementation invokes the same guard. Successful spawn evidence remains admitted by `append_spawn_result_deferred_audited`; its exact retry reuses one source/audit pair. The historical stranded-Result server fixture now inserts with backend-only SQL. |
| 2. Atomic append did not validate durable pre-state | **PASS** | The SQLite writer opens one transaction, reads and replay-validates the exact prefix, rebuilds `CommandIndex`, matches command/target/durable `from_state`, stages Observation then transition, and only then inserts Observation → transition → linked audit. Missing command, wrong `from_state`, disallowed edge, wrong target, and forged correlation all reject without changing the prefix; a valid trio reopens/replays. |
| 3. Exact failed Result retry rejected | **PASS** | The real authenticated adapter RPC returns the original failed-Result source id on the second byte/semantic-exact submission, appends nothing, rejects changed terminal evidence without writes, and leaves projection rebuild plus two completion-driver bootstraps quiescent. |

## BLOCKER finding

### 1. The ordinary atomic transition boundary can terminalize a successful spawn without promotion

**Severity: BLOCKER**

**Anchors:** `core/src/storage/port.rs:642-653`; `core/src/storage/rusqlite.rs:1742-1839`; `core/src/acceptance/observation.rs:166-187`; `core/tests/runtime_evidence_promotion.rs:882-1137`.

The production acceptance route correctly recognizes a successful spawn Result and calls `append_spawn_result_deferred_audited`, leaving terminalization to `SpawnPromotionCommitted`. The storage contract does not make that ownership exclusive across its dedicated methods, however. `do_append_observation_transition_audited` validates the Observation-derived transition, durable target, durable state, and staged command fold, but never checks the durable Operation kind before accepting a successful `Result → Completed` transition. Only the deferred method checks `record.operation.kind == Spawn`.

A fresh storage-port probe seeded a real durable spawn command in `Running`, then called:

```text
append_observation_transition_audited(
  successful spawn Result,
  Running → Completed,
  CommandCompleted audit
)
```

The call succeeded and committed Observation/transition/audit at LSNs `8/9/10`. No `SpawnPromotionCommitted`, completion audit, descendant Grant, staged successor, session publication, or promoted claim was required. The transaction is internally replayable by `CommandIndex`, so restart preserves `CommandState = completed` while the authority/session/claim promotion contract was never satisfied. This is the same structural-exclusivity class as the generic-route blockers: current call-site convention chooses the safe method, but the production storage port still exposes a second valid writer for the forbidden semantic.

**Concrete fix:** after rebuilding the durable command inside the same transaction, make `append_observation_transition_audited` reject `OperationKind::Spawn + successful Result + to_state Completed` before any insert. Route that shape exclusively through `append_spawn_result_deferred_audited`; keep failed/rejected spawn Results and Status transitions on the ordinary atomic boundary. Prefer one shared typed route-classification predicate used by both dedicated writers so their admitted sets are disjoint. Add a cross-dedicated regression test that seeds a delivered/running spawn, proves the ordinary atomic method writes nothing for successful Result, proves the deferred method still succeeds/idempotently retries, and kills removal of the Operation-kind exclusion.

## Adversarial class sweep

- **Diagnostics migration:** PASS. Both real `QueryDiagnostics` success/retry/replay and materialization-failure/retry tests pass. Core-local diagnostics Results commit through the atomic Observation+transition boundary; authenticated adapter diagnostics remain non-transition `Event` Observations and continue through their audited route.
- **Implication detection completeness/soundness:** PASS for the canonical storage contract. Every generated Status/Result with one exact non-empty command correlation and a known failure code is classified by `derive_transition`, including completed, rejected, and failed outcomes. Event/Delta, adapter diagnostics, transcripts, Elicitation evidence, uncorrelated observations, and resource Status facts remain outside the transition fence. An over-broad “reject every Status/Result” mutant was killed by the legitimate uncorrelated resource-Status oracle.
- **Generic callers:** no legitimate production call site still generically appends a transition-producing terminal Observation. Spawn fixtures use the deferred method; diagnostics uses the atomic transition method; the historical stranded prefix uses explicit backend SQL.
- **TOCTOU/replay:** PASS. Durable-prefix read, validation, staged fold, and inserts share one SQLite writer transaction. Hot-path acceptance checks are advisory; the durable transaction independently revalidates the decisive state.
- **Retry exactness:** PASS. Full generated `Observation` equality covers sender, target, correlations, payload, `observed_at`, and failure code. Changed evidence conflicts/rejects. A mutant that ignored `observed_at` was killed.
- **Conflict composition:** PASS. A fresh prefix containing canonical deferred success followed by canonical failure preserved both exact source ids on success/failure retries, reused the staged-successor id, appended nothing on retries, and still made `next_spawn_promotion` fail on conflicting Result evidence.
- **Prior oracles:** PASS. Managed-origin bypass, stale-report outer quarantine, Result ordering, quarantine-context forgery, nested quarantine redispatch, authority-before-session ordering, special-kind generic exclusivity, four-view aggregate publication, fault rollback, and restart/catch-up all passed in the full suite. The one failure is the newly exercised cross-dedicated spawn-success route above.

## Probe and mutant matrix

Reviewer-only probes/mutations ran in detached temporary worktrees and were removed; the target tree remained clean.

| Probe / mutant | Oracle | Result |
|---|---|---|
| Status, successful Result, and `ExecutionFailed` Result through all six generic wrapper routes | `transition_observations_are_exclusive_from_every_generic_storage_route` | **PASS** — every route rejected; zero rows. |
| Direct backend generic append of the same transition shapes | backend guard in the same matrix plus writer-path inspection | **PASS** — rejected. |
| Successful spawn Result through its deferred dedicated append, including exact retry and changed timestamp | `deferred_spawn_result_reuses_exact_source_and_rejects_changed_evidence` | **PASS** — exact source reused; changed evidence conflicts without writes. |
| Missing/wrong command pre-state, wrong `from_state`, disallowed edge, wrong target, forged correlation | `atomic_transition_append_validates_durable_prestate_and_reconciles_exact_retries` | **PASS** — all reject without writes; valid trio and exact retry replay. |
| Identical authenticated `ExecutionFailed` Result twice; then changed failure | `authenticated_failed_result_retry_reuses_canonical_source_across_driver_restart` | **PASS** — original id returned, no retry write, changed evidence rejected, restarts quiescent. |
| Real diagnostics success/replay and materialization failure/retry | two `grpc_smoke` diagnostics tests | **PASS**. |
| Canonical success → canonical failure → exact retries of each → exact staged retry | fresh cross-rule probe | **PASS** — source ids stable, no retry writes, promotion conflict remains monotonic. |
| Successful spawn Result submitted through ordinary dedicated Observation+transition append | fresh cross-dedicated probe | **FAIL / BLOCKER 1** — committed `Running → Completed` plus audit at LSNs `8/9/10` without promotion or descendant authority. |
| Mutant: omit staged transition application before insert | durable-prestate oracle | **KILLED** — disallowed-edge no-write assertion failed. |
| Mutant: classify every Status/Result as transition-producing, including uncorrelated resource Status | resource-Status ingestion oracle | **KILLED** — legitimate resource fact was rejected. |
| Mutant: ignore `observed_at` in retry equality/conflict detection | changed deferred-success evidence oracle | **KILLED** — near-miss incorrectly reconciled. |

## Full verification suite

All required commands ran on the restored clean target tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**, including doctests and warnings-denied clippy.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9 tests.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

The green baseline does not exercise the cross-dedicated successful-spawn bypass.

## Final recommendation

**Return `research-handoff-spawn-runtime-evidence-promotion-contract` to `implementing`.** Preserve all round-5 fixes. Close only the dedicated-method admitted-set overlap so a successful spawn Result is structurally exclusive to deferred evidence and `SpawnPromotionCommitted`; then rerun the thorough convergence lane. Do not advance Leaf 6 to done yet.
