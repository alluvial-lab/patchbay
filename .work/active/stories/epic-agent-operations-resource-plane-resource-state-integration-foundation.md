---
id: epic-agent-operations-resource-plane-resource-state-integration-foundation
kind: story
stage: implementing
tags: [foundation, protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-snapshot-load]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
---

# Close resource-state integration and foundation assertions

## Checkpoint

Exercise authenticated report → durable event → replayed resolver → resource
snapshot end to end, update every exhaustive `StoredEventKind` receiver, and
roll foundation docs forward with the committed resource state/reconnect
semantics and honest implementation-evidence tier. Keep capability-manifest,
cockpit composition, and promoted conformance evidence in their sibling
features.

## Acceptance evidence

- Real server tests prove resource identity registration is durable across
  restart, exact replacement/tombstone routing is preserved, and
  authoritative/partial/none reconnect behavior cannot fabricate current state.
- Existing command, authority, session, diagnostics, security, storage,
  subscription, CLI, and web tests stay green after the new event and snapshot
  view variants.
- Resource and projection payloads remain exact manifest-bound metadata
  envelopes; undeclared/mismatched/unspecified formats reject and docs prohibit
  data-plane traffic or credentials.
- Workspace tests/clippy, TypeScript suites, contract build/drift, vector/model
  metadata, and presentation checks pass without claiming checked-normative
  resource conformance.

## Ordering constraints

Runs after contract, projection/replay, authenticated ingress/reconciliation,
and snapshot loading are integrated. The parent feature is reviewed as the
cohesive boundary; this child does not receive an independent feature review.
