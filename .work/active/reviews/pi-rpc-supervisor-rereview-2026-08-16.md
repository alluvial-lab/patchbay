---
id: pi-rpc-supervisor-rereview-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-rpc-process-supervisor
created: 2026-08-16
updated: 2026-08-16
---

# Thorough rereview — Pi Unit 3 claim-aware RPC process supervisor

**Verdict: MATERIAL.** Commit `0e8e079` closes the pass-1 structural-order defect, the registered ordinary-delivery ambiguity mutant, managed preprovisioning auto-launch, durable promotion replay, and all three vacuous-oracle findings. Two fresh probes show that the transport and journal-recovery closures remain partial: a possibly-written quiesce abort is still reported as a proved execution failure while the disconnected prior is falsely restored live/idle, and a semantically inconsistent promotion journal is accepted and published instead of failing closed.

Review mode: independent fresh-context delegated story rereview, effective weight `thorough`, pass 2 over `50eb3af..0e8e079`. No subagent was needed or attempted. No temporary worktree was created.

## Findings

### MATERIAL 1 — Post-write RPC ambiguity during quiesce is still terminalized as `execution_failed` and live/idle

**Locations:** `pi-adapter/src/spawn_supervisor.ts:651,688-697,891-914,1209-1215`

The new `proved_not_written` / `possibly_written` classification reaches ordinary delivery through `classifyDeliveryFailure`, but supervisor-owned RPCs do not consume it. During continuation quiescence, `requestUnderLease({type: "abort"})` can throw a `PiRpcTransportError` after stdin accepted the request. Because successor launch has not started, `normalizeSupervisorError` converts every such error to `EXECUTION_FAILED`. The pre-launch catch then reports the prior runtime as `live/idle` and releases the local replacement fence even though the abort may have executed and the transport no longer proves connectivity or activity.

A fresh production-`RpcPiSession` probe made N streaming, made abort fail with `requestEffect="possibly_written"`, and required the closed failure-matrix result. It failed with exact actual values `{failureCode: EXECUTION_FAILED, sessionReports: ["session:live:idle"]}` instead of `{failureCode: EXECUTION_OUTCOME_UNKNOWN, sessionReports: ["session:stale:unknown"]}`. No successor was launched. The temporary probe was restored and the clean TypeScript build rerun.

**Required direction:** classify supervisor RPC transport failures by request provenance and process evidence too. A possibly-written abort/handshake/action loss must report `execution_outcome_unknown`; any unproved transport loss must report N stale/failed/offline as exact lifecycle evidence permits and activity unknown. Do not clear the accepted local fence from fabricated live/idle evidence; retain it until an exact recovery/core disposition permits release. Add this active-quiesce regression to the registered mutation set.

### MATERIAL 2 — Promotion replay accepts a semantically corrupted journal with no launch-attempt phase

**Locations:** `pi-adapter/src/spawn_journal.ts:496-550`; `pi-adapter/src/spawn_supervisor.ts:751-800`

`validateStoredState` checks each phase value independently but does not re-enforce the journal state machine on read. It does not require monotonic phase order, at most one launch attempt, or a `LAUNCH_ATTEMPTED` prefix before external identity, staged publication, promotion-observed, or publication-committed state. `acceptPromotion` likewise requires only exact claim/runtime identity plus staged bytes.

A fresh replay probe seeded a valid launch/identity/staged journal, recomputed the production projection digest, then removed only the launch phase while preserving valid JSON, 0600 mode, exact runtime, and staged projection. `acceptPromotion` did **not** reject: it published the recovered projection and committed promotion/publication markers. It did not launch a process, but it failed the required corrupted-journal fail-closed boundary. A malformed-JSON version of the same probe did reject before publication and launch, so the gap is specifically semantic validation rather than parsing. Both temporary probes were restored.

**Required direction:** validate the complete durable phase/state chain during every read: monotonic legal phases, one launch attempt, launch-attempt required before identity/staging/promotion, identity required before staging, exact staging required before promotion, and no contradictory poison/promotion flags. Add one-dimension semantic-corruption replay tests using the production `LocalStagedPiReconciler`; every case must reject before publication, markers, session reports, or launch.

