---
id: epic-agent-operations-resource-plane-resource-state-projection-replay
kind: story
stage: implementing
tags: [protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-contract]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
---

# Fold and replay durable resource state

## Checkpoint

Evolve the identity-only `ResourceRegistry` into the canonical revisioned
resource projection, with active membership, per-view completeness, freshness,
terminal tombstones, replacement links, and deterministic `RESOURCE_STATE`
folding/replay. Make `TargetRegistry` fold both session and resource events so
resolution is populated only by durable resource facts.

## Acceptance evidence

- Replay validates authority domain and strictly increasing LSN order, ignores
  sibling event kinds, and produces the same projection on repeated prefixes.
- Full-tuple resources do not collide; active records resolve, tombstoned
  identities do not, and replacement does not reuse runtime-session generation.
- Per-record and per-view revision LSNs equal the durable event that last changed
  them; older/redelivered events are inert and contradictory newer events fail.
- A tombstoned identity cannot be resurrected; a distinct replacement identity
  is required and retained in the tombstone relation.

## Ordering constraints

Consumes the generated resource-state contract. The report writer and server
composition must use this fold rather than a second membership cache.
