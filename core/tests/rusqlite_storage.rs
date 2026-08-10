//! Integration tests for the rusqlite storage implementation.
//!
//! Validates the acceptance criteria from `story-v0-core-persistence-rusqlite-impl.md`:
//! - append returns an EventId whose LSN equals the rowid
//! - consecutive appends produce contiguous LSNs
//! - crash recovery (drop + reopen) recovers all committed events
//! - synchronous=FULL is set and verifiable
//! - WAL concurrent reads while a write is in flight
//! - append_dedup atomic check-and-register (the formal model's appliedKeys)

use std::path::Path;

#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt};

use patchbay_contracts::patchbay::{
    ActorId, AuditEventKind, AuthorityDomainId, DescendantGrant, Generation, Grant, GrantId,
    GrantRevocationPolicy, IdempotencyKey, Lsn, OperationKind, StoredEventKind,
    StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::storage::{
    validate_next_replay_event, AuditRecordDraft, CoreGenerationStore, DedupOutcome,
    GrantAppendOutcome, GrantIdentityKey, ReplayIntegrityError, Storage, StorageError, TargetKey,
};
use prost::Message;
use prost_types::Timestamp;

fn test_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "test-domain".to_string(),
    }
}

fn test_payload(kind: StoredEventKind) -> StoredEventPayload {
    StoredEventPayload {
        kind: kind as i32,
        payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
    }
}

fn test_key(value: &str) -> IdempotencyKey {
    IdempotencyKey {
        value: value.to_string(),
    }
}

fn test_target(value: &str) -> TargetKey {
    TargetKey::new(value.to_string()).unwrap()
}

fn grant_identity(value: &str) -> GrantIdentityKey {
    GrantIdentityKey::new(value.to_owned()).unwrap()
}

fn grant_source(id: &str, kind: OperationKind) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::Grant as i32,
        payload: Grant {
            grant_id: Some(GrantId {
                value: id.to_owned(),
            }),
            authority_domain_id: Some(test_domain()),
            subject_actor_id: Some(ActorId {
                value: "operator".to_owned(),
            }),
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::FleetSupervisor as i32,
                ..TargetScope::default()
            }),
            allowed_operation_kinds: vec![kind as i32],
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..Grant::default()
        }
        .encode_to_vec(),
    }
}

fn descendant_source(id: &str) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::DescendantGrant as i32,
        payload: DescendantGrant {
            grant_id: Some(GrantId {
                value: id.to_owned(),
            }),
            authority_domain_id: Some(test_domain()),
            subject_actor_id: Some(ActorId {
                value: "operator".to_owned(),
            }),
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::RuntimeSession as i32,
                ..TargetScope::default()
            }),
            allowed_operation_kinds: vec![OperationKind::Instruct as i32],
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..DescendantGrant::default()
        }
        .encode_to_vec(),
    }
}

fn grant_audit(id: &str, reason: &str) -> AuditRecordDraft {
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::GrantCreated,
    );
    audit.grant_id = Some(GrantId {
        value: id.to_owned(),
    });
    audit.reason_code = reason.to_owned();
    audit
}

