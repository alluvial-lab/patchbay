//! Shared runtime-generation fencing and the staged/quarantined/promotion envelopes.
//!
//! The generated envelopes are the replay boundary: quarantined candidates are
//! never recursively dispatched, successor reports reserve identity only, and
//! one `SpawnPromotionCommitted` is the first event that can publish the new
//! runtime, descendant authority, claim consumption, and command completion.

use patchbay_contracts::patchbay::{
    runtime_generation_disposition, typed_correlation, AuthorityDomainId, CommandId, EventId,
    ExternalRuntimeRef, FailureCode, OperationState, QuarantinedRuntimeEvidence,
    RuntimeEvidenceQuarantineReason, RuntimeEvidenceSourceAttachment,
    RuntimeGenerationClaimedSuccessor, RuntimeGenerationCurrent, RuntimeGenerationDisposition,
    RuntimeGenerationIdentityMismatch, RuntimeGenerationTombstoned, RuntimeGenerationUnknown,
    SessionReport, SpawnClaimDisposition, SpawnPromotionCommitted, SpawnSuccessorEvidenceStaged,
    StoredEventKind, StoredEventPayload,
};
use prost::Message;

use crate::{
    acceptance::{AcceptanceError, CommandIndex},
    authority::{AuthorityError, AuthorityRegistry},
    storage::RecordedEvent,
};

