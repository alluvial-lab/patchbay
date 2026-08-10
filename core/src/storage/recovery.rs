//! Crash recovery and replay.
//!
//! On startup, the core reconstructs in-memory state from a compatible typed
//! checkpoint, when one exists, and replays events with `LSN > snapshot_lsn`.
//! A stored snapshot is never allowed to skip a log prefix merely because its
//! storage row has an LSN: callers must validate and decode its projection
//! type, format version, embedded domain/epoch/LSN anchors, and payload first.
//! Any incompatible checkpoint is disposable derived data, so recovery returns
//! no snapshot and replays from LSN 0.
//!
//! # Formal-model alignment
//!
//! This module provides the *mechanism* that supports three stated-normative
//! obligations from `snapshot_recovery.qnt`. It does not itself satisfy them —
//! satisfaction depends on the domain layer's deterministic event application
//! and the acceptance pipeline's commit-before-ack discipline:
//!
//! - `IdempotentLogReplay`: replaying the same committed prefix produces
//!   identical state. This module returns deterministic raw materials (the
//!   same validated snapshot + tail for the same committed log contents); the
//!   domain layer's `apply` must be deterministic for the property to hold
//!   end-to-end.
//! - `CrashNoAcceptedLost`: after a crash, accepted pre-crash commands remain
//!   reconstructable. This depends on the durable event log (this layer) AND
//!   acceptance committing before acknowledgement (the acceptance feature).
//! - `SnapshotConsistentPrefix`: snapshot materialization reads a consistent
//!   log prefix. This is the *caller's* obligation per `port.rs`
//!   `write_snapshot` — the port validates the LSN anchor; the materializer
//!   constructs the consistent-prefix payload.
//!
//! These are stated-normative — they do not yet carry checked formal-model
//! formulas. The v1 formal gate owns the real properties. The proptest suite
//! (`story-v0-core-persistence-proptests`) provides implementation-backed
//! evidence for the storage-layer portion of each obligation.

use patchbay_contracts::patchbay::{AuthorityDomainId, EventId, Lsn};

use super::port::{RecordedEvent, Storage, StorageError, StoredSnapshot};

/// A checkpoint that has passed the caller's projection-specific decoder and
/// compatibility checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedSnapshot<T> {
    /// The durable storage-row anchor that was validated with the payload.
    pub event_id: EventId,
    /// The decoded projection value. Recovery consumers never receive opaque
    /// checkpoint bytes as authority.
    pub value: T,
}

/// The result of recovery: a validated typed starting point, if any, and the
/// events to apply to reconstruct in-memory state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryState<T> {
    /// The compatible decoded checkpoint loaded as the recovery starting
    /// point. `None` means no compatible checkpoint exists and `tail` starts
    /// at LSN 1.
    pub snapshot: Option<ValidatedSnapshot<T>>,
    /// Events with `LSN > snapshot_lsn` (or all events if no compatible
    /// snapshot), in LSN order.
    pub tail: Vec<RecordedEvent>,
}

impl<T> RecoveryState<T> {
    /// The LSN at which recovery starts. Validated snapshots always carry a
    /// positive LSN; no snapshot means replay from the beginning (LSN 0).
    pub fn start_lsn(&self) -> Result<u64, StorageError> {
        Ok(self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.event_id.lsn.as_ref())
            .map_or(0, |lsn| lsn.value))
    }

    /// Iterate over the events to apply, in LSN order.
    pub fn events(&self) -> impl Iterator<Item = &RecordedEvent> {
        self.tail.iter()
    }
}

/// Recover typed in-memory state for an authority domain.
///
/// `validate_snapshot` is the projection boundary. It must return `Some(T)`
/// only after decoding the expected checkpoint type and format version and
/// checking the embedded authority domain, durable continuity epoch, snapshot
/// LSN, and payload invariants. Returning `None` declares the checkpoint
/// incompatible. Recovery then discards it and reads the full durable log from
/// LSN 0; incompatibility is a cache miss, not loss of authoritative state.
///
/// This function independently rejects a storage row with a missing/wrong
/// authority domain or a missing/zero LSN before calling the validator. The
/// accepted row LSN becomes the tail cursor only after both row and payload
/// validation succeed.
///
/// # Determinism (not unconditional idempotency)
///
/// For unchanged storage contents and a deterministic validator, `recover()`
/// returns the same typed snapshot + tail. If events or newer checkpoints
/// commit between calls, the second call may return different (newer) raw
/// materials. This is correct behavior, not a violation.
pub async fn recover<S, T, V>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    validate_snapshot: V,
) -> Result<RecoveryState<T>, StorageError>
where
    S: Storage,
    V: FnOnce(&StoredSnapshot) -> Option<T>,
{
    let candidate = storage
        .load_latest_snapshot(authority_domain_id, None)
        .await?;

    let snapshot = match candidate {
        Some(stored)
            if stored.event_id.authority_domain_id.as_ref() == Some(authority_domain_id)
                && stored
                    .event_id
                    .lsn
                    .as_ref()
                    .is_some_and(|lsn| lsn.value > 0) =>
        {
            validate_snapshot(&stored).map(|value| ValidatedSnapshot {
                event_id: stored.event_id,
                value,
            })
        }
        _ => None,
    };

    let cursor = snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.event_id.lsn.as_ref())
        .cloned()
        .unwrap_or(Lsn { value: 0 });
    let tail = storage.read_after(authority_domain_id, cursor).await?;

    Ok(RecoveryState { snapshot, tail })
}
