---
id: epic-agent-operations-resource-plane-resource-state-contract
kind: story
stage: done
tags: [protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Define the resource-state and snapshot contracts

## Checkpoint

Add the generated resource record with schema-bound resource/projection envelopes,
freshness, per-view completeness/revision, report, normalized durable-delta,
and `ResourceSnapshot` messages. Add the
`StoredEventKind::RESOURCE_STATE` discriminator and make `LoadSnapshot`
explicitly select and echo `SESSION` or `RESOURCE` view kind. Generate both Rust
and TypeScript artifacts; generated output is never hand-edited.

## Acceptance evidence

- Resource state uses the existing typed `ResourceIdentity` and canonical
  `AdapterSnapshotSupport` tier; it does not invent a session generation or
  adapter-specific health enum.
- Report shape distinguishes reconnect snapshots from live deltas and can carry
  atomic upsert/unknown/tombstone-with-replacement mutations across views;
  upserts carry both manifest-bound resource and projection envelopes.
- Unknown/unspecified snapshot views, completeness values, freshness values,
  mutation variants, and payload content types have fail-closed boundary tests.
- Rust/TypeScript contract builds and generated-drift checks pass.

## Ordering constraints

This schema checkpoint must land before projection, replay, ingress, or snapshot
materialization code consumes the new generated types.

## Implementation notes

Added `resources.proto` as the wire source for revisioned resource records,
per-kind view revisions, snapshot/delta reports, normalized durable mutations,
terminal replacement tombstones, and reconciliation freshness. Added
`STORED_EVENT_KIND_RESOURCE_STATE`, typed `ResourceReport` adapter ingress, and
the required/echoed `SnapshotViewKind` discriminator. Rust and TypeScript
bindings were regenerated from the protos; no generated artifact was edited by
hand.

Checkpoint verification: `cargo test -p patchbay-contracts` and
`npm --prefix contracts/ts run build` passed. `buf generate` completed. The
repository-wide `buf lint` continues to report the pre-existing RPC
request/response naming debt documented by the prior capability-manifest
feature; no new resource proto lint finding was introduced.
