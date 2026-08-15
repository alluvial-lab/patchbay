//! Ports used by the operation-acceptance pipeline.
//!
//! Acceptance owns these interfaces while the authority and session-registry
//! features provide adapters. Keeping the interfaces here prevents acceptance
//! from depending on either sibling feature's implementation.

use patchbay_contracts::patchbay::{
    ActorId, AdapterId, AuthorityDomainId, CommandId, ContinuationAuthorityProvenance,
    ElicitationId, EventId, Generation, GrantId, Operation, OperationKind, ResponseContract,
    RuntimeSessionId, SpawnGenerationClaim, SpawnRequest, TargetScope,
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
        async move {
            self.check(authority_domain_id, issuer, operation_kind, target_scope)
                .await
        }
    }

    /// Complete an operation-aware authority decision after target binding.
    ///
    /// Ordinary Operations and fresh spawn retain the selected Grant unchanged.
    /// Continuation-aware implementations additionally select the exact-prior
    /// replacement Grant at the same sampled decision time.
    fn check_resolved_at(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        request: ResolvedGrantCheck<'_>,
    ) -> impl std::future::Future<Output = Result<Authorized, GrantDenied>> + Send {
        std::future::ready(Ok(request.authorization))
    }
}

/// The operation-aware registry seam used to validate and bind a target.
///
/// Target shape is not meaningful without `OperationKind`: an adapter scope is
/// a committed spawn boundary, while runtime-session and resource scopes are
/// existing-target boundaries and must reject spawn before durable acceptance.
pub trait TargetResolver: Send + Sync {
    fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation: &Operation,
        spawn_request: Option<&SpawnRequest>,
    ) -> impl std::future::Future<Output = Result<TargetBinding, TargetNotFound>> + Send;
}

/// The active-Elicitation seam used to validate a response Operation's
/// payload, lifecycle/dedup context, and responder authority before durable
/// acceptance.
///
/// Implementations perform one side-effect-free read against an in-memory
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
    /// The actor authorized to consume this response slot. Missing or empty
    /// projected evidence fails closed at the acceptance boundary.
    pub expected_responder_actor: Option<ActorId>,
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
    /// The durable Grant that authorized the requested target.
    pub grant_id: Option<GrantId>,
    /// Exact-prior replacement authority selected only for continuation.
    pub continuation_authority: Option<ContinuationAuthorityProvenance>,
}

/// Operation-aware inputs for the second, resolved-target authority decision.
pub struct ResolvedGrantCheck<'a> {
    pub operation_kind: OperationKind,
    pub target_scope: &'a TargetScope,
    pub target_binding: &'a TargetBinding,
    pub authorization: Authorized,
    pub evaluated_at: &'a Timestamp,
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
    SpawnAdapter {
        adapter_id: AdapterId,
        claim: Box<SpawnGenerationClaim>,
        /// Filled by the compound authority decision after exact target lookup.
        continuation_authority: Option<Box<ContinuationAuthorityProvenance>>,
    },
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
    #[error("runtime generation has pending replacement by command {command_id:?}")]
    ReplacementPending { command_id: CommandId },
}
