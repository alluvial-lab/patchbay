---
id: epic-agent-operations-resource-plane-resource-state-projection-replay
kind: story
stage: done
tags: [protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-contract]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
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

## Implementation notes

Replaced identity-only membership with the canonical `ResourceRegistry`
projection over durable `RESOURCE_STATE` events. The fold validates event/domain
identity, exact adapter/kind tuples, view completeness, envelopes, prior
revisions, replacement upserts, freshness values, and timestamps before an
atomic clone-and-install. Resource/view revisions come only from the enclosing
committed LSN; tombstones remain auditable but are excluded from ordinary
resolution and cannot be resurrected. Added full-log replay with strict domain
and monotonic-LSN validation, and made the composite `TargetRegistry` fold both
session and resource events. The server rebuild path now restores resource
membership from durable facts rather than constructing an empty cache.

Existing resource resolver/acceptance fixtures were migrated from direct
identity insertion to real durable resource events. New projection and replay
tests cover exact tuple collisions, current/stale/unknown state, replacement,
terminal resurrection rejection, atomic failure, deterministic replay, sibling
event ignoring, cross-domain input, and non-increasing prefixes.

Checkpoint verification: `cargo test -p patchbay-core --tests`,
`cargo check --workspace`, and
`cargo clippy -p patchbay-core --all-targets -- -D warnings` passed.
