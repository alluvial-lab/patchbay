---
id: research-handoff-pi-adapter-capability
kind: feature
stage: implementing
tags: [adapter, v1]
parent: epic-public-product-contract
depends_on: [research-handoff-spawn, capability-manifest-durability-and-reconciliation-depth]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn]
created: 2026-08-08
updated: 2026-08-12
---

# Pi adapter capability surface for v1 (spawn/restart/reload + manifest)

## Redesign status and review authority

This body supersedes the first 2026-08-12 design. The consolidated five-reviewer gate at `.work/active/reviews/spawn-stride-adversarial-review-2026-08-12.md` found the prior spawn/Pi stride unsafe to implement. The redesigned `research-handoff-spawn` feature now owns the six shared contract leaves. This redesign consumes those leaves and closes Pi-side BLOCKER 6 (authoritative cursor replacement), BLOCKER 9 (unimplementable cwd proof), BLOCKER 10 (false durable-JSONL premise), and every Pi-adapter MATERIAL finding.

The earlier untraceable “five-BLOCKER” review is not evidence for this redesign. The cited consolidated review is the current gate. The disposition matrix below links each assigned finding to a concrete checkpoint.

## Brief

Implement the Pi reference adapter for managed fresh spawn, typed spawn continuation, reconnect, and bounded resource reload without importing Pi vocabulary into Patchbay core semantics. The production substrate is one supervised `pi --mode rpc` subprocess per managed runtime generation. Core supplies the accepted logical target, exact generation claim, compound continuation provenance, pending-replacement fence, crash/effect vocabulary, successor staging, and authority-bearing promotion. The Pi adapter supplies an honest external-effect journal, process lifecycle, a control-extension handshake, strict session-tree validation, conditional persistence proof, Pi-session-scoped cursor state, authoritative transcript-projection replacement, and a reload action whose scope matches what Pi actually reloads.

A Pi process may be live before its new session file exists. Such a runtime may be reported as a current new context after core promotion, but it is **not resumable, sealable, restart-stable, or reload-marker-durable** until a materialized file passes the adapter's strict validator. `resumed` is reserved for an exact selected session whose complete persisted tree and post-launch continuity proof pass.

## Grounding and current-code constraints

- Research source: `.research/analysis/campaigns/v1-control-plane-and-spawn/specialists/pi-adapter-probe.md`, especially persisted-entry replay, partial live-event order, session continuation, reload, and process replacement. Relevant attestations are `[pi-rpc]{1,4,5,6,7,8,9}`, `[pi-sessions]{2,3}`, `[pi-extensions]{1,3,4,5,7,8}`, `[pi-loader]{1,2,3,4,5}`, and `[pi-sdk]{1}`.
- `get_state` and `get_session_stats` expose `sessionFile` and `sessionId`, but no cwd (`[pi-rpc]{9}` and installed `docs/rpc.md`). They are supplemental identity checks, not cwd proof.
- Pi extension commands execute immediately even while streaming, and the RPC `prompt` response says only that the command was handled. Extension-handler errors become events rather than a failed correlated response. A positive control proof therefore requires an exact challenge marker, not `success: true`.
- `ctx.reload()` tears down/recreates the extension runtime and enumerated resources in-process; its caller remains an old frame. Loader evidence supports re-reading entrypoints with a cleared factory cache, but not arbitrary transitive dependency graphs, running Pi package aliases, compiled `/dist`, native modules, or the executable (`[pi-extensions]{5}`, `[pi-loader]{2,3,4,5}`).
- Pi defers creation of a new session JSONL until an assistant message exists. `dist/core/session-manager.js:724-736` keeps pre-assistant entries in memory, even when `sessionFile` is non-empty. The adapter has no attested flush RPC and must not invent one.
- Pi does not fail closed on full tree integrity: its parser skips malformed interior lines, its id map overwrites duplicate ids, and `getTree()` promotes orphaned entries to roots (`dist/core/session-manager.js:88-104,671-680,983-1012`). Header/inode/trailing-LF checks alone cannot substantiate `resumed`.
- Current `pi-adapter/src/pi_session.ts` embeds `AgentSessionRuntime`, allocates `current + 1`, and turns unknown `since` into a full result. Managed production replaces those behaviors. An SDK-backed `AgentSessionRuntimeFixture` remains test-only, with `ModelRuntime` and model/catalog/auth behavior fully injected and offline.
- Current transcript Observations append to the durable core log and the cockpit folds them incrementally. Therefore adapter-local full-fetch upsert is insufficient: unknown-cursor repair needs an adapter-specific authoritative replacement envelope whose consuming projection deletes omitted Pi-derived entries while immutable audit history remains.
- The redesigned spawn cursor leaf scopes external continuity by verified external identity, not Patchbay generation. The Pi implementation uses verified Pi session identity and retains a reverse binding to one logical target.

