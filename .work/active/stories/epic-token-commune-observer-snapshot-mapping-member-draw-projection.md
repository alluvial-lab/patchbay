---
id: epic-token-commune-observer-snapshot-mapping-member-draw-projection
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-snapshot-mapping
depends_on: [epic-token-commune-observer-snapshot-mapping-provider-pool-projection]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# Project per-provider member draw without aggregation

## Checkpoint

Map reported `/commune/me` state into one member-draw resource per `(display name, provider)` synthesized identity while retaining every same-provider `DrawReport` row. Preserve `limitFraction`, `fromDecree`, provider-native `consumedUnits`, nullable `drawUnits`, `exceeded`, `enforceable`, and nullable reset time exactly. Do not collapse providers, select one duplicate row, infer calibration, or derive an aggregate enforcement state.

## Files

- `token-commune-adapter/src/snapshot_projection.ts`
- `token-commune-adapter/src/resource_contract.ts`
- `token-commune-adapter/tests/fixtures/snapshot_projection.ts`
- `token-commune-adapter/tests/snapshot_projection.test.ts`

## Acceptance evidence

- Different providers produce distinct member-draw identities and mutations; duplicate same-provider rows remain an ordered deterministic array in one resource.
- `fromDecree` provenance and all nullable/false calibration fields survive envelope JSON exactly.
- No cross-provider draw total, average, hero percentage, or derived enforcement state is emitted.
- An empty reported draw array and an unavailable `/commune/me` source both emit no member mutation, leaving PARTIAL omission to stale prior core state rather than tombstoning it.
- Display-name changes create a new local identity and do not claim replacement or silently merge the old resource.

## Ordering

Depends on provider-pool projection so both kinds share one settled deterministic report assembly and envelope contract.
