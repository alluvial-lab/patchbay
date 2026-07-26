---
id: epic-observability-dogfooding-adapter-log-sink
kind: feature
stage: drafting
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Adapter durable diagnostics log sink

## Brief

pi-adapter currently keeps all diagnostics process-local: `TranscriptEventLog`
is an in-memory partial-snapshot log, `#observationError` and delivery/attach
failures die with the process, and nothing is configurable. The only way to
inspect the adapter during live testing is whatever shell redirect the
operator happened to launch it with.

This feature gives pi-adapter a durable, structured diagnostics log: an
env-configured file sink (`PATCHBAY_ADAPTER_LOG`, defaulting to an XDG state
dir such as `~/.local/state/patchbay/adapter.log`) capturing attach,
registration, delivery, observation, and lifecycle events with their error
detail. It is the fastest inspection unblock in the epic and is deliberately
adapter-local — forwarding diagnostics to core is
`epic-observability-dogfooding-cockpit-diagnostics`.

It does NOT cover: log shipping, rotation policy beyond a sane local default,
or any core-side change.

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: independent capability — no shared types with the other
  children; parallelizable from day one. Priority 1 in the epic's seed order
  (fastest operator unblock).

## Simplification opportunity

- `TranscriptEventLog` may be subsumed or repositioned: decide whether it
  survives as an in-memory ring feeding the durable sink, or is deleted in
  favor of the sink plus core-side transcript durability (transcript events
  already reach the core's durable log via `ingestTranscript`).
- Retain: Pi's own persisted session remains the durable transcript record;
  the sink is for adapter diagnostics, not a second transcript store.

## Foundation references

- `docs/SPEC.md` — post-v0.1.0 observability scope (adapter-process durable diagnostics)
- `docs/ADAPTER-PI.md` — adapter behavior and snapshot-tier context
- `docs/SECURITY.md` — redaction discipline applies to anything the sink writes

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will
fill in log format, rotation defaults, and implementation units. -->
