---
id: epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation
kind: story
stage: implementing
tags: [adapter, protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-projection-replay, epic-agent-operations-resource-plane-capability-manifest-core-admission]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
---

# Ingest authenticated resource reports and reconcile reconnects

## Checkpoint

Add the typed `ResourceReport` branch to adapter ingress, translate the generated
wire report into the validated core report, derive one atomic normalized
resource-state event, and fold it only after durable append. Implement reconnect
and adapter-loss degradation against per-view authoritative/partial/none
semantics while authenticating the exact adapter id and current adapter
generation.

## Acceptance evidence

- Malformed, cross-adapter, stale-generation, undeclared/mixed-kind,
  overclaimed/unknown-tier, schema-mismatched resource/projection, and invalid-
  payload reports reject before append or projection mutation.
- Authoritative snapshot reports upsert listed resources and tombstone omitted
  active members; partial reports update listed members and stale omissions;
  none reports carry no reconstructed members and stale cached state.
- Live delta reports mutate only explicitly named resources. Atomic replacement
  tombstones the old exact identity and registers its distinct same-adapter
  replacement in one durable event.
- Adapter disconnect marks every active owned resource stale in the same
  audited batch as session degradation; a fenced old delivery stream is inert.
- Partial append/fold failures recover by rebuilding the projection from the
  durable log rather than inventing in-memory success.
- Production ingress consumes the sibling manifest's exact
  `resource_capability` / `validate_resource_projection` API; it adds no
  fallback manifest fields or capability-derived authority.

## Ordering constraints

Consumes the resource projection/replay checkpoint. The parallel capability-manifest checkpoint owns declarations; this checkpoint
consumes its admission API, while retaining ownership only of state/tier fold
semantics and adding no manifest fields.
