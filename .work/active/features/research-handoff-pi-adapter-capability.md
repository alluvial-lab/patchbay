---
id: research-handoff-pi-adapter-capability
kind: feature
stage: implementing
tags: [adapter, v1]
parent: epic-public-product-contract
depends_on: [research-handoff-spawn]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn]
created: 2026-08-08
updated: 2026-08-12
---

# Pi adapter capability surface for v1 (spawn/restart/reload + manifest)

## Brief and grounded contract

Implement the Pi edge of `research-handoff-spawn` without moving Pi concepts into the core. A successful fresh spawn consumes the core-prepared stable logical target and generation `1`. An intentional restart is a new accepted `spawn` Operation with a new command id/idempotency key and a typed continuation payload naming the exact prior runtime generation. The core owns identity, authority, monotonic claims, tombstones, stale-event fencing, completion, and descendant grants. This feature owns how the Pi adapter realizes the external continuation.

Research grounding is `.research/analysis/campaigns/v1-control-plane-and-spawn/`, especially `specialists/pi-adapter-probe.md` plus attestations `pi-rpc`, `pi-sessions`, `pi-extensions`, `pi-loader`, and `pi-sdk`:

- Pi RPC `get_entries(since)` is authoritative append order for persisted session entries and the current `leafId`; entry ids are stable cursors, including pre-compaction and abandoned branches. Unknown cursors fail explicitly.
- Live RPC events are notifications, not one universal total order. Parallel tool updates can interleave and completion order differs from assistant source order; finalized message/session entries are the repair source.
- `/reload`/`ctx.reload()` refreshes extension entrypoints and resources in the current process. It does not replace the running Pi executable/runtime package graph.
- Continuation preserves the selected JSONL session tree and persisted custom entries, not arbitrary subscriptions, extension variables, loader cache, handles, or external process state.
- Pi has no process-restart RPC. The adapter must quiesce, terminate, verify the persisted session, respawn, rebind, and reconcile.

The current adapter embeds `AgentSessionRuntime`, preprovisions sessions, lets `PiSession.newSession()` allocate `current + 1`, treats an unknown local entry cursor as a full result, and excludes `spawn`. Those choices are replaced for managed v1 runtimes; `ModelRuntime` remains useful for deterministic test fixtures, not as a second production lifecycle.

**Dispatch rationale:** direct-read only. The relevant Pi adapter, generated manifest, resolved spawn stories, research facet, and source attestations form a bounded surface. This delegated lane cannot fan out; no exploratory or advisory subagent was silently substituted. The pre-mortem below is inline as requested.

## Work-nature test

**Non-zero design surface; full feature-design lane.** This feature chooses a production substrate, process and file-integrity boundaries, continuation ordering, crash evidence, cursor-loss recovery, reload semantics, generated public capability shape, and cross-feature ownership. These are externally visible and expensive to reverse at v1; this is not transcription or config-as-prose.

## Design decisions

