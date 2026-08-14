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

use std::{collections::BTreeMap, sync::Arc};

#[cfg(unix)]
use std::{
    fs::{OpenOptions, Permissions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

use patchbay_contracts::patchbay::{
    runtime_generation_disposition, AuditEventKind, AuditPage, AuditRecord, AuthorityDomainId,
    DescendantGrant, EventId, FailureCode, Generation, Grant, IdempotencyKey, Lsn,
    QuarantinedRuntimeEvidence, RuntimeEvidenceQuarantineReason, SpawnPromotionCommitted,
    SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload,
};
use prost::Message;
use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use tokio::sync::{mpsc, oneshot, Mutex};

use super::port::{
    event_id, AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedBatchAppend,
    AuditedDecisionAppend, AuditedDedupOutcome, CoreGenerationStore, DedupOutcome,
    GrantAppendOutcome, GrantIdentityKey, RecordedEvent, SpawnPromotionAppend, Storage,
    StorageError, StoredSnapshot, TargetKey,
};

pub const LATEST_SCHEMA_VERSION: u32 = 5;

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

const MIGRATION_3: &str = r#"
ALTER TABLE audit_records ADD COLUMN grant_id TEXT;
CREATE INDEX IF NOT EXISTS idx_audit_grant ON audit_records(authority_domain_id, grant_id);
"#;

const MIGRATION_4: &str = r#"
CREATE TABLE authority_domain_metadata (
    authority_domain_id TEXT NOT NULL PRIMARY KEY,
    core_generation INTEGER NOT NULL CHECK(core_generation > 0)
);
"#;

const MIGRATION_5: &str = r#"
CREATE TABLE grant_identities (
    authority_domain_id TEXT NOT NULL,
    grant_id TEXT NOT NULL,
    source_lsn INTEGER NOT NULL UNIQUE CHECK(source_lsn > 0),
    PRIMARY KEY (authority_domain_id, grant_id),
    FOREIGN KEY (source_lsn) REFERENCES events(lsn)
);
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

fn validate_authority_domain_metadata_schema(db: &Connection) -> Result<(), StorageError> {
    #[derive(Debug)]
    struct Column {
        name: String,
        declared_type: String,
        not_null: bool,
        default_value: Option<String>,
        primary_key_position: i64,
    }

    let malformed = |message: String| {
        StorageError::MalformedSchema(format!("table authority_domain_metadata {message}"))
    };
    let mut statement = db
        .prepare(
            "SELECT name, type, \"notnull\", dflt_value, pk
             FROM pragma_table_info('authority_domain_metadata')
             ORDER BY cid",
        )
        .map_err(map_write_err)?;
    let columns = statement
        .query_map([], |row| {
            Ok(Column {
                name: row.get(0)?,
                declared_type: row.get(1)?,
                not_null: row.get::<_, i64>(2)? != 0,
                default_value: row.get(3)?,
                primary_key_position: row.get(4)?,
            })
        })
        .map_err(map_write_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_write_err)?;
    if columns.len() != 2 {
        return Err(malformed(format!(
            "must have exactly authority_domain_id and core_generation columns, found {}",
            columns.len()
        )));
    }
    let authority_domain = &columns[0];
    if authority_domain.name != "authority_domain_id"
        || !authority_domain.declared_type.eq_ignore_ascii_case("TEXT")
        || !authority_domain.not_null
        || authority_domain.default_value.is_some()
        || authority_domain.primary_key_position != 1
    {
        return Err(malformed(
            "must declare authority_domain_id as TEXT NOT NULL PRIMARY KEY with no default"
                .to_owned(),
        ));
    }
    let core_generation = &columns[1];
    if core_generation.name != "core_generation"
        || !core_generation
            .declared_type
            .eq_ignore_ascii_case("INTEGER")
        || !core_generation.not_null
        || core_generation.default_value.is_some()
        || core_generation.primary_key_position != 0
    {
        return Err(malformed(
            "must declare core_generation as INTEGER NOT NULL with no default".to_owned(),
        ));
    }

    let mut indexes = db
        .prepare(
            "SELECT name, \"unique\", origin, partial
             FROM pragma_index_list('authority_domain_metadata')",
        )
        .map_err(map_write_err)?;
    let indexes = indexes
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
            ))
        })
        .map_err(map_write_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_write_err)?;
    let mut has_exact_primary_key_index = false;
    for (name, unique, origin, partial) in indexes {
        if !unique || origin != "pk" || partial {
            continue;
        }
        let mut index_columns = db
            .prepare("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
            .map_err(map_write_err)?;
        let index_columns = index_columns
            .query_map([name], |row| row.get::<_, String>(0))
            .map_err(map_write_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_write_err)?;
        if index_columns == ["authority_domain_id"] {
            has_exact_primary_key_index = true;
        }
    }
    if !has_exact_primary_key_index {
        return Err(malformed(
            "must enforce one unique primary key on authority_domain_id".to_owned(),
        ));
    }

    let create_sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table' AND name = 'authority_domain_metadata'",
            [],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    let normalize_sql = |sql: &str| {
        sql.chars()
            .filter(|character| !character.is_ascii_whitespace())
            .flat_map(char::to_lowercase)
            .collect::<String>()
            .trim_end_matches(';')
            .to_owned()
    };
    if normalize_sql(&create_sql) != normalize_sql(MIGRATION_4) {
        return Err(malformed(
            "must match the canonical v4 definition, including CHECK(core_generation > 0)"
                .to_owned(),
        ));
    }

    Ok(())
}

fn normalize_schema_sql(sql: &str) -> String {
    sql.chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .trim_end_matches(';')
        .to_owned()
}