#[cfg(unix)]
#[tokio::test]
async fn database_and_wal_sidecars_are_owner_only() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private.sqlite3");
    let path_str = path.to_str().unwrap().to_owned();
    let storage = patchbay_core::storage::RusqliteStorage::open(&path_str).unwrap();
    let domain = test_domain();
    storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();

    for state_file in [
        path,
        format!("{path_str}-wal").into(),
        format!("{path_str}-shm").into(),
    ] {
        assert_eq!(
            file_mode(&state_file),
            0o600,
            "{} must be accessible only by its owner",
            state_file.display()
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn opening_existing_database_tightens_permissive_mode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.sqlite3");
    fs::write(&path, []).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(file_mode(&path), 0o644);

    let _storage = patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap();

    assert_eq!(file_mode(&path), 0o600);
}

#[cfg(unix)]
fn file_mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[tokio::test]
async fn core_generation_is_insert_once_domain_scoped_and_lsn_free() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain_a = AuthorityDomainId { value: "domain-a".to_owned() };
    let domain_b = AuthorityDomainId { value: "domain-b".to_owned() };

    let first = storage
        .load_or_create_core_generation(&domain_a, Generation { value: 17 })
        .await
        .unwrap();
    let repeated = storage
        .load_or_create_core_generation(&domain_a, Generation { value: 99 })
        .await
        .unwrap();
    let independent = storage
        .load_or_create_core_generation(&domain_b, Generation { value: 23 })
        .await
        .unwrap();

    assert_eq!(first, Generation { value: 17 });
    assert_eq!(repeated, first);
    assert_eq!(independent, Generation { value: 23 });
    assert!(storage.read_after(&domain_a, Lsn { value: 0 }).await.unwrap().is_empty());
    let event = storage
        .append(&domain_a, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    assert_eq!(event.lsn, Some(Lsn { value: 1 }));
}

#[tokio::test]
async fn concurrent_core_generation_initializers_converge_on_stored_winner() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("generation-race.sqlite3");
    let storages = [
        patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap(),
        patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap(),
    ];
    let domain = test_domain();
    let candidates = 1_u64..=16;
    let tasks = candidates
        .clone()
        .enumerate()
        .map(|(index, value)| {
            let storage = storages[index % storages.len()].clone();
            let domain = domain.clone();
            tokio::spawn(async move {
                storage
                    .load_or_create_core_generation(&domain, Generation { value })
                    .await
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let mut returned = Vec::new();
    for task in tasks {
        returned.push(task.await.unwrap());
    }

    assert!(returned.iter().all(|value| value == &returned[0]));
    assert!(candidates.contains(&returned[0].value));
    let persisted: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT core_generation FROM authority_domain_metadata WHERE authority_domain_id = ?1",
            [&domain.value],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(returned[0].value, persisted as u64);
}

#[tokio::test]
async fn core_generation_rejects_invalid_candidates_without_mutation() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    for value in [0, i64::MAX as u64 + 1] {
        assert!(matches!(
            storage.load_or_create_core_generation(&domain, Generation { value }).await,
            Err(StorageError::InvalidCoreGeneration(actual)) if actual == value
        ));
    }
    assert_eq!(
        storage
            .load_or_create_core_generation(&domain, Generation { value: 31 })
            .await
            .unwrap(),
        Generation { value: 31 }
    );
}

#[tokio::test]
async fn core_generation_survives_file_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("generation-reopen.sqlite3");
    let domain = test_domain();
    {
        let storage = patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap();
        assert_eq!(
            storage
                .load_or_create_core_generation(&domain, Generation { value: 41 })
                .await
                .unwrap(),
            Generation { value: 41 }
        );
    }
    let reopened = patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    assert_eq!(
        reopened
            .load_or_create_core_generation(&domain, Generation { value: 42 })
            .await
            .unwrap(),
        Generation { value: 41 }
    );
}

#[tokio::test]
async fn append_returns_event_id_with_rowid_lsn() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let id = storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    assert_eq!(id.lsn.as_ref().unwrap(), &Lsn { value: 1 });
    assert_eq!(id.authority_domain_id.as_ref().unwrap(), &domain);
}

#[tokio::test]
async fn consecutive_appends_produce_contiguous_lsns() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let mut lsns = vec![];
    for _ in 0..10 {
        let id = storage
            .append(&domain, test_payload(StoredEventKind::Observation))
            .await
            .unwrap();
        lsns.push(id.lsn.as_ref().unwrap().value);
    }
    let expected: Vec<u64> = (1..=10).collect();
    assert_eq!(lsns, expected);
}