- **Production substrate: one Pi RPC subprocess per managed runtime generation.** The TypeScript adapter remains the supervisor and Patchbay client; each generation is an isolated `pi --mode rpc` child. This gives an actual per-target terminate/respawn upgrade boundary and the source-attested `get_entries` failure semantics. The SDK `AgentSessionRuntime`/`ModelRuntime` path remains only behind injected unit-test fixtures.
- **Continuation selects an exact persisted session.** Before termination, the adapter obtains `sessionFile`/`sessionId`, seals the canonical allowed-root file/header identity, and after termination verifies it again. Managed continuation uses `--session <canonical-path>`. `--continue` is permitted only if a selector proves one unambiguous candidate and the child handshake returns the exact expected path/id; it is never the normal managed-restart path.
- **Resume is fail-closed by default.** `PiSpawnTargetSpec.continuation_mode=require_resume` refuses a missing/corrupt/mismatched JSONL file. `allow_new_context` is an explicit adapter-owned target-spec choice and reports `new_context`; it never masquerades as `resumed`. `unknown` is used only when the new process/generation can be identified but persisted-context continuity cannot be proved.
- **Quiesce is bounded and honest.** The supervisor fences new generation-N delivery, sends RPC `abort` for active work, waits a configured bound for `agent_settled`, flushes observations/cursor state, then terminates TERM→KILL. Any running Operation without a proved terminal outcome becomes `failed(execution_outcome_unknown)`; accepted/delivered work follows the core's generation-transition policy and is never executed by the fenced child.
- **Crash state follows evidence, not inference.** Unexpected signal/nonzero child exit is explicit `failed`; loss of RPC framing/pipe without conclusive exit is `stale`; expected clean shutdown or confirmed clean exit is `offline`. Activity becomes `unknown` unless current evidence proves otherwise. Crash, disconnect, and clean exit never allocate a generation or auto-restart.
- **Cursor commit follows durable handling.** A Pi entry cursor is committed only after deterministic projection and core acknowledgement. Unknown cursor is a typed error followed by explicit full replay; it is never normalized to an empty suffix or silently treated as current.
- **Live events preserve partial-order honesty.** The adapter may forward transient deltas for responsiveness and serialize the bytes it actually received, but does not claim that arrival order is source authority. `entry_appended` wakes persisted-entry reconciliation; finalized entry/message evidence repairs transient state.
- **Reload is a resource refresh, not a restart alias.** A typed Pi `reconfigure.reload_resources` action invokes an adapter extension command that performs `await ctx.reload(); return`. Success requires a persisted request/completion marker observed after `session_start(reason=reload)`. Runtime/package upgrades are rejected on this path and require a new spawn continuation.
- **The capability manifest is generated, complete, and advisory.** A runtime-session profile declares mechanisms plus typed limitations. It can guide UX and diagnostics but never replaces grants, core delivery, authenticated reports, or the adapter's delivery result.
- **Project/cwd remains adapter-owned.** `PiSpawnTargetSpec` names a configured `project_ref`; the adapter resolves it to canonical cwd, trust/resource roots, session directory, executable, and launch policy. Raw cwd/labels never become core identity or grant authority.
- **Sibling capability-depth ownership stays separate.** This feature owns the Pi mechanism profile and its concrete limits. `capability-manifest-durability-and-reconciliation-depth` later adds cross-adapter assurance strength (dedup/continuation proof/cursor/generation-fence/reconciliation depth) without re-declaring these nine mechanisms.

## Architectural options

### Option A — Keep all managed sessions embedded in the adapter process

Extend current `AgentSessionRuntime` and `ModelRuntime` use, replace sessions in-process, and restart the whole adapter for package upgrades. This preserves typed APIs and requires the least immediate rewrite, but one target cannot cross the reliable process-upgrade boundary without replacing every colocated runtime and adapter attachment. It also makes crash/process evidence less isolated. **Rejected for production; retained for deterministic tests.**

### Option B — Supervise a Pi RPC child per managed generation

The adapter owns process groups, JSONL framing, exact session selection, lifecycle evidence, and persisted-entry reconciliation. Each target can be quiesced/replaced independently; runtime/package updates load in a fresh child; malformed transport or child exit has an observable boundary. This adds a strict RPC client and process tests, but aligns most directly with the grounded contract and current Remote-Pi supervisory precedent. **Chosen.**

### Option C — Hybrid production: SDK for ordinary delivery, RPC only for restart

Keep the embedded runtime live, then translate to an RPC child only at upgrade/restart. This appears incremental but creates two production event/cursor/reload implementations and makes a single manifest unable to state which guarantees apply at a given moment. The transition itself becomes the least-tested lifecycle. **Rejected.**

## Trickiest unit first: continuation transaction across process and persisted session

The risky unit is not `spawn()`; it is the ordered handoff from a fenced generation N to a verified and reconciled generation N+1 without claiming continuity too early.

