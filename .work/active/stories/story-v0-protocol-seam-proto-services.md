---
id: story-v0-protocol-seam-proto-services
kind: story
stage: done
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

- [x] `control.proto` defines `ControlService` with `Submit`, `Subscribe` (server-streaming), `SubscribeEvent` exactly as above.
- [x] `buf generate` regenerates Rust + TS bindings; `cargo build -p patchbay-contracts` and `npm run build` in `contracts/ts` both succeed.
- [x] Gen diff is additions-only (no drift in existing generated files).

## Notes

- Adapter attach/detach + audit-query RPCs are intentionally NOT in this story — deferred until the adapter-registration and audit-projection core surfaces they depend on are exposed. Adding proto methods is non-breaking for clients.
- This story unblocks the server impl story (which needs the generated types).

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at `high` effort; direct-read inline implementation because the contract change and generation wiring were bounded and the caller prohibited delegation.
- Review weight: `standard` (caller/default); not applicable to this child-story checkpoint, which advances directly to done on green verification.
- Files changed: `contracts/proto/patchbay/control.proto`, `contracts/rust/build.rs`, `contracts/rust/src/gen/patchbay/patchbay.rs`, `contracts/ts/src/gen/patchbay/control_pb.ts`.
- Tests added/removed: none; generated-contract compilation is the stable interface check.
- Simplification: reused all existing common/operation message types and introduced no duplicate protocol DTOs.
- Discrepancies from design: Rust generation required adding `control.proto` to the existing explicit proto list in `contracts/rust/build.rs`; the design named only the new proto file, and the caller's write-scope list omitted this necessary generation-wiring edit.
- Verification: `cargo build -p patchbay-contracts` and `npm run build` in `contracts/ts` pass; generated diffs are additions-only. The repository's Buf STANDARD naming lint rejects the operator-confirmed response names (`SubmissionResult` and `SubscribeEvent`), so the exact contract cannot also satisfy that optional lint without changing the settled wire vocabulary.
- Adjacent issues parked: none.
