//! Session registry: identity, state axes, and generation supersession.
//!
//! This module owns the canonical session-state transition adjacency and the
//! session identity tuple. Both live ingestion and replay use the same
//! functions so recovery cannot acquire different state-machine semantics
//! from the write path. Mirrors `acceptance::transitions`.

use patchbay_contracts::patchbay::Generation;

pub mod events;
pub mod ingest;
pub mod registry;
pub mod replay;
pub mod resolver;
pub mod state;

pub use events::SessionStateEvent;
pub use ingest::{
    adapter_stale_events, ingest_session_report, mark_adapter_sessions_stale, IngestResult, SessionLookup,
    SessionProjection, SessionReport,
};
pub use registry::{SessionRecord, SessionRegistry, SessionTombstone};
pub use replay::rebuild_from_log;
pub use state::{
    allowed_activity_transition, allowed_connectivity_transition, effective_connectivity,
    SessionIdentity,
};

/// Errors detected while constructing, ingesting, or folding session state.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// A session projection cannot exist without an owning authority domain.
    #[error("session registry authority_domain_id must not be empty")]
    EmptyAuthorityDomain,

    /// A caller or event attempted to cross the projection's authority domain.
    #[error("session authority domain mismatch: expected {expected:?}, got {actual:?}")]
    AuthorityDomainMismatch {
        expected: patchbay_contracts::patchbay::AuthorityDomainId,
        actual: patchbay_contracts::patchbay::AuthorityDomainId,
    },

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
