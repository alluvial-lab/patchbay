---
id: backlog-resource-reconciliation-arbitrary-sequences
kind: feature
stage: backlog
tags: [testing, verification, conformance]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Backlog: Expand resource reconciliation generation to arbitrary report sequences

## Source

Parked from the thorough review of
`epic-agent-operations-resource-plane-resource-state`.

## Finding

The generated-sequence test at
`core/tests/resource_reconciliation.rs:18-75` is only a two-report branch
sampler: one authoritative upsert followed by one of four omission branches.
It does not satisfy the broader sequence evidence described in the feature body
at
`.work/active/features/epic-agent-operations-resource-plane-resource-state.md:419,502`:
arbitrary ordered report sequences, adapter-generation transitions,
replacements, and terminal mutation attempts.

## Why parked

The existing focused regressions cover the three material stale/completeness
failures fixed in this review, and the missing breadth does not itself establish
a current implementation defect. The resource-plane conformance feature is the
right owner because it already must connect arbitrary traces to durability,
wrong-target, reconnect-honesty, and terminality obligations.

## Direction

Generate ordered snapshot/delta sequences across authoritative/partial/none
views; multiple adapter generations; distinct replacement identities; repeated
replay; and upsert/unknown/tombstone attempts after terminal retirement. Compare
the hot fold, restart replay, and snapshot projection after every accepted
prefix, and include negative traces whose rejected candidates leave the durable
prefix and projection unchanged.
