---
id: pi-rpc-supervisor-rereview2-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-rpc-process-supervisor
created: 2026-08-16
updated: 2026-08-16
---

# Thorough rereview pass 3 — Pi Unit 3 claim-aware RPC process supervisor

**Verdict: MATERIAL.** Commit `2134c06` closes the pass-2 quiesce-abort misclassification and the tested missing/duplicated-launch promotion corruptions. Possibly-written supervisor RPC loss now preserves transport provenance, reports `execution_outcome_unknown`, reports N as stale/failed/offline with unknown activity from exact process evidence, and poisons the retained fence. Promotion replay now rejects the tested missing launch, duplicated launch, and success-before-handshake shapes before publication or marker writes. One convergence-scoped semantic-chain gap remains: the journal write and read validators intentionally accept a contradictory repeated phase when its disposition changes from `proved_none` to `may_exist`.

Review mode: independent fresh-context delegated story review, effective weight `thorough`, pass 3 over `d812187..2134c06`. No temporary worktree was created. Manual mutations were restored with `git restore`.

## Findings

### MATERIAL 1 — A contradictory repeated phase is accepted on write and read

**Locations:** `pi-adapter/src/spawn_journal.ts:217-235,553-560`

`recordPhase` treats only an exact same-phase/same-disposition call as idempotent. A same phase with a different disposition is appended. `validatePhaseChain` then has an explicit exception that accepts exactly one repeated non-launch phase when the first record is `PROVED_NONE` and the second is `MAY_EXIST`.

A fresh production-API probe created a continuation journal, wrote `QUIESCING_PRIOR / PROVED_NONE`, then wrote `QUIESCING_PRIOR / MAY_EXIST`. The second write resolved successfully, and `reconcile` accepted the file with two phase records (`phaseCount: 2`, dispositions `[PROVED_NONE, MAY_EXIST]`). This violates the required complete ordered chain on both the write and read boundaries and contradicts the r2 claim that duplicated/contradictory phase states fail closed.

The accepted duplicate cannot currently reach promotion because the later `MAY_EXIST` record blocks a following pre-launch phase. It is still MATERIAL under this review's explicit criterion that journal-corruption acceptance is material. The path is also production-relevant: a later possibly-written pre-launch RPC loss can need to strengthen a previously recorded no-successor-effect disposition.

**Required direction:** preserve the fail-safe strengthening without storing a duplicate semantic phase. Atomically replace the last same-phase `PROVED_NONE` record with `MAY_EXIST` (or introduce a distinct typed evidence transition outside the ordered phase chain), retain poison, and make every durable repeated phase reject on read. Add write-boundary and direct-file read-boundary probes for this exact transition.

## Convergence disposition

| Requirement | Result |
|---|---|
| Possibly-written quiesce abort | **PASS.** Re-injected pass-2 normalization failed the focused regression: the mutant returned `execution_failed` instead of `execution_outcome_unknown`. Timeout, unclean exit, and a fresh confirmed-clean-exit probe produced stale/failed/offline respectively, always with activity unknown, no successor launch, and a poisoned retained fence. |
| Handshake-wrapper expiry provenance | **PASS.** Changing the wrapper timeout to `proved_not_written` failed the focused provenance test; clean code preserves `possibly_written`. |
| Missing launch phase | **PASS.** Re-injected permissive parse/promotion behavior made the missing-phase regression fail because publication was admitted. Clean code rejects before publication, markers, session reports, or launch. |
| Duplicated launch phase | **PASS.** The same permissive behavior made the duplicated-launch regression fail because publication was admitted. Clean code rejects before every effect. |
| Success before handshake | **PASS.** A fresh direct-journal corruption probe reordered `SUCCESS_EVIDENCE_REPORTED` before `HANDSHAKE_RECONCILING`; clean code rejected before publication and marker writes. |
| All repeated/contradictory phases reject | **FAIL / MATERIAL.** `QUIESCING_PRIOR / PROVED_NONE → QUIESCING_PRIOR / MAY_EXIST` is durably appended and accepted by reconciliation. |
| Promotion requires full chain and exact staging | **PASS for the tested clean, missing-launch, duplicated-launch, and success-before-handshake shapes.** No probe published or wrote promotion/publication markers without the required suffix. |

## Mutation matrix

| Mutation / probe | Result |
|---|---|
| Re-inject pass-2 generic pre-launch error normalization | **KILLED** — quiesce regression observed `execution_failed` instead of required unknown outcome. |
| Handshake wrapper expiry marked `proved_not_written` | **KILLED** — provenance regression failed. |
| Permissive journal validator, missing `LAUNCH_ATTEMPTED` | **KILLED** — missing-phase regression reported missing expected rejection. |
| Permissive journal validator, duplicated `LAUNCH_ATTEMPTED` | **KILLED** — duplicated-phase regression reported missing expected rejection. |
| Fresh clean-exit possibly-written abort | **PASS** — offline/unknown, poison, no launch, no fabricated live/idle. |
| Fresh success-before-handshake journal | **PASS** — rejected before publication/markers. |
| Fresh repeated quiesce phase with stronger disposition | **SURVIVING BEHAVIOR / MATERIAL** — write and reconcile both accepted two records for one phase. |
| Registered Pi Unit 3 mutation suite | **15/15 KILLED.** |
| Prior spot-check: bypass `RpcPiSession` action gate | **KILLED** by the registered production-seam oracle. |
| Prior spot-check: replace SIGKILL escalation with SIGTERM | **KILLED** by the stubborn process-group oracle. |

## Full clean verification

1. **Rust group:** formatting, all-target build, workspace tests, and clippy with warnings denied — **PASS**.
2. **Contracts group:** generated drift, vectors, models, build, presentation conformance, and presentation meta-tests — **PASS**; 59 vectors, 19 promoted, 29 implementation checks, and 38 mutation witnesses.
3. **Operator-domain group:** **PASS, 28/28**.
4. **Pi-adapter group:** **PASS, 95/95** on the final full rerun; registered mutations **15/15 killed**. The first full run had one real offline RPC handshake timeout (94/95); its focused rerun and the complete second run passed.
5. **Web cockpit:** **PASS, 144/144**.
6. **CLI:** **PASS, 53/53** plus the real-core resource projection.
7. **token-commune adapter:** **PASS, 63/63**, including both real-core flows.

`git diff --check` passed. The tracked tree was clean before this review file was written. `/` retained 54 GiB free.

## Recommendation

**Return `research-handoff-pi-adapter-capability-rpc-process-supervisor` to `implementing`.** Keep the quiesce transport classification and full-chain promotion guards. Remove the stored duplicate-phase exception while retaining atomic escalation to `may_exist` and poison, add read/write regression probes for that exact transition, and run thorough pass 4.
