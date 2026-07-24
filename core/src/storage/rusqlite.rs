//! rusqlite-backed `Storage` implementation.
//!
//! SQLite in WAL mode with `synchronous=FULL` is the v0.1.0 durable substrate.
//! The LSN is the bare `INTEGER PRIMARY KEY` (the rowid) — gap-free on this
//! append-only table because no `AUTOINCREMENT` is used and rows are never
//! deleted. See `feature-v0-core-persistence.md` § Design decisions for the
//! rationale and the research backing.
//!
//! # Architecture
//!
//! - A **writer actor** owns the single write `Connection` on a tokio task. It
//!   receives commands via `mpsc` and executes them in transactions,
//!   replying via `oneshot`. This serializes all writes (single-writer) and
//!   gives async semantics to callers. NOTE: the actor runs on a tokio
//!   runtime worker, so synchronous SQLite calls occupy a worker thread
//!   during each transaction. Under single-writer with modest write volume
//!   this is acceptable for v0.1.0; if SQLite latency becomes a bottleneck,
//!   the actor should move to a dedicated blocking thread (`spawn_blocking`).
//! - A **read connection** (behind a `Mutex`) serves `read_after` and
//!   `load_latest_snapshot`. WAL mode allows concurrent readers while the
//!   writer commits.
//! - The `Storage` trait impl bridges async callers to the actor + read conn.

use std::sync::Arc;

#[cfg(unix)]
use std::{
    fs::{OpenOptions, Permissions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, EventId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::port::{
    event_id, DedupOutcome, RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey,
};

/// SQLite schema. The `lsn` column is a bare `INTEGER PRIMARY KEY` (the rowid);
/// no `AUTOINCREMENT`, so rolled-back transactions do not create gaps and the
/// committed sequence is contiguous on this append-only table.
const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS events (
    lsn INTEGER PRIMARY KEY,
    authority_domain_id TEXT NOT NULL,
    kind INTEGER NOT NULL,
    payload BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_domain_lsn
    ON events(authority_domain_id, lsn);

CREATE TABLE IF NOT EXISTS idempotency_keys (
    authority_domain_id TEXT NOT NULL,
    key TEXT NOT NULL,
    target TEXT NOT NULL,
    lsn INTEGER NOT NULL,
    payload_bytes BLOB NOT NULL,
    PRIMARY KEY (authority_domain_id, key, target)
);

CREATE TABLE IF NOT EXISTS snapshots (
    authority_domain_id TEXT NOT NULL,
    snapshot_lsn INTEGER NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (authority_domain_id, snapshot_lsn)
);
"#;

/// Commands sent to the writer actor.
enum WriterCommand {
    Append {
        authority_domain_id: String,
        payload: StoredEventPayload,
        reply: oneshot::Sender<Result<EventId, StorageError>>,
    },
    AppendDedup {
        authority_domain_id: String,
        key: String,
        target: String,
        payload: StoredEventPayload,
        reply: oneshot::Sender<Result<DedupOutcome, StorageError>>,
    },
    WriteSnapshot {
        authority_domain_id: String,
        snapshot_lsn: u64,
        payload: Vec<u8>,
        reply: oneshot::Sender<Result<(), StorageError>>,
    },
}

/// rusqlite-backed storage. Cloneable — the actor handle and read connection
/// are behind `Arc`.
pub struct RusqliteStorage {
    writer_tx: mpsc::Sender<WriterCommand>,
    read_db: Arc<Mutex<Connection>>,
}

impl Clone for RusqliteStorage {
    fn clone(&self) -> Self {
        Self {
            writer_tx: self.writer_tx.clone(),
            read_db: self.read_db.clone(),
        }
    }
}

impl RusqliteStorage {
    /// Open a storage backend at the given path. Creates the schema if absent.
    /// On Unix, creates or tightens the database file to mode `0600` before
    /// SQLite opens it; SQLite derives WAL/SHM sidecar modes from that file.
    /// Spawns the writer actor on the current tokio runtime.
    pub fn open(path: &str) -> Result<Self, StorageError> {
        secure_database_file(path)?;
        let write_db = Connection::open(path).map_err(map_write_err)?;
        write_db
            .execute_batch(SCHEMA)
            .map_err(|e| StorageError::WriteFailed {
                message: e.to_string(),
                retryable: false,
            })?;
        let read_db = Connection::open(path).map_err(map_read_err)?;
        // Apply WAL + synchronous to the read connection too (WAL is persistent
        // on the DB file, but synchronous is per-connection).
        read_db
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL;")
            .map_err(|e| StorageError::ReadFailed {
                message: e.to_string(),
                retryable: false,
            })?;

        let (writer_tx, writer_rx) = mpsc::channel::<WriterCommand>(64);
        let read_db = Arc::new(Mutex::new(read_db));

        tokio::spawn(writer_actor(write_db, writer_rx));

        Ok(Self { writer_tx, read_db })
    }

    /// Open an in-memory storage backend (for tests). Uses a temp file
    /// because WAL mode requires a file-backed database. The temp file is
    /// intentionally retained for the storage's lifetime (via `keep()`);
    /// tests are short-lived and OS cleanup handles eventual removal. This
    /// leaks one file per `open_in_memory()` call — acceptable for the test
    /// suite, not for production paths (which use `open()`).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let temp = tempfile::NamedTempFile::new().map_err(|e| StorageError::WriteFailed {
            message: format!("temp file creation failed: {e}"),
            retryable: false,
        })?;
        let path = temp
            .into_temp_path()
            .keep()
            .map_err(|e| StorageError::WriteFailed {
                message: format!("temp path keep failed: {e}"),
                retryable: false,
            })?;
        let path_str = path.to_str().ok_or_else(|| StorageError::WriteFailed {
            message: "temp path is not valid UTF-8".to_string(),
            retryable: false,
        })?;
        Self::open(path_str)
    }
}