**Dispatch rationale:** direct-read redesign. The current gate already contains five independent fresh-context reviews, and this delegated lane cannot spawn without tripping the recursion guard. No required fan-out was silently replaced. The forced-adversary pre-mortem is run inline below.

## Work-nature test

**Non-zero design surface; full feature-design lane.** This redesign changes process/session proof, availability claims, opaque manifest shape, reload admission, external cursor authority, transcript replacement, crash evidence, and implementation dependency edges. No UI mock is required: the existing Operation/failure/stale presentation remains the surface, and the Pi profile appears through existing adapter diagnostics.

## Design decisions

### 1. Cwd proof comes from the adapter control extension, not generic RPC state

Every managed child loads one adapter-owned `patchbay-control` extension. After launch the supervisor first confirms the extension command is present through `get_commands`, then submits a random challenge to `/patchbay-control-handshake`. The extension appends a bounded `patchbay.control.handshake.v1` custom entry containing the challenge, the supervisor-provided launch nonce, initialized `ctx.cwd`, `sessionManager.getSessionId()`, `getSessionFile()`, and an extension-instance epoch. The supervisor finds that exact marker through `get_entries`, compares its path/id with `get_state`/`get_session_stats`, and compares its cwd with the adapter-resolved canonical project cwd.

The generic RPCs still prove only what they expose: current Pi session path/id and activity flags. The extension marker is the cwd evidence. A prompt `success: true`, an extension event without the exact challenge/launch nonce, or a path/id-only response is not a handshake. Raw cwd/session paths remain adapter-local and are redacted from core Observations, audits, and diagnostics.

This is process-correlated evidence from the authenticated/current adapter, not cryptographic proof against a dishonest child or adapter. That trust limit matches the spawn contract.

### 2. Session persistence is an explicit conditional state

`PiSessionMaterialization` has three outcomes:

- `memory_only` — Pi reports a path/id, but there is no regular non-empty file containing the current session tree;
- `materialized` — a safely opened regular file under the configured root passes full framing, header, entry-shape, id/reference/tree, and RPC/tree equality checks and yields a seal;
- `invalid` — a file exists but any required check fails.

There is no inferred or synthetic flush. A managed fresh child may be staged/promoted with `new_context` while `memory_only`, but the adapter keeps restart-stable cursor and reload-continuity capabilities unavailable. `require_resume` continuation fails before successor launch when no materialized seal can be obtained after bounded quiescence. It emits exact pre-launch no-successor-effect evidence and re-establishes N's settled/current evidence so the core can decide whether to release the claim/fence. An explicit `allow_new_context` continuation may intentionally replace with a fresh session and reports only `new_context`.

Reload also requires `materialized`: request/completion custom entries are not called persisted proof while Pi is retaining them in memory.

### 3. `resumed` requires strict adapter validation because Pi loading is permissive

The adapter's `PiSessionTreeValidator` safely opens the selected file without following a symlink, checks allowed-root containment and regular-file identity, requires final LF framing, parses every non-empty line, requires exactly one first header with the supported version and expected session id, validates the closed supported entry family and required fields, rejects duplicate/empty ids, validates every parent/reference, requires exactly one tree root when entries exist, and rejects cycles/orphans/self-parenting. It compares raw parsed entries with `get_entries()` rather than assuming Pi's parser failed closed.

A pre-stop `MaterializedSessionSeal` records canonical local path, root id, session id, device/inode, size, content/tree digest, exact ordered entry ids, and sealed leaf. Cwd is deliberately not in this seal: actual initialized cwd is verified through the control handshake. After launch, the same physical/header identity and sealed entry prefix must remain intact; bounded startup/control entries may extend the valid tree. The new challenge marker must be the current leaf, and the raw file, RPC entries, and marker must agree before `resumed` can be reported.

### 4. Cursor scope is verified Pi continuity, never Patchbay generation

The adapter derives one local `PiSessionContinuityKey` from adapter/deployment identity, verified Pi session id, configured session-root id, and canonical root-relative session path. Its wire/external cursor scope uses a bounded opaque digest; raw paths do not leave the adapter. N+1 loads N's cursor when it resumes the same verified Pi session. A reverse index prevents the same Pi continuity key from binding to two logical targets.

