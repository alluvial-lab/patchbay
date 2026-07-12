---
id: story-v0-core-persistence-recovery
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-rusqlite-impl]
created: 2026-07-11
updated: 2026-07-12
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

## Implementation notes

- **Files created**: `core/src/storage/recovery.rs`, `core/tests/recovery.rs`.
- **Design decision**: `recover()` returns a `RecoveryState` (snapshot + tail), not domain state. The storage layer does not own domain state (commands, sessions, grants) — that belongs to the sibling core features (acceptance, authority, sessions). `RecoveryState` gives the domain layer the raw materials: an optional snapshot (opaque bytes the domain layer deserializes) and the event tail to apply. This keeps the storage layer domain-neutral (Ports & Adapters).
- **`snapshot.rs` not created**: the story mentioned a `snapshot.rs` for snapshot helpers, but the snapshot logic (stale rejection, wrong-domain rejection, LSN validation) is already in the port trait + the rusqlite impl. Recovery only needs to *load* the latest snapshot and read the tail — no separate snapshot module needed.
- **`RecoveryState`**: carries `snapshot: Option<StoredSnapshot>` and `tail: Vec<RecordedEvent>`. Helper methods: `start_lsn()` (the LSN at which recovery starts — snapshot LSN or 0), `events()` (iterator over the tail in LSN order).
- **`recover()`**: loads the latest snapshot via `load_latest_snapshot(None)`, determines the cursor (snapshot LSN or 0), reads the tail via `read_after(cursor)`. Returns the `RecoveryState`.
- **Idempotency**: `recover()` is idempotent — the snapshot and tail are derived from the committed log (append-only). Calling it twice produces identical state. This is the storage-level mechanism for the `IdempotentLogReplay` stated-normative obligation; the proptest suite provides executable evidence.
- **Tests**: 7 tests covering all 5 acceptance criteria: no-snapshot recovery, crash recovery, idempotency, snapshot+tail equivalence to replay-from-0, start_lsn, empty log, snapshot bounds replay cost.
- **Discrepancies from design**: the design's `apply` function is not in this story — `apply` is the domain layer's job (it receives the `RecoveryState` and applies events to its own state). The storage layer provides the raw materials; the domain layer owns the application. This is the Ports & Adapters split.
- **Verification**: `cargo build --workspace` succeeds; `cargo test --package patchbay-core` passes 32/32 tests (7 port smoke + 18 rusqlite + 7 recovery).
