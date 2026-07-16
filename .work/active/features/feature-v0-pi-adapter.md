---
id: feature-v0-pi-adapter
kind: feature
stage: implementing
tags: [adapter, protocol]
parent: epic-v0-1-0-implementation
depends_on: [epic-v0-core]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-15
---

# Feature: Pi adapter

## Brief

Build the Pi adapter — the first and only required runtime adapter for v0.1.0. The adapter translates between Patchbay's adapter-neutral protocol and Pi's session model: session discovery and status, prompt/instruction delivery, cancel/interrupt where supported, and replies/events/snapshots streamed back to the core.

The adapter is a principal with an explicit registration lifecycle (attach with capability manifest, detach, failure, capability redeclaration). It declares a snapshot tier of `partial` (per `docs/ADAPTER-PI.md`), meaning it can provide recent/current state via transcript event log replay but not arbitrary historical reconstruction. The core's degraded-behavior rules handle the rest honestly.

Pi know-how harvests from remote_pi's `pi-extension/` (Node + TypeScript), which already implements session gating, turn state, transcript event log, transcript projection, and SDK session projection. The harvest is re-housed behind Patchbay's adapter port — harvesting the Pi know-how, not the extension shape. This is real adapter-implementation work, not a copy. See `.work/backlog/idea-harvest-remote-pi-extension-as-adapter.md` for the harvest mapping (what harvests, what does not, re-housing caveat).

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: parallel with the web chain (protocol-seam → web-server → web-cockpit) after the core lands. The agent-control path (core → pi-adapter) is independent of the phone-usable path.
- The Pi adapter and the web chain can proceed in parallel once the core is up.

## Key design decisions (already settled in `docs/ADAPTER-PI.md`)

- **`session_new` = session replacement, generation bump + tombstone.** Pi's `session_new` tears down the old SDK context and marks it stale; it maps to a `session_generation` bump, not a same-generation clear. Late events binding to the pre-`new` context become `stale_event` audit records.
- **`spawn` not implemented in v0.1.0.** Provisioning is out-of-band sysadmin (pi-supervisord). The adapter declares `spawn` unsupported at delivery (`unsupported_command`); the operator provisions runtimes out-of-band and Patchbay `attach`es.
- **Snapshot tier = `partial`.** remote_pi provides a transcript event log replayed via `session_sync`, not authoritative historical reconstruction.
- **`session_compact` does not bump generation.** Compaction is in-place.

## Foundation references

- `docs/ADAPTER-PI.md` — Pi parity checklist, capability mapping, session_new classification, snapshot tier
- `docs/PROTOCOL.md` — Adapter capabilities, adapter registration and lifecycle, adapter snapshot capability tiers
- `docs/ARCHITECTURE.md` — Adapter plane, adapter registration and lifecycle
- `contracts/proto/patchbay/adapter.proto` — adapter capability, registration, attachment method
- `.work/backlog/idea-harvest-remote-pi-extension-as-adapter.md` — harvest mapping (what to harvest, what to replace, re-housing caveat)
- remote_pi source: `/home/agent/projects/remote_pi/pi-extension/`
  *(Note: the project was forked/renamed to **outpost_pi** at `/home/agent/projects/outpost_pi/`; patchbay still references the pre-fork "remote-pi" name throughout docs/work/research — see parked naming-cleanup item below.)*

## Harvest grounding (2026-07-15)

The harvest source is **outpost_pi** (`/home/agent/projects/outpost_pi/pi-extension/`), not "remote_pi". Two findings reshaped the design:

