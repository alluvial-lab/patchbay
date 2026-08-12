---
id: spawn-delivery-atomic-claim-idempotency-generation
kind: story
stage: drafting
tags: [adapter, protocol]
parent: research-handoff-spawn
depends_on: []
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Atomic claim before delivery (caller idempotency + target generation)

## Origin (research-grounded)
Decomposed from `mc-architectural-harvest` direction **3** (folds into the spawn stride — generation lifecycle).
- **Source campaign:** `.research/analysis/campaigns/v1-control-plane-and-spawn/`.
- **Harvest item:** `.work/active/stories/mc-architectural-harvest.md` (direction 3).

## Direction
Borrow MC's compare-and-swap task claim (prevents two scheduler workers concurrently dispatching one task) and **strengthen it with the operation contract MC lacks**: caller idempotency keys + target generation fencing. An accepted Operation's delivery should atomically claim its target so concurrent delivery attempts for the same (target, generation) cannot both succeed; losing attempts terminate cleanly rather than double-delivering.

## Why it folds into spawn
Target-generation fencing is the spawn stride's generation-lifecycle concern. This story is a child of `research-handoff-spawn`; design it alongside the spawn target/generation model rather than as standalone delivery work. Coordinate with the v0.2.0 adapter-report ordering (generation/revision cursors) + token-commune dedup/reconnect evidence.

## Scope
Design-bearing (acceptance/delivery atomicity + generation fencing) → coordinate within the spawn `feature-design`. Until spawn's target/generation model is set, this stays drafting.
