---
id: restart-continuation-orchestration-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-restart-continuation-orchestration
created: 2026-08-15
updated: 2026-08-15
---

# Thorough rereview pass 3 — Unit 9 generic restart-continuation orchestration

## Verdict

**CLEAN** — advance `research-handoff-spawn-restart-continuation-orchestration` to `done`.

The pass-2 material finding is closed. The three generated non-sentinel values travel independently through authenticated ingress, durable staged storage, replay-derived promotion, and the operator-facing subscription projection without core invention. Both the core oracle and the registered draft-vector implementation check kill a constant-`RESUMED` mutant and a narrower `UNKNOWN → RESUMED` mutant. The pass-1 phase spine, lockdown gate, and safety oracles remain green.

Review mode: independent fresh-context story rereview, effective weight `thorough` pass 3, implementation range `879cccb..3c74e9f` with `3c74e9f` inspected directly.

## Findings

None.

## Checklist disposition

- **Non-vacuous carriage oracle:** pass. `RESUMED`, `NEW_CONTEXT`, and `UNKNOWN` each enter through authenticated exact-continuation `SessionReport` ingress. The test asserts exact equality to the independent table input in the persisted staged envelope and embedded report, in a fresh replay-derived promotion and its embedded report, and after `operator_facing_subscribe_event` applies the production subscription projection. A second replay is asserted byte-equivalent to the first promotion.
- **Vector trace:** pass. `spawn-continuation-context-status-carriage.json` names `patchbay.ContinuationContextStatus`, `patchbay.SessionReport.continuation_context_status`, and `patchbay.SpawnSuccessorEvidenceStaged.continuation_context_status`. `check:vectors` executes its registered `rust-server:continuation_context_status_carriage` case: both source mutants fail specifically in that case, while the clean run reports 24 executed implementation checks. Removing the promoted-only request filter is additive; runner allowlisting, per-vector duplicate checks, exact requested/reported id matching, promoted invariant checks, promoted coverage, mutation witnesses, and traceability drift checks remain unchanged.
- **Regression:** pass. The clean phase-chain oracle still rejects missing quiesce, missing prior-runtime outcome, handshake-before-identity, stage-before-handshake, and phase-before-delivery cases. Focused clean oracles for the four pass-1 mutation seams remain green: accepted-claim fence activation, handshake-required readiness, staged N+1 invisibility before promotion, and poisoned-stream redelivery suppression. Pending and active lockdown both keep session-list spawn inert.
- **Integration:** pass. No new protocol vocabulary or alternate completion/publication owner was introduced; generated bindings remain drift-free and the complete clean-tree suite passes.

## Mutation matrix

Each valid source mutant was applied alone to `server/src/adapter_service.rs` on the main tree, tested through both required paths, restored with `git restore`, and followed by a clean status check.

| Mutant | Core carriage oracle | Vector implementation check | Result |
|---|---|---|---|
| Replace adapter copy with constant generated `RESUMED` | **KILLED**, exit 101 when `NEW_CONTEXT` reached staging; exact report/envelope disagreement failed closed | **KILLED**, `check:vectors` exit 1; `continuation_context_status_carriage` rejected `NEW_CONTEXT` | Both required independent paths fail the pass-2 mutant |
| Map only generated `UNKNOWN` to `RESUMED`, preserve other values | **KILLED**, exit 101 when `UNKNOWN` reached staging | **KILLED**, `check:vectors` exit 1; the registered case rejected `UNKNOWN` | Subtle single-value invention is also detected |
| Clean source after restore | **PASS**, 1/1 focused core oracle | **PASS**, 56 vectors / 24 implementation checks / 38 mutation witnesses | Intended carriage is accepted |

## Focused regression evidence

- `continuation_phase_chain_rejects_missing_and_out_of_order_durable_evidence`: **PASS** — 1/1.
- `promotion_producer_keeps_earliest_exact_success_result_retry_on_both_sides_of_staging`: **PASS** — 1/1.
- `continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically`: **PASS** — 1/1.
- `exact_continuation_report_carries_every_context_status_without_core_invention`: **PASS** — 1/1, including staged-only N+1 invisibility.
- `abnormal_stream_loss_poisons_managed_spawn_and_prevents_redelivery`: **PASS** — 1/1.
- `session-list spawn action is inert while lockdown is pending or active`: **PASS** — 1/1.

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace targets, tests, doctests, and warnings-denied clippy; includes 35 runtime-evidence/promotion, 39 spawn-claim, 82 server-unit, and 12 spawn-completion tests.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — generated drift clean; 56 vectors, 17 promoted vectors, 24 executed implementation checks, 38 killed registered mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 26/26.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and core-restart e2e.
5. `cd web-cockpit && npm test`: **PASS** — 132/132, including browser build.
6. `cd cli && npm test`: **PASS** — 48/48 plus the real-core resource projection smoke test.

The tracked tree was clean before mutation work, after every restore, before the full suite, and before this review file was written. `git diff --check` passed. No temporary worktree was created; `/` retained 57G free after verification.

## Recommendation

**Advance to done.** Pass 3 has no material findings or nits, both required carriage mutation paths fail closed, the vector is demonstrably executed, prior safety probes remain green, and the full clean-tree suite passes.