/// The writer actor loop. Owns the single write `Connection`. Receives
/// commands, executes them in transactions, replies via oneshot.
async fn writer_actor(mut db: Connection, mut rx: mpsc::Receiver<WriterCommand>) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            WriterCommand::Append {
                authority_domain_id,
                payload,
                reply,
            } => {
                let result = do_append(&mut db, &authority_domain_id, &payload);
                let _ = reply.send(result);
            }
            WriterCommand::AppendDedup {
                authority_domain_id,
                key,
                target,
                payload,
                reply,
            } => {
                let result =
                    do_append_dedup(&mut db, &authority_domain_id, &key, &target, &payload);
                let _ = reply.send(result);
            }
            WriterCommand::WriteSnapshot {
                authority_domain_id,
                snapshot_lsn,
                payload,
                reply,
            } => {
                let result =
                    do_write_snapshot(&mut db, &authority_domain_id, snapshot_lsn, &payload);
                let _ = reply.send(result);
            }
        }
    }
}

/// Convert a u64 LSN to i64 for SQLite storage. Returns an error if the
/// value exceeds i64::MAX (Fail Fast — LSNs should never reach this in practice).
/// Surfaces as `WriteFailed` since this is a write-path precondition.
fn lsn_to_i64(lsn: u64) -> Result<i64, StorageError> {
    lsn.try_into().map_err(|_| StorageError::WriteFailed {
        message: format!("LSN {lsn} exceeds i64::MAX"),
        retryable: false,
    })
}

/// Validate the event kind is not unspecified. `try_from` succeeds for
/// `Unspecified` (it's a valid enum value), so we explicitly reject it.
fn validate_kind(payload: &StoredEventPayload) -> Result<StoredEventKind, StorageError> {
    let kind =
        StoredEventKind::try_from(payload.kind).map_err(|_| StorageError::InvalidEventKind)?;
    if kind == StoredEventKind::Unspecified {
        return Err(StorageError::InvalidEventKind);
    }
    Ok(kind)
}

/// Serialize a StoredEventPayload for storage. Uses prost protobuf encoding
/// (not length-delimited — ordinary `Message::encode`).
fn encode_payload(payload: &StoredEventPayload) -> Result<Vec<u8>, StorageError> {
    let mut buf = Vec::with_capacity(payload.payload.len() + 8);
    prost::Message::encode(payload, &mut buf)
        .map_err(|e| StorageError::CorruptRecord(format!("encode failed: {e}")))?;
    Ok(buf)
}

/// Deserialize a StoredEventPayload from storage.
fn decode_payload(bytes: &[u8]) -> Result<StoredEventPayload, StorageError> {
    prost::Message::decode(bytes)
        .map_err(|e| StorageError::CorruptRecord(format!("decode failed: {e}")))
}

