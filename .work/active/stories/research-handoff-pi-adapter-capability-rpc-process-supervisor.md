---
id: research-handoff-pi-adapter-capability-rpc-process-supervisor
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-control-session-integrity, research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract, research-handoff-spawn-claim-registry-contract, research-handoff-spawn-crash-external-effect-evidence-contract, research-handoff-spawn-runtime-evidence-promotion-contract, research-handoff-spawn-idempotency-duplicate-handling, research-handoff-spawn-restart-continuation-orchestration, deployment-authority-workspace-scoped-revocable-keys]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-16
---

# Claim-aware Pi RPC process supervisor and effect journal

## Redesign disposition

Rewritten to consume the redesigned spawn contracts and the new control/session-integrity checkpoint. The supervisor no longer assumes every Pi `sessionFile` exists, no longer verifies cwd through generic RPC, and no longer reports a successor current/live directly.

## Checkpoint

Replace the production embedded SDK lifecycle with one supervised `pi --mode rpc` child per managed runtime generation. Consume the core-prepared logical target, exact claim, compound continuation provenance, pending-replacement fence, crash/effect vocabulary, and staged-successor/promotion envelopes. Journal launch responsibility and identity around the external-effect boundary. Never allocate `current + 1`, release a claim from command terminality, publish ordinary successor output before promotion, or auto-relaunch an ambiguous generation.

## Design

**Files**
- New `pi-adapter/src/rpc_client.ts` — strict bounded LF JSONL, unique request ids, response/event separation, extension-error tracking, bounded stderr, and EOF/process correlation.
- New `pi-adapter/src/pi_process.ts` — absolute executable/argv, sanitized environment, process group, injected launch/TERM→KILL port, and exact process-exit evidence.
- New `pi-adapter/src/spawn_journal.ts` — 0600 atomic claim/phase/launch-nonce/external-identity journal used only as evidence, not generation authority.
- New `pi-adapter/src/runtime_action_gate.ts` — one per-runtime/target mutex and delivery/action fence shared by continuation and reload.
- New `pi-adapter/src/spawn_supervisor.ts` — fresh/continuation orchestration and exact generated evidence reporting.
- `pi-adapter/src/{pi_session,session_registry,delivery,main,core_client}.ts` — bind RPC runtime, remove production SDK replacement/generation allocation, fence callbacks by attachment/process/generation tokens, and wait for core promotion.
- `contracts/proto/patchbay/pi_adapter.proto` — generated target spec and adapter-specific result details only; shared claim/effect/promotion types are imported from the spawn contract leaves.

```ts
export interface SpawnSupervisor {
  spawnFresh(operation: Operation, claim: SpawnGenerationClaim): Promise<StagedPiSuccessor>;
  continueGeneration(
    operation: Operation,
    claim: SpawnGenerationClaim,
    prior: RuntimeGenerationRef,
  ): Promise<StagedPiSuccessor>;
}

export interface SpawnEffectJournal {
  beginClaim(record: PiSpawnClaimJournalRecord): Promise<void>;
  recordPhase(record: PiSpawnPhaseRecord): Promise<void>;
  recordExternalIdentity(record: PiExternalIdentityRecord): Promise<void>;
  reconcile(claimOperationId: string): Promise<PiSpawnJournalState | undefined>;
}
```

Fixed continuation sequence:

1. Under the target/action gate, validate generated Pi target spec, exact `N→N+1` claim, both continuation provenance ids, adapter-local project/deployment authority, journal state, and local continuity reverse binding.
2. Journal the claim and random launch nonce before assuming delivery/launch responsibility; activate the local side of the core pending-replacement fence so N receives no new work.
3. Inspect activity. If work is active, send abort, await acknowledgement and `agent_settled` within the configured bound, flush adapter Observations, and report unproved work effects as `execution_outcome_unknown`.
4. Perform the current challenged control handshake and strict materialization/tree validation. For `require_resume`, `memory_only`/invalid fails before successor launch. N remains alive/settled so the adapter can report renewed N evidence; only core evidence rules may release the claim/fence.
5. Seal materialized N, terminate the process group with TERM→KILL, fence every old callback/handle/stdout subscription, and revalidate the same seal before launch.
6. Record `launch_attempted`, then spawn through an injected process port. Normal resume uses exact `--session <canonical path>`. Only explicit `allow_new_context` may omit it and later report `new_context`.
7. Verify command discovery, challenged control handshake, canonical cwd, session path/id, post-launch strict tree/sealed-prefix integrity, and current process token. `get_state`/`get_session_stats` only cross-check their actual fields.
8. Ask the cursor child to stage a full/suffix projection under the verified Pi continuity key; emit no ordinary transcript while the successor is only claimed.
9. Report exact `SpawnExecutionPhase`, `ExternalEffectDisposition`, successor SessionReport/readiness digest, and successful Result. These are staging evidence, not current state or terminal completion.
10. Wait for `SpawnPromotionCommitted`. Only then allow the cursor child to publish the replacement/current suffix, commit cursor state, open ordinary delivery, and report current connectivity/activity.

