//! In-memory authority projection derived from durable grant events.
//!
//! The event log is authoritative. [`AuthorityRegistry`] is a deterministic
//! hot lookup path rebuilt and kept current by folding committed
//! [`RecordedEvent`] values through [`AuthorityRegistry::observe`].

use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    ActorId, AuditRecord, AuthorityDomainId, CommandTransition, DescendantGrant, EventId,
    FailureCode, Grant, GrantId, GrantRevocationPolicy, Observation, ObservationKind,
    OperationKind, OperationState, Revocation, StoredEventKind, TargetScope, TargetScopeKind,
};
use prost::Message;

use crate::{
    acceptance::exact_command_correlation,
    contract_validation::validate_continuation_authority_provenance,
    resource::ResourceIdentity,
    storage::RecordedEvent,
};

use super::{
    spawn_tail::{descendant_grant_id, validate_descendant_audit_link},
    AuthorityError, GrantLiveness, GrantProvenanceKind, GrantRecord,
    DESCENDANT_GRANT_ALLOWED_KINDS,
};

/// The current in-memory grant state for one authority-domain log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorityRegistry {
    grants: HashMap<GrantId, GrantRecord>,
    /// Original creation-event records, retained so replaying a creation event
    /// after a later revocation can still be recognized as exact redelivery.
    source_grants: HashMap<GrantId, GrantRecord>,
    /// Immutable successful-result or completed-transition records that may be
    /// named by a spawn-completion audit.
    completion_sources: HashMap<(String, u64), RecordedEvent>,
    /// Qualified spawn-completion audit records retained for descendant-link
    /// validation during replay.
    completion_audits: HashMap<(String, u64), RecordedEvent>,
}

impl AuthorityRegistry {
    /// Construct an empty authority projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed event into the authority projection.
    ///
    /// Events owned by other projections are ignored. Exact grant redelivery
    /// and same-generation revocation redelivery are no-ops; conflicting
    /// duplicates fail immediately as corrupt log history.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AuthorityError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;

