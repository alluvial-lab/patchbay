---
id: idempotency-duplicate-handling-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-idempotency-duplicate-handling
created: 2026-08-15
updated: 2026-08-15
---

# Re-review 2: duplicate, ambiguous-outcome, and claim reconciliation

## Verdict

**MATERIAL** — return the story to `implementing`.

The two pass-2 findings are closed: `launch_attempted + identified`, including `failure_code = unspecified`, now commits poison, retains the fence, reserves the exact runtime, suppresses delivery, and replays; and the adapter oracle reaches a real `Delivered → Running → ambiguous Result` prefix and kills removal of `Running`. The row sweep found a separate phase-table deviation in abnormal stream-loss reconciliation: a claim that did not exist until after the delivery stream was gone is still mislabeled `offered/may_exist` and durably poisoned.

## Findings

### MATERIAL — abnormal stream loss poisons accepted claims that the lost stream never offered

**Locations:** `server/src/adapter_service.rs:626`, `server/src/adapter_service.rs:646`, `server/src/adapter_service.rs:1934`; missing negative case beside `server/src/adapter_service/tests.rs:3659`.

`poison_ambiguous_spawn_claims_for_adapter` selects every active managed-spawn claim whose command is `Accepted | Delivered | Running`. It maps every `Accepted` or `Delivered` candidate to `SpawnExecutionPhase::Offered / MayExist`, without proving that the lost stream ever offered that exact accepted command. The disconnect callback catches the command projection up after the stream is gone, so it can include claims accepted after the drop and before the callback acquires the decision gate.

Reviewer phase probe established an empty delivery stream, held the shared decision gate, dropped the stream, appended the replacement claim while the callback was blocked, then released the gate. The stream was already gone before the claim existed and therefore could not have offered it. The callback nevertheless appended execution evidence and changed the claim from `Active` to `PoisonedPendingReconciliation`; the focused oracle failed with exit 101 (`left: PoisonedPendingReconciliation`, `right: Active`). The probe was restored immediately.

This violates the failure-phase table's `claim accepted, before any offer -> active` row and creates a durable false poison: delivery remains suppressed and the continuation fence stays blocked until reconciliation or target abandonment even though this stream had no external-effect opportunity. The existing abnormal-loss test consumes the exact delivery before dropping the stream, so it protects the genuinely offered case but cannot detect this deviation.

**Required direction:** bind disconnect reconciliation to exact per-stream offer evidence. Poison `Delivered`/`Running` and `Accepted` commands whose delivery was actually emitted into the lost stream, but do not infer `Offered` solely from `CommandState::Accepted` after a catch-up. Preserve effect-before-ack ambiguous Result poisoning, which has independent authenticated Result evidence. Add a barrier-controlled accepted-after-drop oracle proving the claim remains active, has no spawn-execution evidence, and is deliverable on a replacement stream; retain the existing offered-before-drop poison oracle.

## Failure-phase table sweep

| Failure-phase row | Observed claim/fence consequence | Result |
|---|---|---|
| Authority/validation rejection before acceptance | no claim/fence | Pass |
| Claim accepted before any offer | closed core proof releases; silence/terminality alone stays active; **disconnect catch-up can falsely poison a never-offered claim** | **MATERIAL** |
| Quiescing prior, prior still running | proved-none remains active until renewed exact prior-N liveness; may-exist poisons; fence retained | Pass |
| Prior terminated before launch | proved-none remains active until renewed exact prior-N liveness; may-exist poisons; fence retained | Pass |
| Launch attempted, identity unknown | may-exist poisons; delivery suppressed; fence retained | Pass |
| Launch attempted, identity known | identified including `unspecified` poisons, reserves exact runtime, rejects competing ownership, suppresses delivery, and replays | Pass |
| External identity known / handshake incomplete | unspecified identified progress remains active and reserved; identified failure poisons | Pass |
| Success evidence reported | unspecified identified progress remains active through restart and promotion; later failure evidence poisons | Pass |
| Atomic promotion | separate promotion path consumes the original active/poisoned claim | Pass |
| Unexplained stream loss after actual offer/running | poisons and suppresses redelivery; running commands also terminalize unknown | Pass |
| Operator abandonment | terminally consumes claim and clears the fence without generation reuse | Pass |

## Mutation matrix

Every mutant was applied alone on the main tree, exercised by one focused test, and reverted with `git restore`. The tree was clean after each probe.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Pass-2 survivor: restore the old failure-only rule for `launch_attempted + identified + unspecified` | `storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row` | **Killed**, exit 101 at `launch_attempted/identified/unspecified`: no disposition append |
| Pass-2 survivor: remove `Running` from ambiguous-Result poison eligibility | `ambiguous_spawn_results_with_or_without_ack_poison_the_exact_claim` | **Killed**, exit 101 from the real running pre-state: claim remained active |
| Fresh phase-table mutant: poison `handshake_reconciling + identified + unspecified` | storage/replay consequence matrix | **Killed**, exit 101: false disposition append on the progress row |
| Remove identified-runtime reservation from execution-evidence replay | storage/replay consequence matrix | **Killed**, exit 101 on launch-identified runtime ownership |
| Remove `Accepted` from ambiguous-Result poison eligibility | ambiguous-Result oracle | **Killed**, exit 101 on accepted-before-ack cancellation |
| Poison successful identified progress | `successful_identified_evidence_stays_active_and_reaches_original_claim_promotion` | **Killed**, exit 101: unexpected poison disposition |
| Bypass the logical-target current/reserved reverse-owner check | `identified_evidence_respects_current_and_reserved_logical_target_owners_after_restart` | **Killed**, exit 101: competing owner was admitted |
| Release a claim directly on terminal command state | `terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant` | **Killed**, exit 101: observed released instead of active |
| Admit no-effect release without closed proof validation | `silence_without_delivered_ack_is_not_no_effect_proof` | **Killed**, exit 101: silence/terminality release was admitted |
| Remove spawn execution evidence from delivery suppression | `reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch` | **Killed**, exit 101: original attempt was not suppressed |
| Ignore the claim-level external-runtime reverse owner | `identified_runtime_is_reserved_to_its_original_claim_at_ingress` | **Killed**, exit 101: second claim was accepted |
| Clear the continuation fence on poison | `typed_external_effect_evidence_poison_transition_fires_and_retains_fence` | **Killed**, exit 101: replacement fence disappeared |
| Behavioral phase probe: accept the claim only after the stream is gone | temporary accepted-after-drop assertion in the abnormal-stream-loss test | **Exposed MATERIAL**, exit 101: never-offered claim was poisoned |

The pass-1 four and pass-2 seven regression kills therefore remain killed; the two pass-2 survivors are now killed by non-vacuous oracles. The fresh no-false-poison mutant is also killed, but the separate real disconnect path still violates that same table row.

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (39 spawn-claim tests, 31 runtime-evidence/promotion tests, 77 server unit tests, all integration/property tests and doctests, warnings-denied clippy).
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 17 promoted, 22 implementation checks, 38 mutation witnesses; generated artifacts clean).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38, including the real core/adapter restart e2e).
- Final pre-review-file `git status --short`, `git diff --check`, and generated-contract diff: **PASS / clean**.

## Recommendation

**Return to implementing.** Correct the accepted-never-offered disconnect classification, add the barrier-controlled negative oracle while preserving the actual-offer poison case, rerun the clean-tree suite, and submit another thorough review pass.
