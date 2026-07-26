---
id: epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding
kind: story
stage: implementing
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