use super::{ExternalRuntimeOwnership, LogicalTargetRegistry, SpawnClaimQuery, SpawnClaimRegistry};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeEvidenceError {
    #[error("runtime evidence is malformed: {0}")]
    Malformed(String),
    #[error("runtime evidence violates the generation fence: {0}")]
    Fence(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnPromotionFoldError {
    #[error(transparent)]
    Authority(#[from] AuthorityError),
    #[error(transparent)]
    Session(#[from] super::SessionError),
    #[error(transparent)]
    Claim(#[from] super::SpawnClaimError),
    #[error(transparent)]
    Command(#[from] AcceptanceError),
}

/// Apply one committed promotion under the decision gate in its mandatory
/// authority → session → claim → command order. All projections are staged on
/// clones and become visible together only after every exact pre-state check
/// succeeds.
pub fn fold_spawn_promotion_ordered(
    authority: &mut AuthorityRegistry,
    sessions: &mut super::SessionRegistry,
    claims: &mut SpawnClaimRegistry,
    commands: &mut CommandIndex,
    event: &RecordedEvent,
) -> Result<(), SpawnPromotionFoldError> {
    if StoredEventKind::try_from(event.payload.kind).ok()
        != Some(StoredEventKind::SpawnPromotionCommitted)
    {
        return Err(SpawnPromotionFoldError::Command(
            AcceptanceError::CorruptRecord(
                "ordered promotion fold received another event kind".to_owned(),
            ),
        ));
    }
    let mut next_authority = authority.clone();
    let mut next_sessions = sessions.clone();
    let mut next_claims = claims.clone();
    let mut next_commands = commands.clone();
    next_authority.observe(event)?;
    next_sessions.observe(event)?;
    next_claims.observe(event)?;
    next_commands.apply(event)?;
    *authority = next_authority;
    *sessions = next_sessions;
    *claims = next_claims;
    *commands = next_commands;
    Ok(())
}

/// Classify one authenticated session report through the shared generation
/// fence. `ClaimedSuccessor` is returned only for an exact active durable claim;
/// it conveys staging authority and nothing else.
pub fn classify_session_report(
    authority_domain_id: &AuthorityDomainId,
    report: &SessionReport,
    source: &RuntimeEvidenceSourceAttachment,
    claims: &SpawnClaimRegistry,
    targets: &LogicalTargetRegistry,
) -> RuntimeGenerationDisposition {
    let Some(candidate) = report_external(report) else {
        return identity_mismatch();
    };
    let source_matches = source.adapter_id.as_ref() == report.adapter_id.as_ref()
        && source
            .adapter_generation
            .is_some_and(|generation| generation.value > 0)
        && report
            .source_cursor
            .as_ref()
            .and_then(|cursor| cursor.adapter_generation)
            == source.adapter_generation;

    if source_matches {
        if let Some(command_id) = report_claim_operation(report) {
            if let Some(record) = claims.claim_for_operation(command_id) {
                let claim = &record.claim;
                let target_id = claim.logical_target_id.as_ref();
                let target = target_id.and_then(|id| targets.get(id));
                let exact_prior = claim.expected_prior.as_ref();
                let current_matches =
                    target.is_some_and(|target| target.current.as_ref() == exact_prior);
                let candidate_matches = claim.claimed_generation.is_some_and(|generation| {
                    generation == report.session_generation.unwrap_or_default()
                }) && target.is_some_and(|target| {
                    report.adapter_id.as_ref() == Some(&target.adapter_id)
                        && report.deployment_scope == target.deployment_scope
                });
                if record.disposition == SpawnClaimDisposition::Active
                    && claim.authority_domain_id.as_ref() == Some(authority_domain_id)
                    && record.adapter_id == report.adapter_id.clone().unwrap_or_default()
                    && current_matches
                    && candidate_matches
                {
                    return RuntimeGenerationDisposition {
                        disposition: Some(
                            runtime_generation_disposition::Disposition::ClaimedSuccessor(
                                RuntimeGenerationClaimedSuccessor {
                                    claim_operation_id: Some(command_id.clone()),
                                    expected_prior: exact_prior.cloned(),
                                    claimed_generation: claim.claimed_generation,
                                },
                            ),
                        ),
                    };
                }
            }
        }
    }

    if let Some(owner) = targets.owner_of(&candidate) {
        if let Some(target) = targets.get(owner) {
            if target
                .current
                .as_ref()
                .is_some_and(|current| current.external_runtime.as_ref() == Some(&candidate))
            {
                return RuntimeGenerationDisposition {
                    disposition: Some(runtime_generation_disposition::Disposition::Current(
                        RuntimeGenerationCurrent {},
                    )),
                };
            }
            if let Some(tombstone) = target
                .tombstones
                .values()
                .find(|tombstone| tombstone.external_runtime_ref == candidate)
            {
                return RuntimeGenerationDisposition {
                    disposition: Some(runtime_generation_disposition::Disposition::Tombstoned(
                        RuntimeGenerationTombstoned {
                            superseded_at_lsn: Some(patchbay_contracts::patchbay::Lsn {
                                value: tombstone.superseded_at_lsn,
                            }),
                        },
                    )),
                };
            }
        }
    }

    let same_native_lineage = targets.records().any(|target| {
        target
            .current
            .iter()
            .filter_map(|runtime| runtime.external_runtime.as_ref())
            .chain(target.reserved_candidate.iter())
            .chain(
                target
                    .tombstones
                    .values()
                    .map(|value| &value.external_runtime_ref),
            )
            .any(|known| same_runtime_without_generation(known, &candidate))
    });
    if same_native_lineage || !source_matches {
        identity_mismatch()
    } else {
        RuntimeGenerationDisposition {
            disposition: Some(runtime_generation_disposition::Disposition::Unknown(
                RuntimeGenerationUnknown {},
            )),
        }
    }
}

pub fn validate_staged_successor(
    staged: &SpawnSuccessorEvidenceStaged,
) -> Result<(), RuntimeEvidenceError> {
    let domain = nonempty_domain(staged.authority_domain_id.as_ref())?;
    let claim = staged
        .exact_claim
        .as_ref()
        .ok_or_else(|| malformed("staged successor has no exact claim"))?;
    if claim.authority_domain_id.as_ref() != Some(domain) {
        return Err(fence("staged successor claim belongs to another domain"));
    }
    let report = staged
        .report
        .as_ref()
        .ok_or_else(|| malformed("staged successor has no session report"))?;
    let candidate = report_external(report)
        .ok_or_else(|| malformed("staged successor report has malformed external identity"))?;
    let target = staged
        .classified_target
        .as_ref()
        .ok_or_else(|| malformed("staged successor has no classified target"))?;
    if target.logical_target_id != claim.logical_target_id
        || target.external_runtime.as_ref() != Some(&candidate)
        || staged.external_runtime_reservation.as_ref() != Some(&candidate)
        || claim.claimed_generation != report.session_generation
    {
        return Err(fence(
            "staged successor target/reservation does not match the exact claim report",
        ));
    }
    let source = staged
        .source_attachment
        .as_ref()
        .ok_or_else(|| malformed("staged successor has no source attachment"))?;
    validate_source(source, report)?;
    let disposition = staged
        .disposition
        .as_ref()
        .and_then(|value| value.disposition.as_ref());
    match disposition {
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(value))
            if value.claim_operation_id == claim.claim_operation_id
                && value.expected_prior == claim.expected_prior
                && value.claimed_generation == claim.claimed_generation => {}
        _ => {
            return Err(fence(
                "staged successor is not the exact ClaimedSuccessor disposition",
            ))
        }
    }
    if report_claim_operation(report) != claim.claim_operation_id.as_ref() {
        return Err(fence(
            "staged successor report does not name the claim operation",
        ));
    }
    Ok(())
}

pub fn validate_quarantined_runtime_evidence(
    quarantined: &QuarantinedRuntimeEvidence,
) -> Result<(), RuntimeEvidenceError> {
    nonempty_domain(quarantined.authority_domain_id.as_ref())?;
    if quarantined.candidate.is_none() {
        return Err(malformed(
            "quarantine has no admitted generated candidate family",
        ));
    }
    let classification = quarantined
        .classification
        .as_ref()
        .and_then(|value| value.disposition.as_ref())
        .and_then(|value| value.disposition.as_ref())
        .ok_or_else(|| malformed("quarantine has no runtime classification"))?;
    let reason = RuntimeEvidenceQuarantineReason::try_from(quarantined.reason)
        .map_err(|_| malformed("quarantine reason is unknown"))?;
    if reason == RuntimeEvidenceQuarantineReason::Unspecified {
        return Err(malformed("quarantine reason is unspecified"));
    }
    if matches!(
        classification,
        runtime_generation_disposition::Disposition::ClaimedSuccessor(_)
    ) {
        return Err(fence(
            "an exact claimed successor must stage, not quarantine",
        ));
    }
    if matches!(
        classification,
        runtime_generation_disposition::Disposition::Current(_)
    ) && reason != RuntimeEvidenceQuarantineReason::StaleSourceOrder
    {
        return Err(fence(
            "current evidence may quarantine only for stale source order",
        ));
    }
    let source = quarantined
        .source_attachment
        .as_ref()
        .ok_or_else(|| malformed("quarantine has no source attachment"))?;
    if source
        .adapter_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
        || source
            .adapter_generation
            .is_none_or(|generation| generation.value == 0)
        || source.attachment_event_id.is_none()
    {
        return Err(malformed("quarantine source attachment is incomplete"));
    }
    Ok(())
}

pub fn validate_spawn_promotion_envelope(
    promotion: &SpawnPromotionCommitted,
    event_id: &EventId,
) -> Result<(), RuntimeEvidenceError> {
    let domain = nonempty_domain(promotion.authority_domain_id.as_ref())?;
    if promotion.promotion_event_id.as_ref() != Some(event_id)
        || event_id.authority_domain_id.as_ref() != Some(domain)
    {
        return Err(fence(
            "promotion identity/domain does not match its durable envelope",
        ));
    }
    let promotion_lsn = event_lsn(event_id)?;
    let audit_id = promotion
        .completion_audit_event_id
        .as_ref()
        .ok_or_else(|| malformed("promotion has no completion audit id"))?;
    if audit_id.authority_domain_id.as_ref() != Some(domain)
        || event_lsn(audit_id)?
            != promotion_lsn
                .checked_add(1)
                .ok_or_else(|| fence("promotion LSN overflow"))?
    {
        return Err(fence(
            "promotion audit is not the atomic immediate successor",
        ));
    }
    let accepted = promotion
        .accepted_claim
        .as_ref()
        .ok_or_else(|| malformed("promotion has no accepted claim"))?;
    let claim = accepted
        .claim
        .as_ref()
        .ok_or_else(|| malformed("promotion accepted decision has no claim"))?;
    let command_id = claim
        .claim_operation_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| malformed("promotion claim has no operation id"))?;
    if claim.authority_domain_id.as_ref() != Some(domain)
        || event_lsn(
            promotion
                .accepted_claim_event_id
                .as_ref()
                .ok_or_else(|| malformed("promotion has no accepted claim event id"))?,
        )? >= promotion_lsn
    {
        return Err(fence(
            "promotion accepted claim is not in the preceding domain prefix",
        ));
    }
    validate_lifecycle(&promotion.lifecycle, command_id, promotion_lsn)?;
    let result = promotion
        .successful_result
        .as_ref()
        .ok_or_else(|| malformed("promotion has no successful result"))?;
    if result.command_id.as_ref() != Some(command_id)
        || FailureCode::try_from(result.failure_code).ok() != Some(FailureCode::Unspecified)
        || event_lsn(
            result
                .event_id
                .as_ref()
                .ok_or_else(|| malformed("result has no event id"))?,
        )? >= promotion_lsn
    {
        return Err(fence(
            "promotion result is not exact prior successful evidence",
        ));
    }
    let staged_ref = promotion
        .staged_successor
        .as_ref()
        .ok_or_else(|| malformed("promotion has no staged successor"))?;
    let staged = staged_ref
        .staged
        .as_ref()
        .ok_or_else(|| malformed("promotion staged evidence has no envelope"))?;
    validate_staged_successor(staged)?;
    if staged.exact_claim.as_ref() != Some(claim)
        || event_lsn(
            staged_ref
                .event_id
                .as_ref()
                .ok_or_else(|| malformed("staged evidence has no event id"))?,
        )? >= promotion_lsn
    {
        return Err(fence(
            "promotion staged evidence does not bind the exact claim/prefix",
        ));
    }
    let promoted = promotion
        .promoted_runtime
        .as_ref()
        .ok_or_else(|| malformed("promotion has no promoted runtime"))?;
    if promoted.logical_target_id != claim.logical_target_id
        || promoted.external_runtime != promotion.external_runtime_reservation
        || promoted
            != staged
                .classified_target
                .as_ref()
                .expect("staged target validated")
    {
        return Err(fence(
            "promotion runtime/reservation does not match staged evidence",
        ));
    }
    validate_exact_generation_transition(claim, promoted)?;
    let authority = promotion
        .authority
        .as_ref()
        .ok_or_else(|| malformed("promotion has no authority evidence"))?;
    let accepted_operation = accepted
        .accepted_operation
        .as_ref()
        .ok_or_else(|| malformed("accepted claim has no accepted operation"))?;
    if authority.spawning_grant_id != accepted_operation.authorizing_grant_id
        || authority.continuation_authority != accepted.compound_authority
    {
        return Err(fence(
            "promotion authority does not preserve accepted compound provenance",
        ));
    }
    let descendant = authority
        .descendant_grant
        .as_ref()
        .ok_or_else(|| malformed("promotion has no descendant grant"))?;
    if descendant.audit_id.as_ref() != Some(audit_id)
        || descendant.authority_domain_id.as_ref() != Some(domain)
        || descendant.provenance.as_ref().is_none_or(|provenance| {
            provenance.spawn_operation_id.as_ref() != Some(command_id)
                || provenance.spawning_grant_id != authority.spawning_grant_id
                || provenance.continuation_authority != authority.continuation_authority
        })
    {
        return Err(fence(
            "promotion descendant does not preserve operation/grant/audit provenance",
        ));
    }
    if promotion.committed_at.is_none() {
        return Err(malformed("promotion has no committed_at"));
    }
    Ok(())
}

#[must_use]
pub fn encode_staged_successor(event: &SpawnSuccessorEvidenceStaged) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SpawnSuccessorEvidenceStaged as i32,
        payload: event.encode_to_vec(),
    }
}

pub fn encode_quarantined_runtime_evidence(
    event: &QuarantinedRuntimeEvidence,
) -> Result<StoredEventPayload, RuntimeEvidenceError> {
    validate_quarantined_runtime_evidence(event)?;
    Ok(StoredEventPayload {
        kind: StoredEventKind::QuarantinedRuntimeEvidence as i32,
        payload: event.encode_to_vec(),
    })
}

#[must_use]
pub fn encode_spawn_promotion(event: &SpawnPromotionCommitted) -> StoredEventPayload {
    StoredEventPayload {
        kind: StoredEventKind::SpawnPromotionCommitted as i32,
        payload: event.encode_to_vec(),
    }
}

fn validate_lifecycle(
    lifecycle: &[patchbay_contracts::patchbay::SpawnPromotionLifecycleEvidence],
    command_id: &CommandId,
    promotion_lsn: u64,
) -> Result<(), RuntimeEvidenceError> {
    if !(lifecycle.len() == 1 || lifecycle.len() == 2) {
        return Err(fence(
            "promotion lifecycle must contain delivered and optional running",
        ));
    }
    let states = if lifecycle.len() == 1 {
        [
            (OperationState::Accepted, OperationState::Delivered),
            (OperationState::Unspecified, OperationState::Unspecified),
        ]
    } else {
        [
            (OperationState::Accepted, OperationState::Delivered),
            (OperationState::Delivered, OperationState::Running),
        ]
    };
    let mut previous_lsn = 0;
    for (index, evidence) in lifecycle.iter().enumerate() {
        let transition = evidence
            .transition
            .as_ref()
            .ok_or_else(|| malformed("promotion lifecycle entry has no transition"))?;
        let lsn = event_lsn(
            evidence
                .event_id
                .as_ref()
                .ok_or_else(|| malformed("promotion lifecycle entry has no event id"))?,
        )?;
        if lsn <= previous_lsn
            || lsn >= promotion_lsn
            || transition.command_id.as_ref() != Some(command_id)
            || OperationState::try_from(transition.from_state).ok() != Some(states[index].0)
            || OperationState::try_from(transition.to_state).ok() != Some(states[index].1)
            || FailureCode::try_from(transition.failure_code).ok() != Some(FailureCode::Unspecified)
        {
            return Err(fence(
                "promotion lifecycle is not exact delivered/running evidence",
            ));
        }
        previous_lsn = lsn;
    }
    Ok(())
}

fn validate_exact_generation_transition(
    claim: &patchbay_contracts::patchbay::SpawnGenerationClaim,
    promoted: &patchbay_contracts::patchbay::RuntimeGenerationRef,
) -> Result<(), RuntimeEvidenceError> {
    let generation = promoted
        .external_runtime
        .as_ref()
        .and_then(|runtime| runtime.generation)
        .ok_or_else(|| malformed("promoted runtime has no generation"))?;
    if Some(generation) != claim.claimed_generation {
        return Err(fence("promoted generation differs from the exact claim"));
    }
    match claim.expected_prior.as_ref() {
        None if generation.value == 1 => Ok(()),
        Some(prior)
            if prior.logical_target_id == claim.logical_target_id
                && prior
                    .external_runtime
                    .as_ref()
                    .and_then(|runtime| runtime.generation)
                    .and_then(|prior| prior.value.checked_add(1))
                    == Some(generation.value) =>
        {
            Ok(())
        }
        _ => Err(fence("promotion is not fresh ∅→1 or exact N→N+1")),
    }
}

fn validate_source(
    source: &RuntimeEvidenceSourceAttachment,
    report: &SessionReport,
) -> Result<(), RuntimeEvidenceError> {
    if source.adapter_id != report.adapter_id
        || source
            .adapter_id
            .as_ref()
            .is_none_or(|id| id.value.is_empty())
        || source
            .adapter_generation
            .is_none_or(|generation| generation.value == 0)
        || source.attachment_event_id.is_none()
        || report
            .source_cursor
            .as_ref()
            .and_then(|cursor| cursor.adapter_generation)
            != source.adapter_generation
    {
        return Err(fence(
            "runtime evidence source does not match the authenticated report epoch",
        ));
    }
    Ok(())
}

fn report_external(report: &SessionReport) -> Option<ExternalRuntimeRef> {
    let adapter_id = report
        .adapter_id
        .clone()
        .filter(|id| !id.value.is_empty())?;
    let runtime_session_id = report
        .runtime_session_id
        .clone()
        .filter(|id| !id.value.is_empty())?;
    let generation = report
        .session_generation
        .filter(|generation| generation.value > 0)?;
    if report.deployment_scope.is_empty() {
        return None;
    }
    Some(ExternalRuntimeRef {
        adapter_id: Some(adapter_id),
        deployment_scope: report.deployment_scope.clone(),
        runtime_session_id: Some(runtime_session_id),
        generation: Some(generation),
    })
}

fn report_claim_operation(report: &SessionReport) -> Option<&CommandId> {
    match report.spawn_origin.as_ref()?.r#ref.as_ref()? {
        typed_correlation::Ref::CommandId(command_id) if !command_id.value.is_empty() => {
            Some(command_id)
        }
        _ => None,
    }
}

fn same_runtime_without_generation(left: &ExternalRuntimeRef, right: &ExternalRuntimeRef) -> bool {
    left.adapter_id == right.adapter_id
        && left.deployment_scope == right.deployment_scope
        && left.runtime_session_id == right.runtime_session_id
}

fn identity_mismatch() -> RuntimeGenerationDisposition {
    RuntimeGenerationDisposition {
        disposition: Some(
            runtime_generation_disposition::Disposition::IdentityMismatch(
                RuntimeGenerationIdentityMismatch {},
            ),
        ),
    }
}

fn nonempty_domain(
    domain: Option<&AuthorityDomainId>,
) -> Result<&AuthorityDomainId, RuntimeEvidenceError> {
    domain
        .filter(|domain| !domain.value.is_empty())
        .ok_or_else(|| malformed("authority domain is missing or empty"))
}

fn event_lsn(event_id: &EventId) -> Result<u64, RuntimeEvidenceError> {
    event_id
        .lsn
        .filter(|lsn| lsn.value > 0)
        .map(|lsn| lsn.value)
        .ok_or_else(|| malformed("event id has no positive LSN"))
}

fn malformed(message: impl Into<String>) -> RuntimeEvidenceError {
    RuntimeEvidenceError::Malformed(message.into())
}

fn fence(message: impl Into<String>) -> RuntimeEvidenceError {
    RuntimeEvidenceError::Fence(message.into())
}