        match kind {
            StoredEventKind::Grant => self.observe_grant(event),
            StoredEventKind::DescendantGrant => self.observe_descendant_grant(event),
            StoredEventKind::Revocation => self.observe_revocation(event),
            StoredEventKind::Observation => self.observe_completion_observation(event),
            StoredEventKind::CommandTransition => self.observe_completion_transition(event),
            StoredEventKind::AuditRecord => self.observe_completion_audit(event),
            StoredEventKind::Operation
            | StoredEventKind::Elicitation
            | StoredEventKind::SessionState
            | StoredEventKind::ResourceState
            | StoredEventKind::SpawnClaim
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::OperatorSessionRevocation
            | StoredEventKind::ControlSurfaceRevocation
            | StoredEventKind::SecurityLockdown => Ok(()),
            StoredEventKind::Unspecified => Err(AuthorityError::CorruptLog(
                "authority replay event kind is unspecified".to_owned(),
            )),
        }
    }

    /// Look up a grant, including grants that have been revoked.
    #[must_use]
    pub fn get_grant(&self, grant_id: &GrantId) -> Option<&GrantRecord> {
        self.grants.get(grant_id)
    }

    pub fn grants(&self) -> impl Iterator<Item = &GrantRecord> {
        self.grants.values()
    }

    /// Iterate over grants live at the supplied core instant.
    pub fn live_grants_at<'a>(
        &'a self,
        now: &'a prost_types::Timestamp,
    ) -> impl Iterator<Item = &'a GrantRecord> + 'a {
        self.grants
            .values()
            .filter(move |grant| grant.is_live_at(now))
    }

    /// Iterate over grants live at the production instant. New authorization
    /// code should use `live_grants_at` so one decision samples time once.
    pub fn live_grants(&self) -> impl Iterator<Item = &GrantRecord> {
        self.grants.values().filter(|grant| !grant.is_revoked())
    }

    pub fn grants_with_liveness_at<'a>(
        &'a self,
        now: &'a prost_types::Timestamp,
    ) -> impl Iterator<Item = (&'a GrantRecord, GrantLiveness)> + 'a {
        self.grants
            .values()
            .map(move |grant| (grant, grant.liveness_at(now)))
    }

    fn observe_grant(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let grant = Grant::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode grant at LSN {event_lsn}: {error}"
            ))
        })?;
        let record = operator_grant_record(grant, event_domain, event_lsn)?;
        self.insert_grant(record, event_lsn)
    }

    fn observe_completion_observation(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<(), AuthorityError> {
        let (_, event_lsn) = event_identity(event)?;
        let observation =
            Observation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode observation at LSN {event_lsn}: {error}"
                ))
            })?;
        if observation.kind == ObservationKind::Result as i32
            && observation.failure_code == FailureCode::Unspecified as i32
            && exact_command_correlation(&observation.correlations).is_some()
        {
            insert_recorded_event(&mut self.completion_sources, event, "completion source")?;
        }
        Ok(())
    }

    fn observe_completion_transition(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<(), AuthorityError> {
        let (_, event_lsn) = event_identity(event)?;
        let transition =
            CommandTransition::decode(event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode command transition at LSN {event_lsn}: {error}"
                ))
            })?;
        if transition.to_state == OperationState::Completed as i32
            && transition.failure_code == FailureCode::Unspecified as i32
            && transition
                .command_id
                .as_ref()
                .is_some_and(|id| !id.value.is_empty())
        {
            insert_recorded_event(&mut self.completion_sources, event, "completion source")?;
        }
        Ok(())
    }

    fn observe_completion_audit(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let audit = AuditRecord::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode audit record at LSN {event_lsn}: {error}"
            ))
        })?;
        if audit.audit_event_id.as_ref() != Some(&event.event_id) {
            return Err(AuthorityError::CorruptLog(format!(
                "audit record identity does not match event at LSN {event_lsn}"
            )));
        }
        if audit.reason_code != "spawn_completion" {
            return Ok(());
        }
        if audit.kind != patchbay_contracts::patchbay::AuditEventKind::CommandCompleted as i32
            || audit
                .command_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
        {
            return Err(AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} has invalid kind or command"
            )));
        }
        let source_id = audit.source_event_id.as_ref().ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} has no source_event_id"
            ))
        })?;
        if source_id.authority_domain_id.as_ref() != Some(event_domain) {
            return Err(AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} has a foreign source"
            )));
        }
        let source_key = event_key(source_id, "spawn-completion audit source")?;
        let source = self.completion_sources.get(&source_key).ok_or_else(|| {
            AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {event_lsn} references a missing or invalid source"
            ))
        })?;
        validate_completion_source_matches_audit(source, &audit, event_lsn)?;
        insert_recorded_event(&mut self.completion_audits, event, "spawn-completion audit")
    }

    fn observe_descendant_grant(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let grant = DescendantGrant::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode descendant grant at LSN {event_lsn}: {error}"
            ))
        })?;
        let audit_id = grant.audit_id.as_ref().ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant at LSN {event_lsn} is missing audit_id"
            ))
        })?;
        if audit_id.authority_domain_id.as_ref() != Some(event_domain)
            || audit_id
                .lsn
                .as_ref()
                .is_none_or(|lsn| lsn.value == 0 || lsn.value >= event_lsn)
        {
            return Err(AuthorityError::InvalidGrant(format!(
                "descendant grant at LSN {event_lsn} has an invalid prior same-domain audit_id"
            )));
        }
        let audit_key = event_key(audit_id, "descendant audit")?;
        let audit_event = self.completion_audits.get(&audit_key).ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant at LSN {event_lsn} references an unknown completion audit"
            ))
        })?;
        let audit =
            AuditRecord::decode(audit_event.payload.payload.as_slice()).map_err(|error| {
                AuthorityError::CorruptRecord(format!(
                    "cannot decode linked audit at LSN {event_lsn}: {error}"
                ))
            })?;
        let source_key = event_key(
            audit.source_event_id.as_ref().ok_or_else(|| {
                AuthorityError::InvalidGrant(
                    "completion audit is missing source_event_id".to_owned(),
                )
            })?,
            "completion source",
        )?;
        let source_event = self.completion_sources.get(&source_key).ok_or_else(|| {
            AuthorityError::InvalidGrant(
                "completion audit references an unknown successful result/completed transition"
                    .to_owned(),
            )
        })?;
        validate_descendant_audit_link(&grant, audit_event, source_event)?;
        let record = descendant_grant_record(grant, event_domain, event_lsn)?;
        self.insert_grant(record, event_lsn)
    }

    fn observe_revocation(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let revocation = Revocation::decode(event.payload.payload.as_slice()).map_err(|error| {
            AuthorityError::CorruptRecord(format!(
                "cannot decode revocation at LSN {event_lsn}: {error}"
            ))
        })?;
        let (authority_domain_id, grant_id) = authority_identity(
            revocation.authority_domain_id.as_ref(),
            revocation.grant_id.as_ref(),
            event_domain,
            "revocation",
            event_lsn,
        )?;
        let revocation_generation = revocation.revocation_generation.ok_or_else(|| {
            AuthorityError::CorruptRecord(format!(
                "revocation for grant {:?} at LSN {event_lsn} is missing revocation_generation",
                grant_id
            ))
        })?;
        let revocation_policy = revocation_policy(
            revocation.accepted_operation_policy,
            "revocation accepted_operation_policy",
            event_lsn,
        )?;

        let record = self.grants.get_mut(&grant_id).ok_or_else(|| {
            AuthorityError::GrantNotFound(format!(
                "revocation at LSN {event_lsn} references unknown grant {:?}",
                grant_id
            ))
        })?;
        if record.authority_domain_id != authority_domain_id {
            return Err(AuthorityError::CorruptLog(format!(
                "revocation for grant {:?} at LSN {event_lsn} belongs to authority domain {:?}, but the recorded grant belongs to {:?}",
                grant_id, authority_domain_id, record.authority_domain_id
            )));
        }

        if let Some(existing_generation) = record.revocation_generation {
            if existing_generation == revocation_generation {
                // Exact-redelivery check (rev3 finding 1): a same-generation
                // revocation must carry identical content — actor, timestamp,
                // policy, reason, and audit link — else it is a conflicting
                // duplicate and the log is corrupt. Comparing the full
                // retained revocation fingerprint, not just generation+policy.
                if record.revoked_at == revocation.revoked_at
                    && record.revocation_policy == revocation_policy
                    && record.revoked_by == revocation.revoked_by
                    && record.revocation_reason == revocation.reason
                    && record.revocation_audit_id == revocation.audit_id
                {
                    return Ok(());
                }
                return Err(AuthorityError::CorruptLog(format!(
                    "grant {:?} has a conflicting revocation at generation {} (same generation, different content) at LSN {event_lsn}",
                    grant_id, revocation_generation.value
                )));
            }
            return Err(AuthorityError::CorruptLog(format!(
                "grant {:?} has conflicting revocation generations {} and {} at LSN {event_lsn}",
                grant_id, existing_generation.value, revocation_generation.value
            )));
        }

        record.revocation_generation = Some(revocation_generation);
        record.revoked_at = revocation.revoked_at;
        record.revocation_policy = revocation_policy;
        record.revoked_by = revocation.revoked_by;
        record.revocation_reason = revocation.reason;
        record.revocation_audit_id = revocation.audit_id;
        Ok(())
    }

    fn insert_grant(&mut self, record: GrantRecord, event_lsn: u64) -> Result<(), AuthorityError> {
        if let Some(source) = self.source_grants.get(&record.grant_id) {
            if source == &record {
                return Ok(());
            }
            return Err(AuthorityError::CorruptLog(format!(
                "grant {:?} has conflicting records at LSN {event_lsn}",
                record.grant_id
            )));
        }

        let grant_id = record.grant_id.clone();
        self.source_grants.insert(grant_id.clone(), record.clone());
        self.grants.insert(grant_id, record);
        Ok(())
    }
}

