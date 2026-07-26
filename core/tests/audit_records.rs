use patchbay_contracts::patchbay::{AuditEventKind, AuthorityDomainId, StoredEventKind, StoredEventPayload};
use prost::Message;
use patchbay_core::storage::{
    AuditPageSpec, AuditRecordDraft, AuditedDedupOutcome, RusqliteStorage, Storage, StorageError,
    TargetKey,
};
use prost_types::Timestamp;
use tempfile::TempDir;

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId { value: value.to_owned() }
}

fn draft(kind: AuditEventKind, reason: &str) -> AuditRecordDraft {
    let mut draft = AuditRecordDraft::new(Timestamp { seconds: 10, nanos: 0 }, kind);
    draft.reason_code = reason.to_owned();
    draft
}

fn page(limit: u16) -> AuditPageSpec {
    AuditPageSpec {
        kinds: Vec::new(),
        actor_id: None,
        endpoint_id: None,
        command_id: None,
        target: None,
        failure_codes: Vec::new(),
        reason_codes: Vec::new(),
        occurred_from: None,
        occurred_before: None,
        before_lsn: None,
        limit,
    }
}

#[tokio::test]
async fn audit_records_are_redacted_indexed_and_cursor_bounded() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let domain = domain("main");
    let mut first = draft(AuditEventKind::LoginFailed, "invalid_credentials");
    first.operator_session_hash = vec![7; 32];
    first.source_network = "127.0.0.1".to_owned();
    let first_id = storage.append_audit(&domain, first).await.unwrap();
    let mut second = draft(AuditEventKind::LoginSucceeded, "authenticated");
    second.occurred_at.seconds = 11;
    let second_id = storage.append_audit(&domain, second).await.unwrap();

    let result = storage.query_audit(&domain, page(1)).await.unwrap();
    assert!(result.has_more);
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].audit_event_id, Some(second_id.clone()));
    assert_eq!(result.next_before_event_id, Some(second_id.clone()));

    let mut next = page(1);
    next.before_lsn = Some(second_id.lsn.as_ref().unwrap().value);
    let result = storage.query_audit(&domain, next).await.unwrap();
    assert!(!result.has_more);
    assert_eq!(result.records[0].audit_event_id, Some(first_id));
    assert_eq!(result.records[0].operator_session_hash, vec![7; 32]);

    let mut other = page(1);
    other.before_lsn = Some(999);
    assert!(matches!(storage.query_audit(&domain, other).await, Err(StorageError::InvalidAuditCursor(_))));
}

#[tokio::test]
async fn audited_append_and_dedup_keep_source_and_audit_atomic() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let domain = domain("main");
    let source = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: vec![1, 2, 3],
    };
    let key = patchbay_contracts::patchbay::IdempotencyKey { value: "query-1".to_owned() };
    let target = TargetKey::new("authority".to_owned()).unwrap();
    let result = storage
        .append_dedup_audited(
            &domain,
            &key,
            &target,
            source.clone(),
            draft(AuditEventKind::CommandSubmissionAccepted, "accepted"),
        )
        .await
        .unwrap();
    let source_event_id = match result {
        AuditedDedupOutcome::Appended(result) => result.source_event_id,
        AuditedDedupOutcome::Duplicate { .. } => panic!("first append must not duplicate"),
    };
    let duplicate = storage
        .append_dedup_audited(
            &domain,
            &key,
            &target,
            source,
            draft(AuditEventKind::CommandSubmissionAccepted, "retry"),
        )
        .await
        .unwrap();
    match duplicate {
        AuditedDedupOutcome::Duplicate { source_event_id: actual, .. } => {
            assert_eq!(actual, source_event_id)
        }
        AuditedDedupOutcome::Appended(_) => panic!("retry must reuse source"),
    }
    let events = storage.read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 3, "source plus one audit per submission");
}

#[test]
fn malformed_legacy_schema_is_rejected_without_mutation() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("malformed.sqlite3");
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE events (lsn INTEGER PRIMARY KEY, authority_domain_id TEXT NOT NULL, kind INTEGER NOT NULL);\n             CREATE TABLE idempotency_keys (authority_domain_id TEXT NOT NULL, key TEXT NOT NULL, target TEXT NOT NULL, lsn INTEGER NOT NULL, payload_bytes BLOB NOT NULL);\n             CREATE TABLE snapshots (authority_domain_id TEXT NOT NULL, snapshot_lsn INTEGER NOT NULL, payload BLOB NOT NULL);",
        )
        .unwrap();
    }
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        RusqliteStorage::open(path.to_str().unwrap()),
        Err(StorageError::MalformedSchema(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let db = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = db.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 0, "preflight failure must not stamp the baseline");
}

#[tokio::test]
async fn legacy_data_survives_versioned_migration() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("legacy.sqlite3");
    let payload = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: vec![9, 8, 7],
    };
    let encoded = payload.encode_to_vec();
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE events (lsn INTEGER PRIMARY KEY, authority_domain_id TEXT NOT NULL, kind INTEGER NOT NULL, payload BLOB NOT NULL);\n             CREATE TABLE idempotency_keys (authority_domain_id TEXT NOT NULL, key TEXT NOT NULL, target TEXT NOT NULL, lsn INTEGER NOT NULL, payload_bytes BLOB NOT NULL);\n             CREATE TABLE snapshots (authority_domain_id TEXT NOT NULL, snapshot_lsn INTEGER NOT NULL, payload BLOB NOT NULL);",
        )
        .unwrap();
        db.execute(
            "INSERT INTO events (lsn, authority_domain_id, kind, payload) VALUES (1, ?1, ?2, ?3)",
            rusqlite::params!["main", StoredEventKind::Observation as i32, encoded],
        )
        .unwrap();
    }
    let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    let events = storage
        .read_after(&domain("main"), patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload, payload);
}

#[test]
fn future_schema_is_rejected_before_mutation() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("future.sqlite3");
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("PRAGMA user_version = 77;").unwrap();
    }
    let error = match RusqliteStorage::open(path.to_str().unwrap()) {
        Ok(_) => panic!("future schema must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, StorageError::UnsupportedSchemaVersion(77)));
    let db = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = db.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 77);
}