fn validate_grant_identities_schema(db: &Connection) -> Result<(), StorageError> {
    validate_columns(
        db,
        "grant_identities",
        &["authority_domain_id", "grant_id", "source_lsn"],
    )?;
    let column_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('grant_identities')",
            [],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    if column_count != 3 {
        return Err(StorageError::MalformedSchema(format!(
            "table grant_identities must have exactly 3 columns, found {column_count}"
        )));
    }
    let create_sql: String = db
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'grant_identities'",
            [],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    if normalize_schema_sql(&create_sql) != normalize_schema_sql(MIGRATION_5) {
        return Err(StorageError::MalformedSchema(
            "table grant_identities must match the canonical v5 identity/index definition"
                .to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct GrantIdentitySource {
    source_lsn: i64,
    envelope_bytes: Vec<u8>,
}

fn decoded_grant_boundary(
    payload: &StoredEventPayload,
    kind: StoredEventKind,
) -> Result<(String, String, &'static str), StorageError> {
    let (grant_id, authority_domain_id, reason_code) = match kind {
        StoredEventKind::Grant => {
            let grant = Grant::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!("cannot decode grant identity: {error}"))
            })?;
            (grant.grant_id, grant.authority_domain_id, "grant_created")
        }
        StoredEventKind::DescendantGrant => {
            let grant = DescendantGrant::decode(payload.payload.as_slice()).map_err(|error| {
                StorageError::CorruptRecord(format!(
                    "cannot decode descendant grant identity: {error}"
                ))
            })?;
            (
                grant.grant_id,
                grant.authority_domain_id,
                "descendant_grant_created",
            )
        }
        StoredEventKind::SpawnPromotionCommitted => {
            let promotion =
                SpawnPromotionCommitted::decode(payload.payload.as_slice()).map_err(|error| {
                    StorageError::CorruptRecord(format!(
                        "cannot decode promotion descendant grant identity: {error}"
                    ))
                })?;
            let grant = promotion
                .authority
                .and_then(|authority| authority.descendant_grant)
                .ok_or_else(|| {
                    StorageError::CorruptRecord(
                        "promotion identity source has no descendant grant".to_owned(),
                    )
                })?;
            (
                grant.grant_id,
                grant.authority_domain_id,
                "descendant_grant_created",
            )
        }
        _ => {
            return Err(StorageError::CorruptRecord(
                "grant identity source is not a grant or descendant grant".to_owned(),
            ));
        }
    };
    let grant_id = grant_id
        .filter(|identity| !identity.value.is_empty())
        .ok_or_else(|| {
            StorageError::CorruptRecord(
                "grant identity source has no non-empty grant_id".to_owned(),
            )
        })?;
    let authority_domain_id = authority_domain_id
        .filter(|identity| !identity.value.is_empty())
        .ok_or_else(|| {
            StorageError::CorruptRecord(
                "grant identity source has no non-empty authority_domain_id".to_owned(),
            )
        })?;
    Ok((grant_id.value, authority_domain_id.value, reason_code))
}

fn authoritative_grant_identities(
    db: &Connection,
) -> Result<BTreeMap<(String, String), GrantIdentitySource>, StorageError> {
    let mut statement = db
        .prepare(
            "SELECT lsn, authority_domain_id, kind, payload
             FROM events
             WHERE kind IN (?1, ?2, ?3)
             ORDER BY lsn",
        )
        .map_err(map_write_err)?;
    let rows = statement
        .query_map(
            rusqlite::params![
                StoredEventKind::Grant as i32,
                StoredEventKind::DescendantGrant as i32,
                StoredEventKind::SpawnPromotionCommitted as i32
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            },
        )
        .map_err(map_write_err)?;
    let mut identities: BTreeMap<(String, String), GrantIdentitySource> = BTreeMap::new();
    for row in rows {
        let (source_lsn, row_domain, sql_kind, envelope_bytes) = row.map_err(map_write_err)?;
        if source_lsn <= 0 {
            return Err(StorageError::CorruptRecord(format!(
                "grant identity source has invalid LSN {source_lsn}"
            )));
        }
        let envelope = decode_payload(&envelope_bytes)?;
        let envelope_kind = decode_stored_kind(&envelope, source_lsn)?;
        if envelope_kind as i32 != sql_kind
            || !matches!(
                envelope_kind,
                StoredEventKind::Grant
                    | StoredEventKind::DescendantGrant
                    | StoredEventKind::SpawnPromotionCommitted
            )
        {
            return Err(StorageError::CorruptRecord(format!(
                "grant source kind disagrees at LSN {source_lsn}"
            )));
        }
        let (grant_id, embedded_domain, _) = decoded_grant_boundary(&envelope, envelope_kind)?;
        if embedded_domain != row_domain {
            return Err(StorageError::CorruptRecord(format!(
                "grant {grant_id} at LSN {source_lsn} embeds authority domain {embedded_domain}, row belongs to {row_domain}"
            )));
        }
        let key = (row_domain, grant_id.clone());
        if let Some(existing) = identities.get(&key) {
            if existing.envelope_bytes != envelope_bytes {
                return Err(StorageError::CorruptRecord(format!(
                    "grant identity {grant_id} conflicts between LSNs {} and {source_lsn}",
                    existing.source_lsn
                )));
            }
            continue;
        }
        identities.insert(
            key,
            GrantIdentitySource {
                source_lsn,
                envelope_bytes,
            },
        );
    }
    Ok(identities)
}

fn validate_grant_identity_index(db: &Connection) -> Result<(), StorageError> {
    let expected = authoritative_grant_identities(db)?;
    let mut statement = db
        .prepare(
            "SELECT authority_domain_id, grant_id, source_lsn
             FROM grant_identities
             ORDER BY authority_domain_id, grant_id",
        )
        .map_err(map_write_err)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(map_write_err)?;
    let mut actual_count = 0_usize;
    for row in rows {
        let (authority_domain_id, grant_id, source_lsn) = row.map_err(map_write_err)?;
        actual_count += 1;
        let Some(source) = expected.get(&(authority_domain_id.clone(), grant_id.clone())) else {
            return Err(StorageError::CorruptRecord(format!(
                "grant identity index has extra row {authority_domain_id}/{grant_id}"
            )));
        };
        if source.source_lsn != source_lsn {
            return Err(StorageError::CorruptRecord(format!(
                "grant identity index {authority_domain_id}/{grant_id} points to LSN {source_lsn}, expected earliest LSN {}",
                source.source_lsn
            )));
        }
    }
    if actual_count != expected.len() {
        return Err(StorageError::CorruptRecord(format!(
            "grant identity index covers {actual_count} identities, expected {}",
            expected.len()
        )));
    }
    Ok(())
}

fn backfill_grant_identity_index(tx: &rusqlite::Transaction<'_>) -> Result<(), StorageError> {
    for ((authority_domain_id, grant_id), source) in authoritative_grant_identities(tx)? {
        tx.execute(
            "INSERT INTO grant_identities (authority_domain_id, grant_id, source_lsn)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![authority_domain_id, grant_id, source.source_lsn],
        )
        .map_err(map_write_err)?;
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
        validate_columns(
            db,
            "events",
            &["lsn", "authority_domain_id", "kind", "payload"],
        )?;
        validate_columns(
            db,
            "idempotency_keys",
            &[
                "authority_domain_id",
                "key",
                "target",
                "lsn",
                "payload_bytes",
            ],
        )?;
        validate_columns(
            db,
            "snapshots",
            &["authority_domain_id", "snapshot_lsn", "payload"],
        )?;
    }
    let audit_exists = table_exists(db, "audit_records")?;
    if version >= 2 || audit_exists {
        let mut required_audit_columns = vec![
            "authority_domain_id",
            "audit_lsn",
            "occurred_at_seconds",
            "occurred_at_nanos",
            "kind",
            "actor_id",
            "endpoint_id",
            "command_id",
            "target_key",
            "failure_code",
            "reason_code",
            "source_lsn",
        ];
        // `grant_id` is introduced by migration 3. A valid v2 database must
        // pass preflight in its own schema shape before that migration runs.
        if version >= 3 {
            required_audit_columns.push("grant_id");
        }
        validate_columns(db, "audit_records", &required_audit_columns)?;
    }
    let metadata_exists = table_exists(db, "authority_domain_metadata")?;
    if metadata_exists && version < 4 {
        return Err(StorageError::MalformedSchema(
            "table authority_domain_metadata exists before schema version 4".to_owned(),
        ));
    }
    if version >= 4 {
        validate_authority_domain_metadata_schema(db)?;
    }
    let grant_identities_exists = table_exists(db, "grant_identities")?;
    if grant_identities_exists && version < 5 {
        return Err(StorageError::MalformedSchema(
            "table grant_identities exists before schema version 5".to_owned(),
        ));
    }
    if version >= 5 {
        validate_grant_identities_schema(db)?;
        validate_grant_identity_index(db)?;
    }

    db.execute_batch(
        "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
    )
    .map_err(map_write_err)?;
    if version == 0 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_1).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 1")
            .map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version < 2 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_2).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 2")
            .map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version < 3 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_3).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 3")
            .map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version < 4 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_4).map_err(map_write_err)?;
        tx.execute_batch("PRAGMA user_version = 4")
            .map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_write_err)?;
    if version < 5 {
        let tx = db.transaction().map_err(map_write_err)?;
        tx.execute_batch(MIGRATION_5).map_err(map_write_err)?;
        backfill_grant_identity_index(&tx)?;
        tx.execute_batch("PRAGMA user_version = 5")
            .map_err(map_write_err)?;
        tx.commit().map_err(map_write_err)?;
    }
    validate_grant_identities_schema(db)?;
    validate_grant_identity_index(db)
}

