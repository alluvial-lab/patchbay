---
id: epic-agent-operations-resource-plane-resource-state
kind: feature
stage: drafting
tags: [foundation, protocol, storage]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-07-30
---

# Resource snapshot, revision & ingestion

## Brief

Give operational resources a durable state model distinct from runtime-session
state: a `ResourceSnapshot` record with view revisions, an explicit
completeness tier (authoritative / partial / none), tombstone/replacement
semantics, and a typed resource-report ingress path (analogous to
`SessionReport`) so adapters submit structured resource state rather than only
generic Observations. Resource Observation deltas fold into a revisioned
projection; reconnect reconciles against the resource snapshot.

Today there is no `ResourceSnapshot`, no resource revision record, no
completeness tier, no typed resource-report ingress, and no `StoredEventKind`
resource variant — `LoadSnapshot` returns opaque bytes implemented as
`SessionSnapshot`, and the live materializer (`server/src/state.rs`) emits
session records only. This feature adds durable resource state + replay and
the resource `LoadSnapshot` path. It must degrade honestly: a resource
adapter may claim only the snapshot tier its complete external view can
actually reconstruct.

It does not define the adapter capability manifest fields that declare which
resource kinds/snapshot tiers an adapter supports (`capability-manifest`) or
cockpit rendering (`cockpit-composition`).

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: state foundation — depends on `resource-identity`; consumed by `capability-manifest` and `cockpit-composition`.

## Simplification opportunity

- Reuse the opaque-domain-keyed storage port (`core/src/storage/port.rs`) and the consistent-prefix materializer pattern already used for sessions; add a resource materializer rather than a parallel store.
- Keep one snapshot/reconnect discipline rather than per-resource-type state stores.

## Foundation references

- `docs/PROTOCOL.md` — snapshots/revisions, reconnect reconciliation, snapshot tiers
- `contracts/proto/patchbay/sessions.proto:40-61` — `SessionSnapshot` (the pattern to mirror, not reuse)
- `contracts/proto/patchbay/adapter_control.proto:38-55` — only `SessionReport` is typed today
- `contracts/proto/patchbay/control.proto:44-53` — `LoadSnapshot` returns opaque bytes
- `contracts/proto/patchbay/common.proto:120-158` — `StoredEventKind` has no resource variant
- `core/src/storage/port.rs:57-64` — opaque domain/LSN-keyed snapshots
- `core/src/acceptance/observation.rs:216-252` — Status/Result folds command state, not resource revisions

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI; the snapshot/revision model the cockpit feature renders.

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will fill in interfaces, signatures, and implementation units. -->