```ts
export interface RuntimeGenerationKey {
  authorityDomainId: string;
  logicalTargetId: string;
  runtimeSessionId: string;
  generation: bigint;
}

export interface SessionFileSeal {
  canonicalPath: string;
  sessionId: string;
  cwd: string;
  device: bigint;
  inode: bigint;
  minimumSize: bigint;
}

export interface PiProcessPort {
  launch(spec: PiLaunchSpec): Promise<PiRpcRuntime>;
  terminate(runtime: PiRpcRuntime, policy: TerminationPolicy): Promise<ProcessExit>;
}

export interface SpawnSupervisor {
  spawnFresh(operation: Operation, claim: SpawnGenerationClaim): Promise<SpawnedRuntime>;
  continueGeneration(
    operation: Operation,
    claim: SpawnGenerationClaim,
    prior: RuntimeGenerationRef,
  ): Promise<SpawnedRuntime & { continuationStatus: ContinuationStatus }>;
}
```

Continuation order is fixed:

1. Under a per-logical-target mutex, validate the accepted Operation/claim, generated Pi target spec, configured project/deployment authority, and adapter spawn journal. The claimed generation must be exact `N+1`.
2. Fence new delivery to N and capture the current child identity, Pi `get_session_stats`, cursor record, and pre-stop `SessionFileSeal`.
3. If active, send `abort`, await correlated acknowledgement and `agent_settled` within the configured quiesce bound, flush observations, and report any known terminal effects. Unproved running work is `execution_outcome_unknown`.
4. Close stdin/request graceful shutdown, then apply TERM→KILL escalation to the whole child process group. Late stdout/events are generation-token fenced.
5. Re-verify the same canonical regular JSONL file: allowed root, header/session id/cwd, device/inode, non-truncation, and complete LF framing. A mismatch fails before launch and never logs the path.
6. Launch a new child with an absolute executable and argv array (`--mode rpc`, exact cwd/session directory, adapter control extension, and normally `--session <canonical path>`); no shell interpolation or payload-supplied flags.
7. Handshake with `get_session_stats`/`get_state`; verify exact path/id/cwd and bind the core-claimed generation. If an explicitly allowed fresh fallback was taken, report `new_context`.
8. Rebind subscriptions and run Pi entry reconciliation. Keep connectivity/activity stale/unknown until both handshake and reconciliation succeed.
9. Report the exact logical-target generation advance, continuation reference/status, source cursor, then the successful spawn Result. The core completion owner issues descendant authority and terminalizes last.

Failure before external launch is an ordinary execution failure with N fenced/failed/offline according to evidence. Failure after launch but before durable external identity/proof is `execution_outcome_unknown` and follows the spawn journal policy; the supervisor does not launch again automatically.

## Minimum generated capability manifest

`contracts/proto/patchbay/adapter.proto` adds one required runtime-session profile when `target_categories` includes `runtime_session`; adapter-owned mechanism ids are bounded identifiers, while values that affect generic presentation/reconciliation use enums/booleans.

