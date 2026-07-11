//! The storage port.
//!
//! Domain logic depends on the [`Storage`] trait, not on rusqlite or SQLite.
//! The LSN is assigned at durable-commit time by the implementation; the
//! `(authority_domain_id, LSN)` tuple forms the canonical [`EventId`].
//!
//! # Formal-model alignment
//!
//! The storage port backs the `BoundaryDedup` promoted property
//! (`specs/seed/command_lifecycle.qnt`): the `appliedKeys` set and `lsn`
//! variable from the model live in the event-log/persistence layer. The
//! stated-normative obligations `IdempotentLogReplay`, `CrashNoAcceptedLost`,
//! and `SnapshotConsistentPrefix` (`specs/seed/snapshot_recovery.qnt`) are
//! satisfied here with implementation-backed evidence (proptests) even though
//! they do not yet carry checked formal-model formulas; the v1 formal gate
//! owns the real properties.

use patchbay_contracts::patchbay::{AuthorityDomainId, EventId, Lsn};

/// A durably-recorded state-transition event in the authority-domain log.
///
/// The `event_id` carries the full `(authority_domain_id, LSN)` tuple, not a
/// bare LSN, so the canonical key shape is preserved per the protocol's
/// federation seam (forward-compatibility hygiene for when federation arrives).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvent {
    pub event_id: EventId,
    /// Serialized Protobuf message (Operation, Observation, etc.). Opaque to
    /// the storage layer — SQLite is a durable append substrate, not the
    /// protocol model.
    pub payload: Vec<u8>,
}

/// Errors at the storage boundary.
///
/// Fail Fast: unknown/invalid input (stale snapshot, wrong domain) is rejected
/// here with a distinct variant rather than swallowed or surfaced as a raw
/// backend error.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A durable append or snapshot write failed at the backend level.
    #[error("durable write failed: {0}")]
    WriteFailed(#[from] rusqlite::Error),

    /// A read (prefix replay, snapshot load) failed at the backend level.
    #[error("read failed: {0}")]
    ReadFailed(#[from] ReadFailed),

    /// A snapshot with an LSN strictly less than the current state was
    /// submitted as an authority source. Rejected per
    /// `docs/PROTOCOL.md` § "Revisions and cursors".
    #[error("snapshot LSN {0} is older than current state")]
    SnapshotStale(u64),

    /// A snapshot from a different authority domain was submitted. Rejected
    /// outright per `docs/PROTOCOL.md` § "Revisions and cursors".
    #[error("snapshot from different authority domain")]
    SnapshotWrongDomain,
}

/// Wrapper so `ReadFailed` can be a distinct `#[from]` source from
/// `WriteFailed` (both originate from `rusqlite::Error`).
#[derive(Debug, thiserror::Error)]
#[error("read failed: {0}")]
pub struct ReadFailed(#[from] pub rusqlite::Error);

/// The storage port. Domain logic depends on this trait, not on rusqlite.
///
/// The LSN is assigned at durable-commit time by the implementation. The
/// append and LSN assignment are atomic — the event is either durably
/// recorded with a committed LSN, or the call fails and nothing is persisted.
///
/// This trait is async because callers (tonic RPC handlers, adapter streams)
/// are async. The rusqlite implementation bridges via a writer actor.
pub trait Storage: Send + Sync {
    /// Durably append an event to the authority-domain log.
    ///
    /// Returns the assigned [`EventId`] — the LSN is allocated at
    /// durable-commit time. The append and LSN assignment are atomic.
    fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<EventId, StorageError>> + Send;

    /// Read events with `LSN > cursor`, in LSN order.
    ///
    /// Used for crash recovery (`cursor = 0`) and cursor reconciliation
    /// (`cursor` = the client's last-known LSN).
    fn read_prefix(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: u64,
    ) -> impl std::future::Future<Output = Result<Vec<RecordedEvent>, StorageError>> + Send;

    /// Write a snapshot materialized at the given LSN.
    ///
    /// Must reflect a consistent log prefix: every event with
    /// `LSN <= snapshot_lsn` and no event with `LSN > snapshot_lsn`. The
    /// implementation must ensure the snapshot LSN corresponds to a real
    /// committed event.
    fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: u64,
        snapshot_payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), StorageError>> + Send;

    /// Load the latest snapshot at or before the given LSN.
    ///
    /// If `at_or_before` is `None`, loads the latest snapshot overall.
    /// Returns `Ok(None)` if no snapshot exists.
    fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<u64>,
    ) -> impl std::future::Future<Output = Result<Option<(u64, Vec<u8>)>, StorageError>> + Send;
}

/// Helper: construct an `EventId` from its components.
///
/// Exposed so the rusqlite implementation (and tests) can build the canonical
/// tuple without repeating the `Option`-wrapping boilerplate.
pub fn event_id(authority_domain_id: AuthorityDomainId, lsn: u64) -> EventId {
    EventId {
        authority_domain_id: Some(authority_domain_id),
        lsn: Some(Lsn { value: lsn }),
    }
}