fn operator_grant_record(
    grant: Grant,
    event_domain: &AuthorityDomainId,
    event_lsn: u64,
) -> Result<GrantRecord, AuthorityError> {
    let (authority_domain_id, grant_id) = authority_identity(
        grant.authority_domain_id.as_ref(),
        grant.grant_id.as_ref(),
        event_domain,
        "grant",
        event_lsn,
    )?;
    let subject_actor_id = subject_actor(grant.subject_actor_id, "grant", event_lsn)?;
    validate_optional_endpoint(grant.subject_endpoint_id.as_ref(), "grant", event_lsn)?;
    let target_scope = target_scope(grant.target_scope, "grant", event_lsn)?;
    let allowed_operation_kinds = operation_kinds(
        grant.allowed_operation_kinds,
        "grant allowed_operation_kinds",
        event_lsn,
    )?;
    if allowed_operation_kinds.is_empty() {
        return Err(AuthorityError::InvalidGrant(format!(
            "grant {:?} at LSN {event_lsn} has no allowed operation kinds",
            grant_id
        )));
    }
    let policy = revocation_policy(
        grant.revocation_policy,
        "grant revocation_policy",
        event_lsn,
    )?;
    let provenance = grant.provenance.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!(
            "grant {:?} at LSN {event_lsn} is missing provenance",
            grant_id
        ))
    })?;

    Ok(GrantRecord {
        grant_id,
        authority_domain_id,
        subject_actor_id,
        subject_endpoint_id: grant.subject_endpoint_id,
        subject_endpoint_class: grant.subject_endpoint_class,
        target_scope,
        allowed_operation_kinds,
        created_at: grant.created_at,
        expires_at: grant.expires_at,
        revocation_generation: grant.revocation_generation,
        revoked_at: grant.revoked_at,
        revocation_policy: policy,
        revoked_by: None,
        revocation_reason: String::new(),
        revocation_audit_id: None,
        is_descendant: false,
        provenance: GrantProvenanceKind::Operator {
            created_by: provenance.created_by,
            created_by_operation_id: provenance.created_by_operation_id,
            audit_id: provenance.audit_id,
            reason: provenance.reason,
        },
    })
}

