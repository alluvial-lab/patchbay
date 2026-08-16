---
id: pi-rpc-supervisor-review-2026-08-16
kind: story
stage: done
tags: [review, spawn, adapter]
parent: research-handoff-pi-adapter-capability-rpc-process-supervisor
created: 2026-08-16
updated: 2026-08-16
---

# Thorough review — Pi Unit 3 claim-aware RPC process supervisor

**Verdict: MATERIAL.** Commit `cb1e8c1` establishes the RPC-only production process port, atomic 0600 effect journal, challenged handshake, strict resume admission, exact-claim staging, claimed-successor transcript quarantine, authority-bearing promotion delivery, and TERM→KILL implementation. It does not yet make the binding ten-step continuation order or crash/recovery behavior true end to end: the target mutex starts too late, the accepted prior-work/fence envelope is not consumed completely, ordinary RPC ambiguity is reported as a proved execution failure, and managed startup can bypass the claim journal and auto-launch a generation after a crash. Three required assurance boundaries also have mutation-surviving oracles.

Review mode: independent fresh-context delegated story review, effective weight `thorough`, deep external-effect-truth lane over `6cf4de0..cb1e8c1`. No subagent was needed or attempted. No temporary worktree was created.

## Findings

### MATERIAL 1 — The fixed continuation prefix is not one structural critical section and ignores accepted prior-work effects

**Locations:** `pi-adapter/src/spawn_supervisor.ts:227-265,683-706`; generated field `SpawnClaimAccepted.prior_work_effects` has no production consumer under `pi-adapter/src/`

`handleAcceptedSpawn` validates the envelope, reconciles/creates the journal, and rechecks deployment authority before acquiring the target replacement lease. The lease at line 265 is the first target mutex and local action fence. This is the reverse of the binding prefix: target mutex → validate/journal → consume fence. Validation and journal state can therefore race another attempt for the same target instead of being structurally serialized with the later effect path.

The continuation envelope is also only partially consumed. `pendingReplacement` is optional in the check at lines 703-705: an omitted fence passes, and the code does not require canonical `SUPERSEDED` / `replacement_pending` values. More importantly, `SpawnClaimAccepted.prior_work_effects` is never read. The adapter consequently cannot correlate the core-precomputed `QUIESCE_OUTCOME_RECONCILIATION` commands or report their unresolved effects as `execution_outcome_unknown`; it merely reports the prior session idle/live after an abort/settle. A review mutation that removed `pendingReplacement` from every continuation fixture **survived** the continuation test and still terminated N and launched N+1.

**Concrete fix:** split target mutual exclusion from fence activation. Acquire the per-target mutex first; inside it validate the exact claim, both Grant/provenance slots, required canonical pending-replacement fence, complete prior-work effects, local reverse binding, deployment authority, and journal; durably record the exact claim/nonce; only then activate the local delivery/action fence. During quiescence, reconcile every accepted prior-work effect by exact command id and disposition, and emit typed `execution_outcome_unknown` for any offered/running effect not proved terminal. Add one-dimension rejection tests for omitted/mutated fence fields and prior-work entries plus an active-command abort/settle test that proves exact unknown-outcome reporting before seal/termination.

### MATERIAL 2 — RPC response loss is falsely terminalized as `execution_failed`

**Locations:** `pi-adapter/src/rpc_client.ts:196-208`; `pi-adapter/src/main.ts:637-654`

After stdin has accepted a command, `PiRpcClient` can time out, lose framing, lose a pipe, reach EOF, or observe a process exit without knowing whether Pi executed the request. The per-request timeout at line 199 only rejects that promise; it does not fail the transport or emit lifecycle evidence. `AdapterProcess.#executeDelivery` then maps every non-`UnsupportedCommandError` to `FailureCode.EXECUTION_FAILED` and may leave the session presented live. The same false certainty applies to a prompt whose write succeeded and response was lost during a pipe/process failure.

That contradicts the closed failure matrix: after an offered external action, response/transport loss is `execution_outcome_unknown`; unexplained transport loss makes connectivity stale and activity unknown. Surfaces use this distinction for retry safety, so the current mapping can tell the operator a potentially executed command merely failed.

**Concrete fix:** make request errors carry write/effect provenance (not-written/proved-none versus written-or-possibly-written). Once a side-effecting command may have crossed stdin, map timeout/framing/pipe/EOF/unproved exit to `execution_outcome_unknown`, emit the corresponding stale/failed + unknown session report, and never forward raw exception text. Preserve `execution_failed` only for a correlated authoritative Pi failure. Add focused request-loss tests at pre-write, post-write/pre-response, nonzero exit, bare EOF, and timeout boundaries, asserting both command failure code and session axes.