```proto
message RuntimeSessionCapability {
  TransportCapability transport = 1;
  PromptingCapability prompting = 2;
  EventCapability events = 3;
  CursorReplayCapability cursor_replay = 4;
  SessionPersistenceCapability session_persistence = 5;
  SessionReplacementCapability session_replacement = 6;
  ReloadCapability reload = 7;
  ResourceScopeCapability resource_scope = 8;
  StateRehydrationCapability state_rehydration = 9;
}

message TransportCapability {
  string mechanism = 1;              // bounded adapter-owned id
  bool process_isolation = 2;
}
message PromptingCapability {
  bool prompt = 1;
  bool steering_queue = 2;
  bool follow_up_queue = 3;
}
enum EventOrderingGuarantee {
  EVENT_ORDERING_GUARANTEE_UNSPECIFIED = 0;
  EVENT_ORDERING_GUARANTEE_TOTAL = 1;
  EVENT_ORDERING_GUARANTEE_PARTIAL = 2;
  EVENT_ORDERING_GUARANTEE_NONE = 3;
}
message EventCapability {
  bool streaming = 1;
  EventOrderingGuarantee ordering = 2;
  bool parallel_tool_interleaving = 3;
  bool finalized_message_authoritative = 4;
}
enum UnknownCursorBehavior {
  UNKNOWN_CURSOR_BEHAVIOR_UNSPECIFIED = 0;
  UNKNOWN_CURSOR_BEHAVIOR_REJECT = 1;
}
enum CursorRecoveryMode {
  CURSOR_RECOVERY_MODE_UNSPECIFIED = 0;
  CURSOR_RECOVERY_MODE_FULL_REPLAY = 1;
  CURSOR_RECOVERY_MODE_MANUAL = 2;
  CURSOR_RECOVERY_MODE_NONE = 3;
}
message CursorReplayCapability {
  string mechanism = 1;
  bool persisted_entries_only = 2;
  bool stable_across_process_restart = 3;
  bool returns_current_leaf = 4;
  UnknownCursorBehavior unknown_cursor = 5;
  CursorRecoveryMode recovery = 6;
}
message SessionPersistenceCapability {
  string mechanism = 1;
  bool tree_and_branches = 2;
  bool pre_compaction_history = 3;
  bool custom_entries = 4;
  bool process_state_preserved = 5;
}
enum SessionReplacementMechanism {
  SESSION_REPLACEMENT_MECHANISM_UNSPECIFIED = 0;
  SESSION_REPLACEMENT_MECHANISM_IN_PROCESS = 1;
  SESSION_REPLACEMENT_MECHANISM_SUPERVISED_PROCESS = 2;
}
message SessionReplacementCapability {
  SessionReplacementMechanism mechanism = 1;
  bool explicit_session_selection = 2;
  bool continuation_status_reported = 3;
}
enum ReloadScope {
  RELOAD_SCOPE_UNSPECIFIED = 0;
  RELOAD_SCOPE_EXTENSION_RESOURCES = 1;
  RELOAD_SCOPE_RUNTIME_PACKAGE_GRAPH = 2;
}
message ReloadCapability {
  ReloadScope scope = 1;
  bool runtime_upgrade_requires_process_replacement = 2;
}
message ResourceScopeCapability {
  bool cwd = 1;
  bool project_trust = 2;
  bool extensions = 3;
  bool skills = 4;
  bool prompts = 5;
  bool themes = 6;
  bool context_files = 7;
}
enum StateRehydrationMode {
  STATE_REHYDRATION_MODE_UNSPECIFIED = 0;
  STATE_REHYDRATION_MODE_PERSISTED_ONLY = 1;
  STATE_REHYDRATION_MODE_FULL_RUNTIME = 2;
  STATE_REHYDRATION_MODE_NONE = 3;
}
message StateRehydrationCapability {
  StateRehydrationMode mode = 1;
  bool requires_resubscribe = 2;
  bool adapter_journal = 3;
}
```

The Pi v1 manifest declares:

| Dimension | Pi declaration |
|---|---|
| transport | `mechanism=pi-rpc-jsonl`, `process_isolation=true` |
| prompting | prompt + steering + follow-up queues |
| events | streaming, `partial`, parallel-tool interleaving, finalized message authoritative |
| cursor replay | `pi-session-entry-id`, persisted entries only, restart-stable, returns leaf, unknown=`reject`, recovery=`full_replay` |
| session persistence | `pi-jsonl-session-tree`, tree/branches + pre-compaction + custom entries; process state not preserved |
| session replacement | supervised process, explicit session selection, continuation status reported |
| reload | extension resources only; runtime upgrade requires process replacement |
| resource scope | cwd, project trust, extensions, skills, prompts, themes, context files |
| state rehydration | persisted only, re-subscribe required, adapter journal present |

