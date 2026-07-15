//! In-memory authority state and grant-matching predicates.

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AuthorityDomainId, CommandId, EndpointId, EventId, Generation,
    GrantId, GrantRevocationPolicy, OperationKind, TargetScope, TargetScopeKind,
};

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
    /// Stored for future clock-backed enforcement; v0.1.0 does not enforce expiry.
    pub expires_at: Option<prost_types::Timestamp>,
    pub revocation_generation: Option<Generation>,
    pub revoked_at: Option<prost_types::Timestamp>,
    pub revocation_policy: GrantRevocationPolicy,
    pub is_descendant: bool,
    pub provenance: GrantProvenanceKind,
}

impl GrantRecord {
    /// Return whether a revocation generation has been recorded for this grant.
    #[must_use]
    pub fn is_revoked(&self) -> bool {
        self.revocation_generation.is_some()
    }

    /// Return whether this grant remains eligible to authorize new operations.
    ///
    /// Expiry is intentionally not evaluated in v0.1.0 because the authority
    /// domain does not yet have a clock port.
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.is_revoked()
    }
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
pub fn grant_authorizes(
    grant: &GrantRecord,
    issuer: &IssuerRef<'_>,
    operation_kind: OperationKind,
    target_scope: &TargetScope,
) -> bool {
    grant.is_live()
        && grant.authority_domain_id == *issuer.authority_domain_id
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