An unmaterialized session may use a volatile in-process cursor for responsiveness, but no restart-stable cursor claim is emitted until materialization and a full authoritative replacement commit.

### 5. Unknown cursor performs authoritative projection replacement

Known cursor reconciliation may send an idempotent suffix batch keyed by stable Pi entry identities. Unknown cursor reconciliation does not clear or upsert the old view. It:

1. holds the existing projection stale;
2. fetches the complete current Pi entry set and leaf;
3. validates the complete tree and exact continuity identity;
4. builds a staged replacement epoch with the exact Pi-derived presentation set;
5. sends one generated Pi authoritative-replacement envelope through the core's opaque Observation path and awaits durable acknowledgement;
6. atomically installs local `{exact projection, leaf, cursor, epoch}` using compare-and-swap + temp-file fsync/rename.

The known Pi compositor in the shared operator domain treats the replacement envelope as one semantic fold: all prior Pi-persisted projection members in that continuity scope are replaced, so omitted stale members disappear; immutable source/audit events remain in the log. Retrying the same epoch with identical content is inert; conflicting content for one epoch fails closed. A crash before core acknowledgement leaves the old stale projection/cursor. A crash after acknowledgement but before local commit resends the same replacement and then commits locally; it never advances the cursor first.

For a claimed successor, replacement is validated and staged adapter-locally but no ordinary transcript Observation is emitted while the core disposition is `ClaimedSuccessor`. Its digest enters successor evidence. After atomic promotion makes N+1 current, the adapter publishes the replacement envelope, commits the cursor, then reports `live`. This consumes the spawn quarantine/promotion contract instead of bypassing it.

### 6. Reload rejects active work and has a narrower evidence-backed scope

Reload uses a per-runtime exclusive action gate. While holding it, the adapter fences new delivery and requires all of the following before invoking the extension command: no in-flight adapter delivery or direct RPC action, `get_state.isStreaming=false`, `isCompacting=false`, `pendingMessageCount=0`, and either no activity-start in the current process incarnation or a tracked `agent_settled` epoch newer than the last start/retry/compaction activity. Because the managed child has one stdin owner, the gate closes the check-to-command race. Busy reload is rejected before effect with a bounded retryable reason; it does not abort operator work.

The Pi profile describes reloadable scope as **the loaded extension entrypoint plus the resource paths Pi enumerates on reload**. Skills, prompts, themes, and context files remain Pi-profile values, not core fields. Arbitrary imported extension dependency graphs, Pi/runtime package aliases, compiled `/dist`, native dependencies, and the executable require spawn continuation/process replacement. Unknown scope is unsupported rather than overclaimed.

On an idle materialized session the control extension appends an exact request marker, calls `await ctx.reload(); return`, and the new instance appends the matching completion marker from `session_start(reason=reload)`. The adapter requires both markers to be materialized, re-runs the challenge handshake against the new extension epoch, rebinds subscriptions, and completes cursor reconciliation before reporting reload success. A marker alone proves neither quiescence nor complete rehydration.

### 7. Pi vocabulary lives in a generated opaque Pi profile

The generic `AdapterCapability` retains only core behavior branches (target categories, supported Operations/shapes, snapshot tier, streaming/cancellation/replacement booleans, idempotency, attachment, known failures, diagnostic reporting, and the durability/reconciliation fields owned by `capability-manifest-durability-and-reconciliation-depth`). It gains at most one bounded opaque generated-profile envelope with schema descriptor and bytes; core validates framing/size but does not interpret Pi fields.

`contracts/proto/patchbay/pi_adapter.proto` owns `PiRuntimeProfile`: RPC mechanism, live-event caveats, session materialization condition, cursor mechanism, control-extension proof, reloadable Pi entrypoint/resource kinds, and process-replacement exclusions. Cwd, project trust, extensions, skills, prompts, themes, and context files are Pi-specific enum/field values inside that profile, not mandatory core manifest fields.

The profile-contract checkpoint does not advertise support by itself. The final conformance checkpoint activates the Pi declaration only after the supervisor, session validator, cursor replacement, and reload mechanisms pass. The feature and profile story depend on `capability-manifest-durability-and-reconciliation-depth`; every generic assurance field is populated from that generated contract and unknown/uncertain values remain false/unknown. No “complete manifest” claim is made before that dependency lands.

### 8. The SDK fixture is named for the runtime port, not the model service