Existing top-level declarations remain: `target_categories=[runtime_session]`, `session_snapshot_support=partial`, `session_replacement_support=true`, `idempotency_strength=at_patchbay_boundary`, and supported Operations include `spawn` only when the supervisor/project resolver/journal are configured. Fresh attach rejects an incomplete runtime profile. Real durable pre-profile registrations are replay-normalized once to unknown/false values and cannot be used to advertise support until the adapter redeclares. The summary contract embeds this generated profile rather than hand-copying its fields.

## Implementation units and child checkpoints

### Unit 1 — Generated runtime capability profile

**Story:** `research-handoff-pi-adapter-capability-manifest-profile`

**Files:** `contracts/proto/patchbay/{adapter,diagnostics}.proto`, `core/src/adapter/{capability,mod}.rs`, `core/src/diagnostics/mod.rs`, `pi-adapter/src/core_client.ts`, diagnostic consumers, and rolling docs.

- Add the minimum profile above, complete fresh-attach validation, replay-only conservative legacy normalization, and the exact Pi declaration.
- Preserve capability-not-authority and capability-not-delivery semantics.
- Leave the additive assurance-strength seam to `capability-manifest-durability-and-reconciliation-depth`.

### Unit 2 — RPC child and continuation supervisor

**Story:** `research-handoff-pi-adapter-capability-rpc-process-supervisor`

**Files:** new `pi-adapter/src/{rpc_client,pi_process,session_file,spawn_supervisor}.ts`, `contracts/proto/patchbay/pi_adapter.proto`, and `pi-adapter/src/{pi_session,session_registry,delivery,main,core_client}.ts`.

```ts
export type PiRpcRecord = PiRpcResponse | PiRpcEvent;
export interface PiRpcClient {
  request<T>(command: PiRpcCommand<T>, signal?: AbortSignal): Promise<T>;
  subscribe(listener: (event: PiRpcEvent) => void): () => void;
  waitForExit(): Promise<ProcessExit>;
}
```

- Enforce strict LF JSONL, maximum record size, unique request ids, fail-closed malformed stdout, bounded stderr, and generation-local subscriptions.
- Resolve `PiSpawnTargetSpec.project_ref` through adapter configuration; never pass payload paths/argv/env through unchecked.
- Implement the fixed continuation transaction and evidence-specific crash mapping.

### Unit 3 — Cursor replay and full resync

**Story:** `research-handoff-pi-adapter-capability-cursor-replay-resync`

**Files:** new `pi-adapter/src/{cursor_store,entry_reconciler}.ts`, `pi-adapter/src/{rpc_client,pi_session,spawn_supervisor,transcript_projection,main}.ts`, core transcript dedup/reconciliation seams, and vectors.

```ts
export interface PiEntryCursorRecord {
  logicalTargetId: string;
  generation: bigint;
  piSessionId: string;
  sessionFileIdentity: string;
  entryId?: string;
  leafId?: string;
}
export interface PiEntryCursorStore {
  load(key: RuntimeGenerationKey): Promise<PiEntryCursorRecord | undefined>;
  commit(record: PiEntryCursorRecord): Promise<void>;
  clear(key: RuntimeGenerationKey): Promise<void>;
}
export type PiCursorReconcile =
  | { kind: "suffix"; processed: number; leafId: string | null }
  | { kind: "full-resync"; processed: number; leafId: string | null };
```

- Atomically store 0600 cursor state under an adapter-local data root.
- Await core ingestion before each cursor advance. Deterministic entry identities make replay after response loss inert.
- On unknown cursor, keep stale, fetch all entries, reconcile, then install the new cursor/leaf. Never clear first and claim an empty/current view.

### Unit 4 — Reload bridge and persisted rehydration

**Story:** `research-handoff-pi-adapter-capability-resource-reload-rehydration`

**Files:** new `pi-adapter/extensions/patchbay-control.ts`, new `pi-adapter/src/reload_controller.ts`, `contracts/proto/patchbay/pi_adapter.proto`, and `pi-adapter/src/{delivery,entry_reconciler,spawn_supervisor,core_client}.ts`.

```ts
export interface ReloadController {
  reloadResources(operation: Operation, runtime: PiRpcRuntime): Promise<void>;
}
```

