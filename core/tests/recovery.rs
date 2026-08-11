//! Tests for crash recovery and replay.
//!
//! Validates the acceptance criteria from `story-v0-core-persistence-recovery.md`:
//! - clean shutdown + restart reconstructs identical state
//! - crash (no clean shutdown) reconstructs up to last committed LSN
//! - replay is idempotent (calling recover() twice produces identical state)
//! - snapshot + tail replay produces state identical to replaying from 0
//! - incompatible typed checkpoints fall back to full replay from LSN 0

extern crate patchbay_test_support;
use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn, StoredEventKind, StoredEventPayload};
use patchbay_core::storage::{recover, RecoveryState, Storage, StoredSnapshot};

fn test_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "test-domain".to_string(),
    }
}

fn test_payload(n: u8) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: vec![n],
    }
}

fn accept_test_snapshot(snapshot: &StoredSnapshot) -> Option<Vec<u8>> {
    Some(snapshot.payload.clone())
}

/// Collect the "state" (payload bytes) from a recovery, for comparison.
/// In a real system the domain layer applies events to in-memory state;
/// here we just collect the payloads to verify reconstruction.
fn collect_state(recovery: &RecoveryState<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut state = Vec::new();
    if let Some(snapshot) = &recovery.snapshot {
        state.push(snapshot.value.clone());
    }
    for event in recovery.events() {
        state.push(event.payload.payload.clone());
    }
    state
}

#[tokio::test]
async fn recover_reconstructs_all_events_no_snapshot() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 5 events
    for i in 0..5u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Recover — no snapshot exists, so replay from 0
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert!(recovery.snapshot.is_none());
    assert_eq!(recovery.tail.len(), 5);
    // Verify the payloads match what we wrote
    let state = collect_state(&recovery);
    assert_eq!(state, vec![vec![0], vec![1], vec![2], vec![3], vec![4]]);
}

#[tokio::test]
async fn recover_after_crash_reconstructs_up_to_last_committed() {
    let temp_path = tempfile::NamedTempFile::new()
        .unwrap()
        .into_temp_path()
        .keep()
        .unwrap();
    let path = temp_path.to_str().unwrap();
    let written_payloads: Vec<Vec<u8>>;
    {
        let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
        let domain = test_domain();
        for i in 0..5u8 {
            storage.append(&domain, test_payload(i)).await.unwrap();
        }
        // Drop storage — simulate crash (no clean shutdown)
        written_payloads = (0..5u8).map(|n| vec![n]).collect();
    }
    // Reopen and recover
    let storage = patchbay_core::storage::RusqliteStorage::open(path).unwrap();
    let domain = test_domain();
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert!(recovery.snapshot.is_none());
    let state = collect_state(&recovery);
    assert_eq!(state, written_payloads);
}

#[tokio::test]
async fn recover_deterministic_for_unchanged_contents() {
    // Two calls with no intervening writes produce identical RecoveryState.
    // (If writes happen between calls, the second may differ — that's correct.)
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    for i in 0..4u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Write a snapshot so the equality covers the complete snapshot + tail result
    storage
        .write_snapshot(&domain, Lsn { value: 2 }, vec![0xEE])
        .await
        .unwrap();
    let recovery1 = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    let recovery2 = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert_eq!(recovery1, recovery2);
}

