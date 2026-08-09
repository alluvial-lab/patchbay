---
id: storage-recovery-correctness
kind: feature
stage: drafting
tags: [foundation, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Storage recovery correctness

## Brief
Consolidate open durability and recovery-cost follow-ups into one storage correctness feature. Absorbed findings:

- `backlog-core-generation-persistence`: implement persistence and cross-incarnation validation for the wire-present core generation once restart ambiguity makes it load-bearing.
- `backlog-snapshot-checkpoint-writer`: add a production checkpoint writer and scheduling policy so recovery replay cost remains bounded as the durable log grows.

## Simplification opportunity
Keep snapshots as derived recovery checkpoints and preserve the durable event log as the sole ordering authority; do not introduce a second state store or a parallel recovery path.
