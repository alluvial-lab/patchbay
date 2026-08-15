---
id: idempotency-duplicate-handling-rereview4-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-idempotency-duplicate-handling
created: 2026-08-15
updated: 2026-08-15
---

# Re-review 4: duplicate, ambiguous-outcome, and claim reconciliation

## Verdict

**CLEAN** — advance the story to `done`.

Round 4 closes the pass-4 relaunch defect. A managed-spawn offer marker commits and enters `CommandIndex` before the outer stream can yield the delivery; replay reconstructs the same durable offer-decision set; startup converts an offered active claim to the existing `offered / may_exist / execution_outcome_unknown` poison path; and a marker-free accepted claim stays active and deliverable. The pass-4 restart probe now passes, both new offer-path mutants are killed, all 13 prior mutants remain killed, and the clean-tree full suite passes.

## Findings

No material findings or nits.

## Durable marker and crash-window adjudication

The marker append at `server/src/adapter_service.rs:1118` is a standalone SQLite audit transaction: the audit event and audit index commit together (`core/src/storage/rusqlite.rs:1856`), but that database transaction cannot include the later gRPC stream yield at `server/src/adapter_service.rs:1195`. The decision gate and stream epoch order the marker against claim/stream decisions; they do not make storage and transport one transaction.

That leaves the unavoidable at-most-once boundary split:

1. **Crash before marker commit:** there is no durable offer decision. The claim remains `active`, retains its fence, has no manufactured execution evidence, and is deliverable on a replacement stream. `abnormal_stream_loss_leaves_claim_accepted_after_drop_active_and_redeliverable` passes.
2. **Marker commits, process dies before the adapter observes the yielded item:** restart poisons the claim. A fresh production-path prefix probe committed the marker without returning an outer-stream item and observed `PoisonedPendingReconciliation`, not `Active` (the physical-visibility assertion failed with exit 101). This is conservative over-poison relative to physical receipt, but not a safety or phase-table deviation: the marker commit is the specified durable offer/responsibility linearization point. The durable prefix cannot distinguish this crash from yield-then-crash, so treating both as ambiguous is required to prevent non-idempotent relaunch. “Never offered” at the replay contract means no durable offer marker.
3. **Delivery before marker:** clean code has no such path. `DeliveryTail` polls the marker future to `Ok(true)` before returning the item. A deliberate yield-before-marker reorder was killed by the restart oracle with exit 101; the durable marker was absent after the offered item.

The marker-only prefix is therefore fail-safe, while the dangerous delivery-without-marker prefix is excluded and mutation-sensitive.

## Failure-phase table sweep

| Failure-phase row | Observed consequence | Result |
|---|---|---|
| Authority/validation rejection before acceptance | no claim/fence | Pass |
| Claim accepted before durable offer | marker-free claim remains active, fenced, evidence-free, and deliverable; only closed core proof releases | Pass |
| Quiesce begun, prior still running | proved-none waits for renewed exact prior-N liveness; ambiguity poisons and retains fence | Pass |
| Prior terminated before launch | proved-none waits for renewed exact prior-N liveness; ambiguity poisons and retains fence | Pass |
| Launch attempted, identity unknown | `may_exist` poisons, suppresses delivery, retains fence, and replays | Pass |
| Launch attempted, identity known | `identified`, including `unspecified`, reserves and poisons; reverse ownership remains exclusive | Pass |
| External identity known / handshake incomplete | unspecified progress remains active and reserved; failure evidence poisons | Pass |
| Success evidence reported | unspecified identified progress remains active through restart/staging and reaches promotion; failure poisons | Pass |
| Atomic promotion committed | exact active/poisoned claim becomes promoted and the old fence is consumed | Pass |
| Unexplained stream loss after durable offer/delivery/running | offered active claims poison at hot disconnect or startup; no redelivery after restart | Pass |
| Operator abandonment | permanently consumes the generation and clears the fence without making it reclaimable | Pass |

The storage/replay consequence matrix, real accepted/delivered/running Result matrix, offered restart regression, marker-free disconnect regression, and full suite all pass on the restored tree.

## Mutation matrix

Every mutant was applied alone on the main tree, exercised by a focused oracle, and restored with `git restore`. The tree was clean after every probe.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Remove `AuditRecord` offer reconstruction from `CommandIndex` | `offered_without_ack_survives_core_restart_as_ambiguous_without_redelivery` | **Killed**, exit 101; marker failed to enter the projection before delivery |
| Yield managed delivery before polling the marker future | same restart oracle | **Killed**, exit 101; emitted item had no durable marker |
| Fresh marker-only prefix, then restart, with a physical-never-seen `Active` assertion | temporary production-path prefix probe | **Assertion failed**, exit 101; observed conservative `PoisonedPendingReconciliation`; adjudicated as the durable-offer linearization rule |
| Unconditionally poison every `Accepted` claim on disconnect | accepted-after-drop negative oracle | **Killed**, exit 101 |
| Restore failure-only handling for `launch_attempted + identified + unspecified` | storage/replay consequence matrix | **Killed**, exit 101 |
| Remove `Running` from ambiguous-Result poison eligibility | accepted/delivered/running Result matrix | **Killed**, exit 101 |
| Remove `Accepted` from ambiguous-Result poison eligibility | accepted/delivered/running Result matrix | **Killed**, exit 101 |
| Falsely poison `handshake_reconciling + identified + unspecified` | storage/replay consequence matrix | **Killed**, exit 101 |
| Falsely poison successful identified progress | identified-success-to-promotion oracle | **Killed**, exit 101 |
| Omit identified-runtime reservation from evidence replay | storage/replay consequence matrix | **Killed**, exit 101 |
| Bypass logical-target current/reserved owner validation | restart owner-collision oracle | **Killed**, exit 101 |
| Bypass claim-level external-runtime reverse ownership | exact original-claim ingress oracle | **Killed**, exit 101 |
| Release directly on failed/cancelled/expired terminal state | terminal-state claim oracle | **Killed**, exit 101 |
| Bypass closed no-effect proof validation and infer release from silence/terminality | silence-without-ack oracle | **Killed**, exit 101 |
| Omit execution-evidence delivery suppression | ambiguity replay/suppression oracle | **Killed**, exit 101 |
| Clear the continuation fence on poison | typed poison/fence oracle | **Killed**, exit 101 |

A narrower control mutation that removed only late core-terminal field checks survived the silence oracle because the earlier closed evidence-family/type check still rejected the non-evidence event. It was not equivalent to the required no-effect-bypass mutant; bypassing the complete proof validator was killed as shown above.

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**, including 39 spawn-claim tests, 31 runtime-evidence/promotion tests, 79 server unit tests, all workspace integration/property tests and doctests, and warnings-denied clippy.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; generated paths clean, 55 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart e2e.
- Final `git status --short`, `git diff --check`, and generated-contract diff: **PASS / clean** before this review file was written.

## Recommendation

**Advance to done.** The pass-4 relaunch bypass is closed without weakening marker-free claim availability or any prior poison/release/ownership invariant.