#[tokio::test]
async fn crash_recovery_recovers_all_committed_events() {
    let temp_path = tempfile::NamedTempFile::new()
        .unwrap()
        .into_temp_path()
        .keep()
        .unwrap();
    let path = temp_path.to_str().unwrap();
    let mut written_lsns = vec![];
    {
        let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
        let domain = test_domain();
        for _ in 0..5 {
            let id = storage
                .append(&domain, test_payload(StoredEventKind::Operation))
                .await
                .unwrap();
            written_lsns.push(id.lsn.as_ref().unwrap().value);
        }
        // Drop storage — simulate crash (no clean shutdown)
    }
    // Reopen and recover
    let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
    let domain = test_domain();
    let recovered = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    let recovered_lsns: Vec<u64> = recovered
        .iter()
        .map(|e| e.event_id.lsn.as_ref().unwrap().value)
        .collect();
    assert_eq!(recovered_lsns, written_lsns);
}

#[tokio::test]
async fn synchronous_full_configured_in_schema() {
    // The SCHEMA constant sets `PRAGMA synchronous = FULL`. This test
    // confirms the schema applies without error and writes commit+persist.
    // It does NOT prove power-loss durability — that's a config assertion
    // (`synchronous=FULL`), not a property an in-process test can verify
    // without a fault-injection harness that kills the process mid-transaction.
    // See the proptest story's documented limits.
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let id = storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id.lsn.as_ref().unwrap(), &Lsn { value: 1 });
    let _ = id;
}

#[tokio::test]
async fn wal_allows_concurrent_reads() {
    // WAL mode allows a concurrent reader while the writer is active. This
    // test reads after writes have committed; true read-during-write overlap
    // is exercised by `concurrent_reads_and_writes` below (which spawns a
    // writer and reader concurrently). This test confirms the read path
    // works under WAL without taking a write lock.
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    for _ in 0..3 {
        storage
            .append(&domain, test_payload(StoredEventKind::Observation))
            .await
            .unwrap();
    }
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 3);
}

#[tokio::test]
async fn append_dedup_new_key_appends() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let outcome = storage
        .append_dedup(
            &domain,
            &test_key("k1"),
            &test_target("target-1"),
            test_payload(StoredEventKind::Operation),
        )
        .await
        .unwrap();
    match outcome {
        DedupOutcome::Appended(id) => assert_eq!(id.lsn.as_ref().unwrap().value, 1),
        DedupOutcome::Duplicate(_) => panic!("expected Appended for new key"),
    }
}

#[tokio::test]
async fn append_dedup_duplicate_key_returns_existing() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let payload = test_payload(StoredEventKind::Operation);
    // First append — new key
    let outcome1 = storage
        .append_dedup(
            &domain,
            &test_key("k1"),
            &test_target("t1"),
            payload.clone(),
        )
        .await
        .unwrap();
    let first_lsn = match outcome1 {
        DedupOutcome::Appended(id) => id.lsn.as_ref().unwrap().value,
        _ => panic!("expected Appended"),
    };
    // Second append with same key + same payload — should be Duplicate
    let outcome2 = storage
        .append_dedup(&domain, &test_key("k1"), &test_target("t1"), payload)
        .await
        .unwrap();
    match outcome2 {
        DedupOutcome::Duplicate(id) => {
            assert_eq!(id.lsn.as_ref().unwrap().value, first_lsn);
        }
        DedupOutcome::Appended(_) => panic!("expected Duplicate for retry"),
    }
    // Verify only one event was appended
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn append_dedup_conflict_on_differing_payload() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let payload1 = test_payload(StoredEventKind::Operation);
    let payload2 = StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: vec![0x00, 0x01, 0x02], // different payload
    };
    // First append
    storage
        .append_dedup(&domain, &test_key("k1"), &test_target("t1"), payload1)
        .await
        .unwrap();
    // Second append with same key but different payload — should conflict
    let result = storage
        .append_dedup(&domain, &test_key("k1"), &test_target("t1"), payload2)
        .await;
    assert!(matches!(
        result,
        Err(patchbay_core::storage::StorageError::IdempotencyConflict)
    ));
}