Production has one implementation of `ManagedPiRuntimePort`: the RPC child. Unit tests may use `AgentSessionRuntimeFixture`, an implementation of that same port backed by Pi's SDK. Its constructor requires a fully injected offline `ModelRuntime`, resource loader, session manager, and model/catalog fixtures. Tests never call ambient credential discovery or treat `ModelRuntime` as the lifecycle under test.

## Architectural options

### Option A — Keep generic RPC/file metadata as the proof

Use `get_state`/`get_session_stats`, inode/header checks, and marker presence. This cannot verify initialized cwd, accepts Pi's permissive tree loading, and mistakes in-memory markers for durability. **Rejected by BLOCKERs 9–10 and the tree-validation MATERIAL.**

### Option B — Patch Pi with a new flush/cwd RPC before Patchbay work

An upstream `flush_session` plus cwd-bearing state response could simplify the edge. Current Pi exposes neither contract; requiring a fork would block the v1 reference adapter and still would not solve cursor replacement or arbitrary reload dependencies. **Reserved as a future simplification, not a v1 premise.**

### Option C — Control-extension proof + conditional materialization + authoritative projection replacement

Use official extension/custom-entry and RPC mechanisms, make absent durability unavailable, validate the raw tree strictly, and publish one exact-set replacement envelope after current-generation fencing. This adds an adapter-owned control bridge and local journal/store, but every claimed guarantee has an observable current Pi mechanism. **Chosen.**

## Trickiest unit first: continuation readiness across three authorities

The hardest unit is the transition from a fenced N process to a successor that is ready to stage but is not yet current. It spans core claim authority, Pi process/session evidence, and the external projection cursor without allowing one to impersonate another.

```ts
export interface PiSessionContinuityKey {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly piSessionId: string;
  readonly sessionRootId: string;
  readonly rootRelativePath: string; // adapter-local; never diagnostics/wire
}

export type PiSessionMaterialization =
  | { readonly kind: "memory_only"; readonly sessionId: string; readonly declaredPath: string }
  | { readonly kind: "materialized"; readonly seal: MaterializedSessionSeal }
  | { readonly kind: "invalid"; readonly reason: PiSessionIntegrityFailure };

export interface ManagedPiRuntimePort {
  launch(spec: PiLaunchSpec): Promise<PiRpcRuntime>;
  handshake(runtime: PiRpcRuntime, challenge: PiHandshakeChallenge): Promise<PiControlHandshake>;
  terminate(runtime: PiRpcRuntime, policy: TerminationPolicy): Promise<ProcessExit>;
}

export interface PiAuthoritativeReconciler {
  reconcileCurrent(scope: ExternalCursorScope, runtime: PiRpcRuntime): Promise<CommittedPiProjection>;
  stageClaimedSuccessor(scope: ExternalCursorScope, runtime: PiRpcRuntime): Promise<StagedPiProjection>;
  publishAfterPromotion(staged: StagedPiProjection): Promise<CommittedPiProjection>;
}
```

Continuation order is fixed:

1. Under the target mutex, validate the generated Pi target spec, accepted exact claim/compound provenance, adapter-local deployment authority, and spawn journal; journal the exact claim/launch nonce before external effect.
2. Consume the core pending-replacement fence and close the local runtime action/delivery gate for N.
3. Quiesce N: abort only if required, await correlated acknowledgement and `agent_settled`, flush adapter Observations, and report unresolved running effects as `execution_outcome_unknown`.
4. Run the current control handshake and materialization validator **after settle and before termination**. `require_resume` with `memory_only`/`invalid` stops here; no successor is launched and N remains a process the adapter can re-establish as settled/current evidence.
5. Seal the fully validated file and cursor/projection state, terminate the old process group with bounded TERM→KILL, then revalidate the seal before launch.
6. Launch via absolute executable/argv/sanitized environment, normally with exact `--session <canonical path>`. `allow_new_context` omits the resume selector and can report only `new_context`.
7. Verify `get_commands`, perform the challenged control-extension handshake, compare actual cwd with configured cwd and path/id with generic RPC, and validate the post-launch raw tree plus exact sealed prefix. Generic RPC alone cannot clear this gate.
8. Stage an authoritative cursor/projection reconcile under the verified Pi continuity key. A claimed successor emits no ordinary transcript evidence yet.
9. Report exact execution phase/effect evidence and the successor SessionReport/readiness digest. Core stages it; successful Result remains non-terminal evidence.
10. After `SpawnPromotionCommitted`, publish the exact replacement/suffix as current, atomically commit local cursor state, then report fresh `live/idle|working` state. The core promotion driver remains the only authority/completion owner.

