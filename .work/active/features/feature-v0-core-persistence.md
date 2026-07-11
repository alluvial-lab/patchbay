---
id: feature-v0-core-persistence
kind: feature
stage: implementing
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Core persistence, event log, and recovery

## Brief

Build the durable event log, storage port, snapshot checkpointing, and crash recovery — the foundation every other core feature writes through. The core owns a single totally-ordered durable event log per authority domain; every accepted state-transition event is assigned a monotonic, gap-free log sequence number (LSN) at durable-commit time. The LSN is the canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

The storage port is the Ports & Adapters boundary: domain semantics read and write through it without depending on the backend choice. The first backend may be embedded (file or embedded database). Snapshots are derived checkpoints used to bound recovery replay cost; they are never an alternate ordering authority. On restart, the core replays the log (or loads the latest snapshot then replays the tail) to reconstruct in-memory state up to the last committed LSN. No accepted command disappears silently after a crash.

This is the root of the core epic — acceptance, authority, and sessions all depend on the event log and storage port. It is the riskiest feature because backend choice affects crash recovery correctness and the qualitative responsiveness floor.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: root. Acceptance, authority, and sessions depend on this feature's storage port and event log. Those three can proceed in parallel once the port interface and event log are designed.

## Formal-model backing

- `BoundaryDedup` (promoted, `command_lifecycle.qnt`) — retrying the same idempotency key cannot double-apply a command at the boundary. The `appliedKeys` set and `lsn` variable live in the event-log/persistence layer.
- Crash/replay/snapshot convergence — stated-normative (v1 formal gate owns the real property). The removed `snapshot_recovery.qnt` draft formulas did not model the claimed failure boundary; the obligation survives as stated-normative.

## Foundation references

- `docs/PROTOCOL.md` — Snapshots and streams; Revisions and cursors; Atomicity between events and snapshots; Persistence and recovery
- `docs/ARCHITECTURE.md` — v0.1.0 persistence topology (single-writer, local-first, port-isolated, log + snapshots, crash recovery)
- `docs/VERIFICATION.md` — property-graded assurance; `BoundaryDedup` promoted property
- `contracts/proto/patchbay/common.proto` — `Lsn`, `ViewRevision`
- `contracts/rust/` — generated Rust bindings (starting contract for types)
- `specs/seed/command_lifecycle.qnt` — `appliedKeys`, `lsn`, `terminalLsn` state
- `specs/seed/snapshot_recovery.qnt` — stated-normative obligations for crash/replay convergence

## Design decisions (feature-design, 2026-07-11)

Resolved interactively with the operator after unpacking each option's trade-offs.

- **Q1 — Storage binding: rusqlite + dedicated writer actor.** A long-lived tokio task owns the single `rusqlite::Connection` and receives append/read commands via an `mpsc` channel. A separate read connection serves snapshot/cursor reads (WAL allows concurrent readers). Chosen over SQLx because the core is single-writer — the async-concurrency benefit doesn't apply, and rusqlite is simpler, has fewer deps, and exposes WAL commit hooks. The research explicitly recommends keeping SQLite on a dedicated writer thread.
- **Q2 — LSN allocation: bare `INTEGER PRIMARY KEY` (no `AUTOINCREMENT`) + `authority_domain_id` column.** The rowid is the LSN; the `(authority_domain_id, LSN)` tuple forms the `EventId` the protocol requires. Chosen over an explicit Patchbay-owned counter after the operator pushed back on the original recommendation. A bare `INTEGER PRIMARY KEY` on an append-only table gives a gap-free, monotonic committed sequence as a standard database guarantee — the gap-on-rollback concern only applies to `AUTOINCREMENT`, which we don't use, and rowid-reuse only matters if the highest row is deleted, which never happens on an append-only log. The research's "make gap-free monotonic LSN an explicit Patchbay invariant" was initially over-read as "own the counter"; it actually means "own the contract" (the tuple shape, acceptance/replay semantics), not "don't trust SQLite to generate a monotonic integer." A proptest asserts the gap-free property empirically against committed LSNs rather than assuming it.
- **Q3 — Snapshot storage: same-DB table.** A `snapshots` table in the same SQLite database; snapshot writes happen in the same transaction as the log prefix they materialize. Chosen over separate files because the protocol's atomicity requirements ("snapshot writes do not reorder the log"; "snapshot reads a consistent log prefix") are trivially satisfied by SQLite transactions, whereas separate files require Patchbay to implement cross-storage crash safety. DB growth is bounded by a pruning policy; negligible at v0.1.0 scale.
- **Q4 — Event log row content: opaque BLOB.** Row shape: `(lsn INTEGER PRIMARY KEY, authority_domain_id TEXT, payload BLOB)`. The payload is the serialized Protobuf message. Chosen over typed columns + BLOB because the safety-critical read paths (crash recovery, cursor reconciliation) are sequential LSN scans that don't need indexing, and typed columns would risk creating a second source of truth for the message shape (violating SSOT and Generated Contracts). If diagnostics grow to need per-command queries, add derived index columns later as a v0.x optimization.
- **Q5 — Workspace layout: new top-level workspace member.** A root `Cargo.toml` workspace with members `contracts/rust` (generated types) and `core/` (application logic). Crate `patchbay-core` depends on `patchbay-contracts`. Chosen over folding the core into `contracts/rust/` because the Generated Contracts principle requires the contracts crate to be purely generated (it has a `build.rs` that regenerates from `.proto`); mixing hand-written logic in would conflate generated and owned code.

