//! Smoke test: verify the storage port trait compiles and the generated
//! contract types compose correctly.
//!
//! This is a compile-time + basic-construction test. The full proptest suite
//! lands in `story-v0-core-persistence-proptests`. This test confirms:
//! - The `Storage` trait is usable (a no-op impl compiles).
//! - `event_id()` builds the canonical `(authority_domain_id, LSN)` tuple.
//! - `RecordedEvent` and `StorageError` construct as designed.

use patchbay_contracts::patchbay::{AuthorityDomainId, EventId, Lsn};
use patchbay_core::storage::{event_id, RecordedEvent, Storage, StorageError};

/// A no-op storage impl that exists only to prove the trait is implementable
/// and the types compose. The real rusqlite impl lands in the next story.
struct NoopStorage;

impl Storage for NoopStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        _payload: Vec<u8>,
    ) -> Result<EventId, StorageError> {
        Ok(event_id(authority_domain_id.clone(), 1))
    }

    async fn read_prefix(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _cursor: u64,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        Ok(vec![])
    }

    async fn write_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _snapshot_lsn: u64,
        _snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn load_latest_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _at_or_before: Option<u64>,
    ) -> Result<Option<(u64, Vec<u8>)>, StorageError> {
        Ok(None)
    }
}

#[tokio::test]
async fn event_id_builds_canonical_tuple() {
    let domain = AuthorityDomainId {
        value: "operator-domain".to_string(),
    };
    let id: EventId = event_id(domain.clone(), 42);
    assert_eq!(id.authority_domain_id.as_ref().unwrap(), &domain);
    assert_eq!(id.lsn.as_ref().unwrap(), &Lsn { value: 42 });
}

#[tokio::test]
async fn noop_storage_trait_compiles_and_runs() {
    let storage = NoopStorage;
    let domain = AuthorityDomainId {
        value: "test".to_string(),
    };
    let id = storage.append(&domain, vec![1, 2, 3]).await.unwrap();
    assert_eq!(id.lsn.as_ref().unwrap().value, 1);
    let events = storage.read_prefix(&domain, 0).await.unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn recorded_event_constructs() {
    let domain = AuthorityDomainId {
        value: "d".to_string(),
    };
    let event = RecordedEvent {
        event_id: event_id(domain, 7),
        payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    assert_eq!(event.event_id.lsn.as_ref().unwrap().value, 7);
    assert_eq!(event.payload, vec![0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn storage_error_variants_construct() {
    let stale = StorageError::SnapshotStale(5);
    assert!(stale.to_string().contains("older than current state"));
    let wrong = StorageError::SnapshotWrongDomain;
    assert!(wrong.to_string().contains("different authority domain"));
}