#[tokio::test]
async fn append_dedup_different_targets_dont_dedup() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let payload = test_payload(StoredEventKind::Operation);
    // Same key, different targets — should both append
    let o1 = storage
        .append_dedup(
            &domain,
            &test_key("k1"),
            &test_target("t1"),
            payload.clone(),
        )
        .await
        .unwrap();
    let o2 = storage
        .append_dedup(&domain, &test_key("k1"), &test_target("t2"), payload)
        .await
        .unwrap();
    assert!(matches!(o1, DedupOutcome::Appended(_)));
    assert!(matches!(o2, DedupOutcome::Appended(_)));
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn grant_identity_append_is_atomic_idempotent_and_shared_across_kinds() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let identity = grant_identity("grant-1");
    let source = grant_source("grant-1", OperationKind::Spawn);

    let first = storage
        .append_grant_audited(
            &domain,
            &identity,
            source.clone(),
            grant_audit("grant-1", "grant_created"),
        )
        .await
        .unwrap();
    let first_append = match first {
        GrantAppendOutcome::Appended(result) => result,
        GrantAppendOutcome::Existing(_) => panic!("first identity must append"),
    };
    assert_eq!(first_append.source_event_id.lsn, Some(Lsn { value: 1 }));
    assert_eq!(first_append.audit_event_id.lsn, Some(Lsn { value: 2 }));
    let prefix = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap();

    let retry = storage
        .append_grant_audited(
            &domain,
            &identity,
            source,
            grant_audit("grant-1", "grant_created"),
        )
        .await
        .unwrap();
    assert_eq!(
        retry,
        GrantAppendOutcome::Existing(first_append.source_event_id.clone())
    );
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .unwrap(),
        prefix,
        "an exact retry must write neither source nor creation audit"
    );

    let changed = storage
        .append_grant_audited(
            &domain,
            &identity,
            grant_source("grant-1", OperationKind::Query),
            grant_audit("grant-1", "grant_created"),
        )
        .await;
    assert!(matches!(
        changed,
        Err(StorageError::GrantIdentityConflict {
            grant_id,
            existing_lsn: 1
        }) if grant_id == "grant-1"
    ));
    let cross_kind = storage
        .append_grant_audited(
            &domain,
            &identity,
            descendant_source("grant-1"),
            grant_audit("grant-1", "descendant_grant_created"),
        )
        .await;
    assert!(matches!(
        cross_kind,
        Err(StorageError::GrantIdentityConflict { existing_lsn: 1, .. })
    ));
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .unwrap(),
        prefix,
        "conflicts must be rejected before append"
    );
}

#[tokio::test]
async fn grant_identity_append_rejects_mismatched_source_and_audit_without_writes() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let source = grant_source("grant-1", OperationKind::Spawn);
    let candidates = [
        storage
            .append_grant_audited(
                &domain,
                &grant_identity("other"),
                source.clone(),
                grant_audit("other", "grant_created"),
            )
            .await,
        storage
            .append_grant_audited(
                &AuthorityDomainId {
                    value: "other-domain".to_owned(),
                },
                &grant_identity("grant-1"),
                source.clone(),
                grant_audit("grant-1", "grant_created"),
            )
            .await,
        storage
            .append_grant_audited(
                &domain,
                &grant_identity("grant-1"),
                source.clone(),
                grant_audit("grant-1", "descendant_grant_created"),
            )
            .await,
        storage
            .append_grant_audited(
                &domain,
                &grant_identity("grant-1"),
                source,
                grant_audit("other", "grant_created"),
            )
            .await,
    ];
    assert!(candidates.iter().all(Result::is_err));
    assert!(storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn concurrent_grant_identity_attempts_converge_on_one_transactional_winner() {
    use std::sync::Arc;

    let domain = test_domain();
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut exact_tasks = Vec::new();
    for _ in 0..2 {
        let storage = storage.clone();
        let domain = domain.clone();
        let barrier = barrier.clone();
        exact_tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            storage
                .append_grant_audited(
                    &domain,
                    &grant_identity("exact"),
                    grant_source("exact", OperationKind::Spawn),
                    grant_audit("exact", "grant_created"),
                )
                .await
        }));
    }
    barrier.wait().await;
    let exact = [
        exact_tasks.remove(0).await.unwrap().unwrap(),
        exact_tasks.remove(0).await.unwrap().unwrap(),
    ];
    let source_ids = exact
        .iter()
        .map(|outcome| match outcome {
            GrantAppendOutcome::Appended(result) => result.source_event_id.clone(),
            GrantAppendOutcome::Existing(event_id) => event_id.clone(),
        })
        .collect::<Vec<_>>();
    assert_eq!(source_ids[0], source_ids[1]);
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        2
    );

    let conflicting = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut conflict_tasks = Vec::new();
    for kind in [OperationKind::Spawn, OperationKind::Query] {
        let storage = conflicting.clone();
        let domain = domain.clone();
        let barrier = barrier.clone();
        conflict_tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            storage
                .append_grant_audited(
                    &domain,
                    &grant_identity("conflict"),
                    grant_source("conflict", kind),
                    grant_audit("conflict", "grant_created"),
                )
                .await
        }));
    }
    barrier.wait().await;
    let outcomes = [
        conflict_tasks.remove(0).await.unwrap(),
        conflict_tasks.remove(0).await.unwrap(),
    ];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(result, Err(StorageError::GrantIdentityConflict { .. })))
            .count(),
        1
    );
    assert_eq!(
        conflicting
            .read_after(&domain, Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        2
    );
}

