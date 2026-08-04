//! Ports used by the operation-acceptance pipeline.
//!
//! Acceptance owns these interfaces while the authority and session-registry
//! features provide adapters. Keeping the interfaces here prevents acceptance
//! from depending on either sibling feature's implementation.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ElicitationId, EventId, Generation, GrantId, Operation,
    OperationKind, ResponseContract, RuntimeSessionId, TargetScope,
};
use prost_types::Timestamp;

use crate::{authority::IssuerContext, resource::ResourceIdentity};

pub use crate::time::{Clock, SystemClock, TestClock};

/// The authority seam used before an operation can become durable command
/// state.
///
/// Implementations perform a side-effect-free read. The durable acceptance
/// event is the audit record for a successful check; denied attempts may be
/// recorded separately by the audit subsystem without creating command state.
/// Domain-owned acceptance fence for a durable security posture.
///
/// The acceptance pipeline invokes this after envelope/time/issuer validation
/// and before grant, target, or dedup work. Production implementations fold
/// the event-native security projection; focused acceptance tests may use the
/// permissive adapter below.
pub trait OperationPosture: Send + Sync {
    fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
    ) -> impl std::future::Future<Output = Result<(), OperationPostureDenied>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OperationPostureDenied {
    #[error("security lockdown is active: {reason_code}")]
    SecurityLockdown {
        reason_code: String,
        entered_event_id: EventId,
    },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AllowOperations;

impl OperationPosture for AllowOperations {
    async fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), OperationPostureDenied> {
        Ok(())
    }
}

pub trait GrantCheck: Send + Sync {
    fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        async move {
            self.check_at(
                authority_domain_id,
                issuer,
                operation_kind,
                target_scope,
                &SystemClock.now(),
            )
            .await
        }
    }

    fn check_at(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
        _evaluated_at: &Timestamp,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        async move { self.check(authority_domain_id, issuer, operation_kind, target_scope).await }
    }
}

/// The session-registry seam used to validate and bind an operation target.
pub trait TargetResolver: Send + Sync {
    fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send;
}

/// The elicitation-contract seam used to validate a response Operation's
/// payload against the active contract before durable acceptance.
///
/// Implementations perform a side-effect-free read against an in-memory
/// projection reconciled under the submit gate.
pub trait ElicitationContractLookup: Send + Sync {
    fn active_contract(
        &self,
        elicitation_id: &ElicitationId,
    ) -> impl std::future::Future<Output = Option<ActiveElicitation>> + Send;
}

/// The contract context a response is validated against.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveElicitation {
    pub contract: ResponseContract,
    pub is_terminal: bool,
    /// The winning response Operation, when this terminal slot was answered.
    /// An exact retry is allowed through validation so storage deduplication
    /// can return the existing command record. Other terminal candidates stay
    /// rejected before acceptance.
    pub winning_response: Option<Operation>,
}

/// Evidence that the authority adapter found a matching live grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorized {
    /// The durable grant that authorized the operation, when retained.
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

/// Concrete target identity returned by the target-kind-polymorphic resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetBinding {
    RuntimeSession {
        adapter_id: AdapterId,
        deployment_scope: String,
        runtime_session_id: RuntimeSessionId,
        session_generation: Generation,
    },
    Resource(ResourceIdentity),
    AuthorityDomain(AuthorityDomainId),
}

/// A target that cannot be bound in the requested authority domain.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetNotFound {
    #[error("target not found: {target}")]
    NotFound { target: String },
}