- The extension records a request marker keyed from the Patchbay command id, calls `await ctx.reload(); return`, and the newly bound instance records completion from `session_start(reason=reload)`.
- The controller treats the RPC prompt response only as receipt; persisted completion plus reconciliation establishes success.
- Rebind subscriptions and extension state from persisted custom entries/journal. No generation bump and no runtime-upgrade claim.

### Unit 5 — Integrated lifecycle conformance

**Story:** `research-handoff-pi-adapter-capability-lifecycle-conformance`

**Files:** Pi focused tests, real-process `pi-adapter/tests/e2e.test.ts`, vector registry/runners, and `docs/VERIFICATION.md` traceability.

- Exercise real child spawn/restart/reload/exit while fakes inject malformed framing, response loss, path corruption, and unknown cursors.
- Await process cleanup, observation flush, journal/cursor durability, and core terminal state so late async failures cannot pass.
- Claim implementation-checked evidence only unless separate formal/vector promotion clears its gate.

## Implementation order and dependency graph

```text
research-handoff-spawn-restart-continuation-orchestration
  └─ research-handoff-pi-adapter-capability-manifest-profile
       └─ research-handoff-pi-adapter-capability-rpc-process-supervisor
            ├─ consumes research-handoff-spawn-idempotency-duplicate-handling
            ├─ consumes deployment-authority-workspace-scoped-revocable-keys
            └─ research-handoff-pi-adapter-capability-cursor-replay-resync
                 ├─ consumes research-handoff-spawn-reconnect-cursor-reconcile
                 └─ research-handoff-pi-adapter-capability-resource-reload-rehydration
                      └─ research-handoff-pi-adapter-capability-lifecycle-conformance
                           └─ also consumes research-handoff-spawn-stale-event-fencing
```

One feature owner remains the baseline. The stories are contract/lifecycle/evidence checkpoints with distinct acceptance, not one-worker-per-package assignments.

## Simplification and cleanup

- Remove production `PiSession.newSession()` generation allocation; every managed generation comes from an accepted core claim.
- Replace production in-process `AgentSessionRuntime` replacement with one RPC subprocess lifecycle. Retain SDK/`ModelRuntime` only as an injected test fixture and delete duplicate production behavior.
- Replace the current `getEntries(since)` behavior that returns the full set on an unknown cursor with a typed failure and one explicit full-resync state machine.
- Consolidate process creation, termination, crash classification, path verification, and continuation in `SpawnSupervisor`; do not scatter child-process calls through delivery/session code.
- Consolidate Pi capability declarations in `piCapabilityManifest()` from generated fields; diagnostics and surfaces consume the projection rather than re-listing it.
- Replace touched ad-hoc reload/model reconfigure payload parsing with generated Pi adapter payloads where the new `pi_adapter.proto` boundary overlaps; do not create a parallel hand-written DTO.
- Roll `docs/ADAPTER-PI.md` forward: its current “`--continue` does not bump generation,” “spawn unsupported,” SDK-internal replacement, and reload-out-of-scope claims contradict the resolved v1 lifecycle.
- Do not add `restarting`, `reloading`, `continued`, or `crashed` protocol states. Reuse Operation state, continuation status, connectivity/activity axes, and failure vocabulary.

## Testing and assurance

Smallest useful risk-based surface:

