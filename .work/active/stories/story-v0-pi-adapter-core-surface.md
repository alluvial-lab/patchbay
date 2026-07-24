---
id: story-v0-pi-adapter-core-surface
kind: story
stage: done
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
research_origin: null
---

# Story: pi-adapter core-facing RPC surface

The adapter-facing gRPC service on the core (does not exist yet — the seam shipped only web-server-facing `ControlService`). The Pi adapter is a client of this surface. Three RPCs: `Attach` (capability manifest + attachment evidence), `IngestObservation` (session reports/events back to core), `ReceiveDeliveries` (server-streaming push of accepted Operations to the adapter).

## Design (from feature-v0-pi-adapter Unit 1)

**Files**: `contracts/proto/patchbay/adapter_control.proto` (new), `server/src/adapter_service.rs` (new), `server/src/main.rs` (wire it), `core/` (possibly a small adapter-registration port — see implementation notes)

```proto
service AdapterControlService {
  rpc Attach(AttachRequest) returns (AttachResult);
  rpc IngestObservation(ObservationRequest) returns (ObservationResult);
  rpc ReceiveDeliveries(ReceiveRequest) returns (stream Delivery);
}

message AttachRequest {
  patchbay.AdapterRegistration registration = 1;
  bytes attachment_evidence = 2;  // adapter-specific (Pi: configured local material)
}
message AttachResult {
  bool accepted = 1;
  patchbay.EventId attach_event_id = 2;
  string failure_code = 3;
}

message ObservationRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  oneof observation {
    patchbay.SessionReport session_report = 2;
  }
}
// ObservationResult confirms the LSN assigned.

message ReceiveRequest {
  patchbay.AdapterId adapter_id = 1;
  patchbay.Lsn cursor = 2;
}
message Delivery {
  patchbay.Operation operation = 1;
  patchbay.EventId delivery_event_id = 2;
}
```

Core-side impl:
- `Attach` → records the adapter id, capability manifest, attach LSN, adapter generation. **May require a small new core port** for adapter registration (currently `session/ingest.rs` handles session reports but there's no attach/capability ingestion). If so, that's a bounded core addition — surface as an implementation note, not a design bounce.
- `IngestObservation` → calls `session::ingest_session_report` (existing core function).
- `ReceiveDeliveries` → streams accepted Operations targeting this adapter's sessions. The core holds a per-adapter delivery queue; the adapter holds the stream open and acknowledges by ingesting the resulting Observation.

## Acceptance criteria

- [x] `AdapterControlService` proto defined; Rust + TS bindings generated (`buf generate` + `cargo build -p patchbay-contracts` + `npm run build` in `contracts/ts`).
- [x] Core impl: `Attach` records the adapter + capability; `IngestObservation` calls `ingest_session_report`; `ReceiveDeliveries` streams accepted Operations for the adapter's sessions.
- [x] A test adapter (Rust or TS) can attach, ingest a session report, and receive a delivered Operation.
- [x] `cargo test -p patchbay-core-server` and `cargo test -p patchbay-core` stay green.

## Notes

- This is a NEW service, not a modification of `ControlService` (different principal, different auth posture — the adapter authenticates via attachment evidence, not the web-server shared secret).
- **If the core needs an adapter-registration port**, that's a bounded `core/` addition. The feature brief says "do NOT edit core/" is NOT a constraint here (that was the web-server feature); this feature may touch `core/` for the registration port. Surface it as an implementation note if it grows beyond a small port.
- Unit 2 (Pi RPC client) is independent and can run in parallel — different write set (`pi-adapter/` vs `server/`+`contracts/`).

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; selected by the caller for the protocol/security-sensitive cross-language surface.
- Review weight: `standard` (caller).
- Files changed: `contracts/proto/patchbay/adapter_control.proto`, generated Rust/TypeScript bindings and generation inputs, `core/src/adapter/mod.rs`, `core/src/lib.rs`, `server/src/adapter_service.rs`, its focused test module, and server composition/build files.
- Tests added/removed: added a focused adapter-service test proving attach → session report ingest → targeted delivery; no tests removed.
- Simplification: adapter registration reuses a redacted, schema-tagged durable Observation rather than adding a competing event writer or a new storage event family.
- Discrepancies from design: added the bounded core adapter-registration port anticipated by the design; attachment evidence is verified on attach and on subsequent adapter RPC metadata and never persisted. `ReceiveDeliveries` uses the explicitly allowed v0.1.0 cursor-polling fallback: each server stream returns the currently durable tail and the client resumes immediately, avoiding a per-adapter in-memory queue. The observation oneof also accepts a generated `Observation` so Unit 3 can stream Pi events without a second RPC.
- Verification: `cargo build -p patchbay-core-server`, `cargo test -p patchbay-core-server`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo test -p patchbay-core`, and `contracts/ts npm run build` passed.
- Adjacent issues parked: none.

### Review-response implementation update

- Added the explicit delivery-acknowledgement path through the existing Observation RPC. The bounded core adapter port commits `accepted → delivered` first, then the audit Observation; `ReceiveDeliveries` rebuilds durable command state and offers only still-`accepted` Operations, making resume core-owned and restart-safe without a proto change.
- Added registration generation preflight before append so a rejected stale attach cannot poison replay.
- Regression coverage: real-process lifecycle/resume assertions in `pi-adapter/tests/e2e.test.ts`; core unit regression for generation-2 attach → generation-1 rejection → successful durable rebuild; adapter-service delivery fixture updated for the command projection filter.
