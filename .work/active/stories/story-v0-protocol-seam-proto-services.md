---
id: story-v0-protocol-seam-proto-services
kind: story
stage: implementing
tags: [protocol, contract]
parent: feature-v0-protocol-seam
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Story: protocol-seam proto service definitions

Defines the first gRPC `service` blocks in the repo: `ControlService` with `Submit`, `Subscribe` (server-streaming), and `LoadSnapshot`. These are the contract the Rust server impl and the TS web-server client both generate from.

## Design (from feature-v0-protocol-seam Unit 1)

**File**: `contracts/proto/patchbay/control.proto` (new)

```proto
syntax = "proto3";
package patchbay;

import "patchbay/common.proto";
import "patchbay/operations.proto";

// The internal control-plane service the TS web server speaks to the Rust
// core over gRPC/HTTP2. Browser-facing surfaces are a subset reachable
// through the web server; control-surface methods are principal-gated.
service ControlService {
  rpc Submit(SubmitRequest) returns (SubmissionResult);
  rpc Subscribe(SubscribeRequest) returns (stream SubscribeEvent);
  rpc LoadSnapshot(LoadSnapshotRequest) returns (LoadSnapshotResponse);
}

message SubmitRequest {
  patchbay.Operation operation = 1;
}
// SubmissionResult is reused from operations.proto (already generated).

message SubscribeRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  patchbay.Lsn cursor = 2;
}

message SubscribeEvent {
  patchbay.EventId event_id = 1;
  patchbay.StoredEventPayload payload = 2;
}

message LoadSnapshotRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  optional patchbay.Lsn at_or_before = 2;
}

message LoadSnapshotResponse {
  bool present = 1;
  patchbay.EventId event_id = 2;
  bytes snapshot_payload = 3;
}
```

## Acceptance criteria

- [ ] `control.proto` defines `ControlService` with `Submit`, `Subscribe` (server-streaming), `SubscribeEvent` exactly as above.
- [ ] `buf generate` regenerates Rust + TS bindings; `cargo build -p patchbay-contracts` and `npm run build` in `contracts/ts` both succeed.
- [ ] Gen diff is additions-only (no drift in existing generated files).

## Notes

- Adapter attach/detach + audit-query RPCs are intentionally NOT in this story — deferred until the adapter-registration and audit-projection core surfaces they depend on are exposed. Adding proto methods is non-breaking for clients.
- This story unblocks the server impl story (which needs the generated types).
