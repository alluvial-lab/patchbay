//! Durable grant and revocation ingestion.
//!
//! Each writer validates its boundary input, appends one schema-owned event,
//! and only then warms the in-memory projection with that committed event.

use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    AuthorityDomainId, DescendantGrant, EventId, FailureCode, Grant, GrantId,
    GrantRevocationEffect, GrantRevocationPolicy, Lsn, OperationState, Revocation,
    StoredEventPayload, TargetScope, TargetScopeKind,
};

use crate::{
    acceptance::Clock,
    storage::{
        validate_next_replay_event, AuditRecordDraft, GrantAppendOutcome, GrantIdentityKey,
        RecordedEvent, Storage, StorageError,
    },
};

use super::{
    events, spawn_tail::validate_descendant_issuance_candidate, AuthorityError, AuthorityRegistry,
    GrantProjection, SpawnDescendantTail, DESCENDANT_GRANT_ALLOWED_KINDS,
};

/// Validate, durably append, and project an operator-issued grant.
pub async fn ingest_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: Grant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    validate_creation_identity(
        grant.authority_domain_id.as_ref(),
        grant.grant_id.as_ref(),
        grant.target_scope.as_ref(),
        authority_domain_id,
        "grant",
    )?;

    let grant_id = required_grant_id(grant.grant_id.as_ref(), "grant")?;
    let audit_actor = grant.subject_actor_id.clone();
    let audit_target = grant.target_scope.clone();
    let payload = events::grant(authority_domain_id.clone(), grant);
    preflight_creation(&payload, authority_domain_id)?;
    let mut audit = AuditRecordDraft::new(
        crate::acceptance::SystemClock.now(),
        patchbay_contracts::patchbay::AuditEventKind::GrantCreated,
    );
    audit.actor_id = audit_actor;
    audit.grant_id = Some(grant_id.clone());
    audit.target_scope = audit_target;
    audit.reason_code = "grant_created".to_owned();
    append_and_warm_grant(
        storage,
        projection,
        authority_domain_id,
        grant_id,
        payload,
        audit,
    )
    .await
}

/// Validate, durably append, and project an auto-issued descendant grant.
pub async fn ingest_descendant_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant: DescendantGrant,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    validate_creation_identity(
        grant.authority_domain_id.as_ref(),
        grant.grant_id.as_ref(),
        grant.target_scope.as_ref(),
        authority_domain_id,
        "descendant grant",
    )?;
    validate_descendant_kinds(&grant.allowed_operation_kinds)?;
    let spawn_operation_id = grant
        .provenance
        .as_ref()
        .and_then(|provenance| provenance.spawn_operation_id.as_ref())
        .filter(|command_id| !command_id.value.is_empty())
        .cloned()
        .ok_or_else(|| {
            AuthorityError::InvalidGrant(
                "descendant grant is missing a non-empty spawn_operation_id".to_owned(),
            )
        })?;

    // Validate against the complete gap-free durable context, not merely the
    // source/audit pair repeated by the candidate. The spawn tail requires a
    // prior exact parent grant, accepted verified sender/target, valid
    // delivered/running lifecycle, successful result (or valid historical
    // completion), contained session registration/bump, and matching audit.
    let durable_prefix = read_validated_authority_prefix(storage, authority_domain_id).await?;
    // Validate the caller's projection against the whole prefix in isolation.
    // Publishing here would expose a partial warm if either this fold or the
    // completion-specific fold rejected a later record. The post-append fold
    // below performs the single live publication.
    let _ = fold_projection_prefix(projection, &durable_prefix)?;
    let mut tail = SpawnDescendantTail::new();
    for event in &durable_prefix {
        tail.observe(event)?;
    }
    let issuance = tail
        .descendant_issuance_for(authority_domain_id, &spawn_operation_id)?
        .ok_or_else(|| {
            AuthorityError::InvalidGrant(
                "durable spawn context is not eligible for descendant grant issuance".to_owned(),
            )
        })?;
    validate_descendant_issuance_candidate(&grant, &issuance)?;

    let grant_id = required_grant_id(grant.grant_id.as_ref(), "descendant grant")?;
    let audit_actor = grant.subject_actor_id.clone();
    let audit_target = grant.target_scope.clone();
    let payload = events::descendant_grant(authority_domain_id.clone(), grant);
    let mut audit = AuditRecordDraft::new(
        crate::acceptance::SystemClock.now(),
        patchbay_contracts::patchbay::AuditEventKind::GrantCreated,
    );
    audit.actor_id = audit_actor;
    audit.grant_id = Some(grant_id.clone());
    audit.target_scope = audit_target;
    audit.reason_code = "descendant_grant_created".to_owned();
    append_and_warm_grant(
        storage,
        projection,
        authority_domain_id,
        grant_id,
        payload,
        audit,
    )
    .await
}