/// Commands sent to the writer actor.
enum WriterCommand {
    LoadOrCreateCoreGeneration {
        authority_domain_id: String,
        candidate: Generation,
        reply: oneshot::Sender<Result<Generation, StorageError>>,
    },
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
        logical_payload: Vec<u8>,
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
    AppendGrantAudited {
        authority_domain_id: String,
        identity: String,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<GrantAppendOutcome, StorageError>>,
    },
    AppendDedupAudited {
        authority_domain_id: String,
        key: String,
        target: String,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        logical_payload: Vec<u8>,
        reply: oneshot::Sender<Result<AuditedDedupOutcome, StorageError>>,
    },
    AppendBatchAudited {
        authority_domain_id: String,
        sources: Vec<StoredEventPayload>,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<AuditedBatchAppend, StorageError>>,
    },
    AppendDecisionAuditedMany {
        authority_domain_id: String,
        source: StoredEventPayload,
        audits: Vec<AuditRecordDraft>,
        reply: oneshot::Sender<Result<AuditedDecisionAppend, StorageError>>,
    },
    AppendSpawnSuccessorStagedIdempotent {
        authority_domain_id: String,
        staged: Box<SpawnSuccessorEvidenceStaged>,
        reply: oneshot::Sender<Result<EventId, StorageError>>,
    },
    AppendSpawnPromotionAudited {
        authority_domain_id: String,
        promotion: Box<SpawnPromotionCommitted>,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<SpawnPromotionAppend, StorageError>>,
    },
    AppendQuarantinedRuntimeEvidenceAudited {
        authority_domain_id: String,
        quarantined: Box<QuarantinedRuntimeEvidence>,
        audit: AuditRecordDraft,
        reply: oneshot::Sender<Result<AuditedAppend, StorageError>>,
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
        Self::open_with_retained_temp(path, None)
    }

    fn open_with_retained_temp(
        path: &str,
        retained_temp_dir: Option<tempfile::TempDir>,
    ) -> Result<Self, StorageError> {
        secure_database_file(path)?;
        let mut write_db = Connection::open(path).map_err(map_write_err)?;
        migrate(&mut write_db)?;
        let read_db = Connection::open(path).map_err(map_read_err)?;
        // Apply WAL + synchronous to the read connection too (WAL is persistent
        // on the DB file, but synchronous is per-connection).
        read_db
            .execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
            )
            .map_err(|e| StorageError::ReadFailed {
                message: e.to_string(),
                retryable: false,
            })?;

        let (writer_tx, writer_rx) = mpsc::channel::<WriterCommand>(64);
        let read_db = Arc::new(Mutex::new(read_db));

        tokio::spawn(writer_actor(write_db, writer_rx, retained_temp_dir));

        Ok(Self { writer_tx, read_db })
    }

    /// Open a file-backed storage backend for tests that need WAL behavior.
    ///
    /// The database lives inside a dedicated `tempfile::TempDir` owned by the
    /// writer actor, so normal shutdown removes the whole directory — main file
    /// *and* the SQLite WAL/SHM sidecars — instead of leaking the sidecars one
    /// pair per call (which previously filled `target/test-tmp` without bound:
    /// `NamedTempFile` tracks only the main DB file, not the `-wal`/`-shm`
    /// siblings SQLite creates in WAL mode). Test runs should still scope
    /// `TMPDIR` to a cleanable directory because an abruptly killed process
    /// cannot run Rust destructors; `patchbay-test-support`'s `#[ctor]` and
    /// `scripts/test-rust` provide that boundary.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let temp_dir = tempfile::tempdir().map_err(|e| StorageError::WriteFailed {
            message: format!("temp dir creation failed: {e}"),
            retryable: false,
        })?;
        let db_path = temp_dir.path().join("storage.sqlite");
        let path_str = db_path.to_str().ok_or_else(|| StorageError::WriteFailed {
            message: "temp path is not valid UTF-8".to_string(),
            retryable: false,
        })?;
        Self::open_with_retained_temp(path_str, Some(temp_dir))
    }
}