Any ambiguity after launch poisons the claim; the supervisor never auto-launches another child for that generation.

## Implementation units and child checkpoints

### Unit 1 — Opaque profile contract and conservative declaration gate

**Story:** `research-handoff-pi-adapter-capability-manifest-profile`

**Files:** `contracts/proto/patchbay/{adapter,pi_adapter,diagnostics}.proto`, `core/src/adapter/{capability,mod}.rs`, `core/src/diagnostics/mod.rs`, `pi-adapter/src/core_client.ts`, generated artifacts, and rolling adapter docs.

Define one generic opaque adapter-profile carriage and the generated Pi profile. Consume the sibling durability/reconciliation contract; uncertain fields remain false/unknown. Do not activate full Pi support until final conformance.

### Unit 2 — Control handshake, materialization, and strict tree integrity

**Story:** `research-handoff-pi-adapter-capability-control-session-integrity`

**Files:** new `pi-adapter/extensions/patchbay-control.ts`; new `pi-adapter/src/{control_handshake,session_file}.ts`; `contracts/proto/patchbay/pi_adapter.proto`; focused fixtures/tests.

Own BLOCKERs 9–10 directly: challenged cwd/session proof, conditional materialization, safe file opening, complete parse/tree/reference validation, raw-vs-RPC comparison, exact pre/post seal, and redaction.

### Unit 3 — Claim-aware RPC process supervisor and effect journal

**Story:** `research-handoff-pi-adapter-capability-rpc-process-supervisor`

**Files:** new `pi-adapter/src/{rpc_client,pi_process,spawn_journal,spawn_supervisor,runtime_action_gate}.ts`; `pi-adapter/src/{pi_session,session_registry,delivery,main,core_client}.ts`; generated Pi target/evidence payloads.

Consume logical identity, continuation authority, claim/fence, crash/effect, runtime-evidence/promotion, duplicate reconciliation, generic restart orchestration, and deployment authority. Production has one RPC lifecycle and no adapter-owned generation increment.

### Unit 4 — Pi-session-scoped authoritative cursor replacement

**Story:** `research-handoff-pi-adapter-capability-cursor-replay-resync`

**Files:** new `pi-adapter/src/{cursor_store,entry_reconciler,pi_projection}.ts`; generated Pi projection envelopes; `operator-domain/src/reconciliation/external_cursor.ts`; known Pi compositor/fold; transcript ingress/presentation vectors.

Implement the spawn cursor leaf: generation-stable verified continuity key, reverse logical-target binding, known suffix idempotency, unknown exact-set/tree staging, one authoritative replacement event, omitted-entry deletion, and atomic local epoch/cursor install only after durable core acknowledgement.

### Unit 5 — Idle-only bounded reload and rehydration

**Story:** `research-handoff-pi-adapter-capability-resource-reload-rehydration`

**Files:** `pi-adapter/extensions/patchbay-control.ts`; new `pi-adapter/src/reload_controller.ts`; `runtime_action_gate.ts`; `entry_reconciler.ts`; `spawn_supervisor.ts`; generated Pi reconfigure payloads and docs.

Reject active/queued/compacting reload before effect, require materialized request/completion markers, re-handshake/reconcile after reload, and distinguish entrypoint/enumerated-resource refresh from process-replacement-only dependency/runtime updates.

### Unit 6 — Integrated Pi lifecycle and mutation-sensitive conformance

**Story:** `research-handoff-pi-adapter-capability-lifecycle-conformance`

**Files:** focused Pi tests, real-process `pi-adapter/tests/e2e.test.ts`, generated vector envelopes/runners, shared operator-domain tests, and `docs/VERIFICATION.md` traceability.

Activate the exact Pi manifest only after all mechanisms pass. Exercise the full core promotion/quarantine path and label evidence implementation-checked only.

## Validated implementation order

```text
spawn logical-target + continuation contract leaves
  └─ Pi control/session-integrity
       └─ claim/crash/runtime-evidence leaves + spawn orchestration
            └─ Pi RPC supervisor/journal
                 └─ spawn cursor-replacement leaf + reconnect contract
                      └─ Pi authoritative cursor replacement

capability durability/reconciliation-depth
  └─ Pi opaque profile contract

Pi profile + supervisor + cursor replacement
  └─ idle-only reload/rehydration
       └─ integrated lifecycle conformance + manifest activation
```