### MATERIAL 3 — Managed restart/promotion recovery can bypass the journal and auto-launch an ambiguous generation

**Locations:** `pi-adapter/src/main.ts:194-215,287-325`; `pi-adapter/src/spawn_supervisor.ts:491-525,600-609`

The journal is reconciled only while handling a delivered accepted spawn. A crash after core promotion but before projection publication, cursor commit, or `journal.markPromoted` leaves no active waiter or `promotionEligibleClaims` entry after restart. The server correctly replays the authority-bearing promotion, but `acceptPromotion` silently ignores every promotion without an in-memory waiter/eligible marker. The adapter therefore cannot finish the required post-promotion publication from durable evidence.

Startup then takes every `PreprovisionedSession`, including one carrying `logicalTargetId`, and calls `#createProductionSession`. That path chooses a fresh nonce and launches immediately with an environment/configured generation; it does not consult the claim journal or exact core promotion. After an adapter crash, the prior detached child may still exist or its outcome may be unknown, so this is an automatic same-generation relaunch outside the poison rule and outside JOURNAL-BEFORE-EFFECT. It also permits a managed logical target to enter the registry through ordinary preprovisioning rather than the accepted claim path.

**Concrete fix:** distinguish unmanaged attach from managed logical-target recovery at the type and composition-root boundary. A managed target must recover from exact core claim/promotion plus journal state before any process action; an unpromoted `launch_attempted` record must poison and must never fall through to preprovisioned launch. Persist/reconstruct enough exact staged-publication state to idempotently finish promotion replay, cursor commit, and live reporting, with separate durable markers for promotion observed and publication committed. Add crash-prefix real-process tests for journal-before-launch, identity known, staged/result, core promotion before local publication, publication before journal acknowledgement, and adapter restart with a possibly surviving child; assert at most one launch and eventual stale/manual or exact publication, never silent ignore.

### MATERIAL 4 — Required action-gate, escalation, and offline-fixture assurances have mutation-surviving oracles

**Locations:** `pi-adapter/src/pi_session.ts:458-464`; `pi-adapter/tests/runtime_supervision_primitives.test.ts:28-54`; `pi-adapter/src/pi_process.ts:177-188`; `pi-adapter/tests/rpc_process_e2e.test.ts:12-59`; `pi-adapter/src/pi_session.ts:594-606`; `pi-adapter/tests/offline_agent_fixture.ts:15-46`; `pi-adapter/tests/pi_session.test.ts:178-212`

Three acceptance-critical mutations survived their focused committed tests:

1. Replacing the production `RpcPiSession` non-lease path with a direct `runtime.rpc.request` bypassed the action gate, yet the gate/supervisor/real-process focused set remained green. The gate test exercises `RuntimeActionGate` directly, not the production wiring whose serialization it is meant to prove.
2. Replacing the SIGKILL escalation with a second SIGTERM left the real-process and primitive tests green because the fixture Pi exits on the first TERM. No test forces the escalation boundary.
3. Replacing the in-memory/no-network `createOfflineModelRuntime` helper with ambient `ModelRuntime.create({ refreshOnCreate: false })` left the fixture tests green. The fixture checks only a forgeable `kind: "offline-injected"` marker and model presence; the advertised mutation that consults ambient credential/catalog discovery is not killed.

The implementation contains the intended guard code, but these surviving mutations make the acceptance evidence vacuous at the actual integration boundaries.

**Concrete fix:** add (a) a `RpcPiSession` race test that holds a real request, starts replacement, and proves no second stdin action crosses until ownership/fencing decides it; (b) a stubborn process-group fixture that ignores TERM, records SIGKILL, and proves bounded group exit and child cleanup; and (c) an opaque/branded offline-services factory backed only by in-memory credential/model stores and disabled network, with an ambient credential/catalog access mutation that deterministically fails. Run these through the production composition seams rather than testing only the helper objects.

## Checklist disposition

