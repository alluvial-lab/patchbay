---
id: epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding
kind: story
stage: done
tags: [observability, dogfooding, adapter]
parent: epic-observability-dogfooding-cockpit-diagnostics
depends_on: [epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Failure-isolated Pi adapter diagnostics forwarding

## Checkpoint

Reuse the Pi adapter's typed diagnostics port and event registry to map a
bounded operational subset into generated adapter-diagnostic payloads. Add an
event-driven, rate-bounded forwarding sink whose network work is queued,
sequential, non-recursive, and best-effort. The local durable JSONL sink remains
the fallback when attachment or core transport is unavailable.

This checkpoint owns Unit 4 and the adapter share of Unit 8 in the parent
feature. The parent design is authoritative for the code mapping, canonical
failure mapping, queue/rate bounds, report timeout, and no-heartbeat rule.

## Acceptance evidence

- The Pi capability manifest declares diagnostic reporting and derives its code
  list from the one adapter-owned mapping registry.
- Attach/lifecycle, registration, delivery, subscription, observation, and
  disposal failures that have canonical failure mappings produce structurally
  safe reports with verified adapter/session/command context.
- `record()` performs no network I/O, forwarding is capped at ten reports per
  second with a 256-record queue and bounded coalescing, and overflow or core
  failure never applies backpressure to the control loop.
- Report rejection, timeout, authentication loss, flush, and close never reject
  attach, delivery, observation, session registration, or shutdown.
- Reporting failures do not recursively emit reports and do not initiate a
  liveness/heartbeat policy; initial attach failures remain available in the
  adapter-local durable log when no authenticated core channel exists.
- Adapter tests prove no prompt, transcript, tool result, arbitrary error
  message/stack/cause, attachment material, or credential can enter a report.

## Ordering constraints

Consumes the generated report contract and core endpoint from
`epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion`. Reuse
`pi-adapter/src/adapter_diagnostics.ts` from the sibling adapter-log-sink design
when it has landed; sequence write ownership rather than cloning that port or
its event registry.

## Implementation notes

- Added `CoreDiagnosticsForwarder` as a second `AdapterDiagnostics` sink and
  composed it with the landed JSONL sink. Existing instrumentation remains the
  sole event source; there is no parallel diagnostics interface or registry.
- `PI_FORWARDED_DIAGNOSTIC_CODES` is the single Pi-owned mapping used both by
  payload construction and `piCapabilityManifest().diagnosticReporting`, with
  canonical failure mappings for warning/error reports. Local `reason` and
  `error` fields are never copied into the report.
- The forwarder is sequential, bounded to 256 pending keys and 10 reports per
  second by default, coalesces identical safe keys to count 1000, times out
  each report at one second, never retries, and keeps flush/close non-throwing.
  `PatchbayCoreClient.reportDiagnostic` deliberately bypasses `#postAttach` so
  auth loss cannot trigger token refresh or recursive control traffic.
- Production `AdapterProcess` composition enables forwarding only at the
  environment composition root; unit/integration test adapters retain the
  injectable local sink by default. A throwing sink cannot veto a healthy sink.
- Verification: the forwarder tests and all other Pi tests pass; the real-process
  e2e passes when run in isolation. The package's parallel `npm test` run has a
  pre-existing intermittent cancellation in that e2e, which was recorded rather
  than weakening or skipping the test.