## Architectural choice

A storage port (trait) owned by the core domain, with a rusqlite-backed implementation behind it. The port exposes append, read-prefix, snapshot-write, and snapshot-load operations; the rusqlite implementation satisfies them via a single-writer actor and a WAL-mode SQLite database with `synchronous=FULL`. The LSN is the SQLite rowid (bare `INTEGER PRIMARY KEY`); the `(authority_domain_id, LSN)` tuple is the `EventId`. Snapshots live in a `snapshots` table in the same DB, written in the same transaction as their log prefix. Event payloads are opaque serialized Protobuf BLOBs.

This shape honors Ports & Adapters (domain logic depends on the storage trait, not on rusqlite/SQLite), Single Source of Truth (the proto schema is the only source of the message shape; SQLite is a durable append substrate, not the protocol model), Generated Contracts (the contracts crate stays purely generated; the core consumes it), and Fail Fast (the storage port rejects inconsistent snapshots and failed writes rather than exposing them).

Approaches considered:

1. **rusqlite + writer actor + rowid LSN + same-DB snapshots + opaque BLOB + separate workspace crate (chosen).** Optimizes for simplicity, minimal deps, honest formal-model alignment, and clean separation of generated vs. owned code. Sacrifices async-native ergonomics (mitigated by the writer-actor channel handoff being a non-bottleneck under single-writer) and per-command queryability (mitigated by sequential-scan sufficiency for v0.1.0 read paths).
2. **SQLx async + explicit counter + separate snapshot files + typed columns (rejected).** Would add async/pool machinery without benefit at single-writer, move crash-safety burden into project code, and risk schema drift between SQL columns and the proto schema.
3. **A Rust event-sourcing framework like cqrs-es (rejected by research).** Its event envelope uniqueness is aggregate-scoped (`aggregate_type + aggregate_id + sequence`), not a global gap-free authority-domain LSN. Patchbay needs a total authority-domain log order for terminal races, snapshots, cursors, and replay.

## Implementation Units

### Unit 1: Workspace scaffolding and storage port trait

**File**: `Cargo.toml` (root workspace), `core/Cargo.toml`, `core/src/lib.rs`, `core/src/storage/mod.rs`, `core/src/storage/port.rs`

**Story**: `story-v0-core-persistence-workspace-and-port`

```rust
// core/Cargo.toml
[package]
name = "patchbay-core"
version = "0.1.0"
edition = "2021"

[dependencies]
patchbay-contracts = { path = "../contracts/rust" }
tokio = { version = "1", features = ["rt", "sync", "macros"] }
rusqlite = { version = "0.40", features = ["bundled"] }
prost = "0.14"
thiserror = "2"
tracing = "0.1"
```

```rust
// core/src/storage/port.rs
use patchbay_contracts::patchbay::{EventId, Lsn, AuthorityDomainId};

/// A durably-recorded state-transition event in the authority-domain log.
pub struct RecordedEvent {
    pub event_id: EventId,
    pub payload: Vec<u8>,  // serialized Protobuf message
}

/// Errors at the storage boundary. Fail Fast: unknown/invalid input is rejected here.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("durable append failed: {0}")]
    AppendFailed(rusqlite::Error),
    #[error("snapshot LSN {0} is older than current state")]
    SnapshotStale(u64),
    #[error("snapshot from different authority domain")]
    SnapshotWrongDomain,
    #[error("read failed: {0}")]
    ReadFailed(rusqlite::Error),
}

/// The storage port. Domain logic depends on this trait, not on rusqlite.
/// The LSN is assigned at durable-commit time by the implementation.
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Durably append an event. Returns the assigned EventId (LSN assigned at commit).
    /// The append and LSN assignment are atomic — the event is either durably
    /// recorded with a committed LSN, or the call fails and nothing is persisted.
    async fn append(&self, authority_domain_id: &AuthorityDomainId, payload: Vec<u8>)
        -> Result<EventId, StorageError>;

    /// Read events with LSN > cursor, in LSN order. Used for crash recovery
    /// (cursor=0) and cursor reconciliation (cursor=client's last-known LSN).
    async fn read_prefix(&self, authority_domain_id: &AuthorityDomainId, cursor: u64)
        -> Result<Vec<RecordedEvent>, StorageError>;

    /// Write a snapshot materialized at the given LSN. Must be in the same
    /// transaction as the log prefix it reflects (same-DB table).
    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: u64,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError>;

    /// Load the latest snapshot at or before the given LSN (or the latest overall
    /// if None). Returns the snapshot LSN and payload.
    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<u64>,
    ) -> Result<Option<(u64, Vec<u8>)>, StorageError>;
}
```

