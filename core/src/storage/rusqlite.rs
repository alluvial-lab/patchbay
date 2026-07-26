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
    AuditEventKind, AuditPage, AuditRecord, AuthorityDomainId, EventId, FailureCode,
    IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use prost::Message;
use rusqlite::{params_from_iter, types::Value, Connection};
use tokio::sync::{mpsc, oneshot, Mutex};

use super::port::{
    event_id, AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedBatchAppend,
    AuditedDedupOutcome, DedupOutcome,
    RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey,
};

pub const LATEST_SCHEMA_VERSION: u32 = 2;

const MIGRATION_1: &str = r#"
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

const MIGRATION_2: &str = r#"
CREATE TABLE IF NOT EXISTS audit_records (
    authority_domain_id TEXT NOT NULL,
    audit_lsn INTEGER NOT NULL,
    occurred_at_seconds INTEGER NOT NULL,
    occurred_at_nanos INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    actor_id TEXT,
    endpoint_id TEXT,
    command_id TEXT,
    target_key TEXT,
    failure_code INTEGER,
    reason_code TEXT NOT NULL,
    source_lsn INTEGER,
    PRIMARY KEY (authority_domain_id, audit_lsn),
    FOREIGN KEY (audit_lsn) REFERENCES events(lsn),
    FOREIGN KEY (source_lsn) REFERENCES events(lsn)
);
CREATE INDEX IF NOT EXISTS idx_audit_domain_lsn ON audit_records(authority_domain_id, audit_lsn DESC);
CREATE INDEX IF NOT EXISTS idx_audit_occurred_at ON audit_records(authority_domain_id, occurred_at_seconds, occurred_at_nanos);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_records(authority_domain_id, actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_command ON audit_records(authority_domain_id, command_id);
CREATE INDEX IF NOT EXISTS idx_audit_target ON audit_records(authority_domain_id, target_key);
CREATE INDEX IF NOT EXISTS idx_audit_kind ON audit_records(authority_domain_id, kind);
"#;

fn table_exists(db: &Connection, name: &str) -> Result<bool, StorageError> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [name],
        |row| row.get(0),
    )
    .map_err(map_write_err)
}

fn validate_columns(db: &Connection, table: &str, required: &[&str]) -> Result<(), StorageError> {
    let mut statement = db
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(map_write_err)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(map_write_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_write_err)?;
    for required_column in required {
        if !columns.iter().any(|column| column == required_column) {
            return Err(StorageError::MalformedSchema(format!(
                "table {table} is missing required column {required_column}"
            )));
        }
    }
    Ok(())
}

