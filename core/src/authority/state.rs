//! In-memory authority state and grant-matching predicates.

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AuthorityDomainId, CommandId, EndpointId, EventId, Generation,
    GrantId, GrantRevocationPolicy, OperationKind, TargetScope, TargetScopeKind,
};
use crate::time::{Clock, SystemClock};

/// The in-memory grant record derived from the durable authority log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRecord {
    pub grant_id: GrantId,
    pub authority_domain_id: AuthorityDomainId,
    pub subject_actor_id: ActorId,
    pub subject_endpoint_id: Option<EndpointId>,
    pub subject_endpoint_class: String,
    pub target_scope: TargetScope,
    pub allowed_operation_kinds: Vec<OperationKind>,
    pub created_at: Option<prost_types::Timestamp>,
    /// Optional half-open expiry boundary. A grant is live only before this instant.
    pub expires_at: Option<prost_types::Timestamp>,
    pub revocation_generation: Option<Generation>,
    pub revoked_at: Option<prost_types::Timestamp>,
    pub revocation_policy: GrantRevocationPolicy,
    /// The actor that revoked the grant. Retained so a conflicting
    /// same-generation revocation (different actor) is detected as corruption
    /// rather than silently collapsed as an exact redelivery.
    pub revoked_by: Option<ActorEndpointRef>,
    /// The revocation reason. Retained for the same conflicting-duplicate check.
    pub revocation_reason: String,
    /// The revocation audit id. Retained for the same conflicting-duplicate check.
    pub revocation_audit_id: Option<EventId>,
    pub is_descendant: bool,
    pub provenance: GrantProvenanceKind,
}

/// The mutually exclusive liveness classification of a grant at one sampled
/// instant. Revocation is deliberately checked first so an expired revoked
/// grant has stable public semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantLiveness {
    Live,
    Expired,
    Revoked,
}

impl GrantRecord {
    /// Return whether a revocation generation has been recorded for this grant.
    #[must_use]
    pub fn is_revoked(&self) -> bool {
        self.revocation_generation.is_some()
    }

    #[must_use]
    pub fn liveness_at(&self, now: &prost_types::Timestamp) -> GrantLiveness {
        if self.is_revoked() {
            return GrantLiveness::Revoked;
        }
        if self.is_expired_at(now) {
            GrantLiveness::Expired
        } else {
            GrantLiveness::Live
        }
    }

    #[must_use]
    pub fn is_live_at(&self, now: &prost_types::Timestamp) -> bool {
        self.liveness_at(now) == GrantLiveness::Live
    }

    #[must_use]
    pub fn is_expired_at(&self, now: &prost_types::Timestamp) -> bool {
        self.expires_at
            .as_ref()
            .is_some_and(|expires_at| timestamp_key(expires_at) <= timestamp_key(now))
    }

    /// Compatibility convenience for callers that are not evaluating a
    /// compound decision. Authorization paths use `*_at` with one sampled
    /// instant instead.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.is_live_at(&SystemClock.now())
    }

    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(&SystemClock.now())
    }
}

fn timestamp_key(timestamp: &prost_types::Timestamp) -> (i64, i32) {
    (timestamp.seconds, timestamp.nanos)
}

/// Provenance retained for both operator-issued and spawned-session grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantProvenanceKind {
    Operator {
        created_by: Option<ActorEndpointRef>,
        created_by_operation_id: Option<CommandId>,
        audit_id: Option<EventId>,
        reason: String,
    },
    Descendant {
        spawn_operation_id: Option<CommandId>,
        spawning_grant_id: Option<GrantId>,
    },
}

/// The minimal verified-issuer view required by [`grant_authorizes`].
pub struct IssuerRef<'a> {
    pub actor: &'a ActorId,
    pub endpoint: Option<&'a EndpointId>,
    pub authority_domain_id: &'a AuthorityDomainId,
}

