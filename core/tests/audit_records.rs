use patchbay_contracts::patchbay::{
    AdapterId, AuditEventKind, AuthorityDomainId, ResourceId, ResourceIdentity, ResourceKind,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use prost::Message;
use patchbay_core::storage::{
    AuditPageSpec, AuditRecordDraft, AuditedDedupOutcome, CoreGenerationStore, RusqliteStorage,
    Storage, StorageError, TargetKey,
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

fn target_key(scope: &TargetScope) -> TargetKey {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = scope.encode_to_vec();
    let encoded = bytes
        .iter()
        .flat_map(|byte| [HEX[(byte >> 4) as usize] as char, HEX[(byte & 0x0f) as usize] as char])
        .collect();
    TargetKey::new(encoded).unwrap()
}

fn page(limit: u16) -> AuditPageSpec {
    AuditPageSpec {
        kinds: Vec::new(),
        actor_id: None,
        endpoint_id: None,
        command_id: None,
        grant_id: None,
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
async fn legacy_tag_eight_and_nested_resource_targets_remain_durably_filterable() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let domain = domain("main");

    // Decode the pre-rename wire shape directly: kind=RESOURCE, tag 8 string.
    let old_bytes = [
        0x08,
        TargetScopeKind::Resource as u8,
        0x42,
        0x09,
        b'p', b'r', b'i', b'n', b'c', b'i', b'p', b'a', b'l',
    ];
    let legacy = TargetScope::decode(old_bytes.as_slice()).unwrap();
    let mut legacy_draft = draft(AuditEventKind::ControlSurfacePrincipalRevoked, "principal_revoked");
    legacy_draft.target_scope = Some(legacy.clone());
    storage.append_audit(&domain, legacy_draft).await.unwrap();

    let nested = TargetScope {
        kind: TargetScopeKind::Resource as i32,
        resource: Some(ResourceIdentity {
            adapter_id: Some(AdapterId { value: "adapter-a".to_owned() }),
            resource_id: Some(ResourceId { value: "shared".to_owned() }),
            resource_kind: Some(ResourceKind { value: "pool".to_owned() }),
        }),
        ..TargetScope::default()
    };
    let mut nested_draft = draft(AuditEventKind::CommandSubmissionAccepted, "resource_query");
    nested_draft.target_scope = Some(nested.clone());
    storage.append_audit(&domain, nested_draft).await.unwrap();

    for expected in [legacy, nested] {
        let mut filter = page(10);
        filter.target = Some(target_key(&expected));
        let result = storage.query_audit(&domain, filter).await.unwrap();
        assert_eq!(result.records.len(), 1);
        assert_eq!(result.records[0].target_scope.as_ref(), Some(&expected));
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

#[tokio::test]
async fn v3_to_v4_migration_preserves_all_durable_rows_without_allocating_lsn() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("v3.sqlite3");
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE events (
                lsn INTEGER PRIMARY KEY,
                authority_domain_id TEXT NOT NULL,
                kind INTEGER NOT NULL,
                payload BLOB NOT NULL
            );
            CREATE TABLE idempotency_keys (
                authority_domain_id TEXT NOT NULL,
                key TEXT NOT NULL,
                target TEXT NOT NULL,
                lsn INTEGER NOT NULL,
                payload_bytes BLOB NOT NULL,
                PRIMARY KEY (authority_domain_id, key, target)
            );
            CREATE TABLE snapshots (
                authority_domain_id TEXT NOT NULL,
                snapshot_lsn INTEGER NOT NULL,
                payload BLOB NOT NULL,
                PRIMARY KEY (authority_domain_id, snapshot_lsn)
            );
            CREATE TABLE audit_records (
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
                grant_id TEXT,
                PRIMARY KEY (authority_domain_id, audit_lsn)
            );
            INSERT INTO events VALUES (1, 'main', 1, X'0102');
            INSERT INTO idempotency_keys VALUES ('main', 'key-1', 'target-1', 1, X'0304');
            INSERT INTO snapshots VALUES ('main', 1, X'0506');
            INSERT INTO audit_records (
                authority_domain_id, audit_lsn, occurred_at_seconds, occurred_at_nanos,
                kind, reason_code, source_lsn, grant_id
            ) VALUES ('main', 1, 10, 0, 1, 'preserved', 1, 'grant-1');
            PRAGMA user_version = 3;",
        )
        .unwrap();
    }

    let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        storage
            .load_or_create_core_generation(
                &domain("main"),
                patchbay_contracts::patchbay::Generation { value: 73 },
            )
            .await
            .unwrap()
            .value,
        73
    );
    let db = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = db.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 4);
    for table in ["events", "idempotency_keys", "snapshots", "audit_records"] {
        let count: u64 = db
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "migration changed {table}");
    }
    let max_lsn: u64 = db.query_row("SELECT MAX(lsn) FROM events", [], |row| row.get(0)).unwrap();
    assert_eq!(max_lsn, 1, "metadata initialization must not allocate an event LSN");
}

#[tokio::test]
async fn malformed_v4_metadata_schema_is_rejected_without_repair_or_version_change() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("malformed-v4.sqlite3");
    let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    drop(storage);
    tokio::task::yield_now().await;
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch(
            "DROP TABLE authority_domain_metadata;
             CREATE TABLE authority_domain_metadata (authority_domain_id TEXT PRIMARY KEY);",
        )
        .unwrap();
    }

    assert!(matches!(
        RusqliteStorage::open(path.to_str().unwrap()),
        Err(StorageError::MalformedSchema(_))
    ));
    let db = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = db.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    assert_eq!(version, 4);
    let columns = db
        .prepare("PRAGMA table_info(authority_domain_metadata)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(columns, vec!["authority_domain_id"]);
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
