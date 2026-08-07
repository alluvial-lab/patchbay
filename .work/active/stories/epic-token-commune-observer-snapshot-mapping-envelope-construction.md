---
id: epic-token-commune-observer-snapshot-mapping-envelope-construction
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-snapshot-mapping
depends_on: [epic-token-commune-observer-snapshot-mapping-projection-contract]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Construct manifest-bound JSON and ResourceReport envelopes

## Checkpoint

Add the token-commune-local envelope builder and generated-Protobuf report factory. Construct JSON bytes only after validating them against the matching Draft 2020-12 schema, select schema refs from `TOKEN_COMMUNE_RESOURCES`, and build canonical generated `PayloadEnvelope`, `ResourceIdentity`, `ResourceReportMutation`, `ResourceViewReport`, and snapshot `ResourceReport` messages. The factory is deterministic and takes observation time explicitly; it never reads a clock or performs I/O.

## Files

- `token-commune-adapter/src/resource_envelope.ts`
- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/package.json`
- `token-commune-adapter/package-lock.json`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

## Acceptance evidence

- Both payload and projection envelopes carry JSON content type and the exact manifest-selected schema ref.
- A schema-invalid constructed value throws before any `ResourceReport` is returned; no valid sibling mutation leaks from a failed projection call.
- Every snapshot report carries the configured adapter id/generation, explicit observed timestamp, exactly two registry-ordered PARTIAL views, and only generated contract messages.
- Identity adapter/kind mismatches fail before report construction.
- Tests use independent literal descriptor expectations and fail when payload/projection descriptors are swapped or hard-coded incorrectly.

## Ordering

Depends on the projection contract checkpoint. Provider and member mapping build only through this envelope/report boundary.
