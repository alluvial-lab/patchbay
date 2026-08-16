---
id: pi-rpc-supervisor-rereview3-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-rpc-process-supervisor
created: 2026-08-16
updated: 2026-08-16
---

# Thorough rereview pass 4 — Pi Unit 3 claim-aware RPC process supervisor

**Verdict: CLEAN.** Commit `aeb6b43` closes the pass-3 contradictory duplicate-phase finding. The write API now preserves a one-record-per-semantic-phase journal while allowing only monotonic worse-news replacement, and both reconciliation paths fail closed on every durable repeated phase. No material finding or nit remains in this convergence scope.

Review mode: independent fresh-context delegated story review, effective weight `thorough`, pass 4 over the pass-3 fix `aeb6b43`. No temporary worktree was created. Manual mutations and the fresh temporary corruption probe were restored with `git restore`; the tracked tree was clean before this review file was written.

## Findings

None.

## Convergence disposition

| Requirement | Result |
|---|---|
| Identical `proved_none` repeat | **PASS.** Idempotent: one durable phase, original timestamp retained, poison remains false. |
| Identical `may_exist` repeat | **PASS.** Idempotent: one durable phase, original timestamp retained, poison remains true. |
| `proved_none → may_exist` | **PASS.** Accepted as monotonic strengthening, atomically replaces the last phase record, records the newer timestamp, and retains poison. |
| `may_exist → proved_none` | **PASS.** Rejected without replacing the stronger durable record or clearing poison. |
| Durable duplicate phases | **PASS.** `validatePhaseChain` has one exception-free observed-phase set, so every repeated legal phase fails before projection. The committed stronger-`QUIESCING_PRIOR` corruption probe rejects through both `reconcile` and `reconcileAll`; a fresh complete-chain probe duplicating identical `HANDSHAKE_RECONCILING / IDENTIFIED` evidence also rejected through both paths. |
| Oracle quality | **PASS.** The downgrade mutation reached the write boundary and survived validation, then the table failed specifically with `Missing expected rejection`; clean behavior passed all four rows. The fresh corruption used valid JSON, a legal phase/disposition, and an otherwise complete semantic chain, so rejection was not attributable to malformed framing or a missing prerequisite. |

## Mutation matrix

| Mutation / probe | Result |
|---|---|
| Re-inject `may_exist → proved_none` replacement and clear poison | **KILLED.** Focused monotonicity table failed with `Missing expected rejection`. |
| Restore clean source and run the four-row monotonicity table | **PASS, 1/1.** Strengthening, downgrade rejection, and both idempotent rows matched exact phase count, disposition, timestamp, and poison state. |
| Committed direct-file `QUIESCING_PRIOR / proved_none → may_exist` duplicate | **PASS.** `reconcile` and `reconcileAll` both rejected it as duplicated or contradictory. |
| Fresh valid complete-chain identical `HANDSHAKE_RECONCILING / identified` duplicate | **PASS.** `reconcile` and `reconcileAll` both rejected it before projection. |
| Registered Pi Unit 3 mutation suite on the restored clean tree | **15/15 KILLED.** The runner restored every mutation; `git status --short` and `git diff --check` were clean afterward. |

## Full clean verification

1. **Rust group:** `cargo fmt --all -- --check`; `cargo build --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. **Contracts group:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests — **PASS**; 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 mutation witnesses.
3. **Operator-domain group:** `npm test` — **PASS, 28/28**.
4. **Pi-adapter group:** `npm test` — **PASS, 97/97**; registered mutations — **15/15 killed**.
5. **Web cockpit:** `npm test` — **PASS, 144/144**.
6. **CLI:** `npm test` — **PASS, 53/53** plus the real-core resource projection.
7. **token-commune adapter:** `npm test` — **PASS, 63/63**, including both real-core flows.

`git diff --check` passed. `/` retained 54 GiB free; no temporary worktree was used.

## Recommendation

**Accept the pass-3 fix and advance the parent review flow.** The required monotonic effect-claim semantics are non-vacuously checked, durable duplicate phases fail closed through both startup and single-claim reconciliation, the registered mutation suite is 15/15, and the complete clean suite is green.
