---
id: epic-agent-operations-resource-plane-resource-state-snapshot-load
kind: story
stage: done
tags: [protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Materialize and load resource snapshots

## Checkpoint

Materialize stable-ordered `ResourceSnapshot` values from the server-owned
resource projection at the same locked applied LSN, route `LoadSnapshot` by its
explicit view kind, and update existing session callers to request the session
view. Resource reads are on-demand projections over the durable log; they must
not decode or overwrite the existing undiscriminated session checkpoint slot.

## Acceptance evidence

- Resource snapshot domain and LSN match the response `EventId`; resource and
  view revisions, source generation, freshness, completeness, tombstones,
  replacements, observed time, and materialized time survive encoding.
- `SESSION` still returns a `SessionSnapshot`; `RESOURCE` returns only a
  `ResourceSnapshot`; unspecified/unknown kinds reject and a stored session
  checkpoint can never answer a resource request.
- A checkpoint older than the current projection is not returned as authority;
  the current materialized view repairs it, including for historical
  `at_or_before` requests the current implementation cannot reconstruct.
- Existing CLI/web/session E2E callers send `SESSION`; resource endpoint tests
  request `RESOURCE`; subscription filtering admits the resource-state event.

## Ordering constraints

Consumes authenticated resource-state folding. This checkpoint does not add
cockpit resource rendering or a second durable snapshot table.

## Implementation notes

Added consistent-prefix `ResourceSnapshot` materialization under the existing
cursor-before-target lock order. Records and per-view revisions are
stable-sorted by their exact identity keys and preserve payload envelopes,
freshness, source generation, revision, observed time, tombstone LSN, and
replacement identity. Restart evidence compares the full resource/view payload
and proves stable ordering.

`LoadSnapshot` now fail-fast parses the required generated `SnapshotViewKind`,
re-verifies the compound issuer under the decision gate, and echoes the selected
view. `RESOURCE` always serves the current durable resource projection and never
reads the session checkpoint slot. `SESSION` accepts a checkpoint only when it
decodes as `SessionSnapshot`, matches domain/response LSN, and is not behind the
current projection; corrupt/raw/older checkpoints repair to the current
materialized session view. Historical bounds that cannot be reconstructed also
repair to current authority. Operator subscriptions now deliver
`RESOURCE_STATE` events.

CLI, web reconciliation, web-server integration, and Pi E2E session consumers
now request `SESSION` explicitly; decoding consumers verify the echoed session
view. The TypeScript contract barrel exports the generated resource schema.
Tests cover unspecified/unknown view rejection, cross-view decode separation,
resource snapshot domain/LSN equality, corrupt session checkpoint repair,
stable resource ordering/restart, and existing session callers.

Checkpoint verification: targeted Rust snapshot tests and warnings-denied
workspace clippy passed. CLI (37), web cockpit (75), web server (31), and Pi
adapter including real-process E2E (24) tests all passed.
