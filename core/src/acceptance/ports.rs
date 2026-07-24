//! Ports used by the operation-acceptance pipeline.
//!
//! Acceptance owns these interfaces while the authority and session-registry
//! features provide adapters. Keeping the interfaces here prevents acceptance
//! from depending on either sibling feature's implementation.

use std::time::{SystemTime, UNIX_EPOCH};

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ElicitationId, Generation, GrantId, Operation, OperationKind,
    ResponseContract, RuntimeSessionId, TargetScope,
};
use prost_types::Timestamp;

use crate::authority::IssuerContext;

/// Time source for acceptance-window validation.
///
/// The domain receives time through this port so validity checks are
/// deterministic in tests and do not depend directly on the system clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Production wall-clock adapter for the acceptance boundary.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        timestamp_from_system_time(SystemTime::now())
    }
}

fn timestamp_from_system_time(time: SystemTime) -> Timestamp {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Timestamp {
            seconds: i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
            nanos: duration.subsec_nanos() as i32,
        },
        Err(error) => {
            let duration = error.duration();
            let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
            if duration.subsec_nanos() == 0 {
                Timestamp {
                    seconds: -seconds,
                    nanos: 0,
                }
            } else {
                Timestamp {
                    seconds: -seconds - 1,
                    nanos: 1_000_000_000 - duration.subsec_nanos() as i32,
                }
            }
        }
    }
}

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
        issuer: &dyn IssuerContext,
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
