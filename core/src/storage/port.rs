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
    ActorId, AdapterDiagnosticDetail, AuditEventKind, AuditPage, AuthorityDomainId, CommandId,
    EndpointId, EventId, FailureCode, Generation, IdempotencyKey, Lsn, StoredEventPayload,
    TargetScope,
};
use prost_types::Timestamp;

/// A canonical, non-empty target identity for idempotency-key scoping.
///
/// Per `docs/PROTOCOL.md` § "Idempotency and retry": a key dedups only
/// against existing commands to the same target. This newtype enforces
/// non-emptiness and gives a canonical type for the dedup-scope rule, rather
/// than accepting a raw `&str` that could be empty or inconsistently
/// serialized. The caller constructs it from the operation's `TargetScope`
/// (or a canonical projection of it) at acceptance time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetKey(String);

impl TargetKey {
    /// Construct a `TargetKey` from a non-empty string. Returns `None` for
    /// empty input (Fail Fast at the boundary).
    pub fn new(s: String) -> Option<Self> {
        if s.is_empty() {
            None
        } else {
            Some(Self(s))
        }
    }

    /// The canonical string form, used as the storage dedup-scope key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A canonical, non-empty immutable grant identity.
///
/// The storage backend scopes this value by `AuthorityDomainId`. Normal and
/// descendant grant records deliberately share the same namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantIdentityKey(String);

impl GrantIdentityKey {
    /// Construct an identity from a complete `GrantId` value.
    pub fn new(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(Self(value))
        }
    }

    /// Return the exact opaque grant-id bytes represented as UTF-8.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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

/// The allowlisted input to a durable audit record.
///
/// This type intentionally has no payload, arbitrary metadata, credential,
/// token, prompt, attachment, or descriptor field. Producers must construct a
/// typed draft before storage can assign the audit event id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecordDraft {
    pub occurred_at: Timestamp,
    pub kind: AuditEventKind,
    pub actor_id: Option<ActorId>,
    pub device_id: Option<patchbay_contracts::patchbay::DeviceId>,
    pub endpoint_id: Option<EndpointId>,
    pub operator_session_hash: Vec<u8>,
    pub command_id: Option<CommandId>,
    pub grant_id: Option<patchbay_contracts::patchbay::GrantId>,
    pub target_scope: Option<TargetScope>,
    pub failure_code: Option<FailureCode>,
    pub reason_code: String,
    pub correlation_id: String,
    pub source_event_id: Option<EventId>,
    pub source_network: String,
    pub adapter_diagnostic: Option<AdapterDiagnosticDetail>,
}

impl AuditRecordDraft {
    #[must_use]
    pub fn new(occurred_at: Timestamp, kind: AuditEventKind) -> Self {
        Self {
            occurred_at,
            kind,
            actor_id: None,
            device_id: None,
            endpoint_id: None,
            operator_session_hash: Vec::new(),
            command_id: None,
            grant_id: None,
            target_scope: None,
            failure_code: None,
            reason_code: String::new(),
            correlation_id: String::new(),
            source_event_id: None,
            source_network: String::new(),
            adapter_diagnostic: None,
        }
    }

