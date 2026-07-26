---
id: epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition
kind: story
stage: done
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

## Implementation notes

- Added the binary gRPC-Web `QueryDiagnostics` bridge with the same verified
  sender, server-stamped validity window, compound issuer headers, and CSRF
  gate as `Submit`. Added integration coverage proving forged browser identity
  is replaced and QueryDiagnostics requires CSRF.
- Added generated `DiagnosticsQuery` construction for the selected adapter
  (`OperationKind.QUERY`, authority-domain target, unique id/key, recent limit
  20), LSN-safe query/live merge keyed by source EventId, and snapshot-cleared
  adapter status. Selection, reconciliation, and selected-session connectivity
  changes trigger at most one in-flight adapter query; there is no timer or
  liveness inference.
- Composed adapter status and recent safe diagnostic evidence into the existing
  session rows, detail header, and timeline. Adapter connection labels remain
  separate from session connectivity; the explicit state mapping reuses the
  existing connectivity indicator CSS/showcase bindings. Warning/error records
  use canonical failure terms and informational records show only safe code,
  generation, count, and observed time.
- Extended `check-presentation.mjs` with a derived-member registry proving
  `AdapterDiagnosticState` maps exhaustively to existing connectivity bindings;
  no new protocol state, screen, route, or CSS state variant was added.
- Verification: web-cockpit tests (50), web-server tests (25), contract
  presentation/meta-tests, TypeScript builds, and the existing adapter/core
  tests pass. Unrelated concurrent `cli/` changes were preserved.
