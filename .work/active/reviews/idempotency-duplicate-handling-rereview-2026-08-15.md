---
id: idempotency-duplicate-handling-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-idempotency-duplicate-handling
created: 2026-08-15
updated: 2026-08-15
---

# Re-review: duplicate, ambiguous-outcome, and claim reconciliation

## Verdict

**MATERIAL** — return the story to `implementing`.

The three pass-1 fixes work on their covered paths: accepted-before-ack ambiguous Results poison, successful identified evidence remains active through restart and promotion, and current/reserved/tombstoned logical-target owners reject a competing claim with zero writes. Row-by-row review found one unhandled allowed evidence cell, however, and mutation review found that the promised `running` Result coverage is vacuous.

## Findings

### MATERIAL — allowed identified-poison cells can roll back instead of poisoning

**Locations:** `core/src/storage/rusqlite.rs:3353`, `core/src/storage/rusqlite.rs:3423`, `core/src/session/spawn_claim.rs:1118`

The closed phase/disposition registry permits `launch_attempted + identified`, and the authoritative table requires that cell to poison the claim and retain its fence. The writer correctly excludes `launch_attempted` from `identified_progress_without_failure` and selects `PoisonedPendingReconciliation`, but replay validation of the generated disposition still requires `poison_failure(typed.failure_code)`. `Unspecified` is not a poison failure, so the transaction rejects the disposition and rolls back the identified evidence itself. The claim remains active with no durable execution-evidence suppression or runtime reservation. The same split also accepts identified evidence at boundary validation but rolls it back during disposition validation when it carries another canonical non-unspecified code outside the narrower `poison_failure` set.

Reviewer row probes: changing the first valid identified-evidence fixture to `LaunchAttempted / Identified / Unspecified`, and separately to `ExternalIdentityKnown / Identified / DeliveryRejected`, made `identified_runtime_is_reserved_to_its_original_claim_at_ingress` fail with exit 101: `claim poison requires typed ambiguous or identified external-effect failure evidence`. Each probe was restored immediately.

This contradicts the failure-phase table's launch-attempted poison outcome and `docs/PROTOCOL.md`'s explicit `launch_attempted + identified -> poison and retain the replacement fence` cell.

**Required direction:** make claim-consequence validation phase-aware rather than requiring the narrow ambiguity-failure set for every identified poison. An identified launch attempt must durably reserve the exact runtime, poison, suppress delivery, and retain the fence even when the failure code is `unspecified`. Add a storage/replay consequence matrix over every allowed phase/disposition cell so success-progress rows remain active, no-effect rows do not poison, and every poison row actually commits.

### MATERIAL — the `running` ambiguous-Result oracle is vacuous

**Locations:** `server/src/adapter_service.rs:583`, `server/src/adapter_service/tests.rs:3554`

Production currently admits `Accepted | Delivered | Running`, but `ambiguous_spawn_results_with_or_without_ack_poison_the_exact_claim` constructs only `Accepted` (no acknowledgement) and `Delivered` (acknowledgement) pre-states. It never advances the command to `Running`.

Fresh mutation: removing only `OperationState::Running` from `poison_ambiguous_spawn_result` survived all 77 server unit tests and every server integration test. Thus cancellation, expiry, and `execution_outcome_unknown` from the running pre-state can regress without an oracle, contrary to the explicit review checklist. The later doctest invocation in that mutant run hit a transient missing-crate artifact error; the relevant unit/integration suite had already passed, and the restored clean-tree full suite later passed exactly.

**Required direction:** add running-prestate cases for all three ambiguous failures (acknowledge delivery, durably advance to running, then report the terminal Result), asserting poison, retained fence, one evidence record, terminal command outcome, and suppression across replay/restart. Re-run the running-guard deletion mutant.

## Failure-phase table cross-check

| Phase/evidence row | Observed consequence | Review result |
|---|---|---|
| `accepted_not_offered + proved_none` | typed core proof may release; terminality/silence alone does not | Pass |
| `offered + proved_none` | valid refusal may release; continuation waits for renewed prior-N liveness | Pass |
| `offered + may_exist` | poisons; fence retained | Pass |
| `quiescing_prior/prior_terminated + proved_none` | proof remains active until renewed prior-N liveness, then releases | Pass |
| `quiescing_prior/prior_terminated + may_exist` | poisons; fence retained | Pass |
| `launch_attempted + may_exist` | poisons; fence retained | Pass |
| `launch_attempted + identified + failure` | poisons and reserves | Pass for covered ambiguity failures |
| `launch_attempted + identified + unspecified` | transaction rejects and rolls back evidence | **MATERIAL** |
| `external_identity_known/handshake_reconciling + identified + unspecified` | reserves runtime; claim remains active | Pass by code path; no false poison |
| `external_identity_known/handshake_reconciling + identified failure` | poisons only for the narrower `poison_failure` set; other boundary-admitted canonical failures roll back | **MATERIAL (same root)** |
| `success_evidence_reported + identified + unspecified` | active across restart; exact retry, staging, and original-claim promotion succeed | Pass |
| atomic promotion | original claim becomes promoted; fence consumed by promotion | Pass |

## Mutation matrix

Every killed mutant was applied alone on the main tree, run through a focused oracle, and reverted with `git restore`. The tree was clean after each probe.

| Mutant | Oracle | Result |
|---|---|---|
| Remove `Accepted` from ambiguous-Result poisoning | `ambiguous_spawn_results_with_or_without_ack_poison_the_exact_claim` | **Killed**, exit 101; no-ack cancellation left the claim active |
| Route successful identified evidence back into poison/rejection | `successful_identified_evidence_stays_active_and_reaches_original_claim_promotion` | **Killed**, exit 101; evidence was rejected by poison validation |
| Remove the logical-target reverse-owner consultation | both `identified_evidence_respects_*` tests | **Killed**, exit 101; current/reserved/tombstone conflicts appended |
| Release on failed/cancelled/expired command terminality | `terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant` | **Killed**, exit 101; observed released instead of active |
| Infer no effect from terminal state plus acknowledgement silence | `silence_without_delivered_ack_is_not_no_effect_proof` | **Killed**, exit 101; unproved release was admitted |
| Omit spawn execution evidence from delivery suppression | `reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch` | **Killed**, exit 101; relaunch suppression disappeared |
| Ignore the claim-level external-runtime reverse owner | `identified_runtime_is_reserved_to_its_original_claim_at_ingress` | **Killed**, exit 101; second claim was accepted |
| Fresh: clear the continuation fence on poison | `typed_external_effect_evidence_poison_transition_fires_and_retains_fence` | **Killed**, exit 101; replacement fence disappeared |
| Fresh: remove `Running` from ambiguous-Result poisoning | full `patchbay-core-server` unit/integration suite | **SURVIVED**; 77/77 unit tests and all integration binaries passed |

The seven fix-round kill claims are therefore reproduced. The additional surviving mutant is the second MATERIAL finding.

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** on the final exact rerun. The first clean attempt reached doctests and hit a transient rustdoc missing-crate artifact error; no cargo process remained, the referenced artifacts existed, and the unchanged exact rerun passed build, all tests/doctests, and warnings-denied clippy.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38, including the real restart e2e).
- Final pre-review-file `git status --short`, generated-contract diff, and `git diff --check`: **PASS / clean**.

## Recommendation

**Return to implementing.** Fix the allowed identified-launch poison cell, add a row-complete storage/replay consequence oracle, and add mutation-sensitive running-state ambiguous-Result coverage before the next thorough pass.
