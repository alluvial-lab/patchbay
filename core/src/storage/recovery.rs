//! Crash recovery and replay.
//!
//! On startup, the core reconstructs in-memory state by loading the latest
//! snapshot (if any) and replaying events with `LSN > snapshot_lsn`. Recovery
//! is idempotent — replaying the same committed prefix produces identical
//! state (the `IdempotentLogReplay` stated-normative obligation).
//!
//! # Formal-model alignment
//!
//! - `IdempotentLogReplay` (stated-normative, `snapshot_recovery.qnt`):
//!   replaying the same committed prefix produces identical state. This module
//!   provides the mechanism; the proptest suite
//!   (`story-v0-core-persistence-proptests`) provides the executable evidence.
//! - `CrashNoAcceptedLost` (stated-normative): after a crash, accepted
//!   pre-crash commands remain reconstructable. The durable event log +
//!   snapshot checkpointing satisfies this at the storage layer.
//! - `SnapshotConsistentPrefix` (stated-normative): snapshot materialization
//!   reads a consistent log prefix. The snapshot + tail replay produces state
//!   identical to replaying from 0, which is the storage-level expression of
//!   this property.
//!
//! These are stated-normative — they do not yet carry checked formal-model
//! formulas. The v1 formal gate owns the real properties. This module
//! provides implementation-backed evidence (the proptest suite).

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
#[derive(Debug)]
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
    pub fn start_lsn(&self) -> u64 {
        match &self.snapshot {
            Some(s) => s.event_id.lsn.as_ref().map(|l| l.value).unwrap_or(0),
            None => 0,
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
/// # Idempotency
///
/// Calling `recover()` twice produces an identical `RecoveryState` — the
/// snapshot and the tail are derived from the committed log, which is
/// append-only. Replaying the same committed prefix produces identical state.
///
/// # Crash safety
///
/// After a crash (no clean shutdown), `recover()` reconstructs state up to
/// the last committed LSN. No accepted event is lost — the durable event log
/// is the source of truth, and snapshots are derived checkpoints that only
/// bound replay cost.
pub async fn recover<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<RecoveryState, StorageError> {
    // Load the latest snapshot.
    let snapshot = storage.load_latest_snapshot(authority_domain_id, None).await?;

    // Determine the cursor: the snapshot's LSN, or 0 if no snapshot.
    let cursor = match &snapshot {
        Some(s) => s
            .event_id
            .lsn
            .as_ref()
            .map(|l| Lsn { value: l.value })
            .unwrap_or(Lsn { value: 0 }),
        None => Lsn { value: 0 },
    };

    // Read events after the cursor.
    let tail = storage.read_after(authority_domain_id, cursor).await?;

    Ok(RecoveryState { snapshot, tail })
}