**Implementation Notes**:
- The port is async because callers (tonic RPC handlers, adapter streams) are async. The rusqlite implementation bridges via a writer actor.
- `RecordedEvent` carries the full `EventId` (authority_domain_id + LSN), not a bare LSN, so the tuple shape is preserved per the protocol's federation seam.
- `StorageError` distinguishes stale-snapshot and wrong-domain rejections (Fail Fast at the boundary) from raw rusqlite failures.

**Acceptance Criteria**:
- [ ] Root `Cargo.toml` workspace compiles with `contracts/rust` and `core/` as members.
- [ ] `patchbay-core` crate depends on `patchbay-contracts`; `cargo build` succeeds.
- [ ] `Storage` trait compiles against the generated `EventId`/`Lsn`/`AuthorityDomainId` types.
- [ ] No hand-written code in `contracts/rust/` (Generated Contracts principle holds).

---

### Unit 2: rusqlite storage implementation + writer actor

**File**: `core/src/storage/rusqlite.rs`, `core/src/storage/writer_actor.rs`

**Story**: `story-v0-core-persistence-rusqlite-impl`

```rust
// core/src/storage/rusqlite.rs
use rusqlite::Connection;

/// SQLite schema. The `lsn` column is a bare INTEGER PRIMARY KEY (the rowid);
/// no AUTOINCREMENT, so rolled-back transactions do not create gaps and the
/// committed sequence is contiguous on this append-only table.
const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS events (
    lsn INTEGER PRIMARY KEY,
    authority_domain_id TEXT NOT NULL,
    payload BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_domain_lsn
    ON events(authority_domain_id, lsn);

CREATE TABLE IF NOT EXISTS snapshots (
    authority_domain_id TEXT NOT NULL,
    snapshot_lsn INTEGER NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (authority_domain_id, snapshot_lsn)
);
"#;

pub struct RusqliteStorage {
    writer_tx: tokio::sync::mpsc::Sender<WriterCommand>,
    read_db: std::sync::Arc<tokio::sync::Mutex<Connection>>,  // read connection (WAL concurrent reader)
}

// The writer actor owns the single write Connection. It receives commands
// via mpsc and executes them on its own task, keeping SQLite calls off the
// async runtime's worker threads.
enum WriterCommand {
    Append {
        authority_domain_id: String,
        payload: Vec<u8>,
        reply: tokio::sync::oneshot::Sender<Result<EventId, StorageError>>,
    },
    WriteSnapshot {
        authority_domain_id: String,
        snapshot_lsn: u64,
        payload: Vec<u8>,
        reply: tokio::sync::oneshot::Sender<Result<(), StorageError>>,
    },
    // ... (read commands route to the read connection, not the writer)
}
```

```rust
// core/src/storage/writer_actor.rs
// The actor loop: receive command -> execute in a transaction -> reply.
// Append: INSERT INTO events (authority_domain_id, payload) VALUES (?, ?);
//   the assigned rowid (last_insert_rowid) is the committed LSN.
// WriteSnapshot: INSERT INTO snapshots ... in the same transaction as
//   the event append that produced snapshot_lsn (caller's responsibility to
//   batch, or a separate transaction if the snapshot is derived post-hoc).
```

**Implementation Notes**:
- `PRAGMA journal_mode = WAL` enables concurrent readers (the read connection) while the writer commits.
- `PRAGMA synchronous = FULL` — the research's explicit recommendation for safety-critical v0 claims. Writers sync the WAL on every commit. This is the durability knob that makes "no accepted command disappears silently" honest.
- The writer actor pattern: the single `Connection` lives on one task; `mpsc` commands serialize appends; `oneshot` replies give async semantics to callers without blocking the runtime. Under single-writer, the channel is never contended.
- `last_insert_rowid()` returns the assigned rowid after the INSERT commits — that's the LSN. Because the table is append-only and uses no `AUTOINCREMENT`, the committed sequence is gap-free.
- Read path: the read connection serves `read_prefix` and `load_latest_snapshot` via `SELECT ... WHERE authority_domain_id = ? AND lsn > ? ORDER BY lsn`.

