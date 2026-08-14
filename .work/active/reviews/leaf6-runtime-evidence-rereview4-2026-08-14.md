---
id: leaf6-runtime-evidence-rereview4-2026-08-14
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

# Deep re-review pass 5 — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

**Verdict: BLOCKER (3), MATERIAL (0), NIT (0).** Independent fresh-context `openai-codex/gpt-5.6-sol` thorough pass 5 over `eee06b2 + cc7cbb1 + 4d18ce0 + 3154368 + c9a0eee`, in completeness → adversarial order. Round 4 genuinely fences contradictory Results in both orders and makes the authenticated transition-producing route transactional. It does not complete the storage contract or retry contract: every generic production storage route can still strand a transition-producing Result, the dedicated atomic append accepts a transition that contradicts the durable command pre-state, and an exact failed-Result retry is rejected rather than reconciled as an idempotent no-op.

## Round-4 closure matrix

| Round-4 requirement | Verdict | Pass-5 evidence |
|---|---|---|
| Conflicting outcomes suppress promotion | **PASS** | Real authenticated success→failure and failure→success probes produced zero promotion/audit/descendant Grant across completion-driver restart. The former returns the intentional corrupt-log fence; the latter rejects the late success and remains quiescent. Direct prefixes fence both orders. |
| Faulted derived transition cannot leave an authenticated Result source | **PASS on the selected ingress path** | A SQLite `CommandTransition` trigger rolls back Result, transition, and audit together. Reconstructing the historical stranded source still makes completion bootstrap fail closed with zero authority. The split-write mutant is killed by this oracle. |
| Exact success and failure retries are idempotent | **FAIL / partial** | Exact successful retries on both sides of staging remain deterministic and promote once. An exact failed Result retried through the real authenticated adapter RPC returns `INTERNAL` and no canonical event id instead of a successful no-op. |
| Atomic storage decision is exclusive and replay-valid | **FAIL** | `AuditedStorage` generic append/audited/decision/batch/many/dedup routes all accept a failed Result without its transition. The dedicated route also commits a typed trio whose `from_state` contradicts the durable command projection. |
| Prior round-1/2/3 protections remain green | **PASS** | Managed bypass, stale-report quarantine, Result-before-delivery rejection, context forgery, nested-quarantine mutant (b), ordered four-view mutant (d), dedicated promotion/quarantine/staged exclusivity, and restart/catch-up oracles all pass. |

## BLOCKER findings

### 1. The production storage wrapper still exposes six generic source-without-transition bypasses

**Severity: BLOCKER**  
**Anchors:** `core/src/storage/audited.rs:357-370,397-443,541-672`; `core/src/storage/rusqlite.rs:2797-2808`; `server/tests/spawn_completion.rs:1450-1489`.

`append_observation_transition_audited` is not an exclusive boundary. `reject_generic_special` and `reject_generic_unaudited_special` reject staged successor, quarantine, and promotion kinds, but do not inspect an `Observation` to reject a correlated Status/Result that implies a transition. The round-4 server regression itself reconstructs its “low-level” stranded failure through `AuditedStorage::append`, which is the production wrapper, not a backend-only corruption hook.

A fresh probe seeded a valid delivered/running spawn prefix and submitted the same `ExecutionFailed` Result through every generic production route. All six committed:

- `append` → source LSN 8;
- `append_audited` → source/audit LSNs 9/10;
- `append_decision` → source LSN 11;
- `append_batch_audited` → source/audit LSNs 13/14;
- `append_decision_audited_many` → source/audit LSNs 15/16;
- `append_dedup` → appended source LSN 17.

None wrote the derived `CommandTransition`. A future caller mistake, refactor, or alternate ingress can therefore recreate the exact prefix round 4 claims the storage decision makes impossible. This is the same boundary-exclusivity class that blocked generic promotion/quarantine/staged appends in earlier rounds.

**Required direction:** make transition-producing Observation payloads ineligible on every generic production route. Preserve the intentional successful-spawn deferred-evidence route behind a typed/dedicated validated method; use backend-only SQL or an explicitly unsafe test fixture to construct historical corruption. Add the same generic-route matrix already used for the other dedicated kinds.

### 2. The dedicated atomic append validates field agreement but not the durable transition pre-state

**Severity: BLOCKER**  
**Anchors:** `core/src/storage/port.rs:626-642`; `core/src/storage/rusqlite.rs:1363-1496`.

The dedicated append checks domain, Observation kind, exact command correlation, derived terminal outcome/failure, and partial audit framing. For `from_state`, it checks only that the integer decodes to some `OperationState`. Unlike staged, quarantine, and promotion appends, it never rebuilds the durable command prefix and never applies the candidate transition before insert.

A fresh probe seeded a valid command already in `Running`, then called the dedicated method with an `ExecutionFailed` Result and a syntactically valid `Accepted → Failed` transition. The method committed Observation/transition/audit at LSNs 8/9/10. Command replay then necessarily rejects the transition because durable pre-state is `Running`, not `Accepted`. The atomicity change therefore prevents a partial trio but still admits a complete unreplayable trio through its public dedicated boundary. The same gap permits correlations not derived from the accepted Operation to be attached to the transition.

**Required direction:** inside the same writer transaction, rebuild/validate the exact command prefix, require the durable command state and target to match the candidate transition, preserve Result-at-replay-position qualification, apply the transition to a staged projection, and only then insert. Add wrong/missing command, wrong `from_state`, disallowed edge, target mismatch, and forged-correlation probes with zero-write assertions.

