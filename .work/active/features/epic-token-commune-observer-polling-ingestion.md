---
id: epic-token-commune-observer-polling-ingestion
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-adapter-foundation, epic-token-commune-observer-snapshot-mapping]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# token-commune polling ingestion and observations

## Brief

The **runtime capability**: the long-running poller that drives the snapshot
projection and emits reports/Observations to the core with honest gap and
staleness behavior. This feature owns the schedule, the streaming/gap logic, and
the Observation mapping — the projection function itself lives in
`snapshot-mapping`.

It delivers: the polling schedule over the gateway read endpoints (no upstream
stream/webhook/cursor exists); resource-report emission via
`IngestObservation.resource_report`; `PoolEvent` → generic `Observation` mapping
(source-authenticated status emissions with operational-resource target scope
and adapter-owned schema refs); deduplication; explicit gap behavior (the
latest-50-event window, missed polls, and reconnect reconciliation — the adapter
must never claim a stream or unlimited-history repair); source-timestamp
propagation (each reading's `observedAt`, since capacity polling is itself
gated/backoff-delayed upstream); and stale-state degradation on disconnect
(reusing the core's adapter-loss inference from the `ReceiveDeliveries` stream
drop).

It does NOT cover the projection logic (`snapshot-mapping`) or the cockpit.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **runtime/streaming** — consumes the projection from
  `snapshot-mapping` and the attach lifecycle from `adapter-foundation`;
  produces the live resource state + Observations that `cockpit-panel` renders.

## Simplification opportunity

- Reuse the Pi adapter's report-ordering/backpressure and delivery-stream
  reconnect/reattach machinery; the poller only adds "when to fetch and what
  gap/staleness to report."
- Do not build a synthetic event stream. Polling is the honest delivery model;
  claiming otherwise is explicitly rejected.

## Foundation references

- `docs/PROTOCOL.md` — snapshots repair missed streams/gaps; partial/none tiers
  degrade as defined; a resource adapter may claim only the tier its complete
  external view can reconstruct.
- `docs/ARCHITECTURE.md` — adapter loss degrades owned resources honestly rather
  than fabricating liveness.
- `contracts/proto/patchbay/adapter_control.proto` — `IngestObservation`
  (`resource_report` + generic `event` arms), `ReceiveDeliveries`.
- `contracts/proto/patchbay/observations.proto` — `Observation`, `ObservationKind`.
- External contract: token-commune `/commune/events` (latest 50, no cursor;
  in-memory fallback ring of 200); declared vs actually-emitted event kinds
  (`window_exhausted`, `calibration` are declared but have no production emitter).

## Key design decisions (inherited)

- **Polling-only, honestly.** No upstream stream exists. The poller reports
  `observedAt` per reading and degrades to stale when the core connection or
  upstream poll is lost. It must not present polling cadence as streaming.
- **Gap behavior is bounded by reality.** A reconnecting observer cannot
  reconstruct more than the latest 50 events or history before initial install;
  the adapter reports the gap honestly rather than fabricating continuity.
  Cursor/replay is an external prerequisite.
- **Partial event coverage.** Only a subset of declared event kinds is emitted
  upstream today; the Observation mapping covers what is actually emitted and
  does not assert lifecycle coverage the upstream does not provide.

<!-- The design pass fills in the poll schedule, dedup keys, gap-detection
rules, Observation schema refs, and implementation units. -->
