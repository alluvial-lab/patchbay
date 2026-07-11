---
id: story-v0-core-persistence-recovery
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-rusqlite-impl]
created: 2026-07-11
updated: 2026-07-11
gate_origin: null
release_binding: null
---

# Story: Crash recovery and replay

## Scope

Implement crash recovery: on startup, load the latest snapshot (if any) and replay events with `LSN > snapshot_lsn` to reconstruct in-memory state. Recovery is idempotent — replaying the same committed prefix produces identical state.

## Units

- `core/src/storage/recovery.rs` — `recover()` function: load latest snapshot → replay tail
- `core/src/storage/snapshot.rs` — snapshot load/write helpers, stale-snapshot rejection

## Key implementation details

- Recovery loads the latest snapshot, then replays the tail (`LSN > snapshot_lsn`). Bounds replay cost.
- `apply` is idempotent — replaying the same committed prefix produces identical state (`IdempotentLogReplay`, stated-normative).
- Snapshot is a derived checkpoint, never an alternate ordering authority. A snapshot with LSN < current state is rejected as stale (Fail Fast).
- `SnapshotConsistentPrefix` (stated-normative): snapshot materialization reads a consistent log prefix — satisfied by writing the snapshot in the same transaction as the event at `snapshot_lsn`.
- A snapshot from a different authority domain is rejected outright.

## Acceptance criteria

- [ ] After a clean shutdown + restart, `recover()` reconstructs state identical to pre-shutdown.
- [ ] After a simulated crash (no clean shutdown), `recover()` reconstructs state up to the last committed LSN; no accepted event is lost.
- [ ] Replay is idempotent: calling `recover()` twice produces identical state.
- [ ] A snapshot at LSN N + replay of events N+1..M produces state identical to replaying from 0.
- [ ] A stale snapshot (LSN < current state) is rejected, not silently applied.

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 3" for the `recover()` signature and semantics.
