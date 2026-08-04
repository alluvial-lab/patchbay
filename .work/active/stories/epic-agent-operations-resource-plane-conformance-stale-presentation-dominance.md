---
id: epic-agent-operations-resource-plane-conformance-stale-presentation-dominance
kind: story
stage: implementing
tags: [verification, protocol]
parent: epic-agent-operations-resource-plane-conformance
depends_on: [epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Prove stale resource presentation dominance

## Checkpoint

Add the promoted stale-never-live vector with both server/core and web-cockpit
implementation checks. The server half begins with current cached resource
state, applies adapter disconnect/reconnect degradation, and materializes a
stale/unknown resource snapshot. The TypeScript half loads the vector's
proto-shaped view and proves that reconciliation state, tombstones, and
freshness dominate adapter-owned domain health in both model predicates and
rendered output.

Extend the existing fast-check style in `web-cockpit/tests/model.test.ts` and
`resource-view.test.ts`. Generate only internally valid `ResourceView` values,
then independently compute whether current presentation is permitted. Explicit
mutants that ignore `reconciled`, `tombstoned`, or `freshness`, or that render
adapter-domain `health = serving` as current, must fail.

## Primary files

- `contracts/vectors/resource-stale-never-live.json` (new)
- `server/tests/conformance_vectors.rs`
- `web-cockpit/tests/conformance-vectors.test.ts`
- `web-cockpit/tests/model.test.ts`
- `web-cockpit/tests/resource-view.test.ts`
- `web-cockpit/tests/reconcile.test.ts`

## Acceptance evidence

- Adapter disconnect turns cached current resource state stale (or preserves
  no-payload unknown); the resource snapshot never fabricates current health.
- `rendersResourceCurrent(view)` is true exactly for reconciled,
  non-tombstoned `CURRENT` records and false for every generated stale,
  unknown, unreconciled, retired, or disconnected case.
- Rendered stale data is labeled last reported, unknown has no current domain
  health/meter, and no disallowed case receives current freshness/live styling,
  even when the decoded adapter projection says `serving` or `ok`.
- Stream-break then snapshot repair follows the same dominance rule without a
  half-reconciled model.
- Each presentation mutant is killed by the generated independent oracle.

## Ordering constraints

Depends on both the execution bridge and durable reconnect checkpoint so the
vector proves the real degradation source before asserting presentation. This
is verification of the existing cockpit contract, not new UI design.