Fresh spawn uses the same launch journal, handshake, staging, promotion wait, and post-promotion publication. A new context may remain `memory_only`; that limits future resume/reload/cursor durability but does not by itself falsify current process liveness.

Failure mapping is closed:

- before successor launch with exact supervisor/journal proof: `proved_none` candidate for core validation;
- after launch attempt without known identity: `may_exist` and poison;
- known child/session identity: `identified` for exact-claim reconciliation;
- unexpected signal/nonzero exit: connectivity `failed`, activity `unknown`;
- RPC/framing/pipe loss without conclusive exit: `stale`, activity `unknown`;
- expected/confirmed clean exit: `offline`, activity `unknown`;
- no crash/detach/clean exit allocates a generation or triggers automatic restart.

## Fixture boundary

Production implements `ManagedPiRuntimePort` only with RPC. Tests may use `AgentSessionRuntimeFixture`, named for the runtime port it substitutes. Its constructor requires an injected offline `ModelRuntime`, resource loader, session manager, and model catalog/auth stubs. Ambient credential/model discovery is forbidden.

## Acceptance evidence

- [x] Fresh generation `1` and continuation exact `N+1` come only from the accepted core claim; production contains no managed `current + 1` path.
- [x] Both continuation Grant provenance records and the pending-replacement fence are carried/validated; adapter-local project authority cannot replace either core Grant.
- [x] `require_resume` cannot terminate/launch from a memory-only or invalid session; explicit new context never reports `resumed`.
- [x] Wrong cwd/path/id, incomplete tree, changed seal, old callback, or stale process/attachment token cannot stage success.
- [x] Journal-before-launch and identity-after-handshake crash prefixes map to the exact shared effect disposition; ambiguity poisons and never auto-relaunches.
- [x] Successor transcript/status output remains staged/quarantined until core promotion; SessionReport is the only claimed-successor ingress used.
- [x] Explicit crash, unexplained transport loss, and clean exit map to failed/stale/offline without generation mutation.
- [x] Production never instantiates SDK lifecycle; `AgentSessionRuntimeFixture` is fully offline/injected and a mutation consulting ambient credentials/catalog fails.

## Implementation evidence

- Production now launches one isolated `pi --mode rpc` process group through `ManagedPiRuntimePort`, performs the challenged control handshake, binds `RpcPiSession`, and classifies clean exit, conclusive crash, and unexplained transport loss without allocating a generation.
- `ClaimAwareSpawnSupervisor` consumes the exact accepted claim and deployment authority, journals the claim/nonce and `launch_attempted` before the process port, quiesces and seals continuations, stages claimed successors, and opens callbacks/publication only after an exact `SpawnPromotionCommitted` delivery.
- `FileSpawnEffectJournal` uses atomic fsync+rename records with owner-only files, monotonic phases, exact runtime correlation, and a single launch attempt. `RuntimeActionGate` serializes runtime actions and retains a poisoned replacement fence after ambiguity.
- Generated contracts add the Pi-local redacted result/persistence vocabulary and an authority-bearing adapter promotion notification; the server routes promotion only to the promoted runtime's adapter.
- The SDK substitute is explicitly named `AgentSessionRuntimeFixture` and requires injected offline model, auth/catalog marker, resource loader, session manager, and settings manager. The real-process E2E launches Pi offline and awaits process-group exit.
- Verification (2026-08-12): `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cd pi-adapter && npm test` (80/80); `cd contracts/ts && npm run build && npm run check:vectors` (59 vectors, 29 implementation checks, 38 mutation witnesses); generated artifacts reproduced byte-for-byte with `npm run gen`; and `cd pi-adapter && npm run test:mutations` (6/6 killed).