### 3. Exact failed-Result retries are not idempotent on the authenticated route

**Severity: BLOCKER**  
**Anchors:** `core/src/acceptance/observation.rs:151-160`; `server/src/adapter_service.rs:1429-1548`; `core/tests/runtime_evidence_promotion.rs:676-699`.

The new producer unit test proves only that two manually adjacent identical failed Result records collapse during promotion scanning. Production cannot create that shape: the first failed Result atomically terminalizes the command, and the second exact authenticated retry reaches the blanket terminal-state rejection before any retry reconciliation. For a spawn's adapter-scoped target, the runtime-target quarantine branch does not apply.

A real adapter-RPC probe sent the identical canonical `ExecutionFailed` Result twice. The first committed the Result/transition/audit trio. The second returned:

```text
INTERNAL: late terminal observation for command spawn-1 requires authenticated runtime quarantine
```

It did not return/reuse the first source id as an idempotent success. This fails the requested exact-success-and-failure retry contract even though it does not add a second durable record.

**Required direction:** reconcile a byte/semantic-exact terminal Result retry against the canonical durable source under the decision gate before blanket late-terminal rejection; return the original source id and append nothing. Changed terminal evidence must remain rejected/quarantined and must never overwrite the canonical outcome. Add authenticated retry, completion-driver, and restart assertions.

## Admitted Result evidence set

| Durable/authenticated evidence | Expected semantic | Pass-5 result |
|---|---|---|
| One qualifying success after delivered/running | Canonical success; may promote once staged | **PASS** |
| Byte-identical successful retries before/after staging | Same canonical success; earliest source retained | **PASS** |
| One qualifying failure | Canonical non-success; no promotion | **PASS** |
| Two byte-identical failures in one replay prefix | Same canonical non-success | **PASS in producer**, **FAIL at authenticated retry API** |
| Two identical failures followed by success | Failure retries are benign with each other; later success is genuinely contradictory | **PASS / fenced** |
| Success then changed failure outcome | Contradictory | **PASS / fenced** |
| Failure then success | Contradictory or rejected after terminal failure | **PASS / no promotion** |
| Changed successful timestamp, payload, or sender identity | Not an exact retry | **PASS / fenced** |
| Staged-report exact retry after conflicting Result pair | Reuses original staged id; conflict remains monotonic in append-only history | **PASS** |

There is no current protocol event that erases or reconciles a contradictory durable Result pair. Re-sending staged evidence or a later exact Result cannot promote through it; explicit compatibility repair would be separate work and is not justified for this unreleased prefix.

## Probe and mutant matrix

All reviewer-only tests and code mutants ran in a detached temporary worktree and were removed. The target tree stayed clean.

| Probe / mutant | Oracle | Result |
|---|---|---|
| Authenticated success, faulted failed transition insert | File-backed SQLite trigger; compare complete prefix | **PASS** — transaction rolls back all three rows. |
| Historical success + stranded failed Result | Production completion bootstrap and restart | **PASS / fail-closed** — both reject the conflict; `(promotion, audit, descendant Grant) = (0,0,0)`. |
| Real authenticated success→failure and failure→success | Completion bootstrap twice per ordering | **PASS** — neither promotes; no descendant authority. |
| Single success and exact success retries around staging | Existing production producer/server restart oracles | **PASS** — earliest Result retained, one completion. |
| Exact failed Result retry over authenticated RPC | Fresh real-route probe | **FAIL — BLOCKER 3** — second call returns `INTERNAL`, not canonical no-op success. |
| Changed failure code/outcome, timestamp, payload, sender | Producer conflict oracles | **PASS** — every changed dimension fences. |
| Two identical failures then success | Fresh producer probe | **PASS** — identical failures collapse; later success conflicts. |
| Exact staged retry after contradictory Result pair | Dedicated staged append + producer | **PASS** — original staged LSN reused; promotion remains fenced. |
| Transition-producing failed Result through generic append/audited/decision/batch/many/dedup | Fresh production-wrapper matrix | **FAIL — BLOCKER 1** — all six routes commit source without transition. |
| Dedicated atomic append with durable `Running` but framed `Accepted → Failed` | Fresh storage-port probe | **FAIL — BLOCKER 2** — complete unreplayable trio commits. |
| Mutant: remove Result conflict suppression | `promotion_producer_fences_conflicting_result_outcomes_in_both_orders` | **KILLED** — expected conflict became `None`. |
| Mutant: restore split Result then transition/audit writes | Authenticated SQLite trigger oracle | **KILLED** — one stranded failed Result remained. |
| Mutant: ignore sender during Result equality | changed-success exactness oracle | **KILLED** — changed identity was accepted and promotion produced. |
| Prior managed bypass, stale SessionReport quarantine, Result ordering, quarantine context forgery, mutants (b)/(d), special-kind generic exclusivity, ordered four-view fold, restart/catch-up convergence | Focused core/server regressions | **PASS**. |

## Full verification suite

Final commands ran on the restored clean target tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS** (including workspace doctests and clippy with warnings denied).
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

The green baseline does not contain the three failing probes above.

## Final recommendation

**Return `research-handoff-spawn-runtime-evidence-promotion-contract` to `implementing`.** Preserve the round-4 conflict fence and SQLite transaction. Close the bounded storage boundary by rejecting all generic transition-producing Observation routes and preflighting the dedicated append against the exact durable command projection, then add authenticated failed-Result retry reconciliation. Re-run the thorough convergence lane; do not advance Leaf 6 to done yet.