/// The writer actor loop. Owns the single write `Connection`. Receives
/// commands, executes them in transactions, replies via oneshot. Test-only
/// storage also transfers its dedicated `TempDir` here so the directory —
/// main DB file *and* its SQLite WAL/SHM sidecars — outlives both SQLite
/// connections and is removed (in full) when the actor shuts down. Owning a
/// `TempDir` (rather than a bare `NamedTempFile`) is what keeps WAL mode from
/// leaking the `-wal`/`-shm` sidecars into `target/test-tmp`.
async fn writer_actor(
    mut db: Connection,
    mut rx: mpsc::Receiver<WriterCommand>,
    _retained_temp_dir: Option<tempfile::TempDir>,
) {
    while let Some(cmd) = rx.recv().await {
        match cmd {
            WriterCommand::LoadOrCreateCoreGeneration {
                authority_domain_id,
                candidate,
                reply,
            } => {
                let result =
                    do_load_or_create_core_generation(&mut db, &authority_domain_id, candidate);
                let _ = reply.send(result);
            }
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
                logical_payload,
                reply,
            } => {
                let result = do_append_dedup(
                    &mut db,
                    &authority_domain_id,
                    &key,
                    &target,
                    &payload,
                    &logical_payload,
                );
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
            WriterCommand::AppendGrantAudited {
                authority_domain_id,
                identity,
                source,
                audit,
                reply,
            } => {
                let result = do_append_grant_audited(
                    &mut db,
                    &authority_domain_id,
                    &identity,
                    source,
                    audit,
                );
                let _ = reply.send(result);
            }
            WriterCommand::AppendDedupAudited {
                authority_domain_id,
                key,
                target,
                source,
                audit,
                logical_payload,
                reply,
            } => {
                let result = do_append_dedup_audited(
                    &mut db,
                    &authority_domain_id,
                    &key,
                    &target,
                    source,
                    audit,
                    logical_payload,
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
            WriterCommand::AppendDecisionAuditedMany {
                authority_domain_id,
                source,
                audits,
                reply,
            } => {
                let result =
                    do_append_decision_audited_many(&mut db, &authority_domain_id, source, audits);
                let _ = reply.send(result);
            }
            WriterCommand::AppendSpawnSuccessorStagedIdempotent {
                authority_domain_id,
                staged,
                reply,
            } => {
                let result = do_append_spawn_successor_staged_idempotent(
                    &mut db,
                    &authority_domain_id,
                    *staged,
                );
                let _ = reply.send(result);
            }
            WriterCommand::AppendSpawnPromotionAudited {
                authority_domain_id,
                promotion,
                audit,
                reply,
            } => {
                let result = do_append_spawn_promotion_audited(
                    &mut db,
                    &authority_domain_id,
                    *promotion,
                    audit,
                );
                let _ = reply.send(result);
            }
            WriterCommand::AppendQuarantinedRuntimeEvidenceAudited {
                authority_domain_id,
                quarantined,
                audit,
                reply,
            } => {
                let result = do_append_quarantined_runtime_evidence_audited(
                    &mut db,
                    &authority_domain_id,
                    *quarantined,
                    audit,
                );
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

fn do_load_or_create_core_generation(
    db: &mut Connection,
    authority_domain_id: &str,
    candidate: Generation,
) -> Result<Generation, StorageError> {
    if candidate.value == 0 || candidate.value > i64::MAX as u64 {
        return Err(StorageError::InvalidCoreGeneration(candidate.value));
    }
    let candidate_value = candidate.value as i64;
    let tx = db.transaction().map_err(map_write_err)?;
    tx.execute(
        "INSERT INTO authority_domain_metadata (authority_domain_id, core_generation)
         VALUES (?1, ?2)
         ON CONFLICT(authority_domain_id) DO NOTHING",
        rusqlite::params![authority_domain_id, candidate_value],
    )
    .map_err(map_write_err)?;
    let stored: i64 = tx
        .query_row(
            "SELECT core_generation FROM authority_domain_metadata WHERE authority_domain_id = ?1",
            [authority_domain_id],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    if stored <= 0 {
        return Err(StorageError::CorruptRecord(format!(
            "authority domain {authority_domain_id} has invalid core generation {stored}"
        )));
    }
    tx.commit().map_err(map_write_err)?;
    Ok(Generation {
        value: stored as u64,
    })
}

/// Validate an append candidate before it reaches the writer transaction.
/// `try_from` succeeds for `Unspecified`, so append validation rejects it
/// explicitly alongside unknown numeric values.
fn validate_append_kind(payload: &StoredEventPayload) -> Result<StoredEventKind, StorageError> {
    let kind =
        StoredEventKind::try_from(payload.kind).map_err(|_| StorageError::InvalidEventKind)?;
    if kind == StoredEventKind::Unspecified {
        return Err(StorageError::InvalidEventKind);
    }
    Ok(kind)
}

/// Parse a stored kind for read-side consumers that need a concrete variant.
/// Database corruption is not an append-validation error.
fn decode_stored_kind(
    payload: &StoredEventPayload,
    lsn: i64,
) -> Result<StoredEventKind, StorageError> {
    let kind = StoredEventKind::try_from(payload.kind).map_err(|_| {
        StorageError::CorruptRecord(format!(
            "event at LSN {lsn} has unknown stored event kind {}",
            payload.kind
        ))
    })?;
    if kind == StoredEventKind::Unspecified {
        return Err(StorageError::CorruptRecord(format!(
            "event at LSN {lsn} has unspecified stored event kind"
        )));
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

fn do_append(
    db: &mut Connection,
    authority_domain_id: &str,
    payload: &StoredEventPayload,
) -> Result<EventId, StorageError> {
    reject_generic_unaudited_special(payload)?;
    let kind = validate_append_kind(payload)?;
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

fn target_key_for_scope(
    scope: Option<&patchbay_contracts::patchbay::TargetScope>,
) -> Option<String> {
    scope.map(|scope| encode_hex(&scope.encode_to_vec()))
}

fn insert_event(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
    payload: &StoredEventPayload,
) -> Result<(i64, Vec<u8>), StorageError> {
    let kind = validate_append_kind(payload)?;
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
        grant_id: draft.grant_id,
        target_scope: draft.target_scope,
        failure_code: draft
            .failure_code
            .map(|code| code as i32)
            .unwrap_or(FailureCode::Unspecified as i32),
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
            kind, actor_id, endpoint_id, command_id, grant_id, target_key, failure_code,
            reason_code, source_lsn
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            authority_domain_id,
            audit_lsn,
            record
                .occurred_at
                .as_ref()
                .map(|time| time.seconds)
                .ok_or_else(|| StorageError::InvalidAuditRecord(
                    "audit record has no timestamp".to_owned()
                ))?,
            record
                .occurred_at
                .as_ref()
                .map(|time| time.nanos)
                .ok_or_else(|| StorageError::InvalidAuditRecord(
                    "audit record has no timestamp".to_owned()
                ))?,
            record.kind,
            actor_id,
            endpoint_id,
            command_id,
            record.grant_id.as_ref().map(|id| id.value.as_str()),
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
        .query_row("SELECT COALESCE(MAX(lsn), 0) + 1 FROM events", [], |row| {
            row.get::<_, i64>(0)
        })
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
    reject_generic_unaudited_special(&source)?;
    validate_append_kind(&source)?;
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    })?;
    let tx = db.transaction().map_err(map_write_err)?;
    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    let source_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn as u64,
    );
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedAppend {
        source_event_id,
        audit_event_id,
    })
}

fn do_append_grant_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    identity: &str,
    source: StoredEventPayload,
    mut audit: AuditRecordDraft,
) -> Result<GrantAppendOutcome, StorageError> {
    if authority_domain_id.is_empty() || identity.is_empty() {
        return Err(StorageError::CorruptRecord(
            "grant identity append requires a non-empty domain and identity".to_owned(),
        ));
    }
    reject_generic_unaudited_special(&source)?;
    let source_kind = validate_append_kind(&source)?;
    let (embedded_identity, embedded_domain, expected_reason) =
        decoded_grant_boundary(&source, source_kind)?;
    if embedded_domain != authority_domain_id {
        return Err(StorageError::CorruptRecord(format!(
            "grant identity source embeds authority domain {embedded_domain}, expected {authority_domain_id}"
        )));
    }
    if embedded_identity != identity {
        return Err(StorageError::CorruptRecord(format!(
            "grant identity source embeds {embedded_identity}, requested {identity}"
        )));
    }
    if audit.kind != AuditEventKind::GrantCreated || audit.reason_code != expected_reason {
        return Err(StorageError::InvalidAuditRecord(format!(
            "grant identity append requires GrantCreated/{expected_reason} audit framing"
        )));
    }
    if audit
        .grant_id
        .as_ref()
        .map(|grant_id| grant_id.value.as_str())
        != Some(identity)
    {
        return Err(StorageError::InvalidAuditRecord(
            "grant creation audit grant_id must match the immutable identity".to_owned(),
        ));
    }
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    })?;
    let candidate_bytes = encode_payload(&source)?;
    let tx = db.transaction().map_err(map_write_err)?;
    let existing = match tx.query_row(
        "SELECT identities.source_lsn, events.authority_domain_id, events.kind, events.payload
         FROM grant_identities identities
         LEFT JOIN events ON events.lsn = identities.source_lsn
         WHERE identities.authority_domain_id = ?1 AND identities.grant_id = ?2",
        rusqlite::params![authority_domain_id, identity],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i32>>(2)?,
                row.get::<_, Option<Vec<u8>>>(3)?,
            ))
        },
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(error) => return Err(map_write_err(error)),
    };
    if let Some((source_lsn, row_domain, sql_kind, existing_bytes)) = existing {
        if source_lsn <= 0 {
            return Err(StorageError::CorruptRecord(format!(
                "grant identity {identity} references invalid LSN {source_lsn}"
            )));
        }
        let row_domain = row_domain.ok_or_else(|| {
            StorageError::CorruptRecord(format!(
                "grant identity {identity} references missing source LSN {source_lsn}"
            ))
        })?;
        let sql_kind = sql_kind.ok_or_else(|| {
            StorageError::CorruptRecord(format!(
                "grant identity {identity} references missing source kind at LSN {source_lsn}"
            ))
        })?;
        let existing_bytes = existing_bytes.ok_or_else(|| {
            StorageError::CorruptRecord(format!(
                "grant identity {identity} references missing source payload at LSN {source_lsn}"
            ))
        })?;
        let existing_envelope = decode_payload(&existing_bytes)?;
        let existing_kind = decode_stored_kind(&existing_envelope, source_lsn)?;
        let (existing_identity, existing_domain, _) =
            decoded_grant_boundary(&existing_envelope, existing_kind)?;
        if row_domain != authority_domain_id
            || existing_domain != authority_domain_id
            || existing_identity != identity
            || existing_kind as i32 != sql_kind
        {
            return Err(StorageError::CorruptRecord(format!(
                "grant identity {identity} disagrees with source LSN {source_lsn}"
            )));
        }
        tx.rollback().map_err(map_write_err)?;
        let source_event_id = event_id(
            AuthorityDomainId {
                value: authority_domain_id.to_owned(),
            },
            source_lsn as u64,
        );
        if existing_bytes == candidate_bytes {
            return Ok(GrantAppendOutcome::Existing(source_event_id));
        }
        return Err(StorageError::GrantIdentityConflict {
            grant_id: identity.to_owned(),
            existing_lsn: source_lsn as u64,
        });
    }

    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    tx.execute(
        "INSERT INTO grant_identities (authority_domain_id, grant_id, source_lsn)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, identity, source_lsn],
    )
    .map_err(map_write_err)?;
    let source_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn as u64,
    );
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(GrantAppendOutcome::Appended(AuditedAppend {
        source_event_id,
        audit_event_id,
    }))
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
    audit.validate(&AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    })?;
    for source in &sources {
        reject_generic_unaudited_special(source)?;
    }
    let tx = db.transaction().map_err(map_write_err)?;
    let mut source_event_ids = Vec::with_capacity(sources.len());
    for source in sources {
        let (lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
        source_event_ids.push(event_id(
            AuthorityDomainId {
                value: authority_domain_id.to_owned(),
            },
            lsn as u64,
        ));
    }
    audit.source_event_id = source_event_ids.last().cloned();
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedBatchAppend {
        source_event_ids,
        audit_event_id,
    })
}

