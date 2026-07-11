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
//! variable from the model live in this layer. The [`Storage::append_dedup`]
//! operation is the atomic check-and-register handle that makes that claim
//! honest — the key is tested and the event appended in one durable
//! transaction, so concurrent acceptance handlers cannot both pass an
//! in-memory check before their appends serialize.
//!
//! The stated-normative obligations `IdempotentLogReplay`, `CrashNoAcceptedLost`,
//! and `SnapshotConsistentPrefix` (`specs/seed/snapshot_recovery.qnt`) must be
//! satisfied by the implementation with proptest-backed evidence; they do not
//! yet carry checked formal-model formulas. The v1 formal gate owns the real
//! properties.

use patchbay_contracts::patchbay::{
    AuthorityDomainId, EventId, IdempotencyKey, Lsn, StoredEventPayload,
};

/// A durably-recorded state-transition event in the authority-domain log.
///
/// The `event_id` carries the full `(authority_domain_id, LSN)` tuple, not a
/// bare LSN, so the canonical key shape is preserved per the protocol's
/// federation seam (forward-compatibility hygiene for when federation arrives).
/// The `payload` is a self-describing generated envelope ([`StoredEventPayload`])
/// whose `kind` discriminates the message type for replay — storage does not
/// need to inspect the payload bytes to know how to route them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvent {
    pub event_id: EventId,
    pub payload: StoredEventPayload,
}

/// A loaded snapshot, preserving the canonical domain + LSN identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSnapshot {
    pub event_id: EventId,
    pub payload: Vec<u8>,
}

/// Errors at the storage boundary.
///
/// This type is deliberately backend-neutral: it does not carry `rusqlite::Error`
/// or any other adapter-specific error. The rusqlite implementation maps its
/// errors into these variants. This keeps the Ports & Adapters boundary honest
/// — domain callers depend on `StorageError`, not on SQLite.
///
/// Fail Fast: unknown/invalid input (stale snapshot, wrong domain, conflicting
/// idempotency payload) is rejected here with a distinct variant rather than
/// swallowed or surfaced as a raw backend error.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A durable write (append or snapshot) failed at the backend level.
    /// `retryable` hints whether the caller may retry (e.g. transient I/O)
    /// versus a non-recoverable backend fault.
    #[error("durable write failed (retryable={retryable}): {message}")]
    WriteFailed { message: String, retryable: bool },

    /// A read (prefix replay, snapshot load) failed at the backend level.
    #[error("read failed (retryable={retryable}): {message}")]
    ReadFailed { message: String, retryable: bool },

    /// The storage backend is unavailable (e.g. writer actor closed, database
    /// locked beyond timeout, connection poisoned). Distinct from a failed
    /// operation so callers can decide whether to retry or degrade.
    #[error("storage unavailable: {0}")]
    Unavailable(String),

    /// A stored record could not be deserialized. Indicates corruption or a
    /// schema-version mismatch; not retryable without migration.
    #[error("corrupt stored record: {0}")]
    CorruptRecord(String),

    /// A snapshot with an LSN strictly less than the current state was
    /// submitted as an authority source. Rejected per
    /// `docs/PROTOCOL.md` § "Revisions and cursors".
    #[error("snapshot LSN {0} is older than current state")]
    SnapshotStale(u64),

    /// A snapshot from a different authority domain was submitted. Rejected
    /// outright per `docs/PROTOCOL.md` § "Revisions and cursors".
    #[error("snapshot from different authority domain")]
    SnapshotWrongDomain,

    /// An idempotency key was already applied to a command to the same target,
    /// but with a non-identical payload. Rejected at submission with
    /// `validation_failed` before acceptance per `docs/PROTOCOL.md` §
    /// "Idempotency and retry" (payload equivalence rule).
    #[error("idempotency key conflict: payload differs from the existing command")]
    IdempotencyConflict,

    /// The requested snapshot LSN does not correspond to a committed event.
    /// A snapshot must materialize at a real committed LSN.
    #[error("snapshot LSN {0} does not correspond to a committed event")]
    InvalidSnapshotLsn(u64),
}