**Acceptance Criteria**:
- [ ] `RusqliteStorage::append` returns an `EventId` whose LSN equals the rowid SQLite assigned.
- [ ] Consecutive appends produce contiguous LSNs (1, 2, 3, ...).
- [ ] A simulated crash (drop the handle, reopen the DB) recovers all committed events via `read_prefix(0)`.
- [ ] A rolled-back transaction does not create a gap in committed LSNs.
- [ ] `synchronous=FULL` is set and verifiable via `PRAGMA synchronous` query.
- [ ] Read connection serves concurrent reads while a write is in flight (WAL).

---

### Unit 3: Crash recovery and replay

**File**: `core/src/storage/recovery.rs`, `core/src/storage/snapshot.rs`

**Story**: `story-v0-core-persistence-recovery`

```rust
// core/src/storage/recovery.rs
/// On startup, reconstruct in-memory state by loading the latest snapshot
/// (if any) and replaying events with LSN > snapshot_lsn.
pub async fn recover<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<RecoveryState, StorageError> {
    let snapshot = storage.load_latest_snapshot(authority_domain_id, None).await?;
    let (start_lsn, mut state) = match snapshot {
        Some((lsn, payload)) => (lsn, deserialize_state(payload)?),
        None => (0, RecoveryState::empty()),
    };
    let tail = storage.read_prefix(authority_domain_id, start_lsn).await?;
    for event in tail {
        state.apply(event)?;  // idempotent: replaying the same prefix produces identical state
    }
    Ok(state)
}
```

**Implementation Notes**:
- Recovery loads the latest snapshot, then replays the tail (`LSN > snapshot_lsn`). This bounds replay cost.
- `apply` is idempotent — replaying the same committed prefix produces identical state (the `IdempotentLogReplay` stated-normative obligation; tested via proptest).
- The snapshot is a derived checkpoint, never an alternate ordering authority. A snapshot with an LSN less than the current state is rejected as stale.
- `SnapshotConsistentPrefix` (stated-normative): snapshot materialization reads a consistent log prefix — satisfied by writing the snapshot in the same transaction as the event at `snapshot_lsn`.

**Acceptance Criteria**:
- [ ] After a clean shutdown + restart, `recover()` reconstructs state identical to pre-shutdown.
- [ ] After a simulated crash (no clean shutdown), `recover()` reconstructs state up to the last committed LSN; no accepted event is lost.
- [ ] Replay is idempotent: calling `recover()` twice produces identical state.
- [ ] A snapshot at LSN N + replay of events N+1..M produces state identical to replaying from 0.

---

### Unit 4: Property tests

**File**: `core/tests/storage_proptest.rs`

**Story**: `story-v0-core-persistence-proptests`

```rust
// core/tests/storage_proptest.rs
use proptest::prelude::*;

proptest! {
    /// Gap-free: committed LSNs are contiguous (1, 2, 3, ...).
    /// Validates the bare-INTEGER-PRIMARY-KEY guarantee empirically rather
    /// than assuming it. This is the test that makes the rowid-as-LSN choice honest.
    #[test]
    fn committed_lsns_are_gap_free_and_monotonic(events in prop::collection::vec(any_payload(), 1..100)) {
        let storage = setup_temp_storage().await;
        let domain = test_domain();
        let mut lsns = vec![];
        for payload in events {
            let event_id = storage.append(&domain, payload).await.unwrap();
            lsns.push(event_id.lsn.unwrap().value);
        }
        let expected: Vec<u64> = (1..=lsns.len() as u64).collect();
        prop_assert_eq!(lsns, expected);
    }

    /// Idempotent replay: replaying the same committed prefix produces
    /// identical state. (IdempotentLogReplay, stated-normative.)
    #[test]
    fn replay_is_idempotent(events in prop::collection::vec(any_payload(), 1..50)) {
        let storage = setup_temp_storage().await;
        let domain = test_domain();
        for payload in &events {
            storage.append(&domain, payload.clone()).await.unwrap();
        }
        let state1 = recover(&storage, &domain).await.unwrap();
        let state2 = recover(&storage, &domain).await.unwrap();
        prop_assert_eq!(state1, state2);
    }

    /// Crash recovery: all committed events survive a reopen.
    #[test]
    fn crash_recovery_loses_no_committed_event(events in prop::collection::vec(any_payload(), 1..50)) {
        let path = temp_db_path();
        let mut lsns = vec![];
        {
            let storage = open_storage(&path).await;
            let domain = test_domain();
            for payload in &events {
                let id = storage.append(&domain, payload.clone()).await.unwrap();
                lsns.push(id.lsn.unwrap().value);
            }
            // drop storage — simulate crash (no clean shutdown)
        }
        let storage = open_storage(&path).await;  // reopen
        let domain = test_domain();
        let recovered = storage.read_prefix(&domain, 0).await.unwrap();
        let recovered_lsns: Vec<u64> = recovered.iter()
            .map(|e| e.event_id.lsn.as_ref().unwrap().value)
            .collect();
        prop_assert_eq!(recovered_lsns, lsns);
    }
}
```

