---
id: epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition
kind: story
stage: implementing
tags: [observability, dogfooding, ui]
parent: epic-observability-dogfooding-cockpit-diagnostics
depends_on: [epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Cockpit adapter-status composition

## Checkpoint

Proxy `ControlService.QueryDiagnostics` through the authenticated gRPC-Web
bridge, query the selected session's adapter through a normal `query` Operation,
and merge bounded `AdapterStatusPage` results with live typed diagnostic
Observations. Compose adapter connection and recent diagnostic evidence into the
existing session row/detail/timeline using the existing connectivity indicator,
alert, failure-vocabulary banner, and list patterns.

This checkpoint owns Units 5-7 and the web/e2e share of Unit 8 in the parent
feature. The parent design is authoritative for query refresh triggers, stale
result merging, display labels, and the mockup-skip decision.

## Acceptance evidence

- The web server applies the same verified compound issuer, server-stamped time
  window, CSRF requirement, framing, and error mapping to `QueryDiagnostics` as
  to `Submit`; there is no JSON REST DTO or persistence read.
- The cockpit queries only the selected adapter at initial reconciliation,
  selection change, or selected-session connectivity change; it does not poll
  time, send heartbeats, or infer liveness from diagnostic silence.
- Older query results cannot overwrite newer streamed diagnostic evidence;
  reconnect clears/requeries adapter status and never renders historical attach
  as a current connection.
- Every adapter diagnostic state maps explicitly onto an existing connectivity
  indicator, the presentation conformance check covers that derived mapping,
  recent warn/error events use canonical failure banners, and labels say “no
  recent reported issues” rather than claiming health from absence.
- Session list and detail tests cover attached/detached/failed/unknown adapter
  state, session-vs-adapter liveness separation, recent event ordering/dedup,
  accessibility roles, mobile/desktop composition, and safe empty/error states.
- The real-process evidence observes an adapter lifecycle diagnostic through
  core storage/query and the cockpit contract without exposing a forbidden
  sentinel.

## Ordering constraints

Consumes the report/query fields established by
`epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion` and the
representative Pi events established by
`epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding`. It adds
no dedicated diagnostics screen, dashboard, or delivery-trace timeline.