fn migrate(db: &mut Connection) -> Result<(), StorageError> {
    // Complete every schema check before changing a persistent pragma or
    // user_version. A malformed legacy database must remain byte-for-byte
    // untouched so an operator can repair or inspect it safely.
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version > LATEST_SCHEMA_VERSION {
        return Err(StorageError::UnsupportedSchemaVersion(version));
    }
    let base_tables = ["events", "idempotency_keys", "snapshots"];
    let any_base = base_tables
        .iter()
        .map(|name| table_exists(db, name))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|exists| exists);
    if (version == 0 && any_base) || version >= 1 {
        validate_columns(db, "events", &["lsn", "authority_domain_id", "kind", "payload"])?;
        validate_columns(db, "idempotency_keys", &["authority_domain_id", "key", "target", "lsn", "payload_bytes"])?;
        validate_columns(db, "snapshots", &["authority_domain_id", "snapshot_lsn", "payload"])?;
    }
    let audit_exists = table_exists(db, "audit_records")?;
    if version >= 2 || audit_exists {
        validate_columns(
            db,
            "audit_records",
            &[
                "authority_domain_id", "audit_lsn", "occurred_at_seconds",
                "occurred_at_nanos", "kind", "actor_id", "endpoint_id",
                "command_id", "target_key", "failure_code", "reason_code",
                "source_lsn",
            ],
        )?;
    }

    db.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")
        .map_err(map_write_err)?;
    if version == 0 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_1).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 1").map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version < 2 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_2).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 2").map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    Ok(())
}

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
    AppendAudit {
        authority_domain_id: String,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<EventId, StorageError>>,
    },
    AppendAudited {
        authority_domain_id: String,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<AuditedAppend, StorageError>>,
    },
    AppendDedupAudited {
        authority_domain_id: String,
        key: String,
        target: String,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<AuditedDedupOutcome, StorageError>>,
    },
    AppendBatchAudited {
        authority_domain_id: String,
        sources: Vec<StoredEventPayload>,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<AuditedBatchAppend, StorageError>>,
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
        let mut write_db = Connection::open(path).map_err(map_write_err)?;
        migrate(&mut write_db)?;
        let read_db = Connection::open(path).map_err(map_read_err)?;
        // Apply WAL + synchronous to the read connection too (WAL is persistent
        // on the DB file, but synchronous is per-connection).
        read_db
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;")
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
            WriterCommand::AppendAudit {
                authority_domain_id,
                audit,
                reply,
            } => {
                let result = do_append_audit(&mut db, &authority_domain_id, audit);
                let _ = reply.send(result);
            }
            WriterCommand::AppendAudited {
                authority_domain_id,
                source,
                audit,
                reply,
            } => {
                let result = do_append_audited(&mut db, &authority_domain_id, source, audit);
                let _ = reply.send(result);
            }
            WriterCommand::AppendDedupAudited {
                authority_domain_id,
                key,
                target,
                source,
                audit,
                reply,
            } => {
                let result = do_append_dedup_audited(
                    &mut db,
                    &authority_domain_id,
                    &key,
                    &target,
                    source,
                    audit,
                );
                let _ = reply.send(result);
            }
            WriterCommand::AppendBatchAudited {
                authority_domain_id,
                sources,
                audit,
                reply,
            } => {
                let result = do_append_batch_audited(&mut db, &authority_domain_id, sources, audit);
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

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

fn target_key_for_scope(scope: Option<&patchbay_contracts::patchbay::TargetScope>) -> Option<String> {
    scope.map(|scope| encode_hex(&scope.encode_to_vec()))
}

fn insert_event(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
    payload: &StoredEventPayload,
) -> Result<(i64, Vec<u8>), StorageError> {
    let kind = validate_kind(payload)?;
    let encoded = encode_payload(payload)?;
    tx.execute(
        "INSERT INTO events (authority_domain_id, kind, payload) VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, kind as i32, encoded],
    )
    .map_err(map_write_err)?;
    Ok((tx.last_insert_rowid(), encoded))
}

fn audit_record_from_draft(
    authority_domain_id: &str,
    audit_lsn: i64,
    draft: AuditRecordDraft,
) -> Result<AuditRecord, StorageError> {
    let domain = AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    };
    draft.validate(&domain)?;
    let audit_event_id = event_id(domain, audit_lsn as u64);
    Ok(AuditRecord {
        audit_event_id: Some(audit_event_id),
        occurred_at: Some(draft.occurred_at),
        kind: draft.kind as i32,
        actor_id: draft.actor_id,
        device_id: draft.device_id,
        endpoint_id: draft.endpoint_id,
        operator_session_hash: draft.operator_session_hash,
        command_id: draft.command_id,
        target_scope: draft.target_scope,
        failure_code: draft.failure_code.map(|code| code as i32).unwrap_or(FailureCode::Unspecified as i32),
        reason_code: draft.reason_code,
        correlation_id: draft.correlation_id,
        source_event_id: draft.source_event_id,
        source_network: draft.source_network,
        adapter_diagnostic: draft.adapter_diagnostic,
    })
}

fn insert_audit_index(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
    record: &AuditRecord,
) -> Result<(), StorageError> {
    let audit_lsn = record
        .audit_event_id
        .as_ref()
        .and_then(|id| id.lsn.as_ref())
        .ok_or_else(|| StorageError::InvalidAuditRecord("audit record has no event id".to_owned()))?
        .value;
    let audit_lsn = lsn_to_i64(audit_lsn)?;
    let source_lsn = record
        .source_event_id
        .as_ref()
        .and_then(|id| id.lsn.as_ref())
        .map(|lsn| lsn_to_i64(lsn.value))
        .transpose()?;
    let target_key = target_key_for_scope(record.target_scope.as_ref());
    let actor_id = record.actor_id.as_ref().map(|id| id.value.as_str());
    let endpoint_id = record.endpoint_id.as_ref().map(|id| id.value.as_str());
    let command_id = record.command_id.as_ref().map(|id| id.value.as_str());
    tx.execute(
        "INSERT INTO audit_records (
            authority_domain_id, audit_lsn, occurred_at_seconds, occurred_at_nanos,
            kind, actor_id, endpoint_id, command_id, target_key, failure_code,
            reason_code, source_lsn
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            authority_domain_id,
            audit_lsn,
            record.occurred_at.as_ref().map(|time| time.seconds).ok_or_else(|| StorageError::InvalidAuditRecord("audit record has no timestamp".to_owned()))?,
            record.occurred_at.as_ref().map(|time| time.nanos).ok_or_else(|| StorageError::InvalidAuditRecord("audit record has no timestamp".to_owned()))?,
            record.kind,
            actor_id,
            endpoint_id,
            command_id,
            target_key,
            record.failure_code,
            record.reason_code,
            source_lsn,
        ],
    )
    .map_err(map_write_err)?;
    Ok(())
}

