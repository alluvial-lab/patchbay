---
id: resource-reconciliation-followups
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Resource reconciliation follow-ups

## Brief
Consolidate the two items parked from the resource-state review into the resource reconciliation follow-up. Absorbed findings:

- `backlog-resource-generation-obsolete-event-no-op`: define and preserve obsolete-event no-op behavior when generation monotonicity and replay/catch-up ordering interact.
- `backlog-resource-reconciliation-arbitrary-sequences`: expand reconciliation evidence from a two-report sampler to arbitrary ordered sequences, generation transitions, replacements, replay, and terminal mutation attempts.

## Simplification opportunity
Express obsolete-event handling and arbitrary sequence evidence through the existing resource conformance/reconciliation fold rather than adding a parallel resource-state mechanism.