/// Validate, durably append, and project a revocation of exactly one grant.
///
/// Revocation is deliberately non-cascading: only `revocation.grant_id` is
/// looked up and mutated by the resulting event.
pub async fn ingest_revocation<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    revocation: Revocation,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    validate_domain(
        revocation.authority_domain_id.as_ref(),
        authority_domain_id,
        "revocation",
    )?;
    let grant_id = required_grant_id(revocation.grant_id.as_ref(), "revocation")?;
    if revocation.revocation_generation.is_none() {
        return Err(AuthorityError::InvalidGrant(format!(
            "revocation for grant {:?} is missing revocation_generation",
            grant_id
        )));
    }
    let policy =
        GrantRevocationPolicy::try_from(revocation.accepted_operation_policy).map_err(|_| {
            AuthorityError::InvalidGrant(format!(
                "revocation for grant {:?} has unknown accepted_operation_policy {}",
                grant_id, revocation.accepted_operation_policy
            ))
        })?;
    if policy == GrantRevocationPolicy::Unspecified {
        return Err(AuthorityError::InvalidGrant(format!(
            "revocation for grant {:?} has an unspecified accepted_operation_policy",
            grant_id
        )));
    }

    let current = projection
        .current_grant(&grant_id)
        .await
        .ok_or_else(|| AuthorityError::GrantNotFound(format!("{:?}", grant_id)))?;
    if current.authority_domain_id != *authority_domain_id {
        return Err(AuthorityError::InvalidGrant(format!(
            "revocation grant {:?} belongs to authority domain {:?}, expected {:?}",
            grant_id, current.authority_domain_id, authority_domain_id
        )));
    }

    validate_revocation_effects(&revocation.command_effects, &grant_id)?;
    let payload = events::revocation(authority_domain_id.clone(), revocation.clone());
    let occurred_at = revocation
        .revoked_at
        .unwrap_or_else(|| crate::time::SystemClock.now());
    let mut audits = Vec::with_capacity(1 + revocation.command_effects.len());
    let revoked_by = revocation.revoked_by.as_ref();
    let mut audit = AuditRecordDraft::new(
        occurred_at,
        patchbay_contracts::patchbay::AuditEventKind::GrantRevoked,
    );
    audit.actor_id = revoked_by.and_then(|attribution| attribution.actor_id.clone());
    audit.endpoint_id = revoked_by.and_then(|attribution| attribution.endpoint_id.clone());
    audit.device_id = revoked_by.and_then(|attribution| attribution.device_id.clone());
    audit.grant_id = Some(grant_id.clone());
    audit.reason_code = "grant_revoked".to_owned();
    audits.push(audit);
    for effect in &revocation.command_effects {
        let mut effect_audit = AuditRecordDraft::new(
            revocation
                .revoked_at
                .unwrap_or_else(|| crate::time::SystemClock.now()),
            match OperationState::try_from(effect.to_state).ok() {
                Some(OperationState::Cancelled) => {
                    patchbay_contracts::patchbay::AuditEventKind::CommandCancelled
                }
                Some(OperationState::Rejected) => {
                    patchbay_contracts::patchbay::AuditEventKind::CommandRejected
                }
                _ => patchbay_contracts::patchbay::AuditEventKind::CommandSubmissionRejected,
            },
        );
        effect_audit.actor_id = revoked_by.and_then(|attribution| attribution.actor_id.clone());
        effect_audit.endpoint_id =
            revoked_by.and_then(|attribution| attribution.endpoint_id.clone());
        effect_audit.device_id = revoked_by.and_then(|attribution| attribution.device_id.clone());
        effect_audit.grant_id = Some(grant_id.clone());
        effect_audit.command_id = effect.command_id.clone();
        effect_audit.failure_code = FailureCode::try_from(effect.failure_code)
            .ok()
            .filter(|code| *code != FailureCode::Unspecified);
        effect_audit.reason_code = "grant_revocation_policy".to_owned();
        audits.push(effect_audit);
    }
    // Revocation is a security decision even when its policy produces no
    // command effects. Always use the audited-many transaction so a raw
    // storage implementation cannot silently persist the source without its
    // required GrantRevoked audit.
    append_and_warm_decision_many(storage, projection, authority_domain_id, payload, audits).await
}