#[tokio::test]
async fn snapshot_plus_tail_equals_replay_from_zero() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 5 events (LSNs 1-5)
    for i in 0..5u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Write a snapshot at LSN 3
    storage
        .write_snapshot(&domain, Lsn { value: 3 }, vec![0xAA])
        .await
        .unwrap();
    // Recover with snapshot — should load snapshot at LSN 3 + tail (LSNs 4,5)
    let recovery_with_snapshot = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert!(recovery_with_snapshot.snapshot.is_some());
    assert_eq!(
        recovery_with_snapshot
            .snapshot
            .as_ref()
            .unwrap()
            .event_id
            .lsn
            .as_ref()
            .unwrap()
            .value,
        3
    );
    assert_eq!(recovery_with_snapshot.tail.len(), 2);
    let state_with_snapshot = collect_state(&recovery_with_snapshot);
    // state = [snapshot_bytes(0xAA), event4_payload(3), event5_payload(4)]
    assert_eq!(state_with_snapshot, vec![vec![0xAA], vec![3], vec![4]]);

    // Now verify: the events in the tail (LSNs 4,5) are the same as
    // events 4,5 from a full replay from 0.
    // To do this, we compare the tail payloads against the full replay's
    // events at the same positions.
    let full_recovery = {
        // Open a fresh storage with the same data but no snapshot
        // (can't easily remove the snapshot, so just verify the tail matches
        // the corresponding events in a no-snapshot recovery on a separate DB)
        let storage2 = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
        let domain2 = test_domain();
        for i in 0..5u8 {
            storage2.append(&domain2, test_payload(i)).await.unwrap();
        }
        recover(&storage2, &domain2, accept_test_snapshot)
            .await
            .unwrap()
    };
    // full_recovery has no snapshot, all 5 events
    assert!(full_recovery.snapshot.is_none());
    assert_eq!(full_recovery.tail.len(), 5);
    // The tail of the snapshot recovery (events 4,5) should match events 4,5
    // of the full replay (indices 3,4)
    let tail_payloads: Vec<Vec<u8>> = recovery_with_snapshot
        .tail
        .iter()
        .map(|e| e.payload.payload.clone())
        .collect();
    let full_payloads: Vec<Vec<u8>> = full_recovery
        .tail
        .iter()
        .skip(3) // skip events 1,2,3 (already in the snapshot)
        .map(|e| e.payload.payload.clone())
        .collect();
    assert_eq!(tail_payloads, full_payloads);
}

#[tokio::test]
async fn recover_start_lsn_is_snapshot_lsn() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 3 events
    for i in 0..3u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Write a snapshot at LSN 2
    storage
        .write_snapshot(&domain, Lsn { value: 2 }, vec![0xBB])
        .await
        .unwrap();
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert_eq!(recovery.start_lsn().unwrap(), 2);
    // No snapshot case
    let storage2 = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain2 = test_domain();
    storage2.append(&domain2, test_payload(0)).await.unwrap();
    let recovery2 = recover(&storage2, &domain2, accept_test_snapshot)
        .await
        .unwrap();
    assert_eq!(recovery2.start_lsn().unwrap(), 0);
}

#[tokio::test]
async fn recover_empty_log() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert!(recovery.snapshot.is_none());
    assert!(recovery.tail.is_empty());
}

#[tokio::test]
async fn recover_with_snapshot_bounds_replay_cost() {
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 10 events (LSNs 1-10)
    for i in 0..10u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Write a snapshot at LSN 8
    storage
        .write_snapshot(&domain, Lsn { value: 8 }, vec![0xCC])
        .await
        .unwrap();
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    // Should only replay events 9,10 — not all 10
    assert_eq!(recovery.tail.len(), 2);
    assert_eq!(recovery.start_lsn().unwrap(), 8);
    let tail_payloads: Vec<u8> = recovery.tail.iter().map(|e| e.payload.payload[0]).collect();
    assert_eq!(tail_payloads, vec![8, 9]); // payloads of events 9,10
}

#[tokio::test]
async fn recover_snapshot_at_log_head_empty_tail() {
    // Snapshot at the last committed LSN → tail is empty, start_lsn = snapshot LSN.
    let storage = patchbay_core::storage::RusqliteStorage::open_in_memory().unwrap();
    let domain = test_domain();
    // Append 3 events (LSNs 1-3)
    for i in 0..3u8 {
        storage.append(&domain, test_payload(i)).await.unwrap();
    }
    // Write a snapshot at LSN 3 (the last committed)
    storage
        .write_snapshot(&domain, Lsn { value: 3 }, vec![0xDD])
        .await
        .unwrap();
    let recovery = recover(&storage, &domain, accept_test_snapshot)
        .await
        .unwrap();
    assert!(recovery.snapshot.is_some());
    assert_eq!(recovery.start_lsn().unwrap(), 3);
    assert!(recovery.tail.is_empty());
    // State = just the snapshot payload
    assert_eq!(collect_state(&recovery), vec![vec![0xDD]]);
}

use patchbay_contracts::patchbay::{EventId, IdempotencyKey};
use patchbay_core::storage::{RecordedEvent, StorageError, TargetKey};
use prost::Message;