- **Generated interface:** Buf generation/drift and Rust/TypeScript builds protect the public profile, Pi payloads, required presence, and summary carriage.
- **Manifest boundary:** fresh missing/unknown/overclaim declarations reject; replay-only legacy normalization is conservative; `spawn` support toggles only with a configured supervisor. Protects capability honesty without making it authority.
- **RPC framing:** split/multiple UTF-8 chunks, CRLF tolerance only as documented, oversized/malformed lines, response/event interleaving, duplicate ids, stderr noise, and EOF. Protects the process trust boundary.
- **Continuation integration:** fresh generation 1, idle resume, active abort/settle, TERM→KILL timeout, exact session-id/path restore, explicit new-context fallback, and effect-before-report loss. Protects continuity and duplicate honesty.
- **File-integrity regression:** symlink/root escape, header mismatch, cwd mismatch, truncation, inode replacement, missing trailing LF, and path redaction. Protects wrong-session continuation.
- **Cursor reconciliation:** known suffix, cursor response loss, unknown cursor full replay, empty session, branch/leaf change, pre-compaction/abandoned entries, and duplicate replay. Protects cursor honesty.
- **Partial-order events:** interleaved parallel tool notifications plus authoritative finalized message/session entry. Protects against treating live arrival as total source order.
- **Reload boundary:** extension/skill/prompt/theme/context change becomes visible after persisted reload completion; Pi/runtime package change remains old until process continuation. Protects reload-vs-restart honesty.
- **Crash vocabulary:** explicit signal/nonzero exit, orphaned RPC with child status unknown, and expected clean exit map to failed/stale/offline with activity unknown and no generation change.
- **Real-process E2E:** run core + adapter + Pi RPC child; assert no orphan child, unhandled async failure, raw session path/credential diagnostic, or early successful spawn/reload completion.
- **Test removal:** retire same-process `newSession()` production expectations and tests that accept unknown cursor as implicit full success. Keep SDK tests only where they exercise projection/fixture behavior shared with the RPC port.

## Pre-mortem and risks

### 1. Cursor dishonesty

**Failure:** the adapter treats an unknown cursor or a quiet live stream as “caught up,” or commits before the core accepted an entry. The cockpit looks current while transcript/leaf state is missing.

**Mitigation:** typed unknown-cursor failure; explicit stale full-resync mode; commit-after-core-ack per entry; deterministic duplicate-inert projection; live notifications only wake reconciliation. Mutation tests remove each guard.

**Fallback:** clear only the generation-scoped staged cursor after recording bounded resync evidence, replay the entire verified session file, and remain stale/unknown. Never fabricate an empty suffix or live state.

### 2. Session-path corruption or wrong-session continuation

**Failure:** a stale path, symlink escape, truncated file, changed header, or ambiguous `--continue` resumes a different conversation under the claimed logical target.

**Mitigation:** configured root/project resolver; pre/post `SessionFileSeal`; bounded JSONL header and LF validation; exact post-launch `get_session_stats`; no raw path in diagnostics; default `require_resume`.

**Fallback:** do not respawn/report N+1. Fail the spawn and leave the logical target failed/stale for operator reconciliation. Only an explicit `allow_new_context` Operation may produce `new_context`.

### 3. Reload mistaken for restart

**Failure:** a successful extension reload is displayed as a Pi/runtime upgrade, but the child still holds the old installed package graph.

**Mitigation:** separate typed reload action and manifest scope; persisted reload completion marker; package-upgrade requests reject on reload; conformance test changes both extension and runtime-package code and proves only process replacement loads the latter.

**Fallback:** keep the current generation and report reload failure/unsupported. The operator submits a new spawn continuation for the upgrade.

### 4. SDK/RPC semantic split

**Failure:** SDK tests pass while production RPC diverges on cursor failure, prompt acknowledgement, replacement, or event ordering.

**Mitigation:** one production `PiRuntimePort` implementation (RPC); SDK only deterministic fixture; real-process lifecycle E2E; manifest says RPC, never “SDK or RPC.”

**Fallback:** disable managed spawn/reload declaration and return `unsupported_command`; keep attach/read-only behavior rather than silently falling back to embedded production semantics.

### 5. Effect-before-proof during continuation

**Failure:** a new child/session exists, but the adapter crashes before journaling/reporting identity. Automatic retry creates a second child.

**Mitigation:** consume the spawn journal/claim contract, journal before external launch and immediately after identity handshake, classify ambiguity as `execution_outcome_unknown`, and never auto-relaunch an ambiguous claim.

**Fallback:** require operator reconciliation/new intentional Operation; keep idempotency strength `at_patchbay_boundary`.