/// The encoded payload bytes for idempotency conflict detection.
///
/// The protocol requires exact payload equivalence for dedup
/// (`docs/PROTOCOL.md` § "Idempotency and retry": "A retry must carry the
/// same payload as the original"). We store the full encoded bytes and
/// compare directly — no hash, so no collision risk. This is byte-exact
/// equivalence, which is what the protocol demands.
fn payload_canonical(payload: &StoredEventPayload) -> Result<Vec<u8>, StorageError> {
    encode_payload(payload)
}

fn do_append(
    db: &mut Connection,
    authority_domain_id: &str,
    payload: &StoredEventPayload,
) -> Result<EventId, StorageError> {
    let kind = validate_kind(payload)?;
    let encoded = encode_payload(payload)?;
    let tx = db.transaction().map_err(map_write_err)?;
    tx.execute(
        "INSERT INTO events (authority_domain_id, kind, payload) VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, kind as i32, encoded],
    )
    .map_err(map_write_err)?;
    let lsn = tx.last_insert_rowid();
    tx.commit().map_err(map_write_err)?;
    Ok(event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_string(),
        },
        lsn as u64,
    ))
}

fn do_append_dedup(
    db: &mut Connection,
    authority_domain_id: &str,
    key: &str,
    target: &str,
    payload: &StoredEventPayload,
) -> Result<DedupOutcome, StorageError> {
    let kind = validate_kind(payload)?;
    let encoded = encode_payload(payload)?;
    let canonical = payload_canonical(payload)?;
    let tx = db.transaction().map_err(map_write_err)?;

    // Check if the key already exists for this target. query_row returns
    // Err(QueryReturnedNoRows) when absent — that's the new-key path.
    let existing: Option<(i64, Vec<u8>)> = match tx.query_row(
        "SELECT lsn, payload_bytes FROM idempotency_keys
         WHERE authority_domain_id = ?1 AND key = ?2 AND target = ?3",
        rusqlite::params![authority_domain_id, key, target],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(map_write_err(e)),
    };

    match existing {
        Some((existing_lsn, existing_bytes)) => {
            if existing_bytes != canonical {
                tx.rollback().map_err(map_write_err)?;
                return Err(StorageError::IdempotencyConflict);
            }
            // Duplicate — return the existing event id, no new append.
            tx.rollback().map_err(map_write_err)?;
            Ok(DedupOutcome::Duplicate(event_id(
                AuthorityDomainId {
                    value: authority_domain_id.to_string(),
                },
                existing_lsn as u64,
            )))
        }
        None => {
            // New key — append the event and register the key in one transaction.
            tx.execute(
                "INSERT INTO events (authority_domain_id, kind, payload) VALUES (?1, ?2, ?3)",
                rusqlite::params![authority_domain_id, kind as i32, encoded],
            )
            .map_err(map_write_err)?;
            let lsn = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO idempotency_keys (authority_domain_id, key, target, lsn, payload_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![authority_domain_id, key, target, lsn, canonical],
            )
            .map_err(map_write_err)?;
            tx.commit().map_err(map_write_err)?;
            Ok(DedupOutcome::Appended(event_id(
                AuthorityDomainId {
                    value: authority_domain_id.to_string(),
                },
                lsn as u64,
            )))
        }
    }
}

fn do_write_snapshot(
    db: &mut Connection,
    authority_domain_id: &str,
    snapshot_lsn: u64,
    payload: &[u8],
) -> Result<(), StorageError> {
    let snapshot_lsn_i64 = lsn_to_i64(snapshot_lsn)?;
    let tx = db.transaction().map_err(map_write_err)?;
    // Validate the snapshot LSN corresponds to a committed event.
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM events WHERE authority_domain_id = ?1 AND lsn = ?2)",
            rusqlite::params![authority_domain_id, snapshot_lsn_i64],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    if !exists {
        tx.rollback().map_err(map_write_err)?;
        return Err(StorageError::InvalidSnapshotLsn(snapshot_lsn));
    }
    tx.execute(
        "INSERT OR REPLACE INTO snapshots (authority_domain_id, snapshot_lsn, payload)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, snapshot_lsn_i64, payload],
    )
    .map_err(map_write_err)?;
    tx.commit().map_err(map_write_err)?;
    Ok(())
}

