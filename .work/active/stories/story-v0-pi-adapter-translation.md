---
id: story-v0-pi-adapter-translation
kind: story
stage: implementing
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: [story-v0-pi-adapter-core-surface, story-v0-pi-adapter-pi-rpc-client]
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Story: pi-adapter translation + session registry + e2e

The adapter process itself: a gRPC client of the core (`AdapterControlService`) + a session registry (`Map<runtime_session_id, PiRpcChild>`) + the bidirectional translation (core `Operation`s → Pi RPC commands; Pi events → core `Observation`s). Plus the end-to-end integration test proving the walking-skeleton loop.

## Design (from feature-v0-pi-adapter Units 3, 4)

**Files**: `pi-adapter/src/core_client.ts`, `pi-adapter/src/session_registry.ts`, `pi-adapter/src/delivery.ts`, `pi-adapter/src/main.ts`, `pi-adapter/tests/e2e.test.ts`

### Adapter process (`main.ts`)

On startup:
1. Attaches to the core via `AdapterControlService.Attach` with the Pi capability manifest (declares `spawn` unsupported, `snapshot=partial`, `cancellation=true`, `session_replacement=true`, `streaming=true`).
2. Discovers pre-provisioned Pi children from a config/registry (the fast-follower `spawn` will populate this dynamically; v0.1.0 reads a static config).
3. Holds `ReceiveDeliveries` open to receive delivered Operations.

### Session registry (`session_registry.ts`)

```typescript
// Maps runtime_session_id → PiRpcChild. v0.1.0 populates from pre-provisioned
// config; the fast-follower spawn adds entries by calling the supervisord's
// addDaemon + _spawnEntry machinery and reporting the new runtime_session_id.
class SessionRegistry {
  resolve(runtimeSessionId: RuntimeSessionId): PiRpcChild | null;
  register(runtimeSessionId: RuntimeSessionId, child: PiRpcChild): void;
  // fast-follower: spawnNew(cwd, name): RuntimeSessionId  (NOT v0.1.0)
}
```

**This registry is the fast-follower seam**: v0.1.0 populates it statically; the `spawn` fast-follower extends it dynamically. The abstraction must hold so `spawn` is additive.

### Delivery translation (`delivery.ts`)

```typescript
class DeliveryTranslator {
  deliver(op: Operation, child: PiRpcChild): Promise<void> {
    switch (op.kind) {
      case OperationKind.INSTRUCT:    return child.prompt(op.payload.text);
      case OperationKind.CANCEL:      return child.abort();
      case OperationKind.QUERY:       return child.getState(); // or getEntries/getAvailableModels per payload
      case OperationKind.RECONFIGURE: return this.reconfigure(op, child); // set_model / set_thinking_level
      case OperationKind.SESSION_MANAGEMENT: return this.sessionMgmt(op, child); // new_session / compact
      case OperationKind.APPROVAL_RESPONSE: return child.respondToUiRequest(/* from payload */);
      case OperationKind.SPAWN:       throw unsupported_command; // v0.1.0; fast-follower
      // INTERRUPT aliased to CANCEL or unsupported at delivery
    }
  }
}
```

### Observation streaming

Pi events → `transcript_projection` → `TranscriptEventLog` (local partial snapshot) + `IngestObservation` to the core (durable). The local log is the partial-snapshot source for reconnect. `session_new` completion → ingest a `SessionReport` with the bumped `session_generation` (the core's existing `ingest_session_report` tombstones the prior generation).

## Acceptance criteria

- [ ] Adapter attaches to the core with the Pi capability manifest; `spawn` declared unsupported.
- [ ] Session registry resolves `runtime_session_id → PiRpcChild`.
- [ ] A delivered `INSTRUCT` reaches the right Pi child; the resulting Pi events stream back to the core as `Observation`s.
- [ ] `session_new` bumps the generation in the core (tombstones prior).
- [ ] A delivered `SPAWN` returns `unsupported_command` (v0.1.0).
- [ ] **E2E (Unit 4)**: operator `Submit(INSTRUCT)` via the core → adapter receives delivery → Pi runs → events stream back → operator sees them via `Subscribe`.
- [ ] `cancel` aborts a running Pi turn.
- [ ] Reconnect: adapter restarts, re-attaches, core reconciles against the partial transcript snapshot.

## Notes

- This story carries Units 3 + 4 as one cohesive bundle (the adapter process + e2e are tightly coupled).
- Depends on BOTH Unit 1 (core surface) and Unit 2 (Pi RPC client) — they can run in parallel, this story waits for both.
- The minimal v0.1.0 slice is `attach` + `instruct` + `cancel` + observation streaming. The remaining §4 kinds (`approval-response`, `query`, `reconfigure`, `session-management`) are additive delivery mappings — if time-boxed, land the minimal slice first and file the rest as follow-on stories.
- **`spawn` fast-follower**: the session-registry abstraction is what keeps `spawn` additive. Verify the registry holds before the fast-follower is designed.