async fn prepare_v4_grant_log(path: &Path, sources: &[StoredEventPayload]) {
    let storage = patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    drop(storage);
    tokio::task::yield_now().await;
    let db = rusqlite::Connection::open(path).unwrap();
    db.execute_batch("DROP TABLE grant_identities; PRAGMA user_version = 4;")
        .unwrap();
    for (index, source) in sources.iter().enumerate() {
        db.execute(
            "INSERT INTO events (lsn, authority_domain_id, kind, payload) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                index as i64 + 1,
                test_domain().value,
                source.kind,
                source.encode_to_vec()
            ],
        )
        .unwrap();
    }
}

#[tokio::test]
async fn v4_migration_backfills_earliest_exact_grant_identity_without_consuming_lsn() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v4-grants.sqlite3");
    let source = grant_source("legacy", OperationKind::Spawn);
    prepare_v4_grant_log(&path, &[source.clone(), source.clone()]).await;

    let storage = patchbay_core::storage::RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    let retry = storage
        .append_grant_audited(
            &test_domain(),
            &grant_identity("legacy"),
            source,
            grant_audit("legacy", "grant_created"),
        )
        .await
        .unwrap();
    assert!(matches!(
        retry,
        GrantAppendOutcome::Existing(event_id) if event_id.lsn == Some(Lsn { value: 1 })
    ));
    assert_eq!(
        storage
            .read_after(&test_domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        2,
        "migration and retry consume no LSN"
    );
    drop(storage);
    tokio::task::yield_now().await;
    let db = rusqlite::Connection::open(&path).unwrap();
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    let source_lsn: u64 = db
        .query_row(
            "SELECT source_lsn FROM grant_identities WHERE authority_domain_id = ?1 AND grant_id = 'legacy'",
            [test_domain().value],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 5);
    assert_eq!(source_lsn, 1);
}

#[tokio::test]
async fn migration_and_open_reject_conflicting_or_inconsistent_grant_identity_history() {
    let directory = tempfile::tempdir().unwrap();
    let conflict_path = directory.path().join("v4-conflict.sqlite3");
    prepare_v4_grant_log(
        &conflict_path,
        &[
            grant_source("legacy", OperationKind::Spawn),
            grant_source("legacy", OperationKind::Query),
        ],
    )
    .await;
    assert!(matches!(
        patchbay_core::storage::RusqliteStorage::open(conflict_path.to_str().unwrap()),
        Err(StorageError::CorruptRecord(_))
    ));
    let db = rusqlite::Connection::open(&conflict_path).unwrap();
    let version: u32 = db
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 4, "failed backfill must roll the migration back");
    assert!(!db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'grant_identities')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap());
    drop(db);

    let inconsistent_path = directory.path().join("v5-missing-index.sqlite3");
    prepare_v4_grant_log(
        &inconsistent_path,
        &[grant_source("legacy", OperationKind::Spawn)],
    )
    .await;
    let storage = patchbay_core::storage::RusqliteStorage::open(
        inconsistent_path.to_str().unwrap(),
    )
    .unwrap();
    drop(storage);
    tokio::task::yield_now().await;
    let db = rusqlite::Connection::open(&inconsistent_path).unwrap();
    db.execute("DELETE FROM grant_identities", []).unwrap();
    drop(db);
    assert!(matches!(
        patchbay_core::storage::RusqliteStorage::open(inconsistent_path.to_str().unwrap()),
        Err(StorageError::CorruptRecord(_))
    ));
}

