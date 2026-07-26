use patchbay_contracts::patchbay::{AuditEventKind, AuthorityDomainId, StoredEventKind, StoredEventPayload};
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
