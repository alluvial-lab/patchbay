---
id: epic-agent-operations-resource-plane-resource-state-snapshot-load
kind: story
stage: implementing
tags: [protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
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