const TEST_CHECKPOINT_KIND: u32 = 1;
const TEST_CHECKPOINT_VERSION: u32 = 1;
const TEST_CHECKPOINT_GENERATION: u64 = 17;
const TEST_CHECKPOINT_PAYLOAD: &[u8] = b"typed-session-state";

#[derive(Clone, PartialEq, Message)]
struct TestCheckpoint {
    #[prost(uint32, tag = "1")]
    kind: u32,
    #[prost(uint32, tag = "2")]
    version: u32,
    #[prost(string, tag = "3")]
    authority_domain_id: String,
    #[prost(uint64, tag = "4")]
    core_generation: u64,
    #[prost(uint64, tag = "5")]
    snapshot_lsn: u64,
    #[prost(bytes = "vec", tag = "6")]
    payload: Vec<u8>,
}

#[derive(Clone)]
struct SnapshotFixtureStorage {
    snapshot: StoredSnapshot,
    events: Vec<RecordedEvent>,
}

impl Storage for SnapshotFixtureStorage {
    async fn append(
        &self,
        _domain: &AuthorityDomainId,
        _payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        unreachable!()
    }

    async fn append_dedup(
        &self,
        _domain: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        _payload: StoredEventPayload,
    ) -> Result<patchbay_core::storage::DedupOutcome, StorageError> {
        unreachable!()
    }

    async fn read_after(
        &self,
        domain: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        Ok(self
            .events
            .iter()
            .filter(|event| {
                event.event_id.authority_domain_id.as_ref() == Some(domain)
                    && event
                        .event_id
                        .lsn
                        .as_ref()
                        .is_some_and(|lsn| lsn.value > cursor.value)
            })
            .cloned()
            .collect())
    }

    async fn write_snapshot(
        &self,
        _domain: &AuthorityDomainId,
        _lsn: Lsn,
        _payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        unreachable!()
    }

    async fn load_latest_snapshot(
        &self,
        _domain: &AuthorityDomainId,
        _at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        Ok(Some(self.snapshot.clone()))
    }
}

fn valid_test_checkpoint() -> StoredSnapshot {
    StoredSnapshot {
        event_id: EventId {
            authority_domain_id: Some(test_domain()),
            lsn: Some(Lsn { value: 2 }),
        },
        payload: TestCheckpoint {
            kind: TEST_CHECKPOINT_KIND,
            version: TEST_CHECKPOINT_VERSION,
            authority_domain_id: test_domain().value,
            core_generation: TEST_CHECKPOINT_GENERATION,
            snapshot_lsn: 2,
            payload: TEST_CHECKPOINT_PAYLOAD.to_vec(),
        }
        .encode_to_vec(),
    }
}

fn fixture(snapshot: StoredSnapshot) -> SnapshotFixtureStorage {
    SnapshotFixtureStorage {
        snapshot,
        events: (1..=3)
            .map(|lsn| RecordedEvent {
                event_id: EventId {
                    authority_domain_id: Some(test_domain()),
                    lsn: Some(Lsn { value: lsn }),
                },
                payload: test_payload(lsn as u8),
            })
            .collect(),
    }
}

fn validate_test_checkpoint(
    stored: &StoredSnapshot,
    domain: &AuthorityDomainId,
) -> Option<Vec<u8>> {
    let checkpoint = TestCheckpoint::decode(stored.payload.as_slice()).ok()?;
    let stored_lsn = stored.event_id.lsn.as_ref()?.value;
    (checkpoint.kind == TEST_CHECKPOINT_KIND
        && checkpoint.version == TEST_CHECKPOINT_VERSION
        && checkpoint.authority_domain_id == domain.value
        && checkpoint.core_generation == TEST_CHECKPOINT_GENERATION
        && checkpoint.snapshot_lsn == stored_lsn
        && checkpoint.payload == TEST_CHECKPOINT_PAYLOAD)
        .then_some(checkpoint.payload)
}

#[tokio::test]
async fn compatible_typed_checkpoint_is_the_only_prefix_skip_authority() {
    let domain = test_domain();
    let recovery = recover(&fixture(valid_test_checkpoint()), &domain, |stored| {
        validate_test_checkpoint(stored, &domain)
    })
    .await
    .unwrap();

    assert_eq!(recovery.start_lsn().unwrap(), 2);
    assert_eq!(
        recovery
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.value.as_slice()),
        Some(TEST_CHECKPOINT_PAYLOAD)
    );
    assert_eq!(recovery.tail.len(), 1);
    assert_eq!(recovery.tail[0].event_id.lsn, Some(Lsn { value: 3 }));
}