**Implementation Notes**:
- The gap-free proptest is the empirical validation of the rowid-as-LSN choice — it doesn't assume the guarantee, it tests it.
- `IdempotentLogReplay` and `CrashNoAcceptedLost` are stated-normative obligations; these proptests give them executable evidence even though they don't (yet) have checked formal-model formulas. The v1 formal gate owns the real properties; these tests are the implementation-backed evidence floor.
- `proptest` shrinks failures to minimal reproductions, which is valuable for crash-recovery edge cases.

**Acceptance Criteria**:
- [ ] `committed_lsns_are_gap_free_and_monotonic` passes for 100 generated event sequences.
- [ ] `replay_is_idempotent` passes.
- [ ] `crash_recovery_loses_no_committed_event` passes.
- [ ] All proptests shrink to minimal failures on mutation (verified by injecting a deliberate bug).

---

## Implementation Order

1. `story-v0-core-persistence-workspace-and-port` — workspace scaffolding + `Storage` trait (no deps; everything else builds on this)
2. `story-v0-core-persistence-rusqlite-impl` — rusqlite implementation + writer actor (depends on 1)
3. `story-v0-core-persistence-recovery` — crash recovery + snapshot loading (depends on 2)
4. `story-v0-core-persistence-proptests` — property tests validating gap-free LSN, idempotent replay, crash recovery (depends on 3)

Stories 1-3 are sequential (each depends on the prior). Story 4 depends on 3. The chain is linear because persistence is foundational — there's no parallelism to exploit within this feature; the parallelism is across the sibling core features (acceptance, authority, sessions) once the storage port exists.

## Testing

### Unit Tests: `core/tests/storage_proptest.rs`
- Gap-free LSN property (validates rowid choice empirically)
- Idempotent replay (stated-normative `IdempotentLogReplay`)
- Crash recovery loses no committed event (stated-normative `CrashNoAcceptedLost`)
- Snapshot prefix consistency (stated-normative `SnapshotConsistentPrefix`)
- Stale snapshot rejection (Fail Fast)
- Wrong-domain snapshot rejection (Fail Fast)

### Integration Points
- The `Storage` trait is the seam the sibling core features (acceptance, authority, sessions) consume. Their `feature-design` passes will define ports that call `Storage::append` and `Storage::read_prefix`.
- The writer actor's `mpsc` channel is the async/sync boundary — tested via concurrent append calls from multiple tokio tasks.
- WAL concurrent-reader behavior — tested by reading while a write is in flight.

## Risks

- **`synchronous=FULL` latency.** The research flags this as a revisit trigger: "SQLite with WAL plus `synchronous=FULL` cannot meet v0 latency targets once acceptance, snapshot, and replay tests run against realistic hardware." v0.1.0 has no quantitative performance target (qualitative responsiveness floor only), so this is monitored against feel, not a number. If it feels laggy, the revisit fires — not before.
- **`last_insert_rowid()` correctness under concurrency.** Safe here because the writer actor serializes all writes on one task; there's no concurrent insertion. If the actor model ever changes, this assumption must be re-verified.
- **Snapshot transaction batching.** The design writes snapshots in the same transaction as the event at `snapshot_lsn`. If a snapshot is derived post-hoc (after the events it covers are already committed), it must be written in a transaction that reads a consistent prefix — the implementation must ensure the snapshot LSN reflects a real committed event, not a partial state. The recovery story's design pass should pin this.
- **Proto serialization format.** The payload BLOB is a serialized Protobuf message. The exact encoding (length-delimited vs. raw) and the message-type discriminator (how the reader knows whether a payload is an `Operation`, `Observation`, etc.) is an implementation detail for the rusqlite-impl story — likely a one-byte type tag prefixing the BLOB, or a separate `event_type` column. This is mechanical, not semantic.