fn descendant_grant_record(
    grant: DescendantGrant,
    event_domain: &AuthorityDomainId,
    event_lsn: u64,
) -> Result<GrantRecord, AuthorityError> {
    let (authority_domain_id, grant_id) = authority_identity(
        grant.authority_domain_id.as_ref(),
        grant.grant_id.as_ref(),
        event_domain,
        "descendant grant",
        event_lsn,
    )?;
    let subject_actor_id = subject_actor(grant.subject_actor_id, "descendant grant", event_lsn)?;
    validate_optional_endpoint(
        grant.subject_endpoint_id.as_ref(),
        "descendant grant",
        event_lsn,
    )?;
    let target_scope = target_scope(grant.target_scope, "descendant grant", event_lsn)?;
    let allowed_operation_kinds = operation_kinds(
        grant.allowed_operation_kinds,
        "descendant grant allowed_operation_kinds",
        event_lsn,
    )?;
    validate_descendant_kinds(&allowed_operation_kinds, &grant_id, event_lsn)?;
    let policy = revocation_policy(
        grant.revocation_policy,
        "descendant grant revocation_policy",
        event_lsn,
    )?;
    if TargetScopeKind::try_from(target_scope.kind).ok() != Some(TargetScopeKind::RuntimeSession) {
        return Err(AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} at LSN {event_lsn} must target one runtime session",
            grant_id
        )));
    }
    let provenance = grant.provenance.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} at LSN {event_lsn} is missing provenance",
            grant_id
        ))
    })?;
    let spawn_operation_id = provenance
        .spawn_operation_id
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant {:?} at LSN {event_lsn} has no non-empty spawn_operation_id",
                grant_id
            ))
        })?;
    let spawning_grant_id = provenance
        .spawning_grant_id
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant {:?} at LSN {event_lsn} has no non-empty spawning_grant_id",
                grant_id
            ))
        })?;
    let continuation_authority = provenance.continuation_authority;
    if let Some(continuation) = continuation_authority.as_ref() {
        validate_continuation_authority_provenance(&spawning_grant_id, continuation).map_err(
            |error| {
                AuthorityError::InvalidGrant(format!(
                    "descendant grant {:?} at LSN {event_lsn} has invalid continuation provenance: {error}",
                    grant_id
                ))
            },
        )?;
    }
    let audit_id = grant
        .audit_id
        .filter(|id| {
            id.authority_domain_id.as_ref() == Some(&authority_domain_id)
                && id
                    .lsn
                    .as_ref()
                    .is_some_and(|lsn| lsn.value > 0 && lsn.value < event_lsn)
        })
        .ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "descendant grant {:?} at LSN {event_lsn} has no valid prior same-domain audit_id",
                grant_id
            ))
        })?;
    if grant_id != descendant_grant_id(&authority_domain_id, &spawn_operation_id) {
        return Err(AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} at LSN {event_lsn} does not use the deterministic spawn id",
            grant_id
        )));
    }
    if grant.created_at.is_none()
        || grant.expires_at.is_some()
        || grant.revocation_generation.is_some()
        || grant.revoked_at.is_some()
        || policy != GrantRevocationPolicy::Continue
    {
        return Err(AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} at LSN {event_lsn} has invalid lifecycle metadata",
            grant_id
        )));
    }

    Ok(GrantRecord {
        grant_id,
        authority_domain_id,
        subject_actor_id,
        subject_endpoint_id: grant.subject_endpoint_id,
        subject_endpoint_class: grant.subject_endpoint_class,
        target_scope,
        allowed_operation_kinds,
        created_at: grant.created_at,
        expires_at: grant.expires_at,
        revocation_generation: grant.revocation_generation,
        revoked_at: grant.revoked_at,
        revocation_policy: policy,
        revoked_by: None,
        revocation_reason: String::new(),
        revocation_audit_id: None,
        is_descendant: true,
        provenance: GrantProvenanceKind::Descendant {
            spawn_operation_id: Some(spawn_operation_id),
            spawning_grant_id: Some(spawning_grant_id),
            continuation_authority,
            audit_id: Some(audit_id),
        },
    })
}