fn do_append_decision_audited_many(
    db: &mut Connection,
    authority_domain_id: &str,
    source: StoredEventPayload,
    mut audits: Vec<AuditRecordDraft>,
) -> Result<AuditedDecisionAppend, StorageError> {
    if audits.is_empty() {
        return Err(StorageError::InvalidAuditRecord(
            "decision must have at least one audit".to_owned(),
        ));
    }
    reject_generic_unaudited_special(&source)?;
    let tx = db.transaction().map_err(map_write_err)?;
    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    let source_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn as u64,
    );
    let mut audit_event_ids = Vec::with_capacity(audits.len());
    for audit in &mut audits {
        audit.source_event_id = Some(source_event_id.clone());
        audit_event_ids.push(append_audit_in_transaction(
            &tx,
            authority_domain_id,
            audit.clone(),
        )?);
    }
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedDecisionAppend {
        source_event_id,
        audit_event_ids,
    })
}

fn recorded_events_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
) -> Result<Vec<RecordedEvent>, StorageError> {
    let mut statement = tx
        .prepare(
            "SELECT lsn, kind, payload FROM events WHERE authority_domain_id = ?1 ORDER BY lsn",
        )
        .map_err(map_write_err)?;
    let rows = statement
        .query_map([authority_domain_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(map_write_err)?;
    let domain = AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    };
    let mut events = Vec::new();
    for row in rows {
        let (lsn, sql_kind, bytes) = row.map_err(map_write_err)?;
        if lsn <= 0 {
            return Err(StorageError::CorruptRecord(format!(
                "durable prefix contains non-positive LSN {lsn}"
            )));
        }
        let payload = decode_payload(&bytes)?;
        if payload.kind != sql_kind {
            return Err(StorageError::CorruptRecord(format!(
                "durable prefix kind mismatch at LSN {lsn}"
            )));
        }
        events.push(RecordedEvent {
            event_id: event_id(domain.clone(), lsn as u64),
            payload,
        });
    }
    Ok(events)
}

fn validate_promotion_replayable(
    tx: &rusqlite::Transaction<'_>,
    authority_domain_id: &str,
    candidate: &RecordedEvent,
) -> Result<(), StorageError> {
    let domain = AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    };
    let events = recorded_events_in_transaction(tx, authority_domain_id)?;
    let mut authority = crate::authority::AuthorityRegistry::new();
    let sessions = crate::session::SessionRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut targets = crate::target::TargetRegistry::with_adapters(
        sessions,
        crate::resource::ResourceRegistry::new(),
        crate::adapter::AdapterRegistry::new(),
    );
    let mut claims = crate::session::SpawnClaimRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut commands = crate::acceptance::CommandIndex::new();
    let mut previous_lsn = 0;
    for event in &events {
        let validated = crate::storage::validate_next_replay_event(&domain, previous_lsn, event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        if validated.kind == StoredEventKind::SpawnPromotionCommitted {
            crate::session::fold_spawn_promotion_ordered(
                &mut authority,
                &mut targets,
                &mut claims,
                &mut commands,
                event,
            )
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        } else {
            authority
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            targets
                .observe_event(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            claims
                .observe(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
            commands
                .apply(event)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        }
        previous_lsn = validated.lsn;
    }
    let candidate_lsn = candidate.event_id.lsn.as_ref().map_or(0, |lsn| lsn.value);
    if candidate_lsn != previous_lsn.saturating_add(1) {
        return Err(StorageError::CorruptRecord(
            "promotion candidate does not immediately follow its validated prefix".to_owned(),
        ));
    }
    crate::session::fold_spawn_promotion_ordered(
        &mut authority,
        &mut targets,
        &mut claims,
        &mut commands,
        candidate,
    )
    .map_err(|error| {
        StorageError::CorruptRecord(format!(
        "promotion is not replayable by the aggregate authority/session/claim/command fold: {error}"
    ))
    })
}

fn do_append_quarantined_runtime_evidence_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    quarantined: QuarantinedRuntimeEvidence,
    mut audit: AuditRecordDraft,
) -> Result<AuditedAppend, StorageError> {
    let domain = AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    };
    if quarantined.authority_domain_id.as_ref() != Some(&domain) {
        return Err(StorageError::CorruptRecord(
            "quarantine envelope domain does not match append domain".to_owned(),
        ));
    }
    crate::session::validate_quarantined_runtime_evidence(&quarantined)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let reason = RuntimeEvidenceQuarantineReason::try_from(quarantined.reason)
        .map_err(|_| StorageError::CorruptRecord("quarantine reason is unknown".to_owned()))?;
    let target = crate::session::quarantined_candidate_scope(&quarantined)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    if audit.kind != AuditEventKind::StaleEventIgnored
        || audit.failure_code != Some(FailureCode::StaleEvent)
        || audit.reason_code != crate::session::quarantine_reason_code(reason)
        || audit.target_scope.as_ref() != Some(&target)
    {
        return Err(StorageError::InvalidAuditRecord(
            "quarantine requires canonical StaleEventIgnored/stale_event reason and exact runtime target framing"
                .to_owned(),
        ));
    }
    audit.source_event_id = None;
    audit.validate(&domain)?;

    let tx = db.transaction().map_err(map_write_err)?;
    let events = recorded_events_in_transaction(&tx, authority_domain_id)?;
    let source = quarantined
        .source_attachment
        .as_ref()
        .expect("quarantine syntactically validated source");
    let attachment_id = source
        .attachment_event_id
        .as_ref()
        .expect("quarantine syntactically validated attachment id");
    let attachment_event = events
        .iter()
        .find(|event| &event.event_id == attachment_id)
        .ok_or_else(|| {
            StorageError::CorruptRecord(
                "quarantine attachment_event_id does not reference the durable prefix".to_owned(),
            )
        })?;
    let mut referenced_attachment = crate::adapter::AdapterRegistry::new();
    referenced_attachment
        .observe(attachment_event)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let adapter_id = source
        .adapter_id
        .as_ref()
        .expect("source syntactically validated");
    let referenced = referenced_attachment.get(adapter_id).ok_or_else(|| {
        StorageError::CorruptRecord(
            "quarantine attachment_event_id is not a canonical registration for its adapter"
                .to_owned(),
        )
    })?;
    if referenced.registration.adapter_generation != source.adapter_generation
        || referenced.attach_event_id != *attachment_id
    {
        return Err(StorageError::CorruptRecord(
            "quarantine source generation disagrees with its durable attachment".to_owned(),
        ));
    }

    let mut adapters = crate::adapter::AdapterRegistry::new();
    let mut sessions = crate::session::SessionRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut claims = crate::session::SpawnClaimRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut previous_lsn = 0;
    for event in &events {
        let validated = crate::storage::validate_next_replay_event(&domain, previous_lsn, event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        adapters
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        sessions
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        claims
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        previous_lsn = validated.lsn;
    }
    let source_is_current =
        crate::session::source_matches_current_attachment(&domain, source, &adapters);
    let candidate_producer_is_current = match quarantined
        .candidate
        .as_ref()
        .expect("candidate validated")
    {
        patchbay_contracts::patchbay::quarantined_runtime_evidence::Candidate::SessionReport(
            report,
        ) => {
            report
                .source_cursor
                .as_ref()
                .and_then(|cursor| cursor.adapter_generation)
                == source.adapter_generation
        }
        _ => true,
    };
    if (reason == RuntimeEvidenceQuarantineReason::StaleAttachment
        && source_is_current
        && candidate_producer_is_current)
        || (reason != RuntimeEvidenceQuarantineReason::StaleAttachment
            && (!source_is_current || !candidate_producer_is_current))
    {
        return Err(StorageError::CorruptRecord(
            "quarantine stale/current producer classification disagrees with the durable prefix"
                .to_owned(),
        ));
    }
    let candidate = quarantined.candidate.as_ref().expect("candidate validated");
    let actual_disposition = match candidate {
        patchbay_contracts::patchbay::quarantined_runtime_evidence::Candidate::SessionReport(
            report,
        ) => crate::session::classify_session_report(
            &domain, report, source, &adapters, &claims, &sessions,
        ),
        _ => crate::session::classify_runtime_target(
            &domain,
            &crate::session::quarantined_candidate_target(&quarantined)
                .map_err(|error| StorageError::CorruptRecord(error.to_string()))?,
            source,
            &adapters,
            &sessions,
        ),
    };
    let framed_context = quarantined
        .classification
        .as_ref()
        .expect("quarantine classification validated");
    let candidate_external = crate::session::quarantined_candidate_target(&quarantined)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let expected_context = crate::session::canonical_runtime_evidence_classification_context(
        &domain,
        candidate,
        &candidate_external,
        actual_disposition.clone(),
        &sessions,
        &claims,
    );
    if framed_context != &expected_context
        || matches!(
            actual_disposition.disposition,
            Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
        )
    {
        return Err(StorageError::CorruptRecord(
            "quarantine classification context does not exactly match the durable runtime/claim prefix"
                .to_owned(),
        ));
    }

    let source_payload = crate::session::encode_quarantined_runtime_evidence(&quarantined)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source_payload)?;
    let source_event_id = event_id(domain, source_lsn as u64);
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedAppend {
        source_event_id,
        audit_event_id,
    })
}