The six shared spawn leaves are consumed where implemented rather than redefined. The profile sibling is an explicit feature dependency. The graph has no edge from a spawn-side contract/operation back into a Pi child.

## Cross-feature dependency edges

| Pi item | Required upstream items | Contract consumed |
|---|---|---|
| feature | `research-handoff-spawn`; `capability-manifest-durability-and-reconciliation-depth` | complete spawn lifecycle plus complete generic assurance registry |
| manifest profile | capability-depth sibling; `research-handoff-spawn-continuation-payload-authority-contract` | generic assurance fields and generated target-spec carriage |
| control/session integrity | logical-target identity leaf; continuation payload/authority leaf | exact external/Pi binding and resume intent |
| RPC supervisor | identity; continuation; claim/fence; crash/effect; runtime-evidence/promotion leaves; duplicate reconciliation; restart orchestration; deployment authority | concrete Pi implementation of five spawn leaves and their operational policy |
| cursor replacement | identity; cursor authoritative-replacement; runtime-evidence/promotion leaves; reconnect convergence | verified continuity scope, exact-set replacement, claimed-successor publication fence |
| lifecycle conformance | runtime-evidence/promotion leaf; stale-event fence; completion/promotion driver; reconnect convergence | integrated staging/quarantine/promotion/replay evidence |

The validated active-substrate dependency walk has no missing edge target and no cycle.

## Failure and availability matrix

| Evidence/state | Resume claim | Cursor/reload availability | Spawn claim outcome |
|---|---|---|---|
| path/id reported; file absent/empty before assistant | never `resumed`; fresh is `new_context` | volatile reconcile only; reload unavailable | require-resume fails pre-launch with exact no-successor-effect evidence |
| file exists but framing/tree/RPC equality fails | none | neither cursor commit nor reload | invalid pre-launch fails closed; post-launch ambiguity poisons |
| materialized valid N, no successor launch | no change | N cursor remains current/stale by evidence | exact pre-launch proof may release only through core contract |
| successor launched, handshake missing/mismatched | none | no publish/commit | effect may exist; poison/reconcile |
| handshake valid, replacement staged, not promoted | `resumed` evidence may be staged but not public current state | replacement remains staged; no ordinary Observation | claim active/poisoned by later evidence |
| promotion committed | status from staged evidence becomes authoritative | publish replacement, commit cursor, then report live | promoted |
| reload requested while busy/unmaterialized | no generation/context change | no marker or reload effect | reconfigure rejected/failed with bounded retryable reason |
| reload markers + re-handshake + reconcile succeed | unchanged | same cursor scope, advanced epoch | reconfigure may complete; no generation bump |

## Simplification and cleanup

- Delete production `PiSession.newSession()` generation allocation and in-process session replacement. Core claims are the only managed generation source.
- Keep one production `ManagedPiRuntimePort` implementation (RPC). Retain one explicitly named `AgentSessionRuntimeFixture`; delete duplicate production SDK lifecycle behavior and ambient model/catalog/credential setup from lifecycle tests.
- Replace unknown-cursor-as-full-success with a typed error and one replacement state machine; do not keep a compatibility upsert path.
- Consolidate safe open, materialization, tree validation, seal, and raw-vs-RPC checks in `session_file.ts`; no scattered path/header checks.
- Consolidate runtime serialization in one action gate used by spawn, delivery, reload, compaction/retry observation, and termination.
- Keep Pi-specific profile vocabulary generated in `pi_adapter.proto`; do not add cwd/trust/extensions/skills/themes/context-file booleans to the core manifest.
- Do not add a second durable authority store. Adapter journal/cursor files are external-effect/reconciliation evidence; core Operation/log state remains authority.
- No `restarting`, `reloading`, `continued`, `materialized`, or `crashed` core protocol state is added. These are Pi profile/evidence details mapped to existing Operation/session/failure semantics.

## Testing and assurance