fn authority_identity(
    message_domain: Option<&AuthorityDomainId>,
    grant_id: Option<&GrantId>,
    event_domain: &AuthorityDomainId,
    record_name: &str,
    event_lsn: u64,
) -> Result<(AuthorityDomainId, GrantId), AuthorityError> {
    let message_domain = message_domain.cloned().ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing authority_domain_id"
        ))
    })?;
    if message_domain.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty authority_domain_id"
        )));
    }
    if &message_domain != event_domain {
        return Err(AuthorityError::CorruptLog(format!(
            "{record_name} authority domain {:?} does not match event authority domain {:?} at LSN {event_lsn}",
            message_domain, event_domain
        )));
    }

    let grant_id = grant_id.cloned().ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} is missing grant_id"
        ))
    })?;
    if grant_id.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has an empty grant_id"
        )));
    }

    Ok((message_domain, grant_id))
}

fn subject_actor(
    actor: Option<ActorId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<ActorId, AuthorityError> {
    let actor = actor.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} is missing subject_actor_id"
        ))
    })?;
    if actor.value.is_empty() {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} has an empty subject_actor_id"
        )));
    }
    Ok(actor)
}

fn validate_optional_endpoint(
    endpoint: Option<&patchbay_contracts::patchbay::EndpointId>,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    if endpoint.is_some_and(|endpoint| endpoint.value.is_empty()) {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} has an empty subject_endpoint_id"
        )));
    }
    Ok(())
}