### 6. Quiesce never settles or late output crosses generations

**Failure:** abort hangs, termination leaks a descendant process, or buffered stdout from N mutates N+1.

**Mitigation:** per-target fence before abort, bounded `agent_settled` wait, process-group TERM→KILL, binding-local generation tokens, shared core stale fence, and tests that emit after replacement.

**Fallback:** fail/unknown the affected work, mark N failed, and do not report N+1 until the old process group is conclusively gone and reconciliation succeeds.

### Riskiest assumption and least-certain boundary

The riskiest assumption is that Pi's persisted JSONL path/id plus append-order entries are sufficient to prove transcript continuation without claiming runtime-state continuation. Research supports the persisted layer but not arbitrary extension or external side effects. The design therefore reports only `resumed/new_context/unknown`, keeps snapshot tier partial, and makes state rehydration persisted-only.

The least-certain implementation boundary is the adapter reload bridge in RPC mode: built-in interactive `/reload` is not a general RPC command, so success must be established through the extension command's persisted request/new-instance completion protocol. If real Pi cannot produce that trace reliably, reload support remains undeclared/unsupported; process continuation still provides the reliable upgrade path.

## UI fallback / Mockups

No net-new screen or navigation flow. Spawn/restart actions are owned by the resolved spawn feature and reuse existing Operation delivery/failure presentation. Reload is a Pi-supported reconfigure action in the existing session-detail action surface; manifest details use existing adapter diagnostics. This is minor composition, so feature-level mockups are skipped.

## Extension pressure classification

- **Committed v1.0.0:** RPC subprocess production substrate; one child per managed generation; exact generation-1/continuation claim consumption; bounded quiesce/terminate/verify/respawn/reconcile; explicit failed/stale/offline crash evidence; persisted entry cursor/full resync; extension-resource-only reload; minimum generated runtime profile; adapter-owned project/cwd resolution; partial snapshot and persisted-only rehydration honesty.
- **Reserved seams:** alternate adapter substrates (including a future SDK-isolated worker) selected by a future manifest declaration; stronger end-to-end spawn idempotency; automatic crash policy; heartbeat/freshness deadlines; Windows/non-POSIX process fencing; full semantic validation of extension custom state; richer cross-adapter durability/reconciliation strength in `capability-manifest-durability-and-reconciliation-depth`; core `ProjectRef`; HA/process-incarnation fencing.
- **Explicitly rejected for this v1 feature:** two production substrates behind one declaration; adapter-local generation allocation; automatic restart on crash; ambiguous `--continue`; reload as runtime/package upgrade; cursor fallback presented as an empty suffix; RPC live events as a universal total order; arbitrary process-state restoration; raw cwd/path as core identity/authority; capability declarations as grant or delivery authority.

The parked multi-human, mesh, desktop, and skin ideas do not become requirements. Generated adapter declarations, logical-target qualification, and surface-neutral canonical states preserve their seams.

## Child stories

- `research-handoff-pi-adapter-capability-manifest-profile` — `depends_on: [research-handoff-spawn-restart-continuation-orchestration]`
- `research-handoff-pi-adapter-capability-rpc-process-supervisor` — `depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-spawn-restart-continuation-orchestration, research-handoff-spawn-idempotency-duplicate-handling, deployment-authority-workspace-scoped-revocable-keys]`
- `research-handoff-pi-adapter-capability-cursor-replay-resync` — `depends_on: [research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-spawn-reconnect-cursor-reconcile]`
- `research-handoff-pi-adapter-capability-resource-reload-rehydration` — `depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-pi-adapter-capability-rpc-process-supervisor, research-handoff-pi-adapter-capability-cursor-replay-resync]`
- `research-handoff-pi-adapter-capability-lifecycle-conformance` — `depends_on: [research-handoff-pi-adapter-capability-cursor-replay-resync, research-handoff-pi-adapter-capability-resource-reload-rehydration, research-handoff-spawn-stale-event-fencing]`