| Requirement | Result |
|---|---|
| Binding ten-step structural order | **FAIL / MATERIAL** — launch/handshake/seal/staging/promotion suffix is substantially ordered, but validation/journal precede the target mutex, the pending fence is optional, and prior-work effects are ignored. |
| Atomic 0600 journal; exact nonce before launch; ambiguity poison | **PASS for the direct supervisor path** — atomic fsync+rename, 0600 files, exact claim/nonce before launch, one guarded launch, and poison are present; managed startup/recovery bypass is the Material 3 exception. |
| One action/stdin owner | **Implementation present; assurance FAIL** — production requests route through the gate and supervisor requests use its lease, but the direct production-wiring bypass mutation survives. |
| No current+1, terminal release, premature successor output, or false `resumed` | **PASS on inspected direct path** — generation comes from the exact claim, `require_resume` uses Unit 2 admission and sealed-prefix proof, and a true pre-promotion publication mutant is killed. |
| Production RPC only / injected SDK fixture | **PARTIAL / MATERIAL evidence gap** — production instantiates RPC only; the ambient model/catalog mutation survives fixture tests. |
| Redaction | **PASS** — successor reports blank project/cwd, spawn result carries bounded status/digest, supervisor failures sent core-side are constants, and local diagnostics structurally discard messages/nonces/paths/secrets. |
| Promotion-delivery server routing | **PASS** — `SpawnPromotionCommitted` is routed only by the promoted runtime's adapter id and remains separate from ordinary Operation delivery. |
| Crash/restart convergence | **FAIL / MATERIAL** — replayed promotion is ignored without in-memory eligibility and preprovisioned managed startup can launch outside the journal. |

## Mutation matrix

Every mutation was made on the main tree and restored with `git restore`; focused clean rebuilds followed. The tracked tree was clean after every restoration.

| Mutation / probe | Result | Focused oracle |
|---|---|---|
| Existing: accept fresh generation other than exact claim | **KILLED** | `fresh generation two` |
| Existing: invoke process launch before durable `launch_attempted` | **KILLED** | exact-generation journal-before-launch test |
| Existing: release instead of poison after launch ambiguity | **KILLED** | ambiguity-poison test |
| Existing: permit `require_resume` from memory-only prior | **KILLED** | memory-only admission test |
| Existing: publish before exact promoted-candidate registry installation | **KILLED** | promotion/publication test |
| Existing: remove fixture offline marker check | **KILLED** | missing-marker test |
| Remove prior launch-attempt auto-relaunch guard | **KILLED** — process port was invoked twice | duplicate-delivery ambiguity test |
| Publish staged transcript before waiting for core promotion | **KILLED** — reconciler observed no exact promoted registry entry | fresh spawn promotion test |
| Bypass `RpcPiSession` action gate | **SURVIVED** | gate primitives + spawn supervisor + real RPC process tests |
| Omit the continuation pending-replacement fence | **SURVIVED** — N was terminated and N+1 launched | `explicit allow_new_context` |
| Replace SIGKILL escalation with a second SIGTERM | **SURVIVED** | real RPC process + supervision primitives |
| Replace offline in-memory model runtime with ambient `ModelRuntime.create` | **SURVIVED** | SDK fixture tests |

`npm run test:mutations` reported **6/6 killed** for its registered mutations. The four additional probes above expose the uncovered boundaries.

## Full clean verification

All commands ran after restoring the clean implementation tree.

1. **Rust group:** `cargo fmt --all -- --check`; `cargo build --workspace --all-targets`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. **Contracts group:** generated drift, vectors, models, TypeScript build, presentation conformance, and presentation meta-tests — **PASS**; 59 vectors, 19 promoted vectors, 29 implementation checks, and 38 registered mutation witnesses.
3. **Operator-domain group:** `npm test` — **PASS, 28/28**.
4. **Pi-adapter group:** `npm test` — **PASS, 80/80**, including the real core/adapter loop and real offline RPC child.
5. **Web cockpit:** `npm test` — **PASS, 144/144**.
6. **CLI:** `npm test` — **PASS, 53/53** plus the real-core resource projection.
7. **token-commune adapter:** `npm test` — **PASS, 63/63**, including both real-core flows.

`git diff --check` passed. The tracked tree was clean before this review file was written. `/` retained 54 GiB free; no temporary worktree was used.

## Recommendation

**Return `research-handoff-pi-adapter-capability-rpc-process-supervisor` to `implementing`.** Preserve the direct supervisor's journal-before-launch, exact resume proof, candidate quarantine, redaction, and promotion routing. Fix the structural continuation prefix/prior-work reconciliation, typed RPC ambiguity mapping, and durable managed restart/publication recovery; add the three non-vacuous integration oracles; then rerun this thorough review before advancing to `done`.