1. **Pi's `--mode rpc` is a complete, documented JSON-over-stdio RPC protocol** (Pi's own `docs/rpc.md` in the installed SDK), not just `prompt`. It covers the *entire* ADAPTER-PI §4 wire-action surface directly: `prompt` (instruct), `abort` (cancel), `get_state`/`get_entries(since)` (query/snapshot — `get_entries` is a durable cursor), `set_model`/`set_thinking_level` (reconfigure), `get_available_models` (query), `new_session` (session-management), `compact` (session-management), and tool-call approval via the `extension_ui_request`/`extension_ui_response` sub-protocol (`select` with "Allow/Block"). Plus a full event stream (`turn_start/end`, `message_update/end`, `tool_execution_*`, `compaction_*`, `agent_end`, `agent_settled`) that maps to Patchbay `Observation`s. **No in-process extension is required** — a separate process can drive Pi entirely over stdio.
2. **outpost-pi's `rpc_child.ts` only implements `sendPrompt`** because its wave-2 scope was fire-and-forget ("stdout consumed line-by-line and ignored"). Initial investigation suggested the RPC protocol was complete and could drive everything — **but verification disproved this for the approval gate** (see point 3): `pi --mode rpc` emits zero tool-approval frames, so the harvest is the **in-process session layer** (`sdk_session_projection.ts`, `transcript_projection.ts`, `transcript_event_log.ts`, `turn_state.ts`) driven via programmatic `createAgentSession()`, NOT the stdio-RPC-client pattern.
3. **Tool-call approval is an in-process hook, not an RPC frame** (verified 2026-07-15 against `dist/modes/rpc/rpc-mode.js` + `dist/core/agent-session.js:191`): the `beforeToolCall` gate is wired exclusively to the in-process extension runner (`runner.emitToolCall`); with no `tool_call` handler, tools auto-execute. The `extension_ui_request` sub-protocol is for generic extension UI (`ctx.ui.select`/`confirm`), not the permission gate. outpost-pi's own source confirms approval is `pi.on("tool_call")` with `{ block: true }`. **This closed off the pure-RPC-client option (b') and selected programmatic `AgentSession` (a).**

## Design decisions (operator-confirmed leans)

