//! Session registry: identity, state axes, and generation supersession.
//!
//! This module owns the canonical session-state transition adjacency and the
//! session identity tuple. Both live ingestion and replay use the same
//! functions so recovery cannot acquire different state-machine semantics
//! from the write path. Mirrors `acceptance::transitions`.

use patchbay_contracts::patchbay::Generation;

pub mod state;

pub use state::{
    allowed_activity_transition, allowed_connectivity_transition, effective_connectivity,
    SessionIdentity,
};

/// Errors detected while constructing, ingesting, or folding session state.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// A durable record cannot form a valid in-memory session projection.
    #[error("corrupt session record: {0}")]
    CorruptRecord(String),

    /// The event sequence violates session identity or state-axis adjacency.
    #[error("corrupt session log: {0}")]
    CorruptLog(String),

    /// An observed state-axis transition is not permitted by the protocol.
    #[error("invalid state-axis transition: {from:?} -> {to:?}")]
    InvalidTransition { from: String, to: String },

    /// An observation reports a generation older than the live generation.
    #[error("stale generation report: live={live:?}, reported={reported:?}")]
    StaleGeneration {
        live: Generation,
        reported: Generation,
    },

    /// The durable storage boundary failed.
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
