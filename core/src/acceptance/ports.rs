//! Ports used by the operation-acceptance pipeline.
//!
//! Acceptance owns these interfaces while the authority and session-registry
//! features provide adapters. Keeping the interfaces here prevents acceptance
//! from depending on either sibling feature's implementation.

use patchbay_contracts::patchbay::{
    ActorEndpointRef, AdapterId, AuthorityDomainId, Generation, GrantId, OperationKind,
    RuntimeSessionId, TargetScope,
};

/// The authority seam used before an operation can become durable command
/// state.
///
/// Implementations perform a side-effect-free read. The durable acceptance
/// event is the audit record for a successful check; denied attempts may be
/// recorded separately by the audit subsystem without creating command state.
pub trait GrantCheck: Send + Sync {
    fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        actor: &ActorEndpointRef,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send;
}

/// The session-registry seam used to validate and bind an operation target.
pub trait TargetResolver: Send + Sync {
    fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send;
}

/// Evidence that the authority adapter found a matching live grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorized {
    /// `None` represents the v0.1.0 deployment's implicit operator authority.
    pub grant_id: Option<GrantId>,
}

/// A deny-by-default result from the authority adapter.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GrantDenied {
    #[error("no grant for {actor} to {kind:?} on {target}")]
    NoGrant {
        actor: String,
        kind: OperationKind,
        target: String,
    },
}

/// Concrete delivery identity returned by the session registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetBinding {
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
    pub adapter_id: AdapterId,
}

/// A target that cannot be bound in the requested authority domain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetNotFound {
    #[error("target not found: {target}")]
    NotFound { target: String },
}