fn append_audit_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
    audit: AuditRecordDraft,
) -> Result<EventId, StorageError> {
    let audit_lsn = tx
        .query_row("SELECT COALESCE(MAX(lsn), 0) + 1 FROM events", [], |row| row.get::<_, i64>(0))
        .map_err(map_write_err)?;
    let record = audit_record_from_draft(authority_domain_id, audit_lsn, audit)?;
    let payload = StoredEventPayload {
        kind: StoredEventKind::AuditRecord as i32,
        payload: record.encode_to_vec(),
    };
    let (actual_lsn, _) = insert_event(tx, authority_domain_id, &payload)?;
    if actual_lsn != audit_lsn {
        return Err(StorageError::CorruptRecord(format!(
            "SQLite assigned audit LSN {actual_lsn}, expected {audit_lsn}"
        )));
    }
    insert_audit_index(tx, authority_domain_id, &record)?;
    Ok(event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        actual_lsn as u64,
    ))
}

fn do_append_audit(
    db: &mut Connection,
    authority_domain_id: &str,
    audit: AuditRecordDraft,
) -> Result<EventId, StorageError> {
    let tx = db.transaction().map_err(map_write_err)?;
    let result = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(result)
}

fn do_append_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    source: StoredEventPayload,
    mut audit: AuditRecordDraft,
) -> Result<AuditedAppend, StorageError> {
    validate_kind(&source)?;
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId { value: authority_domain_id.to_owned() })?;
    let tx = db.transaction().map_err(map_write_err)?;
    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    let source_event_id = event_id(
        AuthorityDomainId { value: authority_domain_id.to_owned() },
        source_lsn as u64,
    );
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedAppend { source_event_id, audit_event_id })
}

fn do_append_batch_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    sources: Vec<StoredEventPayload>,
    mut audit: AuditRecordDraft,
) -> Result<AuditedBatchAppend, StorageError> {
    if sources.is_empty() {
        return Err(StorageError::InvalidAuditRecord(
            "audited batch must contain at least one source event".to_owned(),
        ));
    }
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId { value: authority_domain_id.to_owned() })?;
    let tx = db.transaction().map_err(map_write_err)?;
    let mut source_event_ids = Vec::with_capacity(sources.len());
    for source in sources {
        let (lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
        source_event_ids.push(event_id(
            AuthorityDomainId { value: authority_domain_id.to_owned() },
            lsn as u64,
        ));
    }
    audit.source_event_id = source_event_ids.last().cloned();
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedBatchAppend { source_event_ids, audit_event_id })
}