## Closure disposition

| Pass-1 closure | Result |
|---|---|
| Structural order / complete accepted envelope | **PASS.** Target mutex precedes validation, deployment authority, journal responsibility, and fence activation. Both Grant slots and the canonical pending-replacement fence are mandatory. Canonical accepted/running prior-work effects are consumed and reported by exact command id. |
| RPC ambiguity | **PARTIAL / MATERIAL.** Ordinary delivery correctly maps post-write timeout/framing/pipe/EOF/unproved-exit to unknown with bounded diagnostics, but supervisor-owned quiesce RPC loss still maps to `execution_failed` and false live/idle. |
| Journal-only restart | **PARTIAL / MATERIAL.** Production preprovisioning rejects managed targets; unpromoted recovery poisons without launch; exact replay publishes staged bytes, commits markers, and reports stale/unknown. Malformed journals fail closed, but semantically impossible promotion journals are accepted and published. |
| Action-gate / escalation / offline-fixture oracles | **PASS.** The production `RpcPiSession` bypass, SIGKILL→SIGTERM, and ambient model discovery mutants all fail their focused production-seam tests. |
| Raw exception redaction | **PASS.** Core failures and forwarded diagnostics retain typed/bounded constants; transport exception messages remain adapter-local. |

## Mutation and probe matrix

| Mutation / probe | Result |
|---|---|
| Original six registered Unit 3 mutants | **6/6 KILLED** |
| Target-mutex bypass | **KILLED** |
| Optional continuation fence | **KILLED** |
| Ignored accepted prior-work effects | **KILLED** |
| Ordinary post-write ambiguity misclassified as `execution_failed` | **KILLED** |
| Managed preprovisioning auto-launch | **KILLED** |
| Replayed promotion publication ignored | **KILLED** |
| Production `RpcPiSession` action-gate bypass | **KILLED** |
| SIGKILL escalation replaced with SIGTERM | **KILLED** |
| Offline factory changed to ambient discovery | **KILLED** |
| Fresh: possibly-written abort response loss during quiescence | **SURVIVING BEHAVIOR / MATERIAL** — current code returned `execution_failed` + live/idle |
| Fresh: malformed-JSON promotion journal | **PASS** — rejected; no publication or launch |
| Fresh: valid-JSON promotion journal with launch phase removed | **SURVIVING BEHAVIOR / MATERIAL** — replay published and committed; no launch |

Clean-tree `npm run test:mutations` reported **15/15 killed**. The runner restored every source mutation and rebuilt clean output.

## Full clean verification

1. **Rust group:** `cargo fmt --all -- --check`; `cargo build --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. **Contracts group:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests — **PASS**; 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 registered mutation witnesses.
3. **Operator-domain group:** `npm test` — **PASS, 28/28**.
4. **Pi-adapter group:** `npm test` — **PASS, 92/92**; registered mutations **15/15 killed**.
5. **Web cockpit:** `npm test` — **PASS, 144/144**.
6. **CLI:** `npm test` — **PASS, 53/53** plus the real-core resource projection.
7. **token-commune adapter:** `npm test` — **PASS, 63/63**, including real-core flows.

An initial concurrent web/CLI invocation collided with token-commune's prerequisite rebuild of shared ignored `dist/` outputs; those results were discarded and web then CLI were rerun sequentially to the green results above. `git diff --check` passed and the tracked tree was clean before this review file was written. `/` retained 54 GiB free.

## Recommendation

**Return `research-handoff-pi-adapter-capability-rpc-process-supervisor` to `implementing`.** Preserve the now-verified structural prefix, exact accepted prior-effect reconciliation, no-managed-preprovisioning rule, durable replay publisher, and hardened production oracles. Extend request-effect/lifecycle classification to supervisor quiescence and make journal replay validate the complete durable state machine, then run thorough pass 3.