    /// Validate the structural redaction boundary before durable append.
    pub fn validate(&self, authority_domain_id: &AuthorityDomainId) -> Result<(), StorageError> {
        if self.kind == AuditEventKind::Unspecified {
            return Err(StorageError::InvalidAuditRecord(
                "audit event kind is unspecified".to_owned(),
            ));
        }
        if AuditEventKind::try_from(self.kind as i32).is_err() {
            return Err(StorageError::InvalidAuditRecord(
                "audit event kind is unknown".to_owned(),
            ));
        }
        validate_timestamp(&self.occurred_at)?;
        if !self.operator_session_hash.is_empty() && self.operator_session_hash.len() != 32 {
            return Err(StorageError::InvalidAuditRecord(
                "operator_session_hash must be empty or exactly 32 bytes".to_owned(),
            ));
        }
        validate_bounded_code("reason_code", &self.reason_code)?;
        if self.correlation_id.len() > 128
            || !self.correlation_id.chars().all(|c| c.is_ascii_graphic() && c != '=')
        {
            return Err(StorageError::InvalidAuditRecord(
                "correlation_id must contain at most 128 safe characters".to_owned(),
            ));
        }
        if !self.source_network.is_empty() {
            let parsed = self.source_network.parse::<std::net::IpAddr>().map_err(|_| {
                StorageError::InvalidAuditRecord("source_network must be a normalized IP".to_owned())
            })?;
            if parsed.to_string() != self.source_network {
                return Err(StorageError::InvalidAuditRecord(
                    "source_network must be normalized".to_owned(),
                ));
            }
        }
        if let Some(source) = &self.source_event_id {
            if source.authority_domain_id.as_ref() != Some(authority_domain_id)
                || source.lsn.is_none()
            {
                return Err(StorageError::InvalidAuditRecord(
                    "source_event_id has the wrong domain or no LSN".to_owned(),
                ));
            }
        }
        if let Some(failure_code) = self.failure_code {
            if FailureCode::try_from(failure_code as i32).is_err()
                || failure_code == FailureCode::Unspecified
            {
                return Err(StorageError::InvalidAuditRecord(
                    "failure_code must be a known non-unspecified value".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_bounded_code_for_query(value: &str) -> Result<(), StorageError> {
    validate_bounded_code("reason_code", value)
}

fn validate_bounded_code(name: &str, value: &str) -> Result<(), StorageError> {
    if value.is_empty() {
        return Ok(());
    }
    if value.len() > 64 || !value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return Err(StorageError::InvalidAuditRecord(format!(
            "{name} must match [a-z0-9_]{{1,64}}"
        )));
    }
    Ok(())
}

pub(crate) fn validate_timestamp_for_query(timestamp: &Timestamp) -> Result<(), StorageError> {
    validate_timestamp(timestamp)
}

fn validate_timestamp(timestamp: &Timestamp) -> Result<(), StorageError> {
    const MIN_SECONDS: i64 = -62_135_596_800;
    const MAX_SECONDS: i64 = 253_402_300_799;
    if !(MIN_SECONDS..=MAX_SECONDS).contains(&timestamp.seconds)
        || !(0..1_000_000_000).contains(&timestamp.nanos)
    {
        return Err(StorageError::InvalidAuditRecord(
            "occurred_at is not a valid protobuf Timestamp".to_owned(),
        ));
    }
    Ok(())
}

/// A durable audit append that also identifies the source event when one was
/// written in the same transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedAppend {
    pub source_event_id: EventId,
    pub audit_event_id: EventId,
}

/// Result of atomically claiming and writing an immutable grant identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantAppendOutcome {
    /// The identity was absent and the source plus creation audit committed.
    Appended(AuditedAppend),
    /// Exact canonical content already exists; no event or audit was written.
    Existing(EventId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditedDedupOutcome {
    Appended(AuditedAppend),
    Duplicate {
        source_event_id: EventId,
        audit_event_id: EventId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditPageSpec {
    pub kinds: Vec<AuditEventKind>,
    pub actor_id: Option<ActorId>,
    pub endpoint_id: Option<EndpointId>,
    pub command_id: Option<CommandId>,
    pub grant_id: Option<patchbay_contracts::patchbay::GrantId>,
    pub target: Option<TargetKey>,
    pub failure_codes: Vec<FailureCode>,
    pub reason_codes: Vec<String>,
    pub occurred_from: Option<Timestamp>,
    pub occurred_before: Option<Timestamp>,
    pub before_lsn: Option<u64>,
    pub limit: u16,
}

/// The result of appending a group of source events and one lifecycle audit
/// record in one writer transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedBatchAppend {
    pub source_event_ids: Vec<EventId>,
    pub audit_event_id: EventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedDecisionAppend {
    pub source_event_id: EventId,
    pub audit_event_ids: Vec<EventId>,
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

    /// An immutable grant identity already points at different canonical
    /// source content. The conflict is discovered before another source or
    /// audit append.
    #[error("grant identity {grant_id} conflicts with existing source LSN {existing_lsn}")]
    GrantIdentityConflict {
        grant_id: String,
        existing_lsn: u64,
    },

    /// The requested snapshot LSN does not correspond to a committed event.
    /// A snapshot must materialize at a real committed LSN.
    #[error("snapshot LSN {0} does not correspond to a committed event")]
    InvalidSnapshotLsn(u64),

    /// An append candidate carried `STORED_EVENT_KIND_UNSPECIFIED` or an
    /// unknown kind value. Rejected before durable append (Fail Fast).
    /// Read-side kind corruption is returned as raw stored framing to the
    /// shared replay validator, which distinguishes unspecified log history
    /// from an unknown numeric record kind.
    #[error("stored event kind is unspecified or unknown")]
    InvalidEventKind,

    #[error("invalid audit record: {0}")]
    InvalidAuditRecord(String),

    #[error("invalid audit cursor: {0}")]
    InvalidAuditCursor(String),

    #[error("database schema version {0} is newer than supported")]
    UnsupportedSchemaVersion(u32),

    #[error("malformed database schema: {0}")]
    MalformedSchema(String),

    #[error("core generation must be in 1..=i64::MAX, got {0}")]
    InvalidCoreGeneration(u64),

    #[error("storage operation is unsupported by this backend")]
    UnsupportedOperation,
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

/// Durable authority-domain metadata needed to fence derived checkpoints.
///
/// The caller supplies a nonzero candidate, while the backend atomically
/// persists the first candidate for the authority domain or returns the
/// existing value. This is intentionally separate from [`Storage`]: metadata
/// initialization does not append an event or consume an LSN.
pub trait CoreGenerationStore: Send + Sync {
    fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> impl std::future::Future<Output = Result<Generation, StorageError>> + Send;
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
    /// target identity as a [`TargetKey`]); a key reused across different
    /// targets does not dedup. The check and the append happen in one durable
    /// transaction, so concurrent acceptance handlers cannot both pass the
    /// check before their appends serialize.
    ///
    /// Returns [`DedupOutcome::Appended`] for a new key, or
    /// [`DedupOutcome::Duplicate`] for a retry of the same key + identical
    /// payload. `Duplicate` returns the existing event's [`EventId`] (the log
    /// record identity); the calling layer is responsible for projecting that
    /// to the full command record and state if needed (per `docs/PROTOCOL.md` §
    /// "Idempotency and retry": a retry returns the existing command record).
    /// Returns [`StorageError::IdempotencyConflict`] if the key is already
    /// applied but the payload differs (protocol: reject with
    /// `validation_failed` before acceptance).
    fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> impl std::future::Future<Output = Result<DedupOutcome, StorageError>> + Send;

    fn append_dedup_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
        _logical_payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<DedupOutcome, StorageError>> + Send {
        async move { self.append_dedup(authority_domain_id, key, target, payload).await }
    }

    /// Read the complete authority-domain suffix with `LSN > cursor`.
    ///
    /// The backend contract requires every committed event after the cursor in
    /// exact, gap-free LSN order. Used for crash recovery (`cursor = 0`) and
    /// complete projection catch-up. Replay adjacency validation detects bad
    /// returned rows, but without an independently trusted high-water mark it
    /// cannot prove that a faulty backend did not omit an unknown final tail.
    /// Filtered subscriptions, audit pages, and adapter-specific subsets may
    /// omit unrelated LSNs after this read; those filtered outputs are not
    /// contiguous-prefix inputs.
    fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> impl std::future::Future<Output = Result<Vec<RecordedEvent>, StorageError>> + Send;

    /// Read the authority-domain log through one explicit durable prefix.
    /// Backends should push the upper bound into their query; the default is
    /// retained for small test ports. Missing LSN framing is corruption rather
    /// than an event that may be silently removed by the bound filter.
    fn read_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
        as_of_lsn: Lsn,
    ) -> impl std::future::Future<Output = Result<Vec<RecordedEvent>, StorageError>> + Send {
        async move {
            if as_of_lsn.value < cursor.value {
                return Ok(Vec::new());
            }
            self.read_after(authority_domain_id, cursor)
                .await?
                .into_iter()
                .filter_map(|event| match event.event_id.lsn.as_ref() {
                    Some(lsn) if lsn.value <= as_of_lsn.value => Some(Ok(event)),
                    Some(_) => None,
                    None => Some(Err(StorageError::CorruptRecord(
                        "bounded replay event has no LSN".to_owned(),
                    ))),
                })
                .collect()
        }
    }

    /// Write a snapshot materialized at the given LSN.
    ///
    /// # Consistent-prefix obligation
    ///
    /// The snapshot payload must reflect a consistent log prefix: every event
    /// with `LSN <= snapshot_lsn` and no event with `LSN > snapshot_lsn`
    /// (`docs/PROTOCOL.md` § "Atomicity between events and snapshots"). The
    /// storage port validates that `snapshot_lsn` corresponds to a real
    /// committed event (returns [`StorageError::InvalidSnapshotLsn`] if not),
    /// but the **consistent-prefix construction** of the payload itself is the
    /// caller's responsibility — the caller must materialize the payload by
    /// reading a consistent prefix up to `snapshot_lsn` before calling this.
    /// The implementation writes the snapshot and the log atomically in one
    /// transaction so the snapshot cannot reorder the log.
    ///
    /// This obligation split is deliberate: the port enforces the LSN anchor
    /// and the write atomicity (the snapshot write does not reorder the log);
    /// the caller (the core's snapshot materializer) enforces the prefix
    /// consistency of the payload content. A future revision may move
    /// materialization into the port if a consistent-read transaction boundary
    /// proves necessary.
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

    /// Append a redacted audit record without a source event.
    fn append_audit(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<EventId, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Atomically append a source event and its distinct audit record.
    fn append_audited(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _source: StoredEventPayload,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<AuditedAppend, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Atomically enforce immutable grant identity, append the source and its
    /// one truthful creation audit when absent, or return the original source
    /// id for an exact retry. Backends must not implement this as a read plus
    /// generic append; the fail-closed default is intentional.
    fn append_grant_audited(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _identity: &GrantIdentityKey,
        _source: StoredEventPayload,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<GrantAppendOutcome, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    fn append_dedup_audited_with_payload(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
        _logical_payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<AuditedDedupOutcome, StorageError>> + Send {
        async move { self.append_dedup_audited(authority_domain_id, key, target, source, audit).await }
    }

    /// Atomically append one decision source and all of its typed audit
    /// records. Every audit receives the source event id; either all records
    /// commit or none do.
    fn append_decision_audited_many(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _source: StoredEventPayload,
        _audits: Vec<AuditRecordDraft>,
    ) -> impl std::future::Future<Output = Result<AuditedDecisionAppend, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Append a domain decision. Production decorators override this to pair
    /// the source and audit; bare storage ports retain the historical source
    /// append behavior used by focused domain tests.
    fn append_decision(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<EventId, StorageError>> + Send {
        async move { self.append(authority_domain_id, source).await }
    }

    /// Atomically append a deduplicated source event and audit. A duplicate
    /// source still receives the new submission audit record.
    fn append_dedup_audited(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        _source: StoredEventPayload,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<AuditedDedupOutcome, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Atomically append several related source events and their lifecycle
    /// audit record. This is used for adapter-detach reconciliation, where
    /// session degradation and the detach decision must not split across
    /// transactions.
    fn append_batch_audited(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _sources: Vec<StoredEventPayload>,
        _audit: AuditRecordDraft,
    ) -> impl std::future::Future<Output = Result<AuditedBatchAppend, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Read a bounded, descending audit page from the derived index.
    fn query_audit(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _spec: AuditPageSpec,
    ) -> impl std::future::Future<Output = Result<AuditPage, StorageError>> + Send {
        async { Err(StorageError::UnsupportedOperation) }
    }

    /// Read an audit page through one explicit durable prefix.
    fn query_audit_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
        _as_of_lsn: Lsn,
    ) -> impl std::future::Future<Output = Result<AuditPage, StorageError>> + Send {
        async move { self.query_audit(authority_domain_id, spec).await }
    }
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