fn do_append_dedup_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    key: &str,
    target: &str,
    source: StoredEventPayload,
    mut audit: AuditRecordDraft,
) -> Result<AuditedDedupOutcome, StorageError> {
    validate_kind(&source)?;
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId { value: authority_domain_id.to_owned() })?;
    let canonical = payload_canonical(&source)?;
    let tx = db.transaction().map_err(map_write_err)?;
    let existing: Option<(i64, Vec<u8>)> = match tx.query_row(
        "SELECT lsn, payload_bytes FROM idempotency_keys WHERE authority_domain_id = ?1 AND key = ?2 AND target = ?3",
        rusqlite::params![authority_domain_id, key, target],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(error) => return Err(map_write_err(error)),
    };
    let source_lsn = match existing {
        Some((lsn, bytes)) => {
            if bytes != canonical {
                return Err(StorageError::IdempotencyConflict);
            }
            lsn
        }
        None => {
            let (lsn, encoded) = insert_event(&tx, authority_domain_id, &source)?;
            tx.execute(
                "INSERT INTO idempotency_keys (authority_domain_id, key, target, lsn, payload_bytes) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![authority_domain_id, key, target, lsn, canonical],
            ).map_err(map_write_err)?;
            let _ = encoded;
            let source_event_id = event_id(AuthorityDomainId { value: authority_domain_id.to_owned() }, lsn as u64);
            audit.source_event_id = Some(source_event_id);
            let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
            tx.commit().map_err(map_write_err)?;
            return Ok(AuditedDedupOutcome::Appended(AuditedAppend { source_event_id: event_id(AuthorityDomainId { value: authority_domain_id.to_owned() }, lsn as u64), audit_event_id }));
        }
    };
    let source_event_id = event_id(AuthorityDomainId { value: authority_domain_id.to_owned() }, source_lsn as u64);
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedDedupOutcome::Duplicate { source_event_id, audit_event_id })
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

fn validate_audit_spec(spec: &AuditPageSpec) -> Result<(), StorageError> {
    if !(1..=500).contains(&spec.limit) {
        return Err(StorageError::InvalidAuditRecord(
            "audit page limit must be between 1 and 500".to_owned(),
        ));
    }
    for kind in &spec.kinds {
        if *kind == AuditEventKind::Unspecified || AuditEventKind::try_from(*kind as i32).is_err() {
            return Err(StorageError::InvalidAuditRecord("audit filter contains an unknown kind".to_owned()));
        }
    }
    for code in &spec.failure_codes {
        if *code == FailureCode::Unspecified || FailureCode::try_from(*code as i32).is_err() {
            return Err(StorageError::InvalidAuditRecord("audit filter contains an unknown failure code".to_owned()));
        }
    }
    for reason in &spec.reason_codes {
        super::port::validate_bounded_code_for_query(reason)?;
    }
    if let Some(timestamp) = &spec.occurred_from {
        super::port::validate_timestamp_for_query(timestamp)?;
    }
    if let Some(timestamp) = &spec.occurred_before {
        super::port::validate_timestamp_for_query(timestamp)?;
    }
    if let (Some(from), Some(before)) = (&spec.occurred_from, &spec.occurred_before) {
        if (from.seconds, from.nanos) >= (before.seconds, before.nanos) {
            return Err(StorageError::InvalidAuditRecord("audit time interval is empty or reversed".to_owned()));
        }
    }
    Ok(())
}

