---
id: story-v0-core-persistence-recovery
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-rusqlite-impl]
created: 2026-07-11
updated: 2026-07-12
gate_origin: null
release_binding: v0.1.0
---

# Story: Crash recovery and replay

## Scope

Implement crash recovery: on startup, load the latest snapshot (if any) and replay events with `LSN > snapshot_lsn` to reconstruct in-memory state. Recovery is deterministic for unchanged storage contents — replaying the same committed prefix produces identical raw materials. Full idempotent replay depends on the domain layer's deterministic `apply`.

## Units

- `core/src/storage/recovery.rs` — `recover()` function: load latest snapshot → replay tail
- `core/src/storage/snapshot.rs` — snapshot load/write helpers, stale-snapshot rejection

## Key implementation details

- Recovery loads the latest snapshot, then replays the tail (`LSN > snapshot_lsn`). Bounds replay cost.
- `apply` must be deterministic — replaying the same committed prefix must produce identical state at the domain layer (`IdempotentLogReplay`, stated-normative). This module provides the mechanism (deterministic raw materials); the domain layer's `apply` must be deterministic for the property to hold end-to-end.
- Snapshot is a derived checkpoint, never an alternate ordering authority. A snapshot with LSN < current state is rejected as stale (Fail Fast).
- `SnapshotConsistentPrefix` (stated-normative): snapshot materialization reads a consistent log prefix — this is the *caller's* obligation per `port.rs` `write_snapshot`; the port validates the LSN anchor, the materializer constructs the consistent-prefix payload.
- A snapshot from a different authority domain is rejected outright.

## Acceptance criteria

- [ ] After a clean shutdown + restart, `recover()` reconstructs state identical to pre-shutdown.
- [ ] After a simulated crash (no clean shutdown), `recover()` reconstructs state up to the last committed LSN; no accepted event is lost.
- [ ] Replay is deterministic for unchanged storage contents: calling `recover()` twice with no intervening writes produces identical `RecoveryState`.
- [ ] A snapshot at LSN N + replay of events N+1..M produces state identical to replaying from 0.
- [ ] A stale snapshot (LSN < current state) is rejected, not silently applied.

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 3" for the `recover()` signature and semantics.

## Implementation notes

- **Files created**: `core/src/storage/recovery.rs`, `core/tests/recovery.rs`.
- **Design decision**: `recover()` returns a `RecoveryState` (snapshot + tail), not domain state. The storage layer does not own domain state (commands, sessions, grants) — that belongs to the sibling core features (acceptance, authority, sessions). `RecoveryState` gives the domain layer the raw materials: an optional snapshot (opaque bytes the domain layer deserializes) and the event tail to apply. This keeps the storage layer domain-neutral (Ports & Adapters).
- **`snapshot.rs` not created**: the story mentioned a `snapshot.rs` for snapshot helpers, but the snapshot logic (stale rejection, wrong-domain rejection, LSN validation) is already in the port trait + the rusqlite impl. Recovery only needs to *load* the latest snapshot and read the tail — no separate snapshot module needed.
- **`RecoveryState`**: carries `snapshot: Option<StoredSnapshot>` and `tail: Vec<RecordedEvent>`. Helper methods: `start_lsn()` (the LSN at which recovery starts — snapshot LSN or 0), `events()` (iterator over the tail in LSN order).
- **`recover()`**: loads the latest snapshot via `load_latest_snapshot(None)`, determines the cursor (snapshot LSN or 0), reads the tail via `read_after(cursor)`. Returns the `RecoveryState`.
- **Determinism, not unconditional idempotency**: `recover()` is deterministic for unchanged storage contents — two calls with no intervening writes produce identical `RecoveryState`. If events or newer snapshots commit between calls, the second may return newer raw materials. This is correct behavior, not a violation. Full `IdempotentLogReplay` depends on the domain layer's deterministic `apply`; this module provides the storage-level mechanism.
- **Tests**: 9 tests covering the storage-mechanism portion of the 5 acceptance criteria + edge cases: no-snapshot recovery, crash recovery (storage layer), determinism, snapshot+tail equivalence to replay-from-0, start_lsn, empty log, snapshot bounds replay cost, snapshot-at-log-head (empty tail), malformed-snapshot (no LSN) rejection. End-to-end domain-state equivalence and acceptance-layer loss prevention are not tested here — they depend on the acceptance and domain features.
- **Discrepancies from design**: the design's `apply` function is not in this story — `apply` is the domain layer's job (it receives the `RecoveryState` and applies events to its own state). The storage layer provides the raw materials; the domain layer owns the application. This is the Ports & Adapters split.
- **Verification**: `cargo build --workspace` succeeds; `cargo test --package patchbay-core` passes 34/34 tests (7 port smoke + 18 rusqlite + 9 recovery).

## Re-review (round 3, converged — Approve)

Round 3 returned **Approve** with 2 wording nits, both fixed in-stride:
- `apply` "is deterministic" → "must be deterministic" (it's not implemented here)
- Test coverage claim scoped to the storage-mechanism portion (end-to-end domain-state equivalence depends on the acceptance + domain features)

Story advanced review → done.