- **Pi driving mechanism**: (a) programmatic `AgentSession`. The adapter is a Node process that imports `@earendil-works/pi-coding-agent` and calls `createAgentSession()` directly, setting `beforeToolCall` for the tool-call approval gate and subscribing to the typed `AgentSessionEvent` stream. Pi's own `docs/rpc.md` endorses this as the native Node path ("consider using `AgentSession` directly instead of spawning a subprocess"). **Verification closed off the pure-RPC alternative (b')**: `pi --mode rpc` emits zero tool-approval frames — the `beforeToolCall` gate is wired exclusively to the in-process extension runner, so a pure-RPC client cannot intercept or approve tool calls (they auto-execute with no extension). The harvest is therefore the **in-process session layer** from outpost-pi (`sdk_session_projection.ts`, `transcript_projection.ts`, `transcript_event_log.ts`, `turn_state.ts`) — the original harvest idea's "what harvests" list, re-housed behind Patchbay's adapter port.
- **Adapter↔core transport**: (b) adapter-as-client of the core. The adapter calls `Attach` + `IngestObservation` + holds a `Subscribe`-stream open for delivered Operations. Reuses the seam's gRPC/shared-secret infrastructure; the core stays a passive server (no outbound-initiation logic — matches the single-authoritative-core model).
- **Snapshot source**: (a) adapter-local `TranscriptEventLog`, fed by translating Pi RPC events (`get_entries(since)` + live `message_update`/`tool_execution_*` events) into typed `TranscriptEvent`s via the harvested `transcript_projection.ts` know-how. `get_entries(since)` is itself a durable cursor — the partial-snapshot reconciliation source. The local log survives brief Pi disconnects.
- **v0.1.0 scope**: (b) minimal slice first — `attach` + `instruct` + `cancel` + observation streaming — then add `approval-response`/`query`/`reconfigure`/`session-management` as follow-on stories. The loop proves the architecture; the rest are additive delivery mappings.
- **`spawn` is a fast-follower, not v0.1.0** (operator signal: outpost-pi lacks mobile spawn, and it's the reserved seam the operator most wants next). v0.1.0 declares `spawn` unsupported at delivery (`unsupported_command`); the adapter attaches to *pre-provisioned* in-process `AgentSession`s (targets created at startup from config). **But the adapter is built around a session registry, not a single Pi session**, so the fast-follower `spawn` is "create a new `AgentSession` in-process + report the new `runtime_session_id`" — additive, not architectural. The core's `spawn` authority modeling (descendant grant, fleet-level target scope, idempotency-strength) is already committed in PROTOCOL.md §"Spawn payload and authority commitments"; no core work is needed for the fast-follower.

## Architectural choice

**A Node process (`patchbay-pi-adapter`) that is a gRPC client of the Patchbay core AND an in-process host of one-or-more Pi `AgentSession`s.** It owns three concerns: (1) adapter registration (attach to the core with its capability manifest, declare `spawn` unsupported), (2) a session registry mapping `runtime_session_id → AgentSession` (the fast-follower `spawn` creates a new `AgentSession` in-process — additive to the registry), and (3) bidirectional translation — core `Operation`s → `AgentSession` API calls, and `AgentSession` events → core `Observation`s (via the harvested transcript projection). It owns NO domain logic — no authority, no durable state beyond the local transcript log (which is a partial-snapshot cache, not authoritative).

This realizes the committed v0.1.0 adapter plane: the adapter is a principal with an explicit registration lifecycle, declares `snapshot=partial`, and the core's degraded-behavior rules handle the rest. The adapter is a separate process from the core (matches the two-process topology; the adapter is a third process).

Chosen over:
- *Pure RPC client over `pi --mode rpc` stdio (b')* — **rejected by verification**: `pi --mode rpc` emits no tool-approval frame; the `beforeToolCall` gate is wired only to the in-process extension runner, so a pure-RPC client cannot intercept tool calls (they auto-execute). The approval surface requires being in-process. (See the Harvest grounding + the Q2 verification note below.)
- *Hybrid: `pi --mode rpc` + thin extension bridging approvals over a side channel (c')* — viable but rejected on code economy: it introduces a side-channel protocol to design, two artifacts (extension + adapter), and novel approval-bridging that outpost-pi does in-process. The programmatic `AgentSession` path (a) gets the approval hook directly (`beforeToolCall`) with none of that overhead.
- *Adapter as a gRPC server the core dials out (a-transport)* — rejected: the core would need outbound-initiation logic and per-adapter connection management, complicating the single-writer model. Adapter-as-client reuses the seam's proven gRPC infrastructure.

## The adapter-facing core RPC surface (does not exist yet — Unit 1)

The seam shipped only web-server-facing `ControlService` (`Submit`/`Subscribe`/`LoadSnapshot`). The adapter needs a different surface: it must `Attach` with a capability manifest, `IngestObservation` (session reports + events), and receive delivered `Operation`s over a stream. This is a new proto service + core-side impl, scoped to this feature (or a prerequisite story). It is NOT a modification of `ControlService` (different principal, different auth posture — the adapter authenticates via attachment evidence, not the web-server shared secret).

## Implementation Units

### Unit 1: Adapter-facing proto service + core impl (the new core surface)
**File**: `contracts/proto/patchbay/adapter_control.proto` (new), `server/src/adapter_service.rs` (new), `server/src/main.rs` (wire it)
**Story**: `story-v0-pi-adapter-core-surface`

Defines the gRPC service the Pi adapter calls. The adapter is a client; the core is the server. Three RPCs:

```proto
service AdapterControlService {
  // Adapter attaches with its capability manifest + attachment evidence.
  // Core records the adapter id, capability, attach LSN, adapter generation.
  rpc Attach(AttachRequest) returns (AttachResult);

  // Adapter ingests an Observation (session report, event, reply) back to the core.
  // Maps to session::ingest_session_report + observation ingestion.
  rpc IngestObservation(ObservationRequest) returns (ObservationResult);

  // Adapter holds this open to RECEIVE delivered Operations from the core.
  // Server-streaming: core pushes accepted Operations targeting this adapter's sessions.
  // The adapter acknowledges delivery by ingesting the resulting Observation.
  rpc ReceiveDeliveries(ReceiveRequest) returns (stream Delivery);
}

message AttachRequest {
  patchbay.AdapterRegistration registration = 1;
  // attachment evidence (adapter-specific; Pi uses configured local material)
  bytes attachment_evidence = 2;
}
message AttachResult {
  bool accepted = 1;
  patchbay.EventId attach_event_id = 2;  // the attach LSN
  string failure_code = 3;  // empty on success
}

message ObservationRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  oneof observation {
    patchbay.SessionReport session_report = 2;  // adapter-reported session state
    // future: event/reply observations
  }
}
// ObservationResult confirms the LSN assigned.

message ReceiveRequest {
  patchbay.AdapterId adapter_id = 1;
  patchbay.Lsn cursor = 2;  // resume from last-acknowledged delivery LSN
}
message Delivery {
  patchbay.Operation operation = 1;  // the accepted Operation to deliver
  patchbay.EventId delivery_event_id = 2;
}
```

The core-side impl wires `Attach` → adapter registration (new core port), `IngestObservation` → `session::ingest_session_report`, `ReceiveDeliveries` → a delivery stream sourced from accepted Operations targeting this adapter's sessions. **This unit may reveal that the core needs a small adapter-registration port** (currently `session/ingest.rs` handles reports but there's no `attach`/capability-manifest ingestion). If so, that's a bounded core addition, not a design change — surface it as an implementation note.

**Acceptance Criteria**:
- [ ] `AdapterControlService` proto defined; bindings generated (Rust + TS).
- [ ] Core impl: `Attach` records the adapter + capability; `IngestObservation` calls `ingest_session_report`; `ReceiveDeliveries` streams accepted Operations for the adapter's sessions.
- [ ] A test adapter can attach, ingest a session report, and receive a delivered Operation.

---

### Unit 2: Pi `AgentSession` driver (the harvested in-process session layer)
**File**: `pi-adapter/src/pi_session.ts`, `pi-adapter/src/transcript_projection.ts`, `pi-adapter/src/transcript_event_log.ts`, `pi-adapter/src/turn_state.ts`
**Story**: `story-v0-pi-adapter-pi-rpc-client`

The Node-side Pi driver. Harvests outpost-pi's in-process session layer — `sdk_session_projection.ts` (the `AgentSession` driving surface: `sendMessage`/`sendUserMessage` + event capture), `transcript_projection.ts` (Pi event → typed `TranscriptEvent`), `transcript_event_log.ts` (append-only, dedup-by-eventId, `forSession` replay), and `turn_state.ts` (turn projection) — re-housed behind Patchbay's adapter port. The adapter calls `createAgentSession()` directly (no `pi --mode rpc` subprocess) and sets `beforeToolCall` for the tool-call approval gate.

```typescript
// pi-adapter/src/pi_session.ts
import { createAgentSession, type AgentSession, type AgentSessionEvent } from "@earendil-works/pi-coding-agent";

export class PiSession {
  private readonly session: AgentSession;
  private readonly transcriptLog: TranscriptEventLog;

  static async create(opts: { cwd: string; name?: string; model?: string }): Promise<PiSession> {
    const session = await createAgentSession({ /* cwd, model, etc. */ });
    return new PiSession(session);
  }

  private constructor(session: AgentSession) {
    this.session = session;
    // The tool-call approval gate: a direct typed hook (NOT a stdio sub-protocol).
    // Returns undefined to auto-proceed, or { block: true } to deny, or routes to
    // an Elicitation the operator answers (approval-response OperationKind).
    this.session.beforeToolCall = async ({ toolCall, args }) => {
      return this.approvalGate(toolCall, args);
    };
    // The typed event stream (no JSONL parsing).
    this.session.on((event: AgentSessionEvent) => this.handleEvent(event));
  }

  // Methods map to the AgentSession API (the §4 wire actions):
  prompt(text: string): Promise<void> { return this.session.sendUserMessage(text); }  // instruct
  cancel(): void { this.session.abort(); }                                              // cancel/interrupt
  getState(): SessionState { return this.session.getState(); }                        // query
  getEntries(since?: string): Entries { return this.session.getEntries(since); }      // snapshot cursor
  setModel(provider: string, modelId: string): Promise<void> { /* ... */ }             // reconfigure
  setThinkingLevel(level: ThinkingLevel): Promise<void> { /* ... */ }                  // reconfigure
  getAvailableModels(): Model[] { /* ... */ }                                          // query
  newSession(): Promise<void> { /* session_new → report generation bump */ }           // session-management
  compact(instructions?: string): Promise<void> { /* ... */ }                          // session-management (no gen bump)

  // The transcript projection (harvested): maps AgentSessionEvents to typed
  // TranscriptEvents with deterministic ids (dedup key) and folds into the log.
  private handleEvent(ev: AgentSessionEvent): void {
    const te = projectAgentEvent(ev, this.runtimeSessionId);
    if (te) this.transcriptLog.append(te);
  }
}
```

```typescript
// pi-adapter/src/transcript_projection.ts (harvested from outpost-pi)
// Maps AgentSession events (message deltas, tool_execution_*, turn_*, compaction_*)
// to typed TranscriptEvents with deterministic event ids (dedup key).
export function projectAgentEvent(ev: AgentSessionEvent, sessionId: string): TranscriptEvent | null;
```

**Implementation Notes**:
- **Approval gate**: `beforeToolCall` is the direct hook. For v0.1.0, an unapproved tool call opens an Elicitation (the `approval-response` OperationKind) the operator answers via the core; the hook resolves when the Elicitation is answered. This is cleaner than any stdio/sub-protocol approach — it's a typed async hook.
- **`session_new` → generation bump**: when the adapter creates a new `AgentSession` (or Pi rotates internally), it ingests a `SessionReport` with the bumped `session_generation`; the core tombstones the prior generation.
- **Events are typed `AgentSessionEvent`s** (not parsed JSONL) — the harvest's `transcript_projection` adapts them to `TranscriptEvent`s.
- **Harvest fidelity**: the outpost-pi session layer was built as a Pi *extension* (`ExtensionFactory`/`ExtensionAPI`); re-housing it as a direct `createAgentSession()` caller is the "real adapter-implementation work, not a copy" the harvest idea cautions about. The projection logic harvests; the extension-shape wiring does not.

**Acceptance Criteria**:
- [ ] `PiSession` creates an `AgentSession` in-process, sends a prompt via `sendUserMessage`, receives the typed event stream.
- [ ] `beforeToolCall` is wired and can block/approve a tool call.
- [ ] All §4 actions are implemented (prompt, cancel, getState, getEntries, setModel, setThinkingLevel, getAvailableModels, newSession, compact).
- [ ] `transcript_projection` maps `AgentSessionEvent`s to typed `TranscriptEvent`s with stable dedup ids.
- [ ] `TranscriptEventLog` dedups by eventId and replays `forSession`.
- [ ] A smoke test drives a real `AgentSession` end-to-end (prompt → events → transcript → approval gate fires).

---

### Unit 3: Adapter core-client + session registry + delivery translation
**File**: `pi-adapter/src/core_client.ts`, `pi-adapter/src/session_registry.ts`, `pi-adapter/src/delivery.ts`, `pi-adapter/src/main.ts`
**Story**: `story-v0-pi-adapter-translation` (depends on Units 1 + 2)

The adapter process: a gRPC client of the core (`AdapterControlService`) + a session registry (`Map<runtime_session_id, PiSession>`) + the bidirectional translation. On startup: attaches to the core with the Pi capability manifest (declares `spawn` unsupported, `snapshot=partial`), creates in-process `PiSession`s for pre-provisioned targets (the fast-follower `spawn` will create new `AgentSession`s dynamically), and holds `ReceiveDeliveries` open. For each delivered `Operation`: resolve the target `runtime_session_id` → `PiSession`, translate the `OperationKind` to the `AgentSession` API call, and stream resulting `AgentSession` events back as `Observation`s via `IngestObservation`.

```typescript
// pi-adapter/src/delivery.ts — OperationKind → PiSession API call
class DeliveryTranslator {
  deliver(op: Operation, session: PiSession): Promise<void> {
    switch (op.kind) {
      case OperationKind.INSTRUCT:    return session.prompt(op.payload.text);
      case OperationKind.CANCEL:      return session.cancel();
      case OperationKind.QUERY:       return session.getState(); // or getEntries/getAvailableModels per payload
      case OperationKind.RECONFIGURE: return this.reconfigure(op, session); // setModel / setThinkingLevel
      case OperationKind.SESSION_MANAGEMENT: return this.sessionMgmt(op, session); // newSession / compact
      case OperationKind.APPROVAL_RESPONSE: return session.resolveApproval(/* from payload */); // answers a beforeToolCall Elicitation
      case OperationKind.SPAWN:       throw unsupported_command; // v0.1.0; fast-follower
      // INTERRUPT aliased to CANCEL or unsupported at delivery
    }
  }
}
```

**Implementation Notes**:
- **Session registry is the fast-follower seam**: v0.1.0 populates it from pre-provisioned config (creating `AgentSession`s at startup). The fast-follower `spawn` creates a new `AgentSession` in-process and reports the new `runtime_session_id` — additive, no architectural change.
- **`session_new` → generation bump**: when `newSession()` completes, the adapter ingests a `SessionReport` with the bumped `session_generation`; the core tombstones the prior generation (the core's existing `ingest_session_report` handles this).
- **Approval flow**: `beforeToolCall` opens an Elicitation the operator answers via an `APPROVAL_RESPONSE` Operation; the hook resolves when the Elicitation is answered. This is the `approval-response` OperationKind in flight — the v0.1.0 minimal slice may stub this (auto-proceed) and add the full Elicitation loop as a follow-on.
- **Observation streaming**: `AgentSession` events → `transcript_projection` → `TranscriptEventLog` (local partial snapshot) + `IngestObservation` to the core (durable). The local log is the partial-snapshot source for reconnect.

**Acceptance Criteria**:
- [ ] Adapter attaches to the core with the Pi capability manifest; `spawn` declared unsupported.
- [ ] Session registry resolves `runtime_session_id → PiSession`.
- [ ] A delivered `INSTRUCT` reaches the right `PiSession`; the resulting `AgentSession` events stream back to the core as `Observation`s.
- [ ] `session_new` bumps the generation in the core (tombstones prior).
- [ ] A delivered `SPAWN` returns `unsupported_command` (v0.1.0).

---

### Unit 4: End-to-end integration test
**File**: `pi-adapter/tests/e2e.test.ts`
**Story**: `story-v0-pi-adapter-translation` (same story)

Spins up the core server + a real in-process `AgentSession` (via the adapter) + the adapter, and proves the loop: operator `Submit(INSTRUCT)` → core accepts → adapter receives delivery → Pi runs → events stream back → operator sees them via `Subscribe`. This is the walking-skeleton proof for the agent-control path.

**Acceptance Criteria**:
- [ ] End-to-end: prompt submitted via the core reaches Pi; Pi's response events reach a `Subscribe` client.
- [ ] `cancel` aborts a running Pi turn.
- [ ] Reconnect: adapter restarts, re-attaches, and the core reconciles against the partial transcript snapshot.

## Implementation Order

1. `story-v0-pi-adapter-core-surface` (Unit 1) — the adapter-facing proto + core impl. Unblocks the adapter process (which needs the surface to talk to). May touch `core/` for a small adapter-registration port — surface as an implementation note if so.
2. `story-v0-pi-adapter-pi-rpc-client` (Unit 2) — the harvested in-process `AgentSession` driver. No deps (can build against a real `AgentSession` independently).
3. `story-v0-pi-adapter-translation` (Units 3, 4) — the adapter process + e2e. Depends on Units 1 + 2.

Units 1 and 2 can run in parallel (independent write sets: `server/`+`contracts/` vs `pi-adapter/`).

## Simplification

- The adapter owns NO domain logic — no authority, no durable state beyond the local transcript cache. Pure translation + the session registry.
- v0.1.0 declares `spawn` unsupported; the fast-follower is additive (session-registry extension), not architectural.
- No `pi --mode rpc` subprocess and no in-process extension loaded into a Pi child — the adapter IS the Pi host (calls `createAgentSession()` directly). The approval gate is a direct `beforeToolCall` hook, not a stdio sub-protocol or side channel.
- Minimal slice first (attach + instruct + cancel + stream); the remaining §4 kinds are additive delivery mappings in follow-on stories.
- The local `TranscriptEventLog` is a partial-snapshot cache only; the core's durable log remains authoritative.

## Testing

- **Pi `AgentSession` driver smoke (Unit 2)**: drives a real in-process `AgentSession` — protects the driving surface + approval-gate + command-surface contract.
- **Translation unit tests (Unit 3)**: OperationKind → Pi command mapping; Pi event → Observation mapping.
- **E2E (Unit 4)**: the walking-skeleton loop — core → adapter → Pi → events back. Load-bearing.
- **Reconnect test (Unit 4)**: adapter restart → re-attach → partial-snapshot reconcile.

## Risks

- **`AgentSession` lifecycle coupling (revised risk)**: the adapter owns the Pi session lifecycle (it's a Pi host, not a clean subprocess driver). Restarting the adapter tears down its Pi sessions. Bounded for v0.1.0 single-operator; the local `TranscriptEventLog` survives adapter restart only if persisted (v0.1.0 in-memory — restart loses it, and the core reconciles via `partial`-tier degraded behavior, marking axes `stale`/`unknown`). Promoting to a separate Pi process later is internal to the adapter.
- **Adapter-facing core surface doesn't exist**: Unit 1 may reveal the core needs a small adapter-registration port (attach + capability-manifest ingestion) that isn't there yet. Bounded core addition; surface as an implementation note, not a design bounce. Fallback: scope Unit 1 as a prerequisite story if it grows.
- **`ReceiveDeliveries` stream lifecycle**: the adapter holds a long-lived stream open; core-side, this means keeping a per-adapter delivery queue. If this proves complex, a polling fallback (`FetchDeliveries(cursor)`) is acceptable for v0.1.0 single-adapter — but the stream is preferred (lower latency).
- **Approval-Elicitation loop**: `beforeToolCall` opening an Elicitation the operator answers is the `approval-response` flow. If this round-trip proves complex for the minimal slice, v0.1.0 may stub approvals (auto-proceed with audit) and add the full Elicitation loop as a follow-on story — but this is an explicit v0.1.0 scope cut, documented, not a silent gap.
- **`spawn` fast-follower scope**: v0.1.0 must not bake in a single Pi session. The session-registry design (Unit 3) is what keeps `spawn` additive — verify the registry abstraction holds before the fast-follower.

## Parked: naming cleanup

Patchbay references the pre-fork name "remote-pi" / "Remote Pi" throughout (docs/ADAPTER-PI.md, ARCHITECTURE.md, SPEC.md, PROTOCOL.md, UX.md, VISION.md, ~6 work items, session notes, the harvest idea file `idea-harvest-remote-pi-extension-as-adapter.md`). The project is now **outpost_pi** at `/home/agent/projects/outpost_pi/`. This is a docs-naming cleanup, not a design blocker — file as a separate `[prose]` cleanup item (rename references + the harvest idea file) rather than derail this feature. The `research_origin`/attestation handles (`pi-extension`) remain accurate.