fn query_audit_sync(
    db: &Connection,
    authority_domain_id: &AuthorityDomainId,
    spec: AuditPageSpec,
    as_of_lsn: Option<u64>,
) -> Result<AuditPage, StorageError> {
    validate_audit_spec(&spec)?;
    let max_lsn: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(lsn), 0) FROM events WHERE authority_domain_id = ?1",
            [&authority_domain_id.value],
            |row| row.get(0),
        )
        .map_err(map_read_err)?;
    if let Some(as_of_lsn) = as_of_lsn {
        let as_of_lsn = lsn_to_i64(as_of_lsn)?;
        if as_of_lsn > max_lsn {
            return Err(StorageError::InvalidAuditCursor(format!("prefix LSN {as_of_lsn} is beyond current LSN {max_lsn}")));
        }
    }
    if let Some(before_lsn) = spec.before_lsn {
        let before_lsn = lsn_to_i64(before_lsn).map_err(|_| StorageError::InvalidAuditCursor("cursor exceeds SQLite range".to_owned()))?;
        if before_lsn > max_lsn {
            return Err(StorageError::InvalidAuditCursor(format!("cursor {before_lsn} is beyond current LSN {max_lsn}")));
        }
    }

    let mut clauses = vec!["a.authority_domain_id = ?".to_owned()];
    let mut values = vec![Value::Text(authority_domain_id.value.clone())];
    if let Some(as_of_lsn) = as_of_lsn {
        clauses.push("a.audit_lsn <= ?".to_owned());
        values.push(Value::Integer(lsn_to_i64(as_of_lsn)?));
    }
    if let Some(before_lsn) = spec.before_lsn {
        clauses.push("a.audit_lsn < ?".to_owned());
        values.push(Value::Integer(lsn_to_i64(before_lsn)?));
    }
    if !spec.kinds.is_empty() {
        clauses.push(format!("a.kind IN ({})", vec!["?"; spec.kinds.len()].join(",")));
        values.extend(spec.kinds.iter().map(|kind| Value::Integer(*kind as i64)));
    }
    if let Some(actor_id) = spec.actor_id {
        clauses.push("a.actor_id = ?".to_owned());
        values.push(Value::Text(actor_id.value));
    }
    if let Some(endpoint_id) = spec.endpoint_id {
        clauses.push("a.endpoint_id = ?".to_owned());
        values.push(Value::Text(endpoint_id.value));
    }
    if let Some(command_id) = spec.command_id {
        clauses.push("a.command_id = ?".to_owned());
        values.push(Value::Text(command_id.value));
    }
    if let Some(target) = spec.target {
        clauses.push("a.target_key = ?".to_owned());
        values.push(Value::Text(target.as_str().to_owned()));
    }
    if !spec.failure_codes.is_empty() {
        clauses.push(format!("a.failure_code IN ({})", vec!["?"; spec.failure_codes.len()].join(",")));
        values.extend(spec.failure_codes.iter().map(|code| Value::Integer(*code as i64)));
    }
    if !spec.reason_codes.is_empty() {
        clauses.push(format!("a.reason_code IN ({})", vec!["?"; spec.reason_codes.len()].join(",")));
        values.extend(spec.reason_codes.iter().cloned().map(Value::Text));
    }
    if let Some(from) = spec.occurred_from {
        clauses.push("(a.occurred_at_seconds > ? OR (a.occurred_at_seconds = ? AND a.occurred_at_nanos >= ?))".to_owned());
        values.push(Value::Integer(from.seconds));
        values.push(Value::Integer(from.seconds));
        values.push(Value::Integer(from.nanos as i64));
    }
    if let Some(before) = spec.occurred_before {
        clauses.push("(a.occurred_at_seconds < ? OR (a.occurred_at_seconds = ? AND a.occurred_at_nanos < ?))".to_owned());
        values.push(Value::Integer(before.seconds));
        values.push(Value::Integer(before.seconds));
        values.push(Value::Integer(before.nanos as i64));
    }
    let sql = format!(
        "SELECT a.audit_lsn, a.occurred_at_seconds, a.occurred_at_nanos, a.kind,
                a.actor_id, a.endpoint_id, a.command_id, a.target_key,
                a.failure_code, a.reason_code, a.source_lsn, e.kind, e.payload
         FROM audit_records a JOIN events e
           ON e.authority_domain_id = a.authority_domain_id AND e.lsn = a.audit_lsn
         WHERE {} ORDER BY a.audit_lsn DESC LIMIT ?",
        clauses.join(" AND ")
    );
    values.push(Value::Integer(i64::from(spec.limit) + 1));
    let mut statement = db.prepare(&sql).map_err(map_read_err)?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<i32>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, i32>(11)?,
                row.get::<_, Vec<u8>>(12)?,
            ))
        })
        .map_err(map_read_err)?;
    let mut records = Vec::new();
    for row in rows {
        let (lsn, seconds, nanos, kind, actor_id, endpoint_id, command_id, target_key, failure_code, reason_code, source_lsn, event_kind, payload_bytes) = row.map_err(map_read_err)?;
        if event_kind != StoredEventKind::AuditRecord as i32 {
            return Err(StorageError::CorruptRecord(format!("audit index LSN {lsn} points to event kind {event_kind}")));
        }
        let envelope = decode_payload(&payload_bytes)?;
        if validate_kind(&envelope)? != StoredEventKind::AuditRecord {
            return Err(StorageError::CorruptRecord(format!("audit index LSN {lsn} has a non-audit envelope")));
        }
        let record = AuditRecord::decode(envelope.payload.as_slice()).map_err(|error| StorageError::CorruptRecord(format!("cannot decode audit record at LSN {lsn}: {error}")))?;
        validate_audit_index_row(authority_domain_id, lsn, seconds, nanos, kind, actor_id.as_deref(), endpoint_id.as_deref(), command_id.as_deref(), target_key.as_deref(), failure_code, &reason_code, source_lsn, &record)?;
        records.push(record);
    }
    let has_more = records.len() > usize::from(spec.limit);
    if has_more {
        records.truncate(usize::from(spec.limit));
    }
    let next_before_event_id = has_more.then(|| {
        records.last().and_then(|record| record.audit_event_id.clone())
    }).flatten();
    Ok(AuditPage { records, next_before_event_id, has_more })
}