/// The outcome of an idempotent append ([`Storage::append_dedup`]).
///
/// Mirrors the protocol's acceptance-boundary dedup: a retry of the same
/// command id + idempotency key + target returns the existing record; a
/// new key appends a new event. This is the `appliedKeys` boundary guarantee
/// from `command_lifecycle.qnt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupOutcome {
    /// A new event was appended. The key is now registered against this target.
    Appended(EventId),
    /// The key was already applied to a command to this target with an
    /// identical payload. Returns the existing event's id; no new event written.
    Duplicate(EventId),
}

/// The storage port. Domain logic depends on this trait, not on rusqlite.
///
/// # Object safety
///
/// This trait uses native `impl Future` return-position syntax (RPITIT,
/// stabilized in Rust 1.75). It is **not** object-safe: `Box<dyn Storage>` and
/// `Arc<dyn Storage>` will not compile. This is intentional for v0.1.0 — the
/// core uses static dispatch (generic `S: Storage` parameters), which is sound
/// for tonic handlers and tokio tasks. If runtime composition (`Box<dyn Storage>`)
/// becomes necessary, switch to `async-trait` or explicitly boxed futures.
///
/// # LSN assignment
///
/// The LSN is assigned at durable-commit time by the implementation. Append
/// operations are atomic — the event is either durably recorded with a
/// committed LSN, or the call fails and nothing is persisted.
pub trait Storage: Send + Sync {
    /// Durably append an event to the authority-domain log.
    ///
    /// Returns the assigned [`EventId`] — the LSN is allocated at
    /// durable-commit time. Use this for events that are not subject to
    /// idempotency-key dedup (e.g. adapter-reported Observations, which are
    /// not operator retries). For operator-submitted commands, prefer
    /// [`Storage::append_dedup`].
    fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> impl std::future::Future<Output = Result<EventId, StorageError>> + Send;

    /// Idempotently append an event: atomically check the idempotency key
    /// against the target and append only if absent.
    ///
    /// This is the atomic check-and-register handle for the formal model's
    /// `appliedKeys` set. The key is scoped per-target (the caller passes the
    /// target identity as part of the key context); a key reused across
    /// different targets does not dedup. The check and the append happen in
    /// one durable transaction, so concurrent acceptance handlers cannot both
    /// pass the check before their appends serialize.
    ///
    /// Returns [`DedupOutcome::Appended`] for a new key, or
    /// [`DedupOutcome::Duplicate`] for a retry of the same key + identical
    /// payload. Returns [`StorageError::IdempotencyConflict`] if the key is
    /// already applied but the payload differs (protocol: reject with
    /// `validation_failed` before acceptance).
    fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &str,
        payload: StoredEventPayload,
    ) -> impl std::future::Future<Output = Result<DedupOutcome, StorageError>> + Send;

    /// Read events with `LSN > cursor`, in LSN order.
    ///
    /// Used for crash recovery (`cursor = 0`) and cursor reconciliation
    /// (`cursor` = the client's last-known LSN).
    fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> impl std::future::Future<Output = Result<Vec<RecordedEvent>, StorageError>> + Send;

    /// Write a snapshot materialized at the given LSN.
    ///
    /// Must reflect a consistent log prefix: every event with
    /// `LSN <= snapshot_lsn` and no event with `LSN > snapshot_lsn`. The
    /// implementation must ensure the snapshot LSN corresponds to a real
    /// committed event (returns [`StorageError::InvalidSnapshotLsn`] if not).
    fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<(), StorageError>> + Send;

    /// Load the latest snapshot at or before the given LSN.
    ///
    /// If `at_or_before` is `None`, loads the latest snapshot overall.
    /// Returns `Ok(None)` if no snapshot exists.
    fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> impl std::future::Future<Output = Result<Option<StoredSnapshot>, StorageError>> + Send;
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
