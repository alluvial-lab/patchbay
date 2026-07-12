---
id: story-v0-core-persistence-rusqlite-impl
kind: story
stage: review
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

## Implementation notes

- **Files created**: `core/src/storage/rusqlite.rs`, `core/tests/rusqlite_storage.rs`.
- **Schema**: `events` (lsn INTEGER PRIMARY KEY, authority_domain_id, kind, payload BLOB), `idempotency_keys` ((authority_domain_id, key, target) PK, lsn, payload_hash), `snapshots` ((authority_domain_id, snapshot_lsn) PK, payload BLOB).
- **Writer actor**: owns the single write `Connection` on a tokio task. `mpsc` commands (Append, AppendDedup, WriteSnapshot) serialized; `oneshot` replies give async semantics. `do_*` functions take `&mut Connection` for transactions.
- **Read connection**: separate `Connection` behind `tokio::sync::Mutex`, serves `read_after` and `load_latest_snapshot`. WAL allows concurrent reads.
- **append_dedup**: atomic check-and-register in one transaction. Queries `idempotency_keys` for the (domain, key, target) tuple; if present with matching payload hash → Duplicate(existing_lsn); if present with differing hash → IdempotencyConflict; if absent → INSERT event + INSERT idempotency_key in one transaction → Appended(new_lsn). The `appliedKeys` set lives in the `idempotency_keys` table.
- **Payload encoding**: `StoredEventPayload` serialized via `prost::Message::encode` (length-delimited). Decoded via `prost::Message::decode`. The `kind` field is stored both in the proto envelope and as a SQL column (for future queryability without deserializing) — the proto envelope is authoritative; the column is a derived index.
- **Idempotency hash**: `DefaultHasher` over (kind, payload bytes). Sufficient for conflict detection; not cryptographic. A cryptographic hash (SHA-256) would be stronger but adds a dependency for v0.1.0 single-operator scope where the threat model doesn't require collision resistance.
- **validate_kind**: explicitly rejects `StoredEventKind::Unspecified` (try_from succeeds for it since it's a valid enum value, so the explicit check is necessary).
- **open_in_memory**: uses a temp file under the hood because WAL mode requires a file-backed DB (in-memory SQLite doesn't support WAL). `tempfile` moved from dev-dep to dep.
- **Error mapping**: `map_write_err`/`map_read_err` classify `DatabaseBusy`/`DatabaseLocked` as retryable; everything else as non-retryable. rusqlite errors never leak — they're mapped to backend-neutral `StorageError` variants.
- **Tests**: 12 integration tests covering all 6 acceptance criteria + append_dedup semantics (new key, duplicate, conflict, different targets) + snapshot write/load + invalid LSN rejection + unspecified-kind rejection.
- **Discrepancies from design**: the design's writer-actor command enum was expanded to include `AppendDedup` (added during the port-trait review) and `WriteSnapshot`. The `idempotency_keys` table was added (not in the original schema sketch) to support atomic dedup. These are in-scope extensions of the design.
- **Verification**: `cargo build --workspace` succeeds; `cargo test --package patchbay-core` passes 19/19 tests (7 port smoke + 12 rusqlite integration).

## Review (2026-07-11, fresh-context, openai-codex/gpt-5.6-sol)

**Verdict**: Request changes — bounced for 2 blockers + important findings.

**Blockers (2, fixed)**:
1. **Snapshot errors silently swallowed** — `load_latest_snapshot` used `.ok()` on both branches, collapsing all SQLite failures (including corruption) into `None`. Fixed: now matches `QueryReturnedNoRows` as `Ok(None)` and maps all other errors to `ReadFailed`.
2. **`DefaultHasher` for payload equivalence** — a 64-bit hash can collide, and `DefaultHasher` is not stable across Rust versions. The protocol requires exact payload equivalence. Fixed: replaced the hash with storing the full encoded payload bytes (`payload_bytes` column) and comparing directly — byte-exact equivalence, no collision risk, no hash dependency.

**Important findings (addressed)**:
- Replay kind validation: `read_after` now validates the decoded kind (rejects unspecified/unknown) and checks SQL-column vs envelope-kind agreement (detects corruption).
- u64→i64 bounds checking: added `lsn_to_i64()` helper; all LSN casts now bounds-checked (returns error instead of wrapping).
- Error classification in `do_write_snapshot`: reads inside a write transaction now surface as `WriteFailed` (not `ReadFailed`).
- Added 6 missing tests: rollback-no-gap, cross-domain isolation, empty-log read, no-snapshot load, bounded latest-snapshot selection, concurrent reads+writes. 25/25 tests now pass.

**Important findings (deferred to v0.x — not v0.1.0 blockers)**:
- `tokio::spawn` vs `spawn_blocking`: the writer actor runs on an async worker thread. Under single-writer with no blocking contention, this is fine for v0.1.0. If blocking becomes a problem under load, migrate to `spawn_blocking` or `tokio-rusqlite`.
- Single read connection serializes reads: `Arc<Mutex<Connection>>` allows one read at a time. WAL permits concurrent readers, but a read pool isn't needed for single-operator v0.1.0.
- `open()` panics outside a Tokio runtime: `tokio::spawn` panics if no runtime is entered. Acceptable for v0.1.0 (the core always runs inside a runtime); a `Handle::try_current()` guard is a v0.x hardening.
- Global rowid (not per-domain): interleaving domains produces non-contiguous per-domain LSNs. v0.1.0 has one authority domain, so this doesn't bite; the protocol's per-domain gap-free wording is satisfied by the single-domain constraint. Multi-domain support is a reserved seam.

**Nits (addressed)**: fixed the "length-delimited" comment (prost `encode` is ordinary protobuf encoding); documented the inline writer_actor (story specified a separate file, but inline is cleaner for a single actor).