- **Control proof:** generic RPC path/id without a challenge marker cannot pass; wrong cwd, stale launch nonce, wrong extension epoch/source path, swallowed extension error, marker-only response, and old-child callback all fail.
- **Materialization:** fresh path with no file, user/custom entries without assistant, first assistant flush, abort before assistant, and later materialization transition. No test calls an invented flush or spends real model credentials.
- **Tree integrity:** malformed interior JSON, duplicate ids, missing/forward/self parents, multiple roots, broken label/compaction references, unsupported version/type, truncation, symlink/root escape, inode swap, and raw-vs-RPC mismatch all block `resumed`.
- **Continuation:** idle materialized resume, active abort/settle then seal, memory-only `require_resume`, explicit `allow_new_context`, TERM→KILL, launch-before-journal mutation, handshake loss, and candidate crash before promotion.
- **Cursor replacement:** known suffix response loss; N→N+1 same Pi session loads N cursor; different Pi identity rejects; unknown cursor exact replacement removes an omitted old projected entry; candidate replacement remains staged until promotion; core-ack/local-commit crash is idempotent.
- **Reload:** streaming, compacting, queued, auto-retry, direct-RPC-busy, and unmaterialized requests reject before marker/effect; entrypoint/enumerated resource refresh succeeds only after materialized completion + new handshake + reconcile; transitive dependency and Pi `/dist` changes remain old until process replacement.
- **Manifest honesty:** Pi vocabulary exists only in the generated opaque profile; sibling assurance fields are complete and uncertain false; activation fails if any claimed mechanism/conformance evidence is absent.
- **Fixture boundary:** `AgentSessionRuntimeFixture` receives an injected offline `ModelRuntime` and resource/session services; a mutation that consults ambient credentials/catalog fails.
- **Real process:** use valid prebuilt materialized fixtures and extension commands so process/continuation/reload tests remain offline. Every test awaits child/process-group exit, journal/cursor fsync, Observation acknowledgement, promotion or expected poison, and late async completion.
- **Assurance labels:** green package/E2E/vector evidence is implementation-checked. Formal/release promotion remains separate and must use genuine attempted evidence/mutation-sensitive oracles.

## Adversarial pre-mortem

### Forced adversary — BLOCKER 9: fake cwd proof

**Attack:** launch Pi in the wrong directory while returning the expected `sessionFile/sessionId`; let `get_state`/`get_session_stats` clear the old “cwd handshake.”

**Defense:** those RPCs never clear cwd. The exact current control extension must append the challenged marker containing `ctx.cwd` and the launch nonce; the adapter compares its canonical cwd to the configured project resolution and cross-checks session path/id. Missing/stale/swallowed command evidence fails.

**Fallback:** terminate or quarantine the candidate, poison if launch effect is ambiguous, keep N current/stale/offline by evidence, and never report `resumed` or N+1 live.

### Forced adversary — BLOCKER 10: path exists only in memory

**Attack:** create a fresh Pi session, append a control/reload custom entry before any assistant message, then seal a nonexistent file or call the in-memory marker durable.

**Defense:** materialization is determined by safe file existence + complete raw validation + RPC equality, never by non-empty `sessionFile` or marker presence. `require_resume` and reload are unavailable in `memory_only`; fresh spawn may claim only current new context, not restart-stable persistence.

**Fallback:** leave the current process running and report the capability unavailable, or use an explicitly authorized `allow_new_context` continuation. No hidden model prompt is sent merely to force persistence.

### Forced adversary — BLOCKER 6: stale projection survives full fetch

**Attack:** truncate/switch/corrupt external continuity so the old projection contains entry X, then return a full set omitting X; an upsert-only resync leaves X visible forever.

**Defense:** unknown cursor stages one exact set/tree and publishes one replacement epoch. The consuming projection deletes every prior Pi-persisted member in scope not present in the new set, and cursor/leaf/epoch install only after durable acknowledgement. Generation changes do not change the cursor key when verified Pi continuity is unchanged.

**Fallback:** retain the old view marked stale, keep the new set staged, and retry the same epoch. Never clear first, install a cursor first, or report live.

### Additional material adversaries

- **Reload races streaming:** Pi extension commands execute immediately during streaming. The exclusive runtime gate + settled/state/queue checks reject before invoking the command.
- **Pi parser silently repairs corruption:** malformed lines, duplicate ids, and orphan roots can survive Pi loading. The adapter validates raw bytes and compares the exact RPC set; Pi's permissive load is not the oracle.
- **Reload overclaims dependency updates:** a transitive import or runtime alias stays old. The profile declares only entrypoint/enumerated resources reloadable; unknown/arbitrary dependencies require process replacement.
- **Manifest overclaim:** Pi advertises restart-stable cursor/resume before materialization or before the assurance sibling lands. Declaration activation is gated on conformance and the explicit sibling dependency; uncertainty is false/unknown.
- **Fixture invokes credentials:** an SDK test accidentally exercises model discovery/auth and passes only on a developer host. The named runtime fixture requires fully injected offline services.

### Riskiest assumption and safe fallback