fn do_append_spawn_successor_staged_idempotent(
    db: &mut Connection,
    authority_domain_id: &str,
    staged: SpawnSuccessorEvidenceStaged,
) -> Result<EventId, StorageError> {
    let domain = AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    };
    if staged.authority_domain_id.as_ref() != Some(&domain) {
        return Err(StorageError::CorruptRecord(
            "staged-successor domain does not match append domain".to_owned(),
        ));
    }
    crate::session::validate_staged_successor(&staged)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let command_id = staged
        .exact_claim
        .as_ref()
        .and_then(|claim| claim.claim_operation_id.as_ref())
        .expect("staged successor validated")
        .clone();
    let external = staged
        .external_runtime_reservation
        .as_ref()
        .expect("staged successor validated")
        .clone();

    let tx = db.transaction().map_err(map_write_err)?;
    let events = recorded_events_in_transaction(&tx, authority_domain_id)?;
    let mut existing_exact = None;
    for event in &events {
        if event.payload.kind != StoredEventKind::SpawnSuccessorEvidenceStaged as i32 {
            continue;
        }
        let existing = SpawnSuccessorEvidenceStaged::decode(event.payload.payload.as_slice())
            .map_err(|error| {
                StorageError::CorruptRecord(format!(
                    "cannot decode durable staged successor: {error}"
                ))
            })?;
        crate::session::validate_staged_successor(&existing)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        let existing_command = existing
            .exact_claim
            .as_ref()
            .and_then(|claim| claim.claim_operation_id.as_ref())
            .expect("durable staged successor validated");
        let existing_external = existing
            .external_runtime_reservation
            .as_ref()
            .expect("durable staged successor validated");
        if existing_command == &command_id || existing_external == &external {
            let existing_lsn = event
                .event_id
                .lsn
                .as_ref()
                .expect("recorded event has an LSN")
                .value;
            if existing == staged {
                if existing_exact.is_some() {
                    return Err(StorageError::CorruptRecord(format!(
                        "durable prefix contains duplicate exact staged successor for claim {}",
                        command_id.value
                    )));
                }
                existing_exact = Some(event.event_id.clone());
            } else {
                return Err(StorageError::StagedSuccessorConflict {
                    command_id: command_id.value.clone(),
                    existing_lsn,
                });
            }
        }
    }

    let mut adapters = crate::adapter::AdapterRegistry::new();
    let mut sessions = crate::session::SessionRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut claims = crate::session::SpawnClaimRegistry::new(domain.clone())
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let mut previous_lsn = 0;
    for event in &events {
        let validated = crate::storage::validate_next_replay_event(&domain, previous_lsn, event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        adapters
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        sessions
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        claims
            .observe(event)
            .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
        previous_lsn = validated.lsn;
    }
    if let Some(event_id) = existing_exact {
        tx.commit().map_err(map_write_err)?;
        return Ok(event_id);
    }

    let claim = crate::session::SpawnClaimQuery::claim_for_operation(&claims, &command_id)
        .ok_or_else(|| {
            StorageError::CorruptRecord(
                "staged successor does not reference a durable spawn claim".to_owned(),
            )
        })?;
    if staged.exact_claim.as_ref() != Some(&claim.claim) {
        return Err(StorageError::CorruptRecord(
            "staged successor exact claim disagrees with the durable claim".to_owned(),
        ));
    }
    let report = staged.report.as_ref().expect("staged successor validated");
    let source = staged
        .source_attachment
        .as_ref()
        .expect("staged successor validated");
    let actual_disposition = crate::session::classify_session_report(
        &domain, report, source, &adapters, &claims, &sessions,
    );
    if staged.disposition.as_ref() != Some(&actual_disposition)
        || !matches!(
            actual_disposition.disposition,
            Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
        )
    {
        return Err(StorageError::CorruptRecord(
            "staged successor classification disagrees with the durable prefix".to_owned(),
        ));
    }

    let source = crate::session::encode_staged_successor(&staged);
    let candidate = RecordedEvent {
        event_id: event_id(domain.clone(), previous_lsn.saturating_add(1)),
        payload: source.clone(),
    };
    sessions
        .observe(&candidate)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;
    let (source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    if source_lsn as u64 != previous_lsn.saturating_add(1) {
        return Err(StorageError::CorruptRecord(format!(
            "SQLite assigned staged-successor LSN {source_lsn}, expected {}",
            previous_lsn.saturating_add(1)
        )));
    }
    tx.commit().map_err(map_write_err)?;
    Ok(candidate.event_id)
}

fn do_append_spawn_promotion_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    mut promotion: SpawnPromotionCommitted,
    mut audit: AuditRecordDraft,
) -> Result<SpawnPromotionAppend, StorageError> {
    if authority_domain_id.is_empty()
        || promotion.promotion_event_id.is_some()
        || promotion.completion_audit_event_id.is_some()
    {
        return Err(StorageError::CorruptRecord(
            "promotion append requires a non-empty domain and unstamped event ids".to_owned(),
        ));
    }
    let descendant = promotion
        .authority
        .as_mut()
        .and_then(|authority| authority.descendant_grant.as_mut())
        .ok_or_else(|| {
            StorageError::CorruptRecord("promotion has no descendant grant".to_owned())
        })?;
    if descendant.audit_id.is_some() {
        return Err(StorageError::CorruptRecord(
            "promotion descendant audit id must be assigned by storage".to_owned(),
        ));
    }
    let grant_id = descendant
        .grant_id
        .as_ref()
        .filter(|grant_id| !grant_id.value.is_empty())
        .ok_or_else(|| {
            StorageError::CorruptRecord("promotion descendant has no grant id".to_owned())
        })?
        .value
        .clone();
    let command_id = promotion
        .accepted_claim
        .as_ref()
        .and_then(|accepted| accepted.claim.as_ref())
        .and_then(|claim| claim.claim_operation_id.as_ref())
        .filter(|command_id| !command_id.value.is_empty())
        .ok_or_else(|| {
            StorageError::CorruptRecord("promotion has no claim operation id".to_owned())
        })?
        .clone();
    if audit.kind != AuditEventKind::CommandCompleted
        || audit.reason_code != "spawn_completion"
        || audit.command_id.as_ref() != Some(&command_id)
        || promotion.committed_at.as_ref() != Some(&audit.occurred_at)
    {
        return Err(StorageError::InvalidAuditRecord(
            "promotion requires CommandCompleted/spawn_completion audit for the exact operation"
                .to_owned(),
        ));
    }
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    })?;

    let tx = db.transaction().map_err(map_write_err)?;
    let source_lsn = tx
        .query_row("SELECT COALESCE(MAX(lsn), 0) + 1 FROM events", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(map_write_err)?;
    if source_lsn <= 0 {
        return Err(StorageError::CorruptRecord(
            "promotion source LSN is not positive".to_owned(),
        ));
    }
    let source_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn as u64,
    );
    let audit_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn
            .checked_add(1)
            .ok_or_else(|| StorageError::CorruptRecord("promotion audit LSN overflow".to_owned()))?
            as u64,
    );
    promotion.authority_domain_id = Some(AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    });
    promotion.promotion_event_id = Some(source_event_id.clone());
    promotion.completion_audit_event_id = Some(audit_event_id.clone());
    promotion
        .authority
        .as_mut()
        .and_then(|authority| authority.descendant_grant.as_mut())
        .expect("descendant validated above")
        .audit_id = Some(audit_event_id.clone());
    crate::session::validate_spawn_promotion_envelope(&promotion, &source_event_id)
        .map_err(|error| StorageError::CorruptRecord(error.to_string()))?;

    let existing_identity = tx
        .query_row(
            "SELECT source_lsn FROM grant_identities WHERE authority_domain_id = ?1 AND grant_id = ?2",
            rusqlite::params![authority_domain_id, grant_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(map_write_err)?;
    if let Some(existing_lsn) = existing_identity {
        return Err(StorageError::GrantIdentityConflict {
            grant_id,
            existing_lsn: existing_lsn as u64,
        });
    }

    let source = StoredEventPayload {
        kind: StoredEventKind::SpawnPromotionCommitted as i32,
        payload: promotion.encode_to_vec(),
    };
    let candidate = RecordedEvent {
        event_id: source_event_id.clone(),
        payload: source.clone(),
    };
    validate_promotion_replayable(&tx, authority_domain_id, &candidate)?;
    let (actual_source_lsn, _) = insert_event(&tx, authority_domain_id, &source)?;
    if actual_source_lsn != source_lsn {
        return Err(StorageError::CorruptRecord(format!(
            "SQLite assigned promotion LSN {actual_source_lsn}, expected {source_lsn}"
        )));
    }
    tx.execute(
        "INSERT INTO grant_identities (authority_domain_id, grant_id, source_lsn) VALUES (?1, ?2, ?3)",
        rusqlite::params![authority_domain_id, grant_id, source_lsn],
    )
    .map_err(map_write_err)?;
    audit.source_event_id = Some(source_event_id.clone());
    let actual_audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    if actual_audit_event_id != audit_event_id {
        return Err(StorageError::CorruptRecord(
            "promotion audit did not receive the stamped immediate-successor id".to_owned(),
        ));
    }
    tx.commit().map_err(map_write_err)?;
    Ok(SpawnPromotionAppend {
        source_event_id,
        audit_event_id,
        promotion,
    })
}

fn do_append_dedup_audited(
    db: &mut Connection,
    authority_domain_id: &str,
    key: &str,
    target: &str,
    source: StoredEventPayload,
    mut audit: AuditRecordDraft,
    logical_payload: Vec<u8>,
) -> Result<AuditedDedupOutcome, StorageError> {
    reject_generic_unaudited_special(&source)?;
    validate_append_kind(&source)?;
    audit.source_event_id = None;
    audit.validate(&AuthorityDomainId {
        value: authority_domain_id.to_owned(),
    })?;
    let canonical = logical_payload;
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
            let source_event_id = event_id(
                AuthorityDomainId {
                    value: authority_domain_id.to_owned(),
                },
                lsn as u64,
            );
            audit.source_event_id = Some(source_event_id);
            let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
            tx.commit().map_err(map_write_err)?;
            return Ok(AuditedDedupOutcome::Appended(AuditedAppend {
                source_event_id: event_id(
                    AuthorityDomainId {
                        value: authority_domain_id.to_owned(),
                    },
                    lsn as u64,
                ),
                audit_event_id,
            }));
        }
    };
    let source_event_id = event_id(
        AuthorityDomainId {
            value: authority_domain_id.to_owned(),
        },
        source_lsn as u64,
    );
    audit.source_event_id = Some(source_event_id.clone());
    let audit_event_id = append_audit_in_transaction(&tx, authority_domain_id, audit)?;
    tx.commit().map_err(map_write_err)?;
    Ok(AuditedDedupOutcome::Duplicate {
        source_event_id,
        audit_event_id,
    })
}