## Implementation notes — Pi Unit 3 r1 fix round

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected external-effect truth boundary); direct-read implementation with no subagent attempt because this is a delegated Pi-chain worker.
- Review weight: `thorough` (caller-selected); this item returns to `review` for the required fresh independent re-review.
- Structural continuation order: split the target replacement mutex from action-fence activation. The supervisor now acquires the target mutex before exact envelope/Grant/fence/effect validation, deployment authorization, journal reconciliation, and durable claim/nonce responsibility; only then does it activate the accepted local action fence. The complete canonical `prior_work_effects` list is consumed as authority: never-offered work remains the core's atomic `superseded` decision, while each delivered/running command is correlated and terminalized as `execution_outcome_unknown` before quiescence proceeds.
- RPC ambiguity: `PiRpcTransportError` now carries per-request `proved_not_written` versus `possibly_written` provenance. Post-write timeout, framing/protocol loss, pipe/EOF, and unproved process exit map to `execution_outcome_unknown` plus stale/failed/offline connectivity and unknown activity as exact lifecycle evidence allows; only proved pre-write refusal or authoritative Pi command failure remains `execution_failed`. Core-facing diagnostics use bounded constants rather than exception text.
- Journal-only restart: production preprovisioning structurally excludes managed logical targets. Startup folds every 0600 journal before ordinary session registration, poisons any unpromoted launch attempt, and never launches during recovery. Exact staged projection bytes/digest, promotion-observed, and publication-committed are separate durable markers; replayed promotion validates exact claim/runtime, idempotently republishes the staged projection through the production core port, commits the journal cursor, reports the promoted runtime stale/unknown, and performs no process launch.
- Oracle hardening: a production `RpcPiSession` race proves replacement ownership blocks a second stdin action; a stubborn parent+child process group forces and records real SIGKILL escalation; and the SDK fixture requires a WeakSet-branded offline runtime created with injected in-memory credential/model stores and network disabled. The ambient factory options are directly observed by the test.
- Files changed: `pi-adapter/src/{main,pi_session,rpc_client,runtime_action_gate,spawn_journal,spawn_supervisor}.ts`; `pi-adapter/tests/{delivery,pi_session,rpc_client,rpc_process_e2e,runtime_supervision_primitives,spawn_supervisor}.test.ts`; `pi-adapter/tests/offline_agent_fixture.ts`; `pi-adapter/scripts/mutation-cycle.mjs`; this story.
- Tests added: one-dimension continuation authority/fence/effect rejection; exact prior-work disposition application; held-target-prefix serialization; request-loss provenance at pre-write/malformed/exit/EOF/timeout boundaries; delivery failure/session-axis mapping; managed startup rejection; unpromoted and promoted journal recovery; durable recovered projection replay; production action-gate race; stubborn group escalation; and ambient offline-runtime rejection/options inspection.
- Mutation evidence: the original 6 Unit 3 mutants were re-confirmed and 9 new review-shaped mutants were registered. `npm run test:mutations` killed **15/15**, including target-mutex bypass, optional fence, ignored prior effects, false `execution_failed`, managed auto-launch, ignored replay publication, production action-gate bypass, SIGTERM-for-SIGKILL, and ambient model discovery. Every mutation was restored by the runner; no mutation was committed.
- Full verification (2026-08-16): Rust group `cargo fmt --all -- --check && cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; contracts group `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build && npm run check:presentation && npm run test:presentation`; operator-domain group `cd operator-domain && npm test` (28/28); Pi-adapter group `cd pi-adapter && npm test && npm run test:mutations` (92/92; 15/15 mutations); plus `cd web-cockpit && npm test` (144/144), `cd cli && npm test` (53/53 plus real-core resource projection), and `cd token-commune-adapter && npm test` (63/63). All passed; `git diff --check` passed.
- Simplification: one gate now owns both replacement-target serialization and stdin/action fencing; one journal owns launch, staged-publication, promotion-observed, and publication-commit recovery evidence; raw transport messages no longer cross the command-result boundary.
- Discrepancies from design: none. Rationale for the bounded staged-projection journal payload: Unit 3 must recover post-promotion publication without launching; the later cursor checkpoint can replace this local publisher behind the same reconciler port.
- Adjacent issues parked: none.

## Implementation notes — Pi Unit 3 r2 fix round

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the external-effect/journal truth boundary); direct-read implementation with no subagent attempt because this is a delegated Pi-chain worker.
- Review weight: `thorough` (caller-selected). The story returns to `review` for the autopilot's required fresh-context pass.
- Quiesce ambiguity: supervisor RPC transport failures now preserve `proved_not_written` / `possibly_written` through the production control-handshake wrapper and classify exact process evidence as stale, failed, or offline. A possibly-written quiesce abort/handshake loss reports `execution_outcome_unknown`, records `quiescing_prior/may_exist`, reports N stale-or-failed/offline with activity `unknown`, and poisons rather than releases the local replacement fence. A proved-not-written transport failure remains `execution_failed`, but N is never fabricated live/idle and the fence is retained for exact reconciliation. Correlated Pi command rejection and exact materialization/seal validation remain the audited non-ambiguous `execution_failed` cases.
- Journal replay: one semantic validator now guards every durable write, single-claim read, all-claim startup fold, and promotion eligibility. Fresh replay requires the exact `launch_attempted → external_identity_known → handshake_reconciling → staged publication → success_evidence_reported` chain; continuation prepends `quiescing_prior → prior_terminated`. Missing, skipped, reordered, duplicated, disposition-invalid, identity-before-launch, stage-before-handshake, success-before-stage, and promotion-before-complete states fail closed. Poison/promotion/publication flags must agree with the durable chain; promotion replay has a separate complete-chain assertion before any marker or recovered publication.
- Files changed: `pi-adapter/src/{control_handshake,spawn_journal,spawn_supervisor}.ts`; `pi-adapter/tests/{control_handshake,spawn_supervisor}.test.ts`; this story. No Protobuf edit was required.
- Tests added: production-shaped possibly-written abort timeout and unclean-exit cases assert unknown outcome, exact N connectivity with unknown activity, poison, no launch, and no false live/idle; control-handshake tests preserve transport provenance and classify its own expiry as possibly written; production `LocalStagedPiReconciler` replay tests remove or duplicate only `LAUNCH_ATTEMPTED` and assert no publication, marker, session report, or launch.
- Mutation evidence: the original quiesce misclassification was re-injected and killed; a fresh handshake-expiry provenance mutation was killed. The old parse-and-promote behavior was re-injected by removing read/write/promotion-chain guards and killed; a fresh duplicated-`LAUNCH_ATTEMPTED` acceptance mutation was killed. Every manual mutation was restored with `git restore` and followed by a clean rebuild. The registered Unit 3 suite was re-confirmed at **15/15 killed** and restored its source mutations.
- Full verification (2026-08-16): Rust group `cargo fmt --all -- --check && cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; contracts group `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build && npm run check:presentation && npm run test:presentation` (59 vectors, 19 promoted, 29 implementation checks, 38 mutation witnesses); operator-domain group `cd operator-domain && npm test` (28/28); Pi-adapter group `cd pi-adapter && npm test && npm run test:mutations` (95/95; 15/15 mutations); plus `cd web-cockpit && npm test` (144/144), `cd cli && npm test` (53/53 plus real-core resource projection), and `cd token-commune-adapter && npm test` (63/63). All passed; `git diff --check` passed.
- Simplification: semantic journal order and replay eligibility derive from one conditional phase-chain validator instead of scattered presence checks. Supervisor failure recovery carries one typed N-state/effect/fence disposition rather than reconstructing live/idle in the catch block.
- Discrepancies from design: none. Rationale for treating the control-handshake wrapper timeout as possibly written: the wrapped RPC has already started, so the wrapper cannot prove stdin observed none of the request even if its shorter response deadline wins.
- Adjacent issues parked: none.

## Ordering constraint

Consumes all spawn contract leaves it implements (identity, continuation, claim/fence, crash/effect, runtime evidence/promotion) plus duplicate reconciliation, generic restart orchestration, deployment authority, and Pi control/session integrity. Cursor publication remains a downstream Pi checkpoint.
