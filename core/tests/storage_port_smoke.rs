//! Smoke test: verify the storage port trait compiles and the generated
//! contract types compose correctly.
//!
//! This is a compile-time + basic-construction test. The full proptest suite
//! lands in `story-v0-core-persistence-proptests`. This test confirms:
//! - The `Storage` trait is usable (a no-op impl compiles).
//! - `event_id()` builds the canonical `(authority_domain_id, LSN)` tuple.
//! - `RecordedEvent`, `StoredSnapshot`, `DedupOutcome`, `StorageError` construct.
//! - `append_dedup` is part of the trait (the atomic dedup handle).

use patchbay_contracts::patchbay::{
    AuthorityDomainId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use patchbay_core::storage::{
    event_id, DedupOutcome, RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey,
};

/// A no-op storage impl that exists only to prove the trait is implementable
/// and the types compose. The real rusqlite impl lands in the next story.
struct NoopStorage;

impl Storage for NoopStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        Ok(event_id(authority_domain_id.clone(), 1))
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        _payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        Ok(DedupOutcome::Appended(event_id(authority_domain_id.clone(), 1)))
    }

    async fn read_after(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        Ok(vec![])
    }

    async fn write_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _snapshot_lsn: Lsn,
        _snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn load_latest_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        Ok(None)
    }
}

fn sample_payload() -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::Operation as i32,
        payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
    }
}

#[tokio::test]
async fn event_id_builds_canonical_tuple() {
    let domain = AuthorityDomainId {
        value: "operator-domain".to_string(),
    };
    let id = event_id(domain.clone(), 42);
    assert_eq!(id.authority_domain_id.as_ref().unwrap(), &domain);
    assert_eq!(id.lsn.as_ref().unwrap(), &Lsn { value: 42 });
}

#[tokio::test]
async fn noop_storage_trait_compiles_and_runs() {
    let storage = NoopStorage;
    let domain = AuthorityDomainId {
        value: "test".to_string(),
    };
    let id = storage.append(&domain, sample_payload()).await.unwrap();
    assert_eq!(id.lsn.as_ref().unwrap().value, 1);
    let events = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn append_dedup_returns_outcome() {
    let storage = NoopStorage;
    let domain = AuthorityDomainId {
        value: "test".to_string(),
    };
    let key = IdempotencyKey {
        value: "k1".to_string(),
    };
    let target = TargetKey::new("target-1".to_string()).unwrap();
    let outcome = storage
        .append_dedup(&domain, &key, &target, sample_payload())
        .await
        .unwrap();
    match outcome {
        DedupOutcome::Appended(id) => assert_eq!(id.lsn.as_ref().unwrap().value, 1),
        DedupOutcome::Duplicate(_) => panic!("expected Appended for new key"),
    }
}

#[tokio::test]
async fn recorded_event_and_snapshot_construct() {
    let domain = AuthorityDomainId {
        value: "d".to_string(),
    };
    let event = RecordedEvent {
        event_id: event_id(domain.clone(), 7),
        payload: sample_payload(),
    };
    assert_eq!(event.event_id.lsn.as_ref().unwrap().value, 7);
    assert_eq!(event.payload.kind, StoredEventKind::Operation as i32);

    let snapshot = StoredSnapshot {
        event_id: event_id(domain, 10),
        payload: vec![0x01, 0x02],
    };
    assert_eq!(snapshot.event_id.lsn.as_ref().unwrap().value, 10);
}

#[test]
fn storage_error_variants_construct() {
    let stale = StorageError::SnapshotStale(5);
    assert!(stale.to_string().contains("older than current state"));
    let wrong = StorageError::SnapshotWrongDomain;
    assert!(wrong.to_string().contains("different authority domain"));
    let conflict = StorageError::IdempotencyConflict;
    assert!(conflict.to_string().contains("payload differs"));
    let write_err = StorageError::WriteFailed {
        message: "disk full".to_string(),
        retryable: false,
    };
    assert!(write_err.to_string().contains("disk full"));
    let unavail = StorageError::Unavailable("writer closed".to_string());
    assert!(unavail.to_string().contains("writer closed"));
    let corrupt = StorageError::CorruptRecord("bad magic".to_string());
    assert!(corrupt.to_string().contains("bad magic"));
    let invalid_lsn = StorageError::InvalidSnapshotLsn(99);
    assert!(invalid_lsn.to_string().contains("does not correspond"));
    let invalid_kind = StorageError::InvalidEventKind;
    assert!(invalid_kind.to_string().contains("unspecified or unknown"));
}

#[test]
fn target_key_rejects_empty() {
    assert!(TargetKey::new("".to_string()).is_none());
    assert!(TargetKey::new("target-1".to_string()).is_some());
    let key = TargetKey::new("target-1".to_string()).unwrap();
    assert_eq!(key.as_str(), "target-1");
}

#[test]
fn stored_event_kind_variants_are_concrete() {
    // Verify the kind enum has concrete per-message variants, not family-level.
    // This is what makes replay unambiguous: Grant != DescendantGrant != Revocation.
    assert_ne!(StoredEventKind::Grant as i32, StoredEventKind::DescendantGrant as i32);
    assert_ne!(StoredEventKind::Grant as i32, StoredEventKind::Revocation as i32);
    assert_ne!(StoredEventKind::DescendantGrant as i32, StoredEventKind::Revocation as i32);
    assert_ne!(StoredEventKind::Operation as i32, StoredEventKind::Observation as i32);
}
