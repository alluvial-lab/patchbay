---
id: restart-continuation-orchestration-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-restart-continuation-orchestration
created: 2026-08-15
updated: 2026-08-15
---

# Thorough rereview — Unit 9 generic restart-continuation orchestration

## Verdict

**MATERIAL** — return `research-handoff-spawn-restart-continuation-orchestration` to `implementing`.

The pass-1 phase-spine and lockdown findings are structurally closed: continuation staging and promotion now reconstruct the durable ordered chain, missing quiesce/prior-termination evidence fails closed, staged-before-handshake is rejected, and pending/active lockdown makes session-list spawn inert. Generated context status also flows correctly in the current implementation. Its core-carriage verification is not mutation-surviving, however: a core mutant that replaces every adapter report with `RESUMED` remains green, and no conformance vector constrains the new enum or carriage fields.

Review mode: independent fresh-context story rereview, effective weight `thorough` pass 2, implementation range `c593a15..e44f89b` with fix commit `e44f89b` inspected directly.

## Findings

### MATERIAL — adapter-reported context carriage has a vacuous core oracle and no vector trace

**Locations:** `server/src/adapter_service.rs:544`; `server/src/adapter_service/tests.rs:2755,2784-2787`; `contracts/vectors/`

Production currently copies `report.continuation_context_status` into staged evidence, and downstream promotion/web/CLI consumers preserve or derive the generated value. The server regression does not independently establish that guarantee: it supplies `RESUMED` and then expects `RESUMED`. Replacing the copy with a core-authored constant `ContinuationContextStatus::Resumed` left `exact_continuation_report_stages_n_plus_one_without_publishing_it` green (exit 0). The web model test uses `NEW_CONTEXT`, but directly constructs both sides of the already-promoted envelope and cannot catch invention at core ingress.

No conformance vector references `ContinuationContextStatus`, `SessionReport.continuation_context_status`, or `SpawnSuccessorEvidenceStaged.continuation_context_status`; `check:vectors` therefore remains green without exercising the proto addition. This is a material trustworthy-verification gap under the requested classification: a future core-invented context outcome can pass the claimed core-carriage oracle.

**Required fix:** drive a non-`RESUMED` adapter value (preferably table-test all `RESUMED` / `NEW_CONTEXT` / `UNKNOWN`) through authenticated exact-continuation ingress, staged storage, promotion/replay, and the operator projection, then assert exact equality to independent input. Add or extend the repo-pattern conformance vector with the new enum/field references and an implementation check that kills the constant-`RESUMED` mutant.

## Checklist disposition

- **Phase-order spine:** pass. Durable replay requires `delivered → quiescing_prior → prior_terminated → launch_attempted → external_identity_known → handshake_reconciling → staged → success_evidence_reported`; staging itself rejects an incomplete prefix. Missing quiesce, missing prior termination, duplicate-identity-for-handshake, and stage-before-handshake probes all fail closed.
- **Context-status carriage:** implementation pass, assurance fail as the finding above. The generated enum is the sole Rust/TypeScript vocabulary; local duplicates and hard-coded surface `unknown` output are gone; ingress validates non-sentinel continuation status and exact staged/report equality; promotion/web/CLI carry it. Repository grep found no surface default remnant, but also no vector reference.
- **Lockdown gating:** pass. Sidebar spawn is disabled with an explicit reason for both pending and active posture; the same conditional re-enables it after both values clear without introducing a protocol state.
- **Regression / neighboring units:** pass. Fence activation, handshake requirement, staged-only N+1 invisibility, and no-auto-relaunch remain mutation-sensitive. Unit 4 exact generation/pre-state and tombstone validation tests pass. Unit 7's 12-test sole completion/promotion owner suite passes; no alternate completion owner was introduced.
- **Replay/publication:** pass. Staged evidence remains reservation-only; promotion is the first event that publishes authority/session/claim/command aggregates, and the 12 completion-driver crash/replay tests remain green.

## Mutation matrix

Every source mutant or input probe was applied alone on the main tree, run with a focused test, reverted with `git restore`, and followed by a clean `git status --short`.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Omit the quiesce link from the continuation prefix | `continuation_phase_chain_rejects_missing_and_out_of_order_durable_evidence` | **KILLED**, exit 101 at `missing quiesce evidence must not become promotion-ready`. |
| Omit the prior-terminated link | same phase-chain test | **KILLED**, exit 101 at `missing prior-terminated evidence must not become promotion-ready`. |
| Permit staging after identity and ignore the handshake-before-stage bound | same phase-chain test | **KILLED**, exit 101 at `stage before handshake must not become promotion-ready`. |
| Replace the handshake fixture with a second identity checkpoint (pass-1 probe) | `promotion_producer_keeps_earliest_exact_success_result_retry_on_both_sides_of_staging` | **KILLED**, exit 101; staged replay rejected the incomplete prefix. |
| Remove only the later promotion-time handshake-before-stage comparison | phase-chain test | **SURVIVED as behaviorally equivalent**; the earlier staged-event fold still rejected stage-before-handshake, so no promotion bypass existed. |
| Hard-code staged context to core-authored `RESUMED` instead of copying the report | `exact_continuation_report_stages_n_plus_one_without_publishing_it` | **SURVIVED**, exit 0; material finding above. |
| Force the continuation delivery fence open | `continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically` | **KILLED**, exit 101; expected replacement-pending, observed open. |
| Publish the staged successor into the live session map before promotion | `exact_continuation_report_stages_n_plus_one_without_publishing_it` | **KILLED**, exit 101; N+1 appeared live before promotion. |
| Disable durable managed-spawn delivery suppression | `abnormal_stream_loss_poisons_managed_spawn_and_prevents_redelivery` | **KILLED**, exit 101; poisoned work was offered again. |
| Ignore lockdown when rendering sidebar spawn | `session-list spawn action is inert while lockdown is pending or active` | **KILLED**, exit 1; the action became enabled. |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — including 35 runtime-evidence/promotion tests, 39 spawn-claim tests, 82 server unit tests, 12 spawn-completion tests, and doctests; warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 26/26.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and core-restart e2e.
5. `cd web-cockpit && npm test`: **PASS** — 132/132, including browser build.
6. `cd cli && npm test`: **PASS** — 48/48 plus the real-core resource projection smoke test.

The tracked tree was clean before mutation work, after every restore, before each full-suite group, and before this review file was written. `git diff --check` passed. No temporary worktree was created; `/` retained 57G free after verification.

## Recommendation

**Return to implementing.** Keep the current generated carriage and phase/lockdown fixes, add an independent end-to-end context-status oracle plus vector trace that kills core invention, rerun the clean-tree suite, and submit thorough pass 3.