fn do_append_dedup(
    db: &mut Connection,
    authority_domain_id: &str,
    key: &str,
    target: &str,
    payload: &StoredEventPayload,
    logical_payload: &[u8],
) -> Result<DedupOutcome, StorageError> {
    reject_generic_unaudited_special(payload)?;
    let kind = validate_append_kind(payload)?;
    let encoded = encode_payload(payload)?;
    let canonical = logical_payload.to_vec();
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
    let latest: Option<i64> = tx
        .query_row(
            "SELECT MAX(snapshot_lsn) FROM snapshots WHERE authority_domain_id = ?1",
            rusqlite::params![authority_domain_id],
            |row| row.get(0),
        )
        .map_err(map_write_err)?;
    if latest.is_some_and(|latest| snapshot_lsn_i64 < latest) {
        tx.rollback().map_err(map_write_err)?;
        return Err(StorageError::SnapshotStale(snapshot_lsn));
    }
    tx.execute(
        "DELETE FROM snapshots WHERE authority_domain_id = ?1",
        rusqlite::params![authority_domain_id],
    )
    .map_err(map_write_err)?;
    tx.execute(
        "INSERT INTO snapshots (authority_domain_id, snapshot_lsn, payload)
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
            return Err(StorageError::InvalidAuditRecord(
                "audit filter contains an unknown kind".to_owned(),
            ));
        }
    }
    for code in &spec.failure_codes {
        if *code == FailureCode::Unspecified || FailureCode::try_from(*code as i32).is_err() {
            return Err(StorageError::InvalidAuditRecord(
                "audit filter contains an unknown failure code".to_owned(),
            ));
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
            return Err(StorageError::InvalidAuditRecord(
                "audit time interval is empty or reversed".to_owned(),
            ));
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
            return Err(StorageError::InvalidAuditCursor(format!(
                "prefix LSN {as_of_lsn} is beyond current LSN {max_lsn}"
            )));
        }
    }
    if let Some(before_lsn) = spec.before_lsn {
        let before_lsn = lsn_to_i64(before_lsn).map_err(|_| {
            StorageError::InvalidAuditCursor("cursor exceeds SQLite range".to_owned())
        })?;
        if before_lsn > max_lsn {
            return Err(StorageError::InvalidAuditCursor(format!(
                "cursor {before_lsn} is beyond current LSN {max_lsn}"
            )));
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
        clauses.push(format!(
            "a.kind IN ({})",
            vec!["?"; spec.kinds.len()].join(",")
        ));
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
    if let Some(grant_id) = spec.grant_id {
        clauses.push("a.grant_id = ?".to_owned());
        values.push(Value::Text(grant_id.value));
    }
    if let Some(target) = spec.target {
        clauses.push("a.target_key = ?".to_owned());
        values.push(Value::Text(target.as_str().to_owned()));
    }
    if !spec.failure_codes.is_empty() {
        clauses.push(format!(
            "a.failure_code IN ({})",
            vec!["?"; spec.failure_codes.len()].join(",")
        ));
        values.extend(
            spec.failure_codes
                .iter()
                .map(|code| Value::Integer(*code as i64)),
        );
    }
    if !spec.reason_codes.is_empty() {
        clauses.push(format!(
            "a.reason_code IN ({})",
            vec!["?"; spec.reason_codes.len()].join(",")
        ));
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
                a.actor_id, a.endpoint_id, a.command_id, a.grant_id, a.target_key,
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
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<i32>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, i32>(12)?,
                row.get::<_, Vec<u8>>(13)?,
            ))
        })
        .map_err(map_read_err)?;
    let mut records = Vec::new();
    for row in rows {
        let (
            lsn,
            seconds,
            nanos,
            kind,
            actor_id,
            endpoint_id,
            command_id,
            grant_id,
            target_key,
            failure_code,
            reason_code,
            source_lsn,
            event_kind,
            payload_bytes,
        ) = row.map_err(map_read_err)?;
        if event_kind != StoredEventKind::AuditRecord as i32 {
            return Err(StorageError::CorruptRecord(format!(
                "audit index LSN {lsn} points to event kind {event_kind}"
            )));
        }
        let envelope = decode_payload(&payload_bytes)?;
        if decode_stored_kind(&envelope, lsn)? != StoredEventKind::AuditRecord {
            return Err(StorageError::CorruptRecord(format!(
                "audit index LSN {lsn} has a non-audit envelope"
            )));
        }
        let record = AuditRecord::decode(envelope.payload.as_slice()).map_err(|error| {
            StorageError::CorruptRecord(format!("cannot decode audit record at LSN {lsn}: {error}"))
        })?;
        validate_audit_index_row(
            authority_domain_id,
            lsn,
            seconds,
            nanos,
            kind,
            actor_id.as_deref(),
            endpoint_id.as_deref(),
            command_id.as_deref(),
            grant_id.as_deref(),
            target_key.as_deref(),
            failure_code,
            &reason_code,
            source_lsn,
            &record,
        )?;
        records.push(record);
    }
    let has_more = records.len() > usize::from(spec.limit);
    if has_more {
        records.truncate(usize::from(spec.limit));
    }
    let next_before_event_id = has_more
        .then(|| {
            records
                .last()
                .and_then(|record| record.audit_event_id.clone())
        })
        .flatten();
    Ok(AuditPage {
        records,
        next_before_event_id,
        has_more,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "the SQL index tuple is validated field-by-field against its canonical protobuf payload"
)]
fn validate_audit_index_row(
    authority_domain_id: &AuthorityDomainId,
    lsn: i64,
    seconds: i64,
    nanos: i32,
    kind: i32,
    actor_id: Option<&str>,
    endpoint_id: Option<&str>,
    command_id: Option<&str>,
    grant_id: Option<&str>,
    target_key: Option<&str>,
    failure_code: Option<i32>,
    reason_code: &str,
    source_lsn: Option<i64>,
    record: &AuditRecord,
) -> Result<(), StorageError> {
    let expected_event_id = event_id(authority_domain_id.clone(), lsn as u64);
    if record.audit_event_id.as_ref() != Some(&expected_event_id)
        || record
            .occurred_at
            .as_ref()
            .map(|time| (time.seconds, time.nanos))
            != Some((seconds, nanos))
        || record.kind != kind
        || record.actor_id.as_ref().map(|id| id.value.as_str()) != actor_id
        || record.endpoint_id.as_ref().map(|id| id.value.as_str()) != endpoint_id
        || record.command_id.as_ref().map(|id| id.value.as_str()) != command_id
        || record.grant_id.as_ref().map(|id| id.value.as_str()) != grant_id
        || target_key_for_scope(record.target_scope.as_ref()).as_deref() != target_key
        || record.failure_code != failure_code.unwrap_or(FailureCode::Unspecified as i32)
        || record.reason_code != reason_code
        || record
            .source_event_id
            .as_ref()
            .and_then(|id| id.lsn.as_ref())
            .map(|lsn| lsn.value as i64)
            != source_lsn
    {
        return Err(StorageError::CorruptRecord(format!(
            "audit index disagrees with log payload at LSN {lsn}"
        )));
    }
    Ok(())
}

