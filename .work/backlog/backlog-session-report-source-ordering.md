---
id: backlog-session-report-source-ordering
created: 2026-07-24
tags: [protocol, contract-hardening]
research_origin: null
---

# Backlog: adapter report source-ordering (stale report can roll mutable fields backward)

Surfaced by the `feature-session-model-field` review (2026-07-24, standard
cross-model pass) as an Important finding — parked, not a v0.1.0 blocker.

## The gap

`SessionReport` (`contracts/proto/patchbay/adapter_control.proto`) carries no
adapter-side revision or sequence number. The core derives mutations from the
report's fields (e.g. `SessionModelChanged { from: <current>, to: <reported> }`
in `core/src/session/ingest.rs`). If model `B` is current and a delayed but
sequential report carrying `A` arrives, the core derives a valid `B → A`
mutation and applies it — rolling the current value backward. The registry's
`from` and LSN checks (`core/src/session/registry.rs`) protect durable replay
order but cannot identify a stale *source* report.

## Why parked

- The Pi adapter's promise-tail serialization mitigates this for the only
  v0.1.0 adapter (reports are emitted in order on one connection).
- The durable log stays replayable (the derived mutation is internally
  consistent); the hazard is semantic staleness, not corruption.

## Shape (at design time)

Add a monotonic adapter-side report revision/sequence to `SessionReport` and
have core ingest reject (or mark stale) reports whose revision is not greater
than the last applied revision for that session generation. This is a contract
change → conformance vector + extension-seams classification required. Consider
whether it generalizes beyond `model` to other report-carried mutable fields.
