//! Authority plane: grants, revocation, and grant-check evaluation.
//!
//! Durable authority events are the source of truth. The registry is a
//! deterministic in-memory projection, while matching remains independent of
//! storage and ingress concerns.

pub mod check;
pub mod events;
pub mod ingest;
pub mod issuer;
pub mod operator;
pub mod projection;
pub mod registry;
pub mod replay;
pub mod spawn_tail;
pub mod state;

pub use ingest::{ingest_descendant_grant, ingest_grant, ingest_revocation};
pub use issuer::IssuerContext;
pub use operator::{
    hash_principal_credential, ingest_control_surface_principal, ingest_operator_record,
    rebuild_operator_registry, validate_operator_record, OperatorError, OperatorRegistry,
};
pub use projection::{GrantLookup, GrantProjection};
pub use registry::AuthorityRegistry;
pub use replay::rebuild_from_log;
pub use spawn_tail::{DescendantGrantIssuance, SpawnDescendantTail};
pub use state::{
    authorize_self_revocation_at, grant_authorizes, grant_authorizes_at, grant_matches_request,
    target_scope_matches, GrantAdministrationDenied, GrantLiveness, GrantProvenanceKind,
    GrantRecord, IssuerRef,
    DESCENDANT_GRANT_ALLOWED_KINDS,
};

/// Errors detected while constructing or folding authority state.
#[derive(Debug, thiserror::Error)]
pub enum AuthorityError {
    #[error("corrupt authority record: {0}")]
    CorruptRecord(String),
    #[error("corrupt authority log: {0}")]
    CorruptLog(String),
    #[error("invalid grant: {0}")]
    InvalidGrant(String),
    #[error("grant not found: {0}")]
    GrantNotFound(String),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