impl CoreGenerationStore for RusqliteStorage {
    async fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> Result<Generation, StorageError> {
        if candidate.value == 0 || candidate.value > i64::MAX as u64 {
            return Err(StorageError::InvalidCoreGeneration(candidate.value));
        }
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::LoadOrCreateCoreGeneration {
                authority_domain_id: authority_domain_id.value.clone(),
                candidate,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }
}

fn reject_generic_unaudited_special(payload: &StoredEventPayload) -> Result<(), StorageError> {
    if matches!(
        StoredEventKind::try_from(payload.kind).ok(),
        Some(
            StoredEventKind::SpawnSuccessorEvidenceStaged
                | StoredEventKind::SpawnPromotionCommitted
                | StoredEventKind::QuarantinedRuntimeEvidence
        )
    ) {
        Err(StorageError::UnsupportedOperation)
    } else {
        Ok(())
    }
}

impl Storage for RusqliteStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        reject_generic_unaudited_special(&payload)?;
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
        self.append_dedup_with_payload(
            authority_domain_id,
            key,
            target,
            payload.clone(),
            payload.encode_to_vec(),
        )
        .await
    }

    async fn append_dedup_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
        logical_payload: Vec<u8>,
    ) -> Result<DedupOutcome, StorageError> {
        reject_generic_unaudited_special(&payload)?;
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedup {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_string(),
                payload,
                logical_payload,
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
        self.read_events(authority_domain_id, cursor, Some(as_of_lsn))
            .await
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

    async fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<GrantAppendOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendGrantAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                identity: identity.as_str().to_owned(),
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

    async fn append_spawn_successor_staged_idempotent(
        &self,
        authority_domain_id: &AuthorityDomainId,
        staged: SpawnSuccessorEvidenceStaged,
    ) -> Result<EventId, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendSpawnSuccessorStagedIdempotent {
                authority_domain_id: authority_domain_id.value.clone(),
                staged: Box::new(staged),
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_spawn_promotion_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        promotion: SpawnPromotionCommitted,
        audit: AuditRecordDraft,
    ) -> Result<SpawnPromotionAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendSpawnPromotionAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                promotion: Box::new(promotion),
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_quarantined_runtime_evidence_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        quarantined: QuarantinedRuntimeEvidence,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendQuarantinedRuntimeEvidenceAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                quarantined: Box::new(quarantined),
                audit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| StorageError::Unavailable("writer actor closed".to_owned()))?;
        reply_rx
            .await
            .map_err(|_| StorageError::Unavailable("writer actor dropped reply".to_owned()))?
    }

    async fn append_decision_audited_many(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audits: Vec<AuditRecordDraft>,
    ) -> Result<AuditedDecisionAppend, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDecisionAuditedMany {
                authority_domain_id: authority_domain_id.value.clone(),
                source,
                audits,
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
        self.append_dedup_audited_with_payload(
            authority_domain_id,
            key,
            target,
            source.clone(),
            audit,
            source.encode_to_vec(),
        )
        .await
    }

    async fn append_dedup_audited_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        logical_payload: Vec<u8>,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedupAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_owned(),
                source,
                audit,
                logical_payload,
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
            // Preserve unknown/unspecified numeric framing for the shared
            // complete-prefix validator. SQLite still owns exact agreement
            // between its indexed kind column and the stored envelope.
            if payload.kind != sql_kind {
                return Err(StorageError::CorruptRecord(format!(
                    "kind mismatch at LSN {lsn}: SQL column says {sql_kind}, envelope says {}",
                    payload.kind
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
        logical_payload: Vec<u8>,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.writer_tx
            .send(WriterCommand::AppendDedupAudited {
                authority_domain_id: authority_domain_id.value.clone(),
                key: key.value.clone(),
                target: target.as_str().to_owned(),
                source,
                audit,
                logical_payload,
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
