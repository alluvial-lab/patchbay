---
id: story-v0-pi-adapter-translation
kind: story
stage: done
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: [story-v0-pi-adapter-core-surface, story-v0-pi-adapter-pi-rpc-client]
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
research_origin: null
---

# Story: pi-adapter translation + session registry + e2e

The adapter process itself: a gRPC client of the core (`AdapterControlService`) + a session registry (`Map<runtime_session_id, PiSession>`) + the bidirectional translation (core `Operation`s → `PiSession` API calls; `AgentSession` events → core `Observation`s). Plus the end-to-end integration test proving the walking-skeleton loop.

## Design (from feature-v0-pi-adapter Units 3, 4)

**Files**: `pi-adapter/src/core_client.ts`, `pi-adapter/src/session_registry.ts`, `pi-adapter/src/delivery.ts`, `pi-adapter/src/main.ts`, `pi-adapter/tests/e2e.test.ts`

### Adapter process (`main.ts`)

On startup:
1. Attaches to the core via `AdapterControlService.Attach` with the Pi capability manifest (declares `spawn` unsupported, `snapshot=partial`, `cancellation=true`, `session_replacement=true`, `streaming=true`).
2. Creates in-process `PiSession`s (each wrapping an `AgentSession`) for pre-provisioned targets from a config/registry (the fast-follower `spawn` will create new `AgentSession`s dynamically).
3. Holds `ReceiveDeliveries` open to receive delivered Operations.

### Session registry (`session_registry.ts`)

```typescript
// Maps runtime_session_id → PiSession (in-process AgentSession wrapper).
// v0.1.0 populates from pre-provisioned config; the fast-follower spawn creates
// a new AgentSession in-process and reports the new runtime_session_id.
class SessionRegistry {
  resolve(runtimeSessionId: RuntimeSessionId): PiSession | null;
  register(runtimeSessionId: RuntimeSessionId, session: PiSession): void;
  // fast-follower: spawnNew(cwd, name): RuntimeSessionId  (NOT v0.1.0)
}
```

**This registry is the fast-follower seam**: v0.1.0 populates it statically; the `spawn` fast-follower extends it dynamically. The abstraction must hold so `spawn` is additive.

### Delivery translation (`delivery.ts`)

```typescript
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

### Observation streaming

Pi `AgentSession` events → `transcript_projection` → `TranscriptEventLog` (local partial snapshot) + `IngestObservation` to the core (durable). The local log is the partial-snapshot source for reconnect. `session_new` completion → ingest a `SessionReport` with the bumped `session_generation` (the core's existing `ingest_session_report` tombstones the prior generation).

## Acceptance criteria

- [x] Adapter attaches to the core with the Pi capability manifest; `spawn` declared unsupported.
- [x] Session registry resolves `runtime_session_id → PiSession`.
- [x] A delivered `INSTRUCT` reaches the right `PiSession`; the resulting `AgentSession` events stream back to the core as `Observation`s.
- [x] `session_new` bumps the generation in the core (tombstones prior).
- [x] A delivered `SPAWN` returns `unsupported_command` (v0.1.0).
- [x] **E2E (Unit 4)**: operator `Submit(INSTRUCT)` via the core → adapter receives delivery → Pi runs → events stream back → operator sees them via `Subscribe`.
- [x] `cancel` aborts a running Pi turn.
- [x] Reconnect: adapter restarts, re-attaches, core reconciles against the partial transcript snapshot.

## Notes

- This story carries Units 3 + 4 as one cohesive bundle (the adapter process + e2e are tightly coupled).
- Depends on BOTH Unit 1 (core surface) and Unit 2 (Pi RPC client) — they can run in parallel, this story waits for both.
- The minimal v0.1.0 slice is `attach` + `instruct` + `cancel` + observation streaming. The remaining §4 kinds (`approval-response`, `query`, `reconfigure`, `session-management`) are additive delivery mappings — if time-boxed, land the minimal slice first and file the rest as follow-on stories.
- **`spawn` fast-follower**: the session-registry abstraction is what keeps `spawn` additive. Verify the registry holds before the fast-follower is designed.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; selected by the caller for cross-process protocol integration and reconnect/cancellation risk.
- Review weight: `standard` (caller).
- Files changed: `pi-adapter/src/core_client.ts`, `session_registry.ts`, `delivery.ts`, `main.ts`, package dependencies/scripts, delivery/registry tests, `tests/e2e.test.ts`, plus an adapter-service idempotent no-change response correction discovered by reconnect testing.
- Tests added/removed: added translation and registry tests; added a real walking-skeleton e2e that starts the Rust core, attaches the adapter, submits through `ControlService`, drives a real faux-backed `AgentSession`, observes transcript output via `Subscribe`, checks `session_new` generation tombstoning, cancels an in-flight turn, verifies `spawn → unsupported_command`, and re-attaches a fresh adapter/session generation. No tests removed.
- Simplification: one generated-contract Connect client, one `Map<runtime_session_id, PiSession>`, and one `OperationKind` dispatcher own the integration. The client resumes the core's cursor-tail polling fallback rather than maintaining a second in-memory delivery queue.
- Discrepancies from design: the full approval-response Elicitation round-trip remains the explicit minimal-slice follow-on; the delivery registry rejects it visibly instead of leaving a silent gap. Query, reconfigure, and session-management mappings were small enough to land now. The current adapter RPC has no separate snapshot upload, so reconnect re-attaches with a higher adapter/session generation and reports activity `unknown` when its in-memory partial transcript cache was lost; the core tombstones the old generation instead of fabricating replay history. Identical session reports now succeed idempotently with no new event id.
- Verification: `pi-adapter npm run build && npm test` passed, including the real-process e2e; `cargo build -p patchbay-core-server`, `cargo test -p patchbay-core-server`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all --check`, and `cargo test -p patchbay-core` passed.
- Adjacent issues parked: none.

### Review-response implementation update

- The registry now owns complete runtime entries and transcript wiring; both startup provisioning and future dynamic spawn use `AdapterProcess.registerSession`, and delivery has no lookup against immutable `options.sessions`.
- Delivery now acknowledges durably before execution, validates generation/scope, emits running/result/failure lifecycle Observations, preserves query values, and advances its optimization cursor only after acknowledgement. Reconnect starts from zero safely because the core offers only durable `accepted` commands, then explicitly replays the partial transcript snapshot and reports unreconciled activity as `unknown`.
- The manifest no longer advertises deferred approval/elicitation response delivery.
- Regression coverage: dynamic registration delivery, exact lifecycle transitions, query result payload, stale-generation non-execution, restart with zero historical re-deliveries, partial snapshot replay, unknown reconnect activity, and manifest honesty are integrated into `pi-adapter/tests/e2e.test.ts`; registry ownership has focused unit coverage.
