//! Session registry: identity, state axes, and generation supersession.
//!
//! This module owns the canonical session-state transition adjacency and the
//! session identity tuple. Both live ingestion and replay use the same
//! functions so recovery cannot acquire different state-machine semantics
//! from the write path. Mirrors `acceptance::transitions`.

use patchbay_contracts::patchbay::{Generation, SessionReportSourceCursor};

pub mod events;
pub mod ingest;
pub mod logical_target;
pub mod registry;
pub mod replay;
pub mod resolver;
pub mod runtime_evidence;
pub mod spawn_claim;
pub mod spawn_orchestration;
pub mod state;

pub use events::SessionStateEvent;
pub use ingest::{
    adapter_stale_events, ingest_session_report, mark_adapter_sessions_stale, IngestResult,
    SessionLookup, SessionProjection,
};
pub use logical_target::{
    external_runtime_key, ExternalRuntimeKey, ExternalRuntimeOwnership, LogicalTargetError,
    LogicalTargetRecord, LogicalTargetRegistry, LogicalTargetTombstone,
    ReconciledRuntimeGenerationFence,
};
pub use patchbay_contracts::patchbay::SessionReport;
pub use registry::{ManagedLineageCheckpoint, SessionRecord, SessionRegistry, SessionTombstone};
pub use replay::rebuild_from_log;
pub(crate) use runtime_evidence::validate_spawn_promotion_result_order;
pub use runtime_evidence::{
    canonical_runtime_evidence_classification_context, classify_runtime_target,
    classify_session_report, encode_quarantined_runtime_evidence, encode_spawn_promotion,
    encode_staged_successor, fold_spawn_promotion_ordered, next_spawn_promotion,
    next_spawn_promotion_excluding, quarantine_reason_code, quarantine_reason_for,
    quarantined_candidate_scope, quarantined_candidate_target, quarantined_observation,
    quarantined_runtime_candidate, quarantined_session_report, runtime_evidence_candidate_target,
    source_matches_current_attachment, validate_quarantined_runtime_evidence,
    validate_spawn_promotion_envelope, validate_staged_successor, RuntimeEvidenceError,
    SpawnPromotionFoldError,
};
pub use spawn_claim::{
    allowed_external_effect_disposition, allowed_spawn_claim_transition, encode_spawn_claim_event,
    encode_spawn_execution_evidence, rebuild_spawn_claims_from_log,
    runtime_ref_matches_target_scope, validate_execution_evidence_contract,
    validate_spawn_claim_accepted, SpawnClaimError, SpawnClaimKey, SpawnClaimQuery,
    SpawnClaimRecord, SpawnClaimRegistry, SpawnClaimability, SpawnDeliveryFence,
    REPLACEMENT_PENDING_REASON,
};
pub use spawn_orchestration::{
    phase_outcome, runtime_matches_claim, validate_continuation_prior_quiesced, CandidateOutcome,
    ClaimFenceOutcome, PriorRuntimeOutcome, SpawnOrchestrationError, SpawnPhaseOutcome,
};
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

    /// An equal-generation report did not advance adapter source order.
    #[error("stale session-report source cursor: live={live:?}, reported={reported:?}")]
    StaleSourceCursor {
        live: SessionReportSourceCursor,
        reported: SessionReportSourceCursor,
    },

    /// Logical-target identity or external-runtime ownership is invalid.
    #[error(transparent)]
    LogicalTarget(#[from] LogicalTargetError),

    /// The durable storage boundary failed.
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