impl Storage for RusqliteStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::Append {
                authority_domain_id: authority_domain_id.value.clone(),
                payload,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_string()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_string()))?
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedup {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_string(),
                payload,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_string()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_string()))?
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        let cursor_i64 = lsn_to_i64(cursor.value)?;
        let db = self.read_db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT lsn, kind, payload FROM events
                 WHERE authority_domain_id = ?1 AND lsn > ?2
                 ORDER BY lsn",
            )
            .map_err(map_read_err)?;
        let rows = stmt
            .query_map(
                rusqlite::params![authority_domain_id.value, cursor_i64],
                |row| {
                    let lsn: i64 = row.get(0)?;
                    let sql_kind: i32 = row.get(1)?;
                    let payload_bytes: Vec<u8> = row.get(2)?;
                    Ok((lsn, sql_kind, payload_bytes))
                },
            )
            .map_err(map_read_err)?;
        let mut events = Vec::new();
        for row in rows {
            let (lsn, sql_kind, payload_bytes) = row.map_err(map_read_err)?;
            let payload = decode_payload(&payload_bytes)?;
            // Validate the decoded kind and check SQL/envelope agreement.
            let decoded_kind = validate_kind(&payload)?;
            if decoded_kind as i32 != sql_kind {
                return Err(StorageError::CorruptRecord(format!(
                    "kind mismatch at LSN {lsn}: SQL column says {sql_kind}, envelope says {}",
                    decoded_kind as i32
                )));
            }
            events.push(RecordedEvent {
                event_id: event_id(
                    AuthorityDomainId {
                        value: authority_domain_id.value.clone(),
                    },
                    lsn as u64,
                ),
                payload,
            });
        }
        Ok(events)
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::WriteSnapshot {
                authority_domain_id: authority_domain_id.value.clone(),
                snapshot_lsn: snapshot_lsn.value,
                payload: snapshot_payload,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_string()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_string()))?
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        let db = self.read_db.lock().await;
        let row_result = match at_or_before {
            Some(lsn) => {
                let lsn_i64 = lsn_to_i64(lsn.value)?;
                db.query_row(
                    "SELECT snapshot_lsn, payload FROM snapshots
                     WHERE authority_domain_id = ?1 AND snapshot_lsn <= ?2
                     ORDER BY snapshot_lsn DESC LIMIT 1",
                    rusqlite::params![authority_domain_id.value, lsn_i64],
                    |row| {
                        Ok(StoredSnapshot {
                            event_id: event_id(
                                AuthorityDomainId {
                                    value: authority_domain_id.value.clone(),
                                },
                                row.get::<_, i64>(0)? as u64,
                            ),
                            payload: row.get(1)?,
                        })
                    },
                )
            }
            None => db.query_row(
                "SELECT snapshot_lsn, payload FROM snapshots
                 WHERE authority_domain_id = ?1
                 ORDER BY snapshot_lsn DESC LIMIT 1",
                rusqlite::params![authority_domain_id.value],
                |row| {
                    Ok(StoredSnapshot {
                        event_id: event_id(
                            AuthorityDomainId {
                                value: authority_domain_id.value.clone(),
                            },
                            row.get::<_, i64>(0)? as u64,
                        ),
                        payload: row.get(1)?,
                    })
                },
            ),
        };
        match row_result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(map_read_err(e)),
        }
    }
}

#[cfg(unix)]
fn secure_database_file(path: &str) -> Result<(), StorageError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|error| state_file_error(path, error))?;
    file.set_permissions(Permissions::from_mode(0o600))
        .map_err(|error| state_file_error(path, error))
}

#[cfg(not(unix))]
fn secure_database_file(_path: &str) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(unix)]
fn state_file_error(path: &str, error: std::io::Error) -> StorageError {
    StorageError::WriteFailed {
        message: format!("cannot secure SQLite state file {path}: {error}"),
        retryable: false,
    }
}

fn map_write_err(e: rusqlite::Error) -> StorageError {
    let retryable = matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::DatabaseBusy
                    | rusqlite::ffi::ErrorCode::DatabaseLocked,
                ..
            },
            _
        )
    );
    StorageError::WriteFailed {
        message: e.to_string(),
        retryable,
    }
}

fn map_read_err(e: rusqlite::Error) -> StorageError {
    let retryable = matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::DatabaseBusy
                    | rusqlite::ffi::ErrorCode::DatabaseLocked,
                ..
            },
            _
        )
    );
    StorageError::ReadFailed {
        message: e.to_string(),
        retryable,
    }
}
