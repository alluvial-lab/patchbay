---
id: epic-agent-operations-resource-plane-conformance
kind: feature
stage: drafting
tags: [foundation, verification]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity, epic-agent-operations-resource-plane-resource-state, epic-agent-operations-resource-plane-capability-manifest, epic-agent-operations-resource-plane-cockpit-composition]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-07-30
---

# Resource-plane conformance evidence

## Brief

Prove, via executable conformance vectors and property tests, that a resource
adapter cannot bypass Patchbay authority, durability, or stale-state rules.
This is the v1 adapter-boundary evidence surface for the operational-resource
shape — the resource-plane analogue of the session-shape conformance the
public-product-contract arc already requires.

Coverage includes: a resource Operation is grant-gated and authority-checked
like a session Operation; resource Observations are source-authenticated and
cannot fabricate authority; resource snapshot/reconnect honors the
completeness tier a resource declares (partial/none degrades honestly);
stale/offline resource state never renders as live; cross-adapter resource-ID
collision is fenced; and a resource adapter cannot inject core-only state.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: closes the arc — depends on identity, state, manifest, and cockpit composition. Feeds the parent `epic-public-product-contract` adapter-portability proof.

## Simplification opportunity

- Extend the existing conformance-vector and property-test machinery rather than a parallel resource-only harness.

## Foundation references

- `docs/VERIFICATION.md` — conformance vectors, property-graded baseline
- `docs/PROTOCOL.md` — authority, snapshots, stale-state rules
- `contracts/vectors/` — existing vector corpus to extend

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No UI; verification artifacts.

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will fill in interfaces, signatures, and implementation units. -->