#[tokio::test]
async fn incompatible_checkpoint_dimensions_fall_back_to_full_replay() {
    let domain = test_domain();
    let mut candidates = Vec::new();

    let mutate_payload = |mutate: fn(&mut TestCheckpoint)| {
        let mut stored = valid_test_checkpoint();
        let mut checkpoint = TestCheckpoint::decode(stored.payload.as_slice()).unwrap();
        mutate(&mut checkpoint);
        stored.payload = checkpoint.encode_to_vec();
        stored
    };
    candidates.extend([
        mutate_payload(|checkpoint| checkpoint.kind = 2),
        mutate_payload(|checkpoint| checkpoint.version = 2),
        mutate_payload(|checkpoint| checkpoint.authority_domain_id = "other".to_owned()),
        mutate_payload(|checkpoint| checkpoint.core_generation += 1),
        mutate_payload(|checkpoint| checkpoint.snapshot_lsn += 1),
        mutate_payload(|checkpoint| checkpoint.payload.clear()),
    ]);

    let mut corrupt = valid_test_checkpoint();
    corrupt.payload = vec![0xff];
    candidates.push(corrupt);
    let mut wrong_row_domain = valid_test_checkpoint();
    wrong_row_domain.event_id.authority_domain_id = Some(AuthorityDomainId {
        value: "other".to_owned(),
    });
    candidates.push(wrong_row_domain);
    let mut missing_row_lsn = valid_test_checkpoint();
    missing_row_lsn.event_id.lsn = None;
    candidates.push(missing_row_lsn);
    let mut zero_row_lsn = valid_test_checkpoint();
    zero_row_lsn.event_id.lsn = Some(Lsn { value: 0 });
    candidates.push(zero_row_lsn);

    for candidate in candidates {
        let recovery = recover(&fixture(candidate), &domain, |stored| {
            validate_test_checkpoint(stored, &domain)
        })
        .await
        .unwrap();
        assert!(recovery.snapshot.is_none());
        assert_eq!(recovery.start_lsn().unwrap(), 0);
        assert_eq!(
            recovery
                .tail
                .iter()
                .filter_map(|event| event.event_id.lsn.as_ref().map(|lsn| lsn.value))
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }
}

fn snapshot_tail_fixture(events: Vec<RecordedEvent>) -> SnapshotFixtureStorage {
    SnapshotFixtureStorage {
        snapshot: StoredSnapshot {
            event_id: EventId {
                authority_domain_id: Some(test_domain()),
                lsn: Some(Lsn { value: 5 }),
            },
            payload: vec![0xA5],
        },
        events,
    }
}

fn recovery_event(lsn: u64, kind: StoredEventKind) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(test_domain()),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: vec![lsn as u8],
        },
    }
}

#[tokio::test]
async fn recovery_validates_complete_snapshot_tail_from_snapshot_cursor() {
    let storage = snapshot_tail_fixture(vec![
        recovery_event(6, StoredEventKind::Grant),
        recovery_event(7, StoredEventKind::Observation),
    ]);
    let recovered = recover(&storage, &test_domain(), |snapshot| {
        Some(snapshot.payload.clone())
    })
    .await
    .unwrap();

    assert_eq!(recovered.start_lsn().unwrap(), 5);
    assert_eq!(
        recovered
            .tail
            .iter()
            .map(|event| event.event_id.lsn.as_ref().unwrap().value)
            .collect::<Vec<_>>(),
        vec![6, 7]
    );
}

#[tokio::test]
async fn recovery_rejects_missing_snapshot_successor_and_unspecified_kind() {
    for events in [
        vec![recovery_event(7, StoredEventKind::Grant)],
        vec![recovery_event(6, StoredEventKind::Unspecified)],
    ] {
        let error = recover(&snapshot_tail_fixture(events), &test_domain(), |snapshot| {
            Some(snapshot.payload.clone())
        })
        .await
        .expect_err("corrupt complete snapshot tail must fail closed");
        assert!(matches!(error, StorageError::CorruptRecord(_)));
        assert!(error.to_string().contains("corrupt replay"));
    }
}