fn target_scope(
    scope: Option<TargetScope>,
    record_name: &str,
    event_lsn: u64,
) -> Result<TargetScope, AuthorityError> {
    let scope = scope.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} is missing target_scope"
        ))
    })?;
    validate_target_scope(&scope, record_name, event_lsn)?;
    Ok(scope)
}

fn validate_target_scope(
    scope: &TargetScope,
    record_name: &str,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    if scope
        .actor_id
        .as_ref()
        .is_some_and(|id| id.value.is_empty())
        || scope
            .adapter_id
            .as_ref()
            .is_some_and(|id| id.value.is_empty())
        || scope
            .runtime_session_id
            .as_ref()
            .is_some_and(|id| id.value.is_empty())
    {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} contains an empty target identifier"
        )));
    }

    let kind = TargetScopeKind::try_from(scope.kind).map_err(|_| {
        AuthorityError::CorruptRecord(format!(
            "{record_name} at LSN {event_lsn} has unknown target scope kind {}",
            scope.kind
        ))
    })?;
    let valid = match kind {
        TargetScopeKind::Unspecified => false,
        TargetScopeKind::Actor => scope.actor_id.is_some(),
        TargetScopeKind::Adapter => scope.adapter_id.is_some(),
        TargetScopeKind::RuntimeSession => {
            scope.adapter_id.is_some()
                && scope.runtime_session_id.is_some()
                && scope.session_generation.is_some()
        }
        TargetScopeKind::ProjectSessionGroup => !scope.project_or_group.is_empty(),
        TargetScopeKind::FleetSupervisor | TargetScopeKind::AuthorityDomain => true,
        TargetScopeKind::Resource => ResourceIdentity::try_from_scope(scope).is_ok(),
    };
    if !valid {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} at LSN {event_lsn} has an incomplete {kind:?} target scope"
        )));
    }
    Ok(())
}

fn operation_kinds(
    raw_kinds: Vec<i32>,
    field_name: &str,
    event_lsn: u64,
) -> Result<Vec<OperationKind>, AuthorityError> {
    raw_kinds
        .into_iter()
        .map(|raw| {
            let kind = OperationKind::try_from(raw).map_err(|_| {
                AuthorityError::CorruptRecord(format!(
                    "{field_name} at LSN {event_lsn} contains unknown operation kind {raw}"
                ))
            })?;
            if kind == OperationKind::Unspecified {
                return Err(AuthorityError::InvalidGrant(format!(
                    "{field_name} at LSN {event_lsn} contains an unspecified operation kind"
                )));
            }
            Ok(kind)
        })
        .collect()
}

fn validate_descendant_kinds(
    allowed: &[OperationKind],
    grant_id: &GrantId,
    event_lsn: u64,
) -> Result<(), AuthorityError> {
    let actual: HashSet<_> = allowed.iter().copied().collect();
    let expected: HashSet<_> = DESCENDANT_GRANT_ALLOWED_KINDS.iter().copied().collect();
    if allowed.len() != DESCENDANT_GRANT_ALLOWED_KINDS.len() || actual != expected {
        return Err(AuthorityError::InvalidGrant(format!(
            "descendant grant {:?} at LSN {event_lsn} must contain exactly the canonical existing-session operation kinds",
            grant_id
        )));
    }
    Ok(())
}

fn revocation_policy(
    raw: i32,
    field_name: &str,
    event_lsn: u64,
) -> Result<GrantRevocationPolicy, AuthorityError> {
    let policy = GrantRevocationPolicy::try_from(raw).map_err(|_| {
        AuthorityError::CorruptRecord(format!(
            "{field_name} at LSN {event_lsn} has unknown policy {raw}"
        ))
    })?;
    if policy == GrantRevocationPolicy::Unspecified {
        return Err(AuthorityError::InvalidGrant(format!(
            "{field_name} at LSN {event_lsn} is unspecified"
        )));
    }
    Ok(policy)
}

