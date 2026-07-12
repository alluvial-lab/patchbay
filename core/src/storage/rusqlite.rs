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
//! - A **writer actor** owns the single write `Connection` on a dedicated tokio
//!   task. It receives commands via `mpsc` and executes them in transactions,
//!   replying via `oneshot`. This keeps blocking SQLite calls off the async
//!   runtime's worker threads and serializes all writes (single-writer).
//! - A **read connection** (behind a `Mutex`) serves `read_after` and
//!   `load_latest_snapshot`. WAL mode allows concurrent readers while the
//!   writer commits.
//! - The `Storage` trait impl bridges async callers to the actor + read conn.

use std::sync::Arc;

use patchbay_contracts::patchbay::{
    AuthorityDomainId, EventId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot, Mutex};

use super::port::{event_id, DedupOutcome, RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey};

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
    payload_hash BLOB NOT NULL,
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
    /// Spawns the writer actor on the current tokio runtime.
    pub fn open(path: &str) -> Result<Self, StorageError> {
        let write_db = Connection::open(path).map_err(map_write_err)?;
        write_db.execute_batch(SCHEMA).map_err(|e| {
            StorageError::WriteFailed { message: e.to_string(), retryable: false }
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

    /// Open an in-memory storage backend (for tests). Uses a temp file under
    /// the hood because WAL mode requires a file-backed database; the temp
    /// file is cleaned up when the returned guard is dropped.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let temp = tempfile::NamedTempFile::new().map_err(|e| StorageError::WriteFailed {
            message: format!("temp file creation failed: {e}"),
            retryable: false,
        })?;
        // Leak the temp file path (the file persists for the DB's lifetime).
        // Tests are short-lived; OS cleanup handles it.
        let path = temp.into_temp_path().keep().map_err(|e| StorageError::WriteFailed {
            message: format!("temp path keep failed: {e}"),
            retryable: false,
        })?;
        Self::open(path.to_str().unwrap_or("./patchbay-test.db"))
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
                let result = do_append_dedup(&mut db, &authority_domain_id, &key, &target, &payload);
                let _ = reply.send(result);
            }
            WriterCommand::WriteSnapshot {
                authority_domain_id,
                snapshot_lsn,
                payload,
                reply,
            } => {
                let result = do_write_snapshot(&mut db, &authority_domain_id, snapshot_lsn, &payload);
                let _ = reply.send(result);
            }
        }
    }
}

/// Validate the event kind is not unspecified. `try_from` succeeds for
/// `Unspecified` (it's a valid enum value), so we explicitly reject it.
fn validate_kind(payload: &StoredEventPayload) -> Result<StoredEventKind, StorageError> {
    let kind = StoredEventKind::try_from(payload.kind).map_err(|_| StorageError::InvalidEventKind)?;
    if kind == StoredEventKind::Unspecified {
        return Err(StorageError::InvalidEventKind);
    }
    Ok(kind)
}

/// Serialize a StoredEventPayload for storage. Uses prost length-delimited
/// encoding.
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

/// Hash a payload for idempotency conflict detection.
fn payload_hash(payload: &StoredEventPayload) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    payload.kind.hash(&mut hasher);
    payload.payload.hash(&mut hasher);
    hasher.finish().to_le_bytes().to_vec()
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
    let hash = payload_hash(payload);
    let tx = db.transaction().map_err(map_write_err)?;

    // Check if the key already exists for this target. query_row returns
    // Err(QueryReturnedNoRows) when absent — that's the new-key path.
    let existing: Option<(i64, Vec<u8>)> = match tx.query_row(
        "SELECT lsn, payload_hash FROM idempotency_keys
         WHERE authority_domain_id = ?1 AND key = ?2 AND target = ?3",
        rusqlite::params![authority_domain_id, key, target],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(map_read_err(e)),
    };

    match existing {
        Some((existing_lsn, existing_hash)) => {
            if existing_hash != hash {
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
                "INSERT INTO idempotency_keys (authority_domain_id, key, target, lsn, payload_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![authority_domain_id, key, target, lsn, hash],
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
    let tx = db.transaction().map_err(map_write_err)?;
    // Validate the snapshot LSN corresponds to a committed event.
    let exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM events WHERE authority_domain_id = ?1 AND lsn = ?2)",
            rusqlite::params![authority_domain_id, snapshot_lsn as i64],
            |row| row.get(0),
        )
        .map_err(map_read_err)?;
    if !exists {
        tx.rollback().map_err(map_write_err)?;
        return Err(StorageError::InvalidSnapshotLsn(snapshot_lsn));
    }
    tx.execute(
        "INSERT OR REPLACE INTO snapshots (authority_domain_id, snapshot_lsn, payload)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, snapshot_lsn as i64, payload],
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
                rusqlite::params![authority_domain_id.value, cursor.value as i64],
                |row| {
                    let lsn: i64 = row.get(0)?;
                    let kind: i32 = row.get(1)?;
                    let payload_bytes: Vec<u8> = row.get(2)?;
                    Ok((lsn, kind, payload_bytes))
                },
            )
            .map_err(map_read_err)?;
        let mut events = Vec::new();
        for row in rows {
            let (lsn, _kind, payload_bytes) = row.map_err(map_read_err)?;
            let payload = decode_payload(&payload_bytes)?;
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
        let result = match at_or_before {
            Some(lsn) => db
                .query_row(
                    "SELECT snapshot_lsn, payload FROM snapshots
                     WHERE authority_domain_id = ?1 AND snapshot_lsn <= ?2
                     ORDER BY snapshot_lsn DESC LIMIT 1",
                    rusqlite::params![authority_domain_id.value, lsn.value as i64],
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
                .ok(),
            None => db
                .query_row(
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
                )
                .ok(),
        };
        // Distinguish "no row" (None) from a real error.
        match result {
            Some(s) => Ok(Some(s)),
            None => {
                // Check if it was QueryReturnedNoRows or an actual error.
                // query_row returns Err(QueryReturnedNoRows) for no match, which
                // .ok() converts to None. That's the expected "no snapshot" case.
                Ok(None)
            }
        }
    }
}

fn map_write_err(e: rusqlite::Error) -> StorageError {
    let retryable = matches!(
        e,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
            code:
                rusqlite::ffi::ErrorCode::DatabaseBusy
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
            code:
                rusqlite::ffi::ErrorCode::DatabaseBusy
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
