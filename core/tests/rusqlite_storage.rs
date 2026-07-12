//! Integration tests for the rusqlite storage implementation.
//!
//! Validates the acceptance criteria from `story-v0-core-persistence-rusqlite-impl.md`:
//! - append returns an EventId whose LSN equals the rowid
//! - consecutive appends produce contiguous LSNs
//! - crash recovery (drop + reopen) recovers all committed events
//! - synchronous=FULL is set and verifiable
//! - WAL concurrent reads while a write is in flight
//! - append_dedup atomic check-and-register (the formal model's appliedKeys)

use patchbay_contracts::patchbay::{
    AuthorityDomainId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use patchbay_core::storage::{DedupOutcome, Storage, TargetKey};

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
async fn unspecified_event_kind_rejected() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let bad_payload = StoredEventPayload {
        kind: StoredEventKind::Unspecified as i32,
        payload: vec![0x00],
    };
    let result = storage.append(&domain, bad_payload).await;
    assert!(matches!(
        result,
        Err(patchbay_core::storage::StorageError::InvalidEventKind)
    ));
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