/// The canonical existing-session operation set for auto-issued descendant grants.
///
/// Spawn requires a separate fleet/adapter grant, and attach is excluded because
/// the spawned session is already attached to its spawner's control plane.
pub const DESCENDANT_GRANT_ALLOWED_KINDS: &[OperationKind] = &[
    OperationKind::Instruct,
    OperationKind::Cancel,
    OperationKind::Interrupt,
    OperationKind::Query,
    OperationKind::ApprovalResponse,
    OperationKind::ElicitationResponse,
    OperationKind::Reconfigure,
    OperationKind::SessionManagement,
];

/// Return whether `grant` authorizes the verified issuer, kind, and target.
///
/// Matching is deny-by-default and requires a live grant, exact authority
/// domain and actor identity, optional endpoint narrowing, kind membership,
/// and target-scope containment.
#[must_use]
pub fn grant_authorizes_at(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
    now: &prost_types::Timestamp,
) -> bool {
    grant.is_live_at(now) && grant_matches_request(grant, issuer, operation_kind, target_scope)
}

#[must_use]
pub fn grant_authorizes(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    grant_authorizes_at(grant, issuer, operation_kind, target_scope, &SystemClock.now())
}

#[must_use]
pub fn grant_matches_request(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    grant.authority_domain_id == *issuer.authority_domain_id
        && grant.subject_actor_id == *issuer.actor
        && grant_endpoint_matches(grant, issuer)
        && grant.allowed_operation_kinds.contains(&operation_kind)
        && target_scope_matches(&grant.target_scope, target_scope)
}

/// Return whether a requested target falls within a grant target scope.
///
/// Fleet-supervisor and authority-domain scopes are wildcards within the
/// already-verified authority domain. Every narrower scope requires its
/// identifying fields to be present and equal. Unspecified and unknown kinds
/// never match.
#[must_use]
pub fn target_scope_matches(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    use TargetScopeKind as Kind;

    match Kind::try_from(grant_scope.kind) {
        Ok(Kind::FleetSupervisor | Kind::AuthorityDomain) => true,
        Ok(Kind::Adapter) => same_adapter(grant_scope, requested),
        Ok(Kind::RuntimeSession) => same_session(grant_scope, requested),
        Ok(Kind::ProjectSessionGroup) => same_project_group(grant_scope, requested),
        Ok(Kind::Actor) => same_actor(grant_scope, requested),
        Ok(Kind::Resource) => same_resource(grant_scope, requested),
        Ok(Kind::Unspecified) | Err(_) => false,
    }
}

fn grant_endpoint_matches(grant: &GrantRecord, issuer: &IssuerRef<'_>) -> bool {
    grant
        .subject_endpoint_id
        .as_ref()
        .is_none_or(|expected| issuer.endpoint == Some(expected))
}

fn same_adapter(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    matches!(
        (&grant_scope.adapter_id, &requested.adapter_id),
        (Some(grant_adapter), Some(requested_adapter)) if grant_adapter == requested_adapter
    )
}

fn same_session(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    same_adapter(grant_scope, requested)
        && same_deployment(grant_scope, requested)
        && matches!(
            (
                &grant_scope.runtime_session_id,
                &requested.runtime_session_id,
                &grant_scope.session_generation,
                &requested.session_generation,
            ),
            (
                Some(grant_runtime),
                Some(requested_runtime),
                Some(grant_generation),
                Some(requested_generation),
            ) if grant_runtime == requested_runtime && grant_generation == requested_generation
        )
}

fn same_deployment(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    !grant_scope.deployment_scope.is_empty()
        && !requested.deployment_scope.is_empty()
        && grant_scope.deployment_scope == requested.deployment_scope
}

fn same_project_group(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    !grant_scope.project_or_group.is_empty()
        && grant_scope.project_or_group == requested.project_or_group
}

fn same_actor(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    matches!(
        (&grant_scope.actor_id, &requested.actor_id),
        (Some(grant_actor), Some(requested_actor)) if grant_actor == requested_actor
    )
}

fn same_resource(grant_scope: &TargetScope, requested: &TargetScope) -> bool {
    !grant_scope.resource_id.is_empty() && grant_scope.resource_id == requested.resource_id
}