The riskiest assumption is that a custom-entry challenge plus exact raw/RPC tree comparison is sufficient to correlate the running Pi child with the intended cwd/session without a native cwd RPC. This remains bounded by the authenticated-adapter/Pi-child honesty assumption. If the control extension cannot reliably emit and recover the marker, managed `spawn`/continuation support is undeclared and delivery returns `unsupported_command`; attach/read-only behavior remains available.

The second riskiest boundary is authoritative replacement through an opaque core Observation. If the shared operator-domain fold cannot install an exact replacement atomically, implementation must stop and extend that consumer port. It must not fall back to upsert-only replay.

## Review traceability

| Assigned review finding | Resolution | Owning checkpoint(s) |
|---|---|---|
| BLOCKER 6 cursor replacement | verified Pi continuity key; staged exact-set/tree; one replacement epoch deleting omissions; cursor after core ack | cursor-replay-resync; lifecycle conformance |
| BLOCKER 9 cwd RPC proof impossible | challenged control-extension marker reports `ctx.cwd`; RPC verifies only path/id/activity | control-session-integrity; supervisor |
| BLOCKER 10 deferred JSONL materialization | explicit memory-only/materialized/invalid states; no invented flush; resume/reload conditional | control-session-integrity; supervisor; reload |
| Pi vocabulary in core manifest | generic opaque profile carriage; all Pi resource vocabulary generated in `pi_adapter.proto` | manifest-profile |
| reload not fenced | exclusive action gate; settled/stream/compaction/queue/outstanding checks; reject before effect | reload-rehydration |
| seal does not validate tree | strict every-line/schema/id/reference/tree/raw-vs-RPC validation | control-session-integrity |
| manifest “complete” overclaim | feature/story depend on capability-depth sibling; uncertainty false; activation only after conformance | manifest-profile; lifecycle conformance |
| reload scope broader than evidence | entrypoint + enumerated resources only; arbitrary deps/runtime `/dist` require process replacement | manifest-profile; reload-rehydration |
| generation-scoped cursor | verified Pi session continuity key, reverse logical-target binding, N cursor reused by N+1 | cursor-replay-resync |
| `ModelRuntime` fixture naming | `AgentSessionRuntimeFixture` implements runtime port; fully injected offline `ModelRuntime` | supervisor; lifecycle conformance |

## UI fallback / Mockups

No net-new screen or journey. Spawn/restart/reload remain existing Operation actions. Memory-only resume unavailability, busy reload, reconciliation stale state, and ambiguous execution use canonical failure/retry/stale presentation. The opaque Pi profile is rendered through the existing adapter diagnostics composition. Feature-level mockups are skipped.

## Extension pressure classification

- **Committed v1.0.0:** supervised Pi RPC child; generated control extension; challenge-based cwd/session handshake; conditional session materialization; strict tree validation before `resumed`; explicit `new_context`; exact core claim/effect/promotion consumption; Pi-session-scoped cursor; exact-set authoritative replacement; idle-only materialized reload of entrypoint/enumerated resources; generated opaque Pi profile; offline runtime fixture; implementation-backed conformance.
- **Reserved seams:** native Pi cwd/flush/restart RPCs; stronger child attestation; Windows/non-POSIX process fencing; alternate production substrate declared by a future profile; semantic validation of arbitrary extension custom data; automatic crash/restart policy; heartbeat; richer automatic reconciliation; core `ProjectRef`; arbitrary dependency hot-swap if Pi later documents it.
- **Explicitly rejected for this v1 feature:** cwd proof from `get_state`/`get_session_stats`; sealing a missing/empty deferred session file; forcing a hidden LLM turn to create durability; treating Pi load success as full tree validity; generation-keyed native cursor; unknown-cursor upsert; ordinary successor transcript before promotion; reload while active; reload as arbitrary dependency/runtime-package upgrade; mandatory Pi resource fields in core; production SDK fallback; capability as authority.

The parked multi-human, mesh, desktop, and skin ideas remain pressure-test inputs only. Authority-domain-qualified logical identity, generated opaque profiles, and surface-neutral state/failure presentation preserve those seams.

## Child stories

The child files carry authoritative dependency metadata.

- `research-handoff-pi-adapter-capability-manifest-profile`
- `research-handoff-pi-adapter-capability-control-session-integrity`
- `research-handoff-pi-adapter-capability-rpc-process-supervisor`
- `research-handoff-pi-adapter-capability-cursor-replay-resync`
- `research-handoff-pi-adapter-capability-resource-reload-rehydration`
- `research-handoff-pi-adapter-capability-lifecycle-conformance`