fn validate_completion_source_matches_audit(
    source: &RecordedEvent,
    audit: &AuditRecord,
    audit_lsn: u64,
) -> Result<(), AuthorityError> {
    let (_, source_lsn) = event_identity(source)?;
    if source_lsn >= audit_lsn {
        return Err(AuthorityError::CorruptLog(format!(
            "spawn-completion audit at LSN {audit_lsn} references non-prior source LSN {source_lsn}"
        )));
    }
    let command_id = audit.command_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord(format!(
            "spawn-completion audit at LSN {audit_lsn} has no command_id"
        ))
    })?;
    match StoredEventKind::try_from(source.payload.kind).ok() {
        Some(StoredEventKind::Observation) => {
            let observation =
                Observation::decode(source.payload.payload.as_slice()).map_err(|error| {
                    AuthorityError::CorruptRecord(format!(
                        "cannot decode completion source at LSN {source_lsn}: {error}"
                    ))
                })?;
            if observation.kind != ObservationKind::Result as i32
                || observation.failure_code != FailureCode::Unspecified as i32
                || exact_command_correlation(&observation.correlations).as_ref() != Some(command_id)
            {
                return Err(AuthorityError::CorruptLog(format!(
                    "spawn-completion audit at LSN {audit_lsn} references a non-matching result"
                )));
            }
        }
        Some(StoredEventKind::CommandTransition) => {
            let transition =
                CommandTransition::decode(source.payload.payload.as_slice()).map_err(|error| {
                    AuthorityError::CorruptRecord(format!(
                        "cannot decode completion source at LSN {source_lsn}: {error}"
                    ))
                })?;
            if transition.command_id.as_ref() != Some(command_id)
                || transition.to_state != OperationState::Completed as i32
                || transition.failure_code != FailureCode::Unspecified as i32
            {
                return Err(AuthorityError::CorruptLog(format!(
                    "spawn-completion audit at LSN {audit_lsn} references a non-matching transition"
                )));
            }
        }
        _ => {
            return Err(AuthorityError::CorruptLog(format!(
                "spawn-completion audit at LSN {audit_lsn} references the wrong source kind"
            )));
        }
    }
    Ok(())
}

fn insert_recorded_event(
    map: &mut HashMap<(String, u64), RecordedEvent>,
    event: &RecordedEvent,
    record_name: &str,
) -> Result<(), AuthorityError> {
    let key = event_key(&event.event_id, record_name)?;
    match map.get(&key) {
        Some(existing) if existing == event => Ok(()),
        Some(_) => Err(AuthorityError::CorruptLog(format!(
            "{record_name} has conflicting immutable records at LSN {}",
            key.1
        ))),
        None => {
            map.insert(key, event.clone());
            Ok(())
        }
    }
}

fn event_key(event_id: &EventId, record_name: &str) -> Result<(String, u64), AuthorityError> {
    let domain = event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord(format!("{record_name} has no authority domain"))
    })?;
    if domain.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} has an empty authority domain"
        )));
    }
    let lsn = event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AuthorityError::CorruptRecord(format!("{record_name} has no LSN")))?;
    if lsn.value == 0 {
        return Err(AuthorityError::CorruptRecord(format!(
            "{record_name} has zero LSN"
        )));
    }
    Ok((domain.value.clone(), lsn.value))
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), AuthorityError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AuthorityError::CorruptRecord("authority event has no authority domain".to_owned())
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(AuthorityError::CorruptRecord(
            "authority event has an empty authority domain".to_owned(),
        ));
    }
    let lsn =
        event.event_id.lsn.as_ref().ok_or_else(|| {
            AuthorityError::CorruptRecord("authority event has no LSN".to_owned())
        })?;
    Ok((authority_domain_id, lsn.value))
}