async fn read_validated_authority_prefix<S>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<Vec<RecordedEvent>, AuthorityError>
where
    S: Storage,
{
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut previous_lsn = 0;
    for event in &events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, event)
            .map_err(|error| {
                error.map(AuthorityError::CorruptRecord, AuthorityError::CorruptLog)
            })?;
        previous_lsn = validated.lsn;
    }
    Ok(events)
}

fn fold_projection_prefix<L>(
    projection: &L,
    events: &[RecordedEvent],
) -> Result<L, AuthorityError>
where
    L: GrantProjection,
{
    let mut staged = projection.clone();
    for event in events {
        staged.observe(event)?;
    }
    Ok(staged)
}

async fn append_and_warm_grant<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    grant_id: GrantId,
    payload: StoredEventPayload,
    audit: AuditRecordDraft,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    let identity = GrantIdentityKey::new(grant_id.value.clone()).ok_or_else(|| {
        AuthorityError::InvalidGrant("grant identity is empty after validation".to_owned())
    })?;
    let event_id = match storage
        .append_grant_audited(authority_domain_id, &identity, payload.clone(), audit)
        .await
    {
        Ok(GrantAppendOutcome::Appended(result)) => result.source_event_id,
        Ok(GrantAppendOutcome::Existing(event_id)) => event_id,
        Err(StorageError::GrantIdentityConflict {
            grant_id,
            existing_lsn,
        }) => {
            return Err(AuthorityError::CorruptLog(format!(
                "authority domain {} grant {grant_id} conflicts with immutable source LSN {existing_lsn}",
                authority_domain_id.value
            )));
        }
        Err(error) => return Err(AuthorityError::Storage(error)),
    };
    validate_event_id(&event_id, authority_domain_id)?;
    let source_lsn = event_id
        .lsn
        .as_ref()
        .expect("validated event id has an LSN")
        .value;
    let durable_prefix = read_validated_authority_prefix(storage, authority_domain_id).await?;
    let mut found_source = false;
    for committed in &durable_prefix {
        if committed.event_id == event_id {
            if committed.payload != payload {
                return Err(AuthorityError::CorruptLog(format!(
                    "grant {grant_id:?} source LSN {source_lsn} differs from the canonical candidate"
                )));
            }
            found_source = true;
        }
    }
    if !found_source {
        return Err(AuthorityError::CorruptLog(format!(
            "grant {grant_id:?} source LSN {source_lsn} is missing during read-back"
        )));
    }

    // Storage identity and audit commit precede projection mutation. Fold the
    // complete validated authority prefix into an isolated clone so a later
    // semantic failure cannot leak earlier grants into the live registry.
    // A fresh retry still observes later revocation and sibling authority
    // facts, and the live projection is published exactly once on success.
    let staged = fold_projection_prefix(projection, &durable_prefix)?;
    *projection = staged;
    Ok(event_id)
}

async fn append_and_warm_decision_many<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
    audits: Vec<AuditRecordDraft>,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    let result = storage
        .append_decision_audited_many(authority_domain_id, payload.clone(), audits)
        .await?;
    validate_event_id(&result.source_event_id, authority_domain_id)?;
    let committed = RecordedEvent {
        event_id: result.source_event_id.clone(),
        payload,
    };
    let staged = fold_projection_prefix(projection, std::slice::from_ref(&committed))?;
    *projection = staged;
    Ok(result.source_event_id)
}

/// Run the registry's canonical shape validation before durability.
fn preflight_creation(
    payload: &StoredEventPayload,
    authority_domain_id: &AuthorityDomainId,
) -> Result<(), AuthorityError> {
    let mut validator = AuthorityRegistry::new();
    validator.observe(&RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(authority_domain_id.clone()),
            lsn: Some(Lsn { value: 1 }),
        },
        payload: payload.clone(),
    })
}

fn validate_creation_identity(
    message_domain: Option<&AuthorityDomainId>,
    grant_id: Option<&GrantId>,
    target_scope: Option<&TargetScope>,
    expected_domain: &AuthorityDomainId,
    record_name: &str,
) -> Result<(), AuthorityError> {
    validate_domain(message_domain, expected_domain, record_name)?;
    required_grant_id(grant_id, record_name)?;

    let target_scope = target_scope.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!("{record_name} is missing target_scope"))
    })?;
    let target_kind = TargetScopeKind::try_from(target_scope.kind).map_err(|_| {
        AuthorityError::InvalidGrant(format!(
            "{record_name} has unknown target scope kind {}",
            target_scope.kind
        ))
    })?;
    if target_kind == TargetScopeKind::Unspecified {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} target scope kind is unspecified"
        )));
    }
    Ok(())
}

