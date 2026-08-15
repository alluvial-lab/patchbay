---
id: idempotency-duplicate-handling-rereview3-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-idempotency-duplicate-handling
created: 2026-08-15
updated: 2026-08-15
---

# Re-review 3: duplicate, ambiguous-outcome, and claim reconciliation

## Verdict

**MATERIAL** — return the story to `implementing`.

Round 3 closes the pass-3 same-process race: a claim accepted after a stream disappears stays active, retains its fence, has no manufactured execution evidence, and is deliverable on a replacement stream; unconditional accepted-state poisoning is mutation-sensitive. The new stream-to-claim set is process-local, however, so an accepted spawn actually emitted before an ungraceful core restart loses its offer fact and is automatically re-delivered after replay. That is a non-idempotent relaunch bypass.

## Findings

### MATERIAL — accepted-but-offered evidence disappears across core restart

**Locations:** `server/src/adapter_service.rs:985`, `server/src/adapter_service.rs:1011`, `server/src/adapter_service.rs:1929`; replay initialization at `server/src/adapter_service.rs:256`.

`DeliveryTail.offered_spawn_claims` is an in-memory `HashSet` populated only when the outer stream yields a managed delivery. Its only consumer is the same process's `Drop`/error callback. No durable event records that an `Accepted` managed spawn was emitted, and service reconstruction replays only the durable command/claim/session projections. Therefore an ungraceful core process loss also loses the only distinction between accepted-and-never-offered and accepted-and-offered-before-ack.

Reviewer probe: append a managed claim, poll its delivery without acknowledging it, suppress the in-process `Drop` callback to model abrupt process death, rebuild `AdapterControlServiceImpl` from the unchanged durable prefix, attach generation 2, and open a replacement delivery stream. The replacement stream immediately returned the same command; the expected no-relaunch assertion failed with exit 101. The durable prefix contains neither `SpawnExecutionEvidence` nor another offer marker, so replay reconstructs the command as deliverable `Accepted` and the claim as `Active` even though the pre-crash adapter may already have launched it.

This violates the story's restart/replay and ambiguous-outcome obligation and the feature's `offered`/unexplained-loss rows. It also makes the requested “skip one mapping-rebuild path on replay” mutant impossible to construct: there is no replay reconstruction path to skip; the omission is production behavior.

**Required direction:** make the exact accepted-managed-spawn offer fact durable before yielding the non-idempotent delivery, or introduce an equivalently durable per-stream/attachment offer record whose fold reconstructs the same claim set and whose crash/reconnect handling suppresses or poisons only those offered claims. Preserve the separate atomic `accepted_not_offered` proof/release path: a claim with no durable offer must remain active and releasable under the existing closed proof contract. Add an ungraceful file-backed or equivalent restart oracle proving offered-before-ack cannot relaunch while accepted-never-offered remains deliverable.

## Per-stream and failure-phase sweep

The same-process mapping itself is exact: a temporary two-stream probe passed. Stream A emitted one fresh managed claim; stream B was opened but never polled, a different fresh claim was accepted, and stream B was dropped. Neither stream A's offered claim nor stream B's never-emitted claim was poisoned by B's loss, and no execution-evidence event was appended. The remaining defect is loss of that distinction at process/replay scope.

| Failure-phase row | Observed consequence | Result |
|---|---|---|
| Authority/validation rejection before acceptance | no claim/fence | Pass |
| Claim accepted before any offer | round-3 barrier case remains active, fenced, evidence-free, replacement-deliverable; closed core proof can release | Pass |
| Quiesce begun, prior still running | proved-none waits for renewed exact prior-N liveness; ambiguity poisons and retains fence | Pass |
| Prior terminated cleanly before launch | proved-none waits for renewed exact prior-N liveness; ambiguity poisons and retains fence | Pass |
| Launch attempted, identity unknown | `may_exist` poisons, suppresses delivery, retains fence, and replays | Pass |
| Launch attempted, identity known | `identified`, including unspecified failure, reserves and poisons; reverse ownership remains exclusive | Pass |
| External identity known / handshake incomplete | unspecified progress remains active and reserved; failure evidence poisons | Pass |
| Success evidence reported | unspecified identified progress stays active through restart/staging and reaches original-claim promotion; failure poisons | Pass |
| Atomic promotion committed | original active/poisoned claim becomes promoted and the old fence is consumed | Pass |
| Unexplained stream loss after durable delivered/running or same-process actual offer | delivered/running and same-process offered-before-ack poison; **accepted actual offer is forgotten on ungraceful core restart and relaunches** | **MATERIAL** |
| Operator target abandonment | permanently consumes the generation and clears the fence without making it reclaimable | Pass |

