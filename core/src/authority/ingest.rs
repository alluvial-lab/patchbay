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

use crate::{acceptance::Clock, storage::{AuditRecordDraft, RecordedEvent, Storage}};

use super::{
    events, AuthorityError, AuthorityRegistry, GrantProjection, DESCENDANT_GRANT_ALLOWED_KINDS,
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

    let grant_id = grant.grant_id.clone();
    let payload = events::grant(authority_domain_id.clone(), grant);
    preflight_creation(&payload, authority_domain_id)?;
    let kind = if projection.current_grant(&grant_id.expect("validated grant id")).await.is_some() {
        patchbay_contracts::patchbay::AuditEventKind::GrantChanged
    } else {
        patchbay_contracts::patchbay::AuditEventKind::GrantCreated
    };
    let mut audit = AuditRecordDraft::new(crate::acceptance::SystemClock.now(), kind);
    audit.reason_code = if kind == patchbay_contracts::patchbay::AuditEventKind::GrantChanged {
        "grant_changed"
    } else {
        "grant_created"
    }.to_owned();
    append_and_warm_decision(storage, projection, authority_domain_id, payload, audit).await
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

    let payload = events::descendant_grant(authority_domain_id.clone(), grant);
    preflight_creation(&payload, authority_domain_id)?;
    append_and_warm(storage, projection, authority_domain_id, payload).await
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
    let occurred_at = revocation.revoked_at.unwrap_or_else(|| crate::time::SystemClock.now());
    let mut audits = Vec::with_capacity(1 + revocation.command_effects.len());
    let mut audit = AuditRecordDraft::new(occurred_at, patchbay_contracts::patchbay::AuditEventKind::GrantRevoked);
    audit.grant_id = Some(grant_id.clone());
    audit.reason_code = "grant_revoked".to_owned();
    audits.push(audit);
    for effect in &revocation.command_effects {
        let mut effect_audit = AuditRecordDraft::new(
            revocation.revoked_at.unwrap_or_else(|| crate::time::SystemClock.now()),
            match OperationState::try_from(effect.to_state).ok() {
                Some(OperationState::Cancelled) => patchbay_contracts::patchbay::AuditEventKind::CommandCancelled,
                Some(OperationState::Rejected) => patchbay_contracts::patchbay::AuditEventKind::CommandRejected,
                _ => patchbay_contracts::patchbay::AuditEventKind::CommandSubmissionRejected,
            },
        );
        effect_audit.grant_id = Some(grant_id.clone());
        effect_audit.command_id = effect.command_id.clone();
        effect_audit.failure_code = FailureCode::try_from(effect.failure_code).ok().filter(|code| *code != FailureCode::Unspecified);
        effect_audit.reason_code = "grant_revocation_policy".to_owned();
        audits.push(effect_audit);
    }
    if revocation.command_effects.is_empty() {
        let audit = audits.into_iter().next().expect("grant revocation has grant audit");
        append_and_warm_decision(storage, projection, authority_domain_id, payload, audit).await
    } else {
        append_and_warm_decision_many(storage, projection, authority_domain_id, payload, audits).await
    }
}

async fn append_and_warm<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    let event_id = storage.append(authority_domain_id, payload.clone()).await?;
    validate_event_id(&event_id, authority_domain_id)?;

    // Durability precedes projection mutation. If this fold fails, callers
    // must rebuild the hot projection from the authoritative log.
    projection.observe(&RecordedEvent {
        event_id: event_id.clone(),
        payload,
    })?;
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
    let result = storage.append_decision_audited_many(authority_domain_id, payload.clone(), audits).await?;
    validate_event_id(&result.source_event_id, authority_domain_id)?;
    projection.observe(&RecordedEvent { event_id: result.source_event_id.clone(), payload })?;
    Ok(result.source_event_id)
}

async fn append_and_warm_decision<S, L>(
    storage: &S,
    projection: &mut L,
    authority_domain_id: &AuthorityDomainId,
    payload: StoredEventPayload,
    audit: AuditRecordDraft,
) -> Result<EventId, AuthorityError>
where
    S: Storage,
    L: GrantProjection,
{
    let event_id = storage.append_decision(authority_domain_id, payload.clone(), audit).await?;
    validate_event_id(&event_id, authority_domain_id)?;
    projection.observe(&RecordedEvent { event_id: event_id.clone(), payload })?;
    Ok(event_id)
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
            AuthorityError::InvalidGrant(format!("revocation of grant {:?} has an effect without command_id", grant_id))
        })?;
        if command_id.value.is_empty() || !command_ids.insert(command_id.value.clone()) {
            return Err(AuthorityError::InvalidGrant(format!(
                "revocation of grant {:?} has a missing or duplicate command effect",
                grant_id
            )));
        }
        let from = OperationState::try_from(effect.from_state).map_err(|_| {
            AuthorityError::InvalidGrant(format!("revocation effect for {:?} has unknown from_state", command_id))
        })?;
        let to = OperationState::try_from(effect.to_state).map_err(|_| {
            AuthorityError::InvalidGrant(format!("revocation effect for {:?} has unknown to_state", command_id))
        })?;
        let failure = FailureCode::try_from(effect.failure_code).map_err(|_| {
            AuthorityError::InvalidGrant(format!("revocation effect for {:?} has unknown failure_code", command_id))
        })?;
        if !matches!((from, to, failure),
            (OperationState::Accepted | OperationState::Delivered | OperationState::Running, OperationState::Cancelled, FailureCode::Cancelled)
            | (OperationState::Accepted, OperationState::Rejected, FailureCode::AuthorizationDenied)
        ) {
            return Err(AuthorityError::InvalidGrant(format!("revocation effect for {:?} has invalid state/failure combination", command_id)));
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
    if event_id.lsn.is_none() {
        return Err(AuthorityError::CorruptRecord(
            "storage returned authority event without an LSN".to_owned(),
        ));
    }
    Ok(())
}