fn validate_domain(
    message_domain: Option<&AuthorityDomainId>,
    expected_domain: &AuthorityDomainId,
    record_name: &str,
) -> Result<(), AuthorityError> {
    if expected_domain.value.is_empty() {
        return Err(AuthorityError::InvalidGrant(
            "authority_domain_id is empty".to_owned(),
        ));
    }
    let message_domain = message_domain.ok_or_else(|| {
        AuthorityError::InvalidGrant(format!("{record_name} is missing authority_domain_id"))
    })?;
    if message_domain.value.is_empty() {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} authority_domain_id is empty"
        )));
    }
    if message_domain != expected_domain {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} authority domain {:?} does not match requested domain {:?}",
            message_domain, expected_domain
        )));
    }
    Ok(())
}

fn required_grant_id(
    grant_id: Option<&GrantId>,
    record_name: &str,
) -> Result<GrantId, AuthorityError> {
    let grant_id = grant_id.cloned().ok_or_else(|| {
        AuthorityError::InvalidGrant(format!("{record_name} is missing grant_id"))
    })?;
    if grant_id.value.is_empty() {
        return Err(AuthorityError::InvalidGrant(format!(
            "{record_name} grant_id is empty"
        )));
    }
    Ok(grant_id)
}

fn validate_descendant_kinds(raw_kinds: &[i32]) -> Result<(), AuthorityError> {
    let actual: HashSet<_> = raw_kinds.iter().copied().collect();
    let expected: HashSet<_> = DESCENDANT_GRANT_ALLOWED_KINDS
        .iter()
        .map(|kind| *kind as i32)
        .collect();
    if raw_kinds.len() != DESCENDANT_GRANT_ALLOWED_KINDS.len() || actual != expected {
        return Err(AuthorityError::InvalidGrant(
            "descendant grant must contain exactly the canonical existing-session operation kinds"
                .to_owned(),
        ));
    }
    Ok(())
}

fn validate_revocation_effects(
    effects: &[GrantRevocationEffect],
    grant_id: &GrantId,
) -> Result<(), AuthorityError> {
    let mut command_ids = HashSet::new();
    for effect in effects {
        let command_id = effect.command_id.as_ref().ok_or_else(|| {
            AuthorityError::InvalidGrant(format!(
                "revocation of grant {:?} has an effect without command_id",
                grant_id
            ))
        })?;
        if command_id.value.is_empty() || !command_ids.insert(command_id.value.clone()) {
            return Err(AuthorityError::InvalidGrant(format!(
                "revocation of grant {:?} has a missing or duplicate command effect",
                grant_id
            )));
        }
        let from = OperationState::try_from(effect.from_state).map_err(|_| {
            AuthorityError::InvalidGrant(format!(
                "revocation effect for {:?} has unknown from_state",
                command_id
            ))
        })?;
        let to = OperationState::try_from(effect.to_state).map_err(|_| {
            AuthorityError::InvalidGrant(format!(
                "revocation effect for {:?} has unknown to_state",
                command_id
            ))
        })?;
        let failure = FailureCode::try_from(effect.failure_code).map_err(|_| {
            AuthorityError::InvalidGrant(format!(
                "revocation effect for {:?} has unknown failure_code",
                command_id
            ))
        })?;
        if !matches!(
            (from, to, failure),
            (
                OperationState::Accepted | OperationState::Delivered | OperationState::Running,
                OperationState::Cancelled,
                FailureCode::Cancelled
            ) | (
                OperationState::Accepted,
                OperationState::Rejected,
                FailureCode::AuthorizationDenied
            )
        ) {
            return Err(AuthorityError::InvalidGrant(format!(
                "revocation effect for {:?} has invalid state/failure combination",
                command_id
            )));
        }
    }
    Ok(())
}

fn validate_event_id(
    event_id: &EventId,
    expected_domain: &AuthorityDomainId,
) -> Result<(), AuthorityError> {
    match event_id.authority_domain_id.as_ref() {
        Some(actual_domain) if actual_domain == expected_domain => {}
        Some(actual_domain) => {
            return Err(AuthorityError::CorruptRecord(format!(
                "storage returned authority event for domain {:?}, expected {:?}",
                actual_domain, expected_domain
            )));
        }
        None => {
            return Err(AuthorityError::CorruptRecord(
                "storage returned authority event without authority_domain_id".to_owned(),
            ));
        }
    }
    if event_id.lsn.as_ref().is_none_or(|lsn| lsn.value == 0) {
        return Err(AuthorityError::CorruptRecord(
            "storage returned authority event without a positive LSN".to_owned(),
        ));
    }
    Ok(())
}