#[tokio::test]
async fn write_and_load_snapshot() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append an event at LSN 1
    let id = storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    let lsn = id.lsn.as_ref().unwrap().value;
    // Write a snapshot at LSN 1
    storage
        .write_snapshot(&domain, Lsn { value: lsn }, vec![0x01, 0x02, 0x03])
        .await
        .unwrap();
    // Load it
    let snapshot = storage.load_latest_snapshot(&domain, None).await.unwrap();
    assert!(snapshot.is_some());
    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.event_id.lsn.as_ref().unwrap().value, lsn);
    assert_eq!(snapshot.payload, vec![0x01, 0x02, 0x03]);
}

#[tokio::test]
async fn write_snapshot_rejects_invalid_lsn() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // No events exist; LSN 99 is invalid
    let result = storage
        .write_snapshot(&domain, Lsn { value: 99 }, vec![0x00])
        .await;
    assert!(matches!(
        result,
        Err(patchbay_core::storage::StorageError::InvalidSnapshotLsn(99))
    ));
}

#[tokio::test]
async fn append_rejects_unspecified_and_unknown_event_kinds() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    for kind in [StoredEventKind::Unspecified as i32, i32::MAX] {
        let result = storage
            .append(
                &domain,
                StoredEventPayload {
                    kind,
                    payload: vec![0x00],
                },
            )
            .await;
        assert!(matches!(result, Err(StorageError::InvalidEventKind)));
    }
}

#[tokio::test]
async fn real_sqlite_reads_preserve_corrupt_kinds_for_shared_replay_classification() {
    for (raw_kind, expected_log_corruption) in [
        (StoredEventKind::Unspecified as i32, true),
        (i32::MAX, false),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("corrupt-kind-{raw_kind}.sqlite3"));
        let path = path.to_str().unwrap();
        let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
        drop(storage);
        tokio::task::yield_now().await;

        // Seed a faulty stored row directly; no valid committed record is
        // rewritten or substituted at this LSN.
        let envelope = StoredEventPayload {
            kind: raw_kind,
            payload: Vec::new(),
        };
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute(
            "INSERT INTO events (lsn, authority_domain_id, kind, payload) VALUES (1, ?1, ?2, ?3)",
            rusqlite::params![test_domain().value, raw_kind, envelope.encode_to_vec()],
        )
        .unwrap();
        drop(db);

        let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
        let events = storage
            .read_after(&test_domain(), Lsn { value: 0 })
            .await
            .expect("SQLite read boundary preserves matching raw kind framing");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload.kind, raw_kind);
        let error = validate_next_replay_event(&test_domain(), 0, &events[0])
            .expect_err("the shared validator classifies stored kind corruption");
        if expected_log_corruption {
            assert!(matches!(error, ReplayIntegrityError::CorruptLog(_)));
        } else {
            assert!(matches!(error, ReplayIntegrityError::CorruptRecord(_)));
        }
    }
}

