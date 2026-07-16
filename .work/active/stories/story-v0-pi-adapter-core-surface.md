---
id: story-v0-pi-adapter-core-surface
kind: story
stage: implementing
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
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

- [ ] `AdapterControlService` proto defined; Rust + TS bindings generated (`buf generate` + `cargo build -p patchbay-contracts` + `npm run build` in `contracts/ts`).
- [ ] Core impl: `Attach` records the adapter + capability; `IngestObservation` calls `ingest_session_report`; `ReceiveDeliveries` streams accepted Operations for the adapter's sessions.
- [ ] A test adapter (Rust or TS) can attach, ingest a session report, and receive a delivered Operation.
- [ ] `cargo test -p patchbay-core-server` and `cargo test -p patchbay-core` stay green.

## Notes

- This is a NEW service, not a modification of `ControlService` (different principal, different auth posture — the adapter authenticates via attachment evidence, not the web-server shared secret).
- **If the core needs an adapter-registration port**, that's a bounded `core/` addition. The feature brief says "do NOT edit core/" is NOT a constraint here (that was the web-server feature); this feature may touch `core/` for the registration port. Surface it as an implementation note if it grows beyond a small port.
- Unit 2 (Pi RPC client) is independent and can run in parallel — different write set (`pi-adapter/` vs `server/`+`contracts/`).
