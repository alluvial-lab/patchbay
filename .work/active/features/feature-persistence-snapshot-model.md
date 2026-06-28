---
id: feature-persistence-snapshot-model
kind: feature
stage: drafting
tags: [prose, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
---

# Feature: Define persistence, event ordering, and snapshot convergence

Durable acceptance and snapshot recovery are core Patchbay promises, but the docs do not yet define the persistence backend, event ordering, snapshot revisions, or core crash behavior.

## Scope

- v0 persistence model and default backend.
- Event log / inbox / command state ownership.
- Core restart and crash recovery semantics.
- Snapshot revision/cursor model.
- Event stream vs snapshot atomicity.
- Older snapshot rejection and reconciliation rules.
- Adapter snapshot capability tiers and degraded behavior.

## Acceptance criteria

- `docs/ARCHITECTURE.md` states v0 persistence/topology assumptions.
- `docs/PROTOCOL.md` defines revision/cursor semantics for events and snapshots.
- `docs/UX.md` can describe reconnect behavior without relying on wall-clock freshness alone.
- `docs/VERIFICATION.md` has enough state variables to model snapshot convergence.