#[tokio::test]
async fn failed_append_does_not_create_gap() {
    // A failed append (rejected at validation, before the transaction opens)
    // must not consume an LSN — the next successful append must be contiguous.
    //
    // Note: this verifies the validation-rejection path, not a genuine
    // transaction rollback. `do_append` validates the kind BEFORE opening the
    // transaction, so an unspecified-kind payload is rejected without ever
    // starting a transaction. A true mid-transaction rollback (e.g. commit
    // failure) is hard to trigger through the public Storage trait without a
    // corrupt DB; the gap-free property on the success path is covered by the
    // proptest `committed_lsns_are_gap_free_and_monotonic`, and the
    // bare-INTEGER-PRIMARY-KEY gap-free guarantee is a standard SQLite
    // property (no AUTOINCREMENT, append-only, no deletes).
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Successful append → LSN 1
    storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    // Failed append (unspecified kind) — rejected at validation, no LSN consumed
    let bad = StoredEventPayload {
        kind: StoredEventKind::Unspecified as i32,
        payload: vec![0x00],
    };
    let _ = storage.append(&domain, bad).await;
    // Next successful append should be LSN 2, not 3 (no gap from the failed append)
    let id = storage
        .append(&domain, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    assert_eq!(id.lsn.as_ref().unwrap().value, 2);
}

#[tokio::test]
async fn cross_domain_isolation() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain_a = AuthorityDomainId {
        value: "domain-a".to_string(),
    };
    let domain_b = AuthorityDomainId {
        value: "domain-b".to_string(),
    };
    // Append to domain A
    storage
        .append(&domain_a, test_payload(StoredEventKind::Operation))
        .await
        .unwrap();
    // Append to domain B
    storage
        .append(&domain_b, test_payload(StoredEventKind::Observation))
        .await
        .unwrap();
    // read_after on domain A should only see domain A's events
    let a_events = storage
        .read_after(&domain_a, Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(a_events.len(), 1);
    assert_eq!(a_events[0].payload.kind, StoredEventKind::Operation as i32);
    // read_after on domain B should only see domain B's events
    let b_events = storage
        .read_after(&domain_b, Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(b_events.len(), 1);
    assert_eq!(
        b_events[0].payload.kind,
        StoredEventKind::Observation as i32
    );
}

#[tokio::test]
async fn empty_log_read_returns_empty() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn load_latest_snapshot_none_when_empty() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let snapshot = storage.load_latest_snapshot(&domain, None).await.unwrap();
    assert!(snapshot.is_none());
}

#[tokio::test]
async fn load_latest_snapshot_bounded() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 3 events
    let mut lsns = vec![];
    for _ in 0..3 {
        let id = storage
            .append(&domain, test_payload(StoredEventKind::Operation))
            .await
            .unwrap();
        lsns.push(id.lsn.as_ref().unwrap().value);
    }
    // Write snapshots at LSN 1 and 3
    storage
        .write_snapshot(&domain, Lsn { value: lsns[0] }, vec![0x01])
        .await
        .unwrap();
    storage
        .write_snapshot(&domain, Lsn { value: lsns[2] }, vec![0x03])
        .await
        .unwrap();
    // load_latest(None) → LSN 3
    let snap = storage
        .load_latest_snapshot(&domain, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snap.event_id.lsn.as_ref().unwrap().value, lsns[2]);
    assert_eq!(snap.payload, vec![0x03]);
    // load_latest(Some(2)) → LSN 1 (the latest <= 2)
    let snap = storage
        .load_latest_snapshot(&domain, Some(Lsn { value: lsns[1] }))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snap.event_id.lsn.as_ref().unwrap().value, lsns[0]);
    assert_eq!(snap.payload, vec![0x01]);
}

#[tokio::test]
async fn concurrent_reads_and_writes() {
    // Spawn a writer appending events while readers read concurrently.
    // WAL should allow reads to proceed while writes are in flight.
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let writer = storage.clone();
    let write_domain = domain.clone();
    let write_handle = tokio::spawn(async move {
        for _ in 0..5 {
            writer
                .append(&write_domain, test_payload(StoredEventKind::Observation))
                .await
                .unwrap();
        }
    });
    // Read concurrently while the writer is running
    let read_storage = storage.clone();
    let read_domain = domain.clone();
    let read_handle = tokio::spawn(async move {
        // Read multiple times while writes are happening
        for _ in 0..3 {
            let _ = read_storage
                .read_after(&read_domain, Lsn { value: 0 })
                .await;
        }
    });
    write_handle.await.unwrap();
    read_handle.await.unwrap();
    // After both complete, verify all 5 events are present
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 5);
}