## Mutation matrix

Every applied mutant was isolated on the main tree, exercised by one focused oracle, and reverted with `git restore`. The tree was clean after each probe.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Unconditionally poison every `Accepted` claim on disconnect | `abnormal_stream_loss_leaves_claim_accepted_after_drop_active_and_redeliverable` | **Killed**, exit 101; observed poisoned instead of active |
| Omit replay reconstruction of the stream→claims mapping | No mutation site exists | **MATERIAL / absent path**; the clean-tree restart probe below demonstrates the production omission |
| Fresh: ungraceful core restart after actual offer but before ack | temporary `offered_without_ack_survives_core_restart_as_ambiguous` | **Exposed MATERIAL**, exit 101; replacement stream re-delivered the exact command |
| Fresh: apply stream A's offer set when stream B dies | temporary two-stream isolation probe | **Rejected by oracle / pass**; both claims stayed active and no evidence was appended |
| Old failure-only rule for `launch_attempted + identified + unspecified` | storage/replay consequence matrix | **Killed**, exit 101 |
| Remove `Running` from ambiguous-Result poisoning | real accepted/delivered/running Result matrix | **Killed**, exit 101 on running cancellation |
| Remove `Accepted` from ambiguous-Result poisoning | real accepted/delivered/running Result matrix | **Killed**, exit 101 on effect-before-ack cancellation |
| Poison `handshake_reconciling + identified + unspecified` progress | storage/replay consequence matrix | **Killed**, exit 101; unexpected disposition append |
| Poison successful identified progress | successful identified evidence → staging/promotion oracle | **Killed**, exit 101 |
| Omit identified-runtime reservation during evidence replay | storage/replay consequence matrix | **Killed**, exit 101; replay ownership was absent |
| Bypass logical-target current/reserved owner consultation | restart owner-collision oracle | **Killed**, exit 101 |
| Bypass claim-level external-runtime reverse owner | exact original-claim ownership oracle | **Killed**, exit 101 |
| Release directly on failed/cancelled/expired command transition | terminal-state claim oracle | **Killed**, exit 101 |
| Admit no-effect release without closed proof validation | silence-without-ack oracle | **Killed**, exit 101 |
| Omit execution-evidence delivery suppression | ambiguity replay/suppression oracle | **Killed**, exit 101 |
| Clear the continuation fence on poison | typed poison/fence oracle | **Killed**, exit 101 |

All 12 prior mutants remain killed. In particular, the effect-before-ack and actual-running cases assert their real command prestates before injecting the ambiguous Result, so those oracles are non-vacuous.

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** on the unchanged exact rerun, including 39 spawn-claim tests, 31 runtime-evidence/promotion tests, 78 server unit tests, all integration/property tests and doctests, and warnings-denied clippy. The first attempt reached doctests and hit the previously observed transient rustdoc missing-`tokio` artifact error; the exact rerun passed.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses; generated paths clean).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38, including the real core/adapter restart e2e).
- Final pre-review-file `git status --short`, `git diff --check`, and generated-contract diff: **PASS / clean**.

## Recommendation

**Return to implementing.** Persist or otherwise durably reconstruct exact offered-before-ack managed-spawn evidence across ungraceful core restart, add the paired offered-vs-never-offered restart regressions, rerun the full suite, and submit another thorough pass.