#[allow(clippy::too_many_arguments, reason = "the SQL index tuple is validated field-by-field against its canonical protobuf payload")]
fn validate_audit_index_row(
    authority_domain_id: &AuthorityDomainId,
    lsn: i64,
    seconds: i64,
    nanos: i32,
    kind: i32,
    actor_id: Option<&str>,
    endpoint_id: Option<&str>,
    command_id: Option<&str>,
    target_key: Option<&str>,
    failure_code: Option<i32>,
    reason_code: &str,
    source_lsn: Option<i64>,
    record: &AuditRecord,
) -> Result<(), StorageError> {
    let expected_event_id = event_id(authority_domain_id.clone(), lsn as u64);
    if record.audit_event_id.as_ref() != Some(&expected_event_id)
        || record.occurred_at.as_ref().map(|time| (time.seconds, time.nanos)) != Some((seconds, nanos))
        || record.kind != kind
        || record.actor_id.as_ref().map(|id| id.value.as_str()) != actor_id
        || record.endpoint_id.as_ref().map(|id| id.value.as_str()) != endpoint_id
        || record.command_id.as_ref().map(|id| id.value.as_str()) != command_id
        || target_key_for_scope(record.target_scope.as_ref()).as_deref() != target_key
        || record.failure_code != failure_code.unwrap_or(FailureCode::Unspecified as i32)
        || record.reason_code != reason_code
        || record.source_event_id.as_ref().and_then(|id| id.lsn.as_ref()).map(|lsn| lsn.value as i64) != source_lsn
    {
        return Err(StorageError::CorruptRecord(format!("audit index disagrees with log payload at LSN {lsn}")));
    }
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
        self.read_events(authority_domain_id, cursor, None).await
    }

    async fn read_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
        as_of_lsn: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.read_events(authority_domain_id, cursor, Some(as_of_lsn)).await
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

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<EventId, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendAudit {
                authority_domain_id: authority_domain_id.value.clone(),
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                source,
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_batch_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        sources: Vec<StoredEventPayload>,
        audit: AuditRecordDraft,
    ) -> Result<AuditedBatchAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendBatchAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                sources,
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_dedup_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedupAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_owned(),
                source,
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn query_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
    ) -> Result<AuditPage, StorageError> {
        let db = self.read_db.lock().await;
        query_audit_sync(&db, authority_domain_id, spec, None)
    }

    async fn query_audit_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
        as_of_lsn: Lsn,
    ) -> Result<AuditPage, StorageError> {
        let db = self.read_db.lock().await;
        query_audit_sync(&db, authority_domain_id, spec, Some(as_of_lsn.value))
    }
}

#[allow(dead_code)]
impl RusqliteStorage {
    async fn read_events(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
        upper_bound: Option<Lsn>,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        let cursor_i64 = lsn_to_i64(cursor.value)?;
        let upper_bound_i64 = upper_bound.map(|lsn| lsn_to_i64(lsn.value)).transpose()?;
        let db = self.read_db.lock().await;
        let mut stmt = db
            .prepare(
                "SELECT lsn, kind, payload FROM events
                 WHERE authority_domain_id = ?1 AND lsn > ?2
                   AND (?3 IS NULL OR lsn <= ?3)
                 ORDER BY lsn",
            )
            .map_err(map_read_err)?;
        let rows = stmt
            .query_map(
                rusqlite::params![authority_domain_id.value, cursor_i64, upper_bound_i64],
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

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<EventId, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendAudit {
                authority_domain_id: authority_domain_id.value.clone(),
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                source,
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_dedup_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedupAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_owned(),
                source,
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn query_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
    ) -> Result<AuditPage, StorageError> {
        let db = self.read_db.lock().await;
        query_audit_sync(&db, authority_domain_id, spec, None)
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
