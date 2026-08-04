---
id: backlog-resource-generation-obsolete-event-no-op
kind: feature
stage: backlog
tags: [protocol, verification, conformance]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Backlog: Preserve obsolete resource-event no-op behavior under generation monotonicity

## Source

Parked from the thorough review of
`epic-agent-operations-resource-plane-resource-state`.

## Finding

The generation monotonicity guard in
`core/src/resource/registry.rs:100-112,119-146` rejects a later-observed
resource event whose source adapter generation is below the projected maximum.
That conflicts with the feature's documented observer contract at
`.work/active/features/epic-agent-operations-resource-plane-resource-state.md:303`,
which says an event at or below an already-projected record revision is inert.
The interaction needs an explicit rule for obsolete events so replay/catch-up
cannot turn a documented no-op into corruption merely because generation
validation runs before per-record obsolete-event filtering.

## Why parked

Ordered durable replay does not produce this path in the current implementation,
so the operational risk is lower than the stale-state blockers fixed in the
current cycle. Resolve it with the resource-plane conformance work, where the
observer contract can be represented by ordered/obsolete-event vectors and a
mutation-survivable invariant rather than patched from prose alone.

## Direction

Define whether an obsolete event is ignored before generation comparison or
whether the feature contract should explicitly narrow the no-op promise. Add
replay/catch-up tests covering already-projected LSNs across adapter-generation
transitions and trace the selected rule into the resource conformance feature.
