---
id: story-v0-core-persistence-rusqlite-impl
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-persistence
depends_on: [story-v0-core-persistence-workspace-and-port]
created: 2026-07-11
updated: 2026-07-11
gate_origin: null
release_binding: null
---

# Story: rusqlite storage implementation + writer actor

## Scope

Implement the `Storage` trait with a rusqlite-backed implementation: SQLite schema (WAL mode, `synchronous=FULL`, `events` table with bare `INTEGER PRIMARY KEY` as LSN, `snapshots` table), a writer actor owning the single write `Connection`, and a read connection for concurrent reads.

## Units

- `core/src/storage/rusqlite.rs` — `RusqliteStorage` struct, schema initialization, `Storage` trait impl
- `core/src/storage/writer_actor.rs` — writer actor loop (mpsc command → transaction → oneshot reply)

## Key implementation details

- `PRAGMA journal_mode = WAL` — concurrent readers
- `PRAGMA synchronous = FULL` — safety-critical durability (research recommendation)
- `events` table: `lsn INTEGER PRIMARY KEY` (bare, no AUTOINCREMENT), `authority_domain_id TEXT`, `payload BLOB`
- `snapshots` table: `(authority_domain_id, snapshot_lsn)` primary key, `payload BLOB`
- LSN = `last_insert_rowid()` after INSERT commits (gap-free on append-only table without AUTOINCREMENT)
- Writer actor: single `Connection` on one tokio task; `mpsc` commands serialize appends; `oneshot` replies give async semantics
- Read connection: separate `Connection` serving `read_prefix` and `load_latest_snapshot` via `SELECT ... WHERE authority_domain_id = ? AND lsn > ? ORDER BY lsn`

## Acceptance criteria

- [ ] `RusqliteStorage::append` returns an `EventId` whose LSN equals the rowid SQLite assigned.
- [ ] Consecutive appends produce contiguous LSNs (1, 2, 3, ...).
- [ ] A simulated crash (drop the handle, reopen the DB) recovers all committed events via `read_prefix(0)`.
- [ ] A rolled-back transaction does not create a gap in committed LSNs.
- [ ] `synchronous=FULL` is set and verifiable via `PRAGMA synchronous` query.
- [ ] Read connection serves concurrent reads while a write is in flight (WAL).

## Design reference

See `feature-v0-core-persistence.md` § "Implementation Units" → "Unit 2" for the schema, struct shape, and writer-actor command enum.
