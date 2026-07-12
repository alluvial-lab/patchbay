//! Crash recovery and replay.
//!
//! On startup, the core reconstructs in-memory state by loading the latest
//! snapshot (if any) and replaying events with `LSN > snapshot_lsn`. Recovery
//! is deterministic — for unchanged storage contents (events and snapshots),
//! it returns identical raw materials. Full idempotent replay depends on the
//! domain layer's deterministic `apply` (`IdempotentLogReplay` obligation).
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
//!   same snapshot + tail for the same committed log contents); the domain
//!   layer's `apply` must be deterministic for the property to hold end-to-end.
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

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};

use super::port::{RecordedEvent, Storage, StorageError, StoredSnapshot};

/// The result of recovery: the starting point (snapshot, if any) and the
/// events to apply to reconstruct in-memory state.
///
/// The storage layer does not own domain state (commands, sessions, grants) —
/// that belongs to the sibling core features (acceptance, authority, sessions).
/// This struct gives the domain layer the raw materials: a snapshot (opaque
/// bytes the domain layer knows how to deserialize) and the event tail to
/// apply on top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryState {
    /// The snapshot loaded as the recovery starting point, if any.
    /// `None` means no snapshot exists — replay from LSN 0.
    pub snapshot: Option<StoredSnapshot>,
    /// Events with `LSN > snapshot_lsn` (or all events if no snapshot),
    /// in LSN order. The domain layer applies these to reconstruct state.
    pub tail: Vec<RecordedEvent>,
}

impl RecoveryState {
    /// The LSN at which recovery starts. If a snapshot was loaded, this is
    /// the snapshot's LSN (events at or before this LSN are already reflected
    /// in the snapshot). If no snapshot, this is 0 (replay from the beginning).
    ///
    /// Returns `StorageError::CorruptRecord` if the snapshot exists but has
    /// no LSN (malformed — Fail Fast rather than silently defaulting to 0).
    pub fn start_lsn(&self) -> Result<u64, StorageError> {
        match &self.snapshot {
            Some(s) => s
                .event_id
                .lsn
                .as_ref()
                .map(|l| l.value)
                .ok_or_else(|| StorageError::CorruptRecord("snapshot has no LSN".to_string())),
            None => Ok(0),
        }
    }

    /// Iterate over the events to apply, in LSN order.
    pub fn events(&self) -> impl Iterator<Item = &RecordedEvent> {
        self.tail.iter()
    }
}

/// Recover in-memory state for an authority domain.
///
/// Loads the latest snapshot (if any), then reads events with
/// `LSN > snapshot_lsn` (or all events if no snapshot). The caller (the core's
/// domain layer) applies the snapshot payload and the event tail to
/// reconstruct its in-memory state.
///
/// # Determinism (not unconditional idempotency)
///
/// For unchanged storage contents (the same committed log), `recover()` is
/// deterministic — it returns the same snapshot + tail. If events or newer
/// snapshots commit between two calls, the second call may return different
/// (newer) raw materials. This is correct behavior, not a violation: recovery
/// reflects the current committed state at call time.
///
/// # Crash safety (storage-layer portion)
///
/// After a crash (no clean shutdown), `recover()` returns raw materials
/// reflecting the last committed LSN. No committed event is absent from the
/// returned snapshot+tail — the durable event log is the source of truth, and
/// snapshots are derived checkpoints that only bound replay cost. Full
/// "no accepted event is lost" depends additionally on the acceptance pipeline
/// committing before acknowledgement (the acceptance feature) and the domain
/// layer's deterministic application of these raw materials.
pub async fn recover<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<RecoveryState, StorageError> {
    // Load the latest snapshot.
    let snapshot = storage
        .load_latest_snapshot(authority_domain_id, None)
        .await?;

    // Determine the cursor: the snapshot's LSN, or 0 if no snapshot.
    // Fail Fast: a snapshot without an LSN is malformed — reject it rather
    // than silently defaulting to 0 (which would replay events the snapshot
    // already reflects, causing duplicate application).
    let cursor = match &snapshot {
        Some(s) => s
            .event_id
            .lsn
            .as_ref()
            .map(|l| Lsn { value: l.value })
            .ok_or_else(|| StorageError::CorruptRecord("snapshot has no LSN".to_string()))?,
        None => Lsn { value: 0 },
    };

    // Read events after the cursor.
    let tail = storage.read_after(authority_domain_id, cursor).await?;

    Ok(RecoveryState { snapshot, tail })
}
