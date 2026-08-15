//! Shared runtime-generation fencing and the staged/quarantined/promotion envelopes.
//!
//! The generated envelopes are the replay boundary: quarantined candidates are
//! never recursively dispatched, successor reports reserve identity only, and
//! one `SpawnPromotionCommitted` is the first event that can publish the new
//! runtime, descendant authority, claim consumption, and command completion.

use std::collections::{HashMap, HashSet};

use patchbay_contracts::patchbay::{
    quarantined_runtime_evidence, runtime_generation_disposition, spawn_claim_event,
    typed_correlation, AuthorityDomainId, CommandId, CommandTransition, DescendantGrant,
    DescendantGrantProvenance, EventId, ExternalRuntimeRef, FailureCode, GrantId,
    GrantRevocationPolicy, LogicalTargetTombstone, Observation, ObservationKind, OperationKind,
    OperationState, QuarantinedRuntimeEvidence, RuntimeEvidenceClassificationContext,
    RuntimeEvidenceQuarantineReason, RuntimeEvidenceSourceAttachment,
    RuntimeGenerationClaimedSuccessor, RuntimeGenerationCurrent, RuntimeGenerationDisposition,
    RuntimeGenerationIdentityMismatch, RuntimeGenerationRef, RuntimeGenerationTombstoned,
    RuntimeGenerationUnknown, SessionReport, SpawnClaimAccepted, SpawnClaimDisposition,
    SpawnClaimEvent, SpawnPromotionAuthorityEvidence, SpawnPromotionCommitted,
    SpawnPromotionLifecycleEvidence, SpawnPromotionResultEvidence, SpawnPromotionStagedEvidence,
    SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind,
};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    acceptance::{
        exact_command_correlation, AcceptanceError, CommandIndex, RuntimeEvidenceCandidate,
    },
    adapter::AdapterRegistry,
    authority::{
        descendant_grant_id, AuthorityError, AuthorityRegistry, DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::RecordedEvent,
    target::TargetRegistry,
};

use super::{ExternalRuntimeOwnership, SpawnClaimQuery, SpawnClaimRegistry};

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
    #[error(transparent)]
    Target(#[from] crate::target::TargetRegistryError),
}

#[derive(Debug, Clone)]
struct PromotionFacts {
    accepted_event_id: EventId,
    accepted: SpawnClaimAccepted,
    lifecycle: Vec<SpawnPromotionLifecycleEvidence>,
    successful_result: Option<SpawnPromotionResultEvidence>,
    canonical_result_source: Option<Observation>,
    staged_successor: Option<SpawnPromotionStagedEvidence>,
    promoted: bool,
}

/// Derive the next unstamped promotion from an exact durable prefix.
///
/// This is the production producer paired with the dedicated atomic storage
/// append. It never infers a successor from ordinary session registration: the
/// exact accepted claim, delivered/running lifecycle, successful Result, and
/// typed staged-successor envelope must all already be durable.
pub fn next_spawn_promotion(
    authority_domain_id: &AuthorityDomainId,
    events: &[RecordedEvent],
    committed_at: Timestamp,
) -> Result<Option<SpawnPromotionCommitted>, RuntimeEvidenceError> {
    let mut commands = CommandIndex::new();
    let mut facts: HashMap<CommandId, PromotionFacts> = HashMap::new();

    for event in events {
        if event.event_id.authority_domain_id.as_ref() != Some(authority_domain_id) {
            return Err(fence("promotion producer observed a cross-domain event"));
        }
        let event_id = event.event_id.clone();
        let kind = StoredEventKind::try_from(event.payload.kind)
            .map_err(|_| malformed("promotion producer observed an unknown event kind"))?;
        match kind {
            StoredEventKind::SpawnClaim => {
                let envelope = SpawnClaimEvent::decode(event.payload.payload.as_slice())
                    .map_err(|error| malformed(format!("cannot decode spawn claim: {error}")))?;
                if let Some(spawn_claim_event::Mutation::Accepted(accepted)) = envelope.mutation {
                    let command_id = accepted
                        .claim
                        .as_ref()
                        .and_then(|claim| claim.claim_operation_id.as_ref())
                        .filter(|id| !id.value.is_empty())
                        .ok_or_else(|| malformed("accepted spawn claim has no operation id"))?
                        .clone();
                    if facts
                        .insert(
                            command_id,
                            PromotionFacts {
                                accepted_event_id: event_id,
                                accepted,
                                lifecycle: Vec::new(),
                                successful_result: None,
                                canonical_result_source: None,
                                staged_successor: None,
                                promoted: false,
                            },
                        )
                        .is_some()
                    {
                        return Err(fence(
                            "promotion producer observed duplicate accepted claim",
                        ));
                    }
                }
            }
            StoredEventKind::CommandTransition => {
                let transition = CommandTransition::decode(event.payload.payload.as_slice())
                    .map_err(|error| {
                        malformed(format!("cannot decode promotion lifecycle: {error}"))
                    })?;
                if let Some(progress) = transition
                    .command_id
                    .as_ref()
                    .and_then(|command_id| facts.get_mut(command_id))
                {
                    let edge = (
                        OperationState::try_from(transition.from_state).ok(),
                        OperationState::try_from(transition.to_state).ok(),
                    );
                    if matches!(
                        edge,
                        (
                            Some(OperationState::Accepted),
                            Some(OperationState::Delivered)
                        ) | (
                            Some(OperationState::Delivered),
                            Some(OperationState::Running)
                        )
                    ) {
                        progress.lifecycle.push(SpawnPromotionLifecycleEvidence {
                            event_id: Some(event_id),
                            transition: Some(transition),
                        });
                    }
                }
            }
            StoredEventKind::Observation => {
                let observation =
                    Observation::decode(event.payload.payload.as_slice()).map_err(|error| {
                        malformed(format!("cannot decode promotion Result: {error}"))
                    })?;
                if ObservationKind::try_from(observation.kind).ok() == Some(ObservationKind::Result)
                {
                    if let Some(command_id) = exact_command_correlation(&observation.correlations) {
                        // Reconcile every exact target-matched Result while the
                        // spawn is delivered/running before considering its
                        // outcome. This makes a stranded failed Result a
                        // durable promotion fence rather than evidence the
                        // success-only filter can skip.
                        let qualified_at_result_lsn =
                            commands.get_command(&command_id).is_some_and(|record| {
                                record.operation.kind == OperationKind::Spawn as i32
                                    && observation.target_scope == record.operation.target_scope
                                    && matches!(
                                        record.state,
                                        OperationState::Delivered | OperationState::Running
                                    )
                            });
                        if qualified_at_result_lsn {
                            let failure_code = FailureCode::try_from(observation.failure_code)
                                .map_err(|_| {
                                    malformed(
                                        "promotion producer observed Result with unknown failure code",
                                    )
                                })?;
                            if let Some(progress) = facts.get_mut(&command_id) {
                                if let Some(existing) = progress.canonical_result_source.as_ref() {
                                    if existing != &observation {
                                        return Err(fence(
                                            "promotion producer observed conflicting Result evidence",
                                        ));
                                    }
                                } else {
                                    if failure_code == FailureCode::Unspecified {
                                        progress.successful_result =
                                            Some(SpawnPromotionResultEvidence {
                                                event_id: Some(event_id),
                                                command_id: Some(command_id),
                                                target_scope: observation.target_scope.clone(),
                                                failure_code: observation.failure_code,
                                                observed_at: observation.observed_at,
                                            });
                                    }
                                    progress.canonical_result_source = Some(observation);
                                }
                            }
                        }
                    }
                }
            }
            StoredEventKind::SpawnSuccessorEvidenceStaged => {
                let staged = SpawnSuccessorEvidenceStaged::decode(event.payload.payload.as_slice())
                    .map_err(|error| {
                        malformed(format!("cannot decode staged successor: {error}"))
                    })?;
                validate_staged_successor(&staged)?;
                let command_id = staged
                    .exact_claim
                    .as_ref()
                    .and_then(|claim| claim.claim_operation_id.as_ref())
                    .ok_or_else(|| malformed("staged successor has no claim operation id"))?;
                if let Some(progress) = facts.get_mut(command_id) {
                    if progress.staged_successor.is_some() {
                        return Err(fence(
                            "promotion producer observed duplicate staged successor",
                        ));
                    }
                    progress.staged_successor = Some(SpawnPromotionStagedEvidence {
                        event_id: Some(event_id),
                        staged: Some(staged),
                    });
                }
            }
            StoredEventKind::SpawnPromotionCommitted => {
                let promotion = SpawnPromotionCommitted::decode(event.payload.payload.as_slice())
                    .map_err(|error| {
                    malformed(format!("cannot decode committed promotion: {error}"))
                })?;
                let command_id = promotion
                    .accepted_claim
                    .as_ref()
                    .and_then(|accepted| accepted.claim.as_ref())
                    .and_then(|claim| claim.claim_operation_id.as_ref())
                    .ok_or_else(|| malformed("committed promotion has no operation id"))?;
                if let Some(progress) = facts.get_mut(command_id) {
                    progress.promoted = true;
                }
            }
            _ => {}
        }
        commands
            .apply(event)
            .map_err(|error| fence(format!("promotion producer command fold failed: {error}")))?;
    }

    let mut ready = Vec::new();
    for (command_id, progress) in facts {
        if progress.promoted {
            continue;
        }
        let Some(record) = commands.get_command(&command_id) else {
            return Err(fence("accepted spawn claim has no command projection"));
        };
        if !matches!(
            record.state,
            OperationState::Delivered | OperationState::Running
        ) {
            continue;
        }
        let Some(result) = progress.successful_result else {
            continue;
        };
        let Some(staged_ref) = progress.staged_successor else {
            continue;
        };
        let accepted_operation = progress
            .accepted
            .accepted_operation
            .as_ref()
            .ok_or_else(|| malformed("accepted claim has no accepted Operation"))?;
        let operation = accepted_operation
            .operation
            .as_ref()
            .ok_or_else(|| malformed("accepted claim has no Operation"))?;
        let sender = operation
            .sender
            .as_ref()
            .ok_or_else(|| malformed("accepted spawn has no sender"))?;
        let claim = progress
            .accepted
            .claim
            .as_ref()
            .ok_or_else(|| malformed("accepted claim has no generation claim"))?;
        let continuation_authority = progress.accepted.compound_authority.clone();
        let staged = staged_ref
            .staged
            .as_ref()
            .ok_or_else(|| malformed("staged reference has no envelope"))?;
        if staged.exact_claim.as_ref() != Some(claim)
            || result.target_scope != operation.target_scope
        {
            return Err(fence(
                "promotion producer facts do not bind the exact accepted claim/target",
            ));
        }
        let promoted_runtime = staged
            .classified_target
            .clone()
            .ok_or_else(|| malformed("staged successor has no classified target"))?;
        let external = promoted_runtime
            .external_runtime
            .clone()
            .ok_or_else(|| malformed("staged successor has no external runtime"))?;
        let descendant_target = TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: external.adapter_id.clone(),
            deployment_scope: external.deployment_scope.clone(),
            runtime_session_id: external.runtime_session_id.clone(),
            session_generation: external.generation,
            ..TargetScope::default()
        };
        let spawning_grant_id = accepted_operation
            .authorizing_grant_id
            .clone()
            .ok_or_else(|| malformed("accepted spawn has no authorizing grant"))?;
        let descendant = DescendantGrant {
            grant_id: Some(descendant_grant_id(authority_domain_id, &command_id)),
            authority_domain_id: Some(authority_domain_id.clone()),
            subject_actor_id: sender.actor_id.clone(),
            subject_endpoint_id: sender.endpoint_id.clone(),
            subject_endpoint_class: String::new(),
            target_scope: Some(descendant_target),
            allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
                .iter()
                .map(|kind| *kind as i32)
                .collect(),
            provenance: Some(DescendantGrantProvenance {
                spawn_operation_id: Some(command_id.clone()),
                spawning_grant_id: Some(spawning_grant_id.clone()),
                continuation_authority: progress.accepted.compound_authority.clone(),
            }),
            created_at: Some(committed_at),
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..DescendantGrant::default()
        };
        let staged_lsn = event_lsn(
            staged_ref
                .event_id
                .as_ref()
                .ok_or_else(|| malformed("staged reference has no event id"))?,
        )?;
        ready.push((
            staged_lsn,
            command_id.value.clone(),
            SpawnPromotionCommitted {
                authority_domain_id: Some(authority_domain_id.clone()),
                accepted_claim_event_id: Some(progress.accepted_event_id),
                accepted_claim: Some(progress.accepted),
                lifecycle: progress.lifecycle,
                successful_result: Some(result),
                staged_successor: Some(staged_ref),
                promoted_runtime: Some(promoted_runtime),
                external_runtime_reservation: Some(external),
                authority: Some(SpawnPromotionAuthorityEvidence {
                    spawning_grant_id: Some(spawning_grant_id),
                    continuation_authority,
                    descendant_grant: Some(descendant),
                }),
                committed_at: Some(committed_at),
                ..SpawnPromotionCommitted::default()
            },
        ));
    }
    ready.sort_by(|left, right| (left.0, left.1.as_bytes()).cmp(&(right.0, right.1.as_bytes())));
    Ok(ready.into_iter().next().map(|(_, _, promotion)| promotion))
}

/// Capability produced only after the staged authority projection has
/// installed the promotion's exact descendant Grant. Requiring this witness in
/// the session phase makes authority → session ordering structural rather than
/// an incidental ordering of two independent calls.
struct AuthorityInstalledPromotion<'a> {
    event: &'a RecordedEvent,
    descendant_id: GrantId,
}

fn install_promotion_authority<'a>(
    authority: &mut AuthorityRegistry,
    event: &'a RecordedEvent,
    descendant_id: &GrantId,
) -> Result<AuthorityInstalledPromotion<'a>, SpawnPromotionFoldError> {
    authority.observe(event)?;
    if authority.get_grant(descendant_id).is_none() {
        return Err(SpawnPromotionFoldError::Authority(
            AuthorityError::CorruptLog(
                "ordered promotion did not install descendant authority first".to_owned(),
            ),
        ));
    }
    Ok(AuthorityInstalledPromotion {
        event,
        descendant_id: descendant_id.clone(),
    })
}

fn publish_promotion_session_after_authority(
    authority: &AuthorityRegistry,
    targets: &mut TargetRegistry,
    installed: &AuthorityInstalledPromotion<'_>,
) -> Result<(), SpawnPromotionFoldError> {
    if authority.get_grant(&installed.descendant_id).is_none() {
        return Err(SpawnPromotionFoldError::Authority(
            AuthorityError::CorruptLog(
                "session promotion phase lacks installed descendant authority".to_owned(),
            ),
        ));
    }
    targets
        .sessions_mut()
        .observe(installed.event)
        .map_err(SpawnPromotionFoldError::Session)
}

/// Apply one committed promotion under the decision gate in its mandatory
/// authority → session → claim → command order. All projections are staged on
/// clones and become visible together only after every exact pre-state check
/// succeeds.
pub fn fold_spawn_promotion_ordered(
    authority: &mut AuthorityRegistry,
    targets: &mut TargetRegistry,
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
    let promotion =
        SpawnPromotionCommitted::decode(event.payload.payload.as_slice()).map_err(|error| {
            SpawnPromotionFoldError::Command(AcceptanceError::CorruptRecord(format!(
                "cannot decode ordered promotion: {error}"
            )))
        })?;
    validate_spawn_promotion_result_order(&promotion).map_err(|error| {
        SpawnPromotionFoldError::Command(AcceptanceError::CorruptLog(error.to_string()))
    })?;
    validate_spawn_promotion_envelope(&promotion, &event.event_id).map_err(|error| {
        SpawnPromotionFoldError::Command(AcceptanceError::CorruptLog(error.to_string()))
    })?;
    validate_spawn_promotion_generation_prestate(targets, claims, &promotion)?;
    let accepted_claim = promotion
        .accepted_claim
        .as_ref()
        .and_then(|accepted| accepted.claim.as_ref())
        .ok_or_else(|| {
            SpawnPromotionFoldError::Command(AcceptanceError::CorruptRecord(
                "ordered promotion has no exact claim".to_owned(),
            ))
        })?;
    let command_id = accepted_claim.claim_operation_id.as_ref().ok_or_else(|| {
        SpawnPromotionFoldError::Command(AcceptanceError::CorruptRecord(
            "ordered promotion has no command id".to_owned(),
        ))
    })?;
    let descendant_id = promotion
        .authority
        .as_ref()
        .and_then(|authority| authority.descendant_grant.as_ref())
        .and_then(|grant| grant.grant_id.as_ref())
        .ok_or_else(|| {
            SpawnPromotionFoldError::Command(AcceptanceError::CorruptRecord(
                "ordered promotion has no descendant grant id".to_owned(),
            ))
        })?;
    let promoted_external = promotion
        .promoted_runtime
        .as_ref()
        .and_then(|runtime| runtime.external_runtime.as_ref())
        .ok_or_else(|| {
            SpawnPromotionFoldError::Command(AcceptanceError::CorruptRecord(
                "ordered promotion has no promoted runtime".to_owned(),
            ))
        })?;

    let mut next_authority = authority.clone();
    let mut next_targets = targets.clone();
    let mut next_claims = claims.clone();
    let mut next_commands = commands.clone();

    // This order is the publication invariant. The session phase cannot be
    // invoked until the authority phase returns its private installed witness.
    // TargetRegistry's resource/adapter children consume the sibling phase
    // later while resource still advances its aggregate cursor.
    let authority_installed =
        install_promotion_authority(&mut next_authority, event, descendant_id)?;
    publish_promotion_session_after_authority(
        &next_authority,
        &mut next_targets,
        &authority_installed,
    )?;
    let adapter_id = promoted_external
        .adapter_id
        .as_ref()
        .expect("promotion validated");
    let runtime_id = promoted_external
        .runtime_session_id
        .as_ref()
        .expect("promotion validated");
    let generation = promoted_external
        .generation
        .as_ref()
        .expect("promotion validated");
    if next_targets
        .sessions()
        .get_session(&super::SessionIdentity {
            adapter_id: adapter_id.clone(),
            deployment_scope: promoted_external.deployment_scope.clone(),
            runtime_session_id: runtime_id.clone(),
            session_generation: *generation,
        })
        .is_none()
    {
        return Err(SpawnPromotionFoldError::Session(
            super::SessionError::CorruptLog(
                "ordered promotion did not publish the exact session after authority".to_owned(),
            ),
        ));
    }
    next_claims.observe(event)?;
    if next_claims
        .claim_for_operation(command_id)
        .is_none_or(|record| record.disposition != SpawnClaimDisposition::Promoted)
    {
        return Err(SpawnPromotionFoldError::Claim(
            super::SpawnClaimError::CorruptLog(
                "ordered promotion did not consume the exact claim after session publication"
                    .to_owned(),
            ),
        ));
    }
    next_commands.apply(event)?;
    if next_commands
        .get_command(command_id)
        .is_none_or(|record| record.state != OperationState::Completed)
    {
        return Err(SpawnPromotionFoldError::Command(
            AcceptanceError::CorruptLog(
                "ordered promotion did not complete the command after claim consumption".to_owned(),
            ),
        ));
    }
    next_targets.observe_promotion_siblings(event)?;

    *authority = next_authority;
    *targets = next_targets;
    *claims = next_claims;
    *commands = next_commands;
    Ok(())
}

fn validate_spawn_promotion_generation_prestate(
    targets: &TargetRegistry,
    claims: &SpawnClaimRegistry,
    promotion: &SpawnPromotionCommitted,
) -> Result<(), SpawnPromotionFoldError> {
    let accepted = promotion
        .accepted_claim
        .as_ref()
        .expect("promotion envelope validated");
    let claim = accepted
        .claim
        .as_ref()
        .expect("promotion envelope validated");
    let command_id = claim
        .claim_operation_id
        .as_ref()
        .expect("promotion envelope validated");
    let projected_claim = claims.claim_for_operation(command_id).ok_or_else(|| {
        SpawnPromotionFoldError::Claim(super::SpawnClaimError::CorruptLog(
            "promotion has no exact projected claim pre-state".to_owned(),
        ))
    })?;
    if projected_claim.claim != *claim
        || projected_claim.compound_authority != accepted.compound_authority
        || projected_claim.pending_replacement != claim.expected_prior
        || !matches!(
            projected_claim.disposition,
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation
        )
    {
        return Err(SpawnPromotionFoldError::Claim(
            super::SpawnClaimError::CorruptLog(
                "promotion does not match the exact unconsumed claim/fence pre-state".to_owned(),
            ),
        ));
    }

    let logical_target_id = claim
        .logical_target_id
        .as_ref()
        .expect("promotion envelope validated");
    let promoted_external = promotion
        .promoted_runtime
        .as_ref()
        .and_then(|runtime| runtime.external_runtime.as_ref())
        .expect("promotion envelope validated");
    let target = targets
        .sessions()
        .logical_targets()
        .get(logical_target_id)
        .ok_or_else(|| {
            SpawnPromotionFoldError::Session(super::SessionError::CorruptLog(
                "promotion references an unknown logical-target pre-state".to_owned(),
            ))
        })?;
    if target.current.as_ref() != claim.expected_prior.as_ref()
        || target.reserved_candidate.as_ref() != Some(promoted_external)
    {
        return Err(SpawnPromotionFoldError::Session(
            super::SessionError::CorruptLog(
                "promotion is not the immediate exact current-to-reserved transition".to_owned(),
            ),
        ));
    }
    validate_exact_generation_transition(
        claim,
        promotion
            .promoted_runtime
            .as_ref()
            .expect("promotion envelope validated"),
    )
    .map_err(|error| {
        SpawnPromotionFoldError::Session(super::SessionError::CorruptLog(error.to_string()))
    })?;

    let ownership = targets.sessions().logical_targets();
    if ownership.owner_of(promoted_external) != Some(logical_target_id)
        || claim.expected_prior.as_ref().is_some_and(|prior| {
            prior
                .external_runtime
                .as_ref()
                .is_none_or(|external| ownership.owner_of(external) != Some(logical_target_id))
        })
    {
        return Err(SpawnPromotionFoldError::Session(
            super::SessionError::CorruptLog(
                "promotion current/reservation reverse ownership is not retained by the exact logical target"
                    .to_owned(),
            ),
        ));
    }
    Ok(())
}

/// Classify one authenticated session report through the shared generation
/// fence. `ClaimedSuccessor` is returned only for an exact active durable claim;
/// it conveys staging authority and nothing else.
pub fn classify_session_report(
    authority_domain_id: &AuthorityDomainId,
    report: &SessionReport,
    source: &RuntimeEvidenceSourceAttachment,
    adapters: &AdapterRegistry,
    claims: &SpawnClaimRegistry,
    sessions: &super::SessionRegistry,
) -> RuntimeGenerationDisposition {
    let Some(candidate) = report_external(report) else {
        return identity_mismatch();
    };
    let targets = sessions.logical_targets();
    let source_matches = source.adapter_id.as_ref() == report.adapter_id.as_ref()
        && report
            .source_cursor
            .as_ref()
            .and_then(|cursor| cursor.adapter_generation)
            == source.adapter_generation
        && source_matches_current_attachment(authority_domain_id, source, adapters);

    if source_matches {
        if let Some(command_id) = report_claim_operation(report) {
            if let Some(record) = claims.claim_for_operation(command_id) {
                if claim_matches_report_candidate(
                    authority_domain_id,
                    record,
                    report,
                    targets,
                    true,
                ) {
                    let claim = &record.claim;
                    return RuntimeGenerationDisposition {
                        disposition: Some(
                            runtime_generation_disposition::Disposition::ClaimedSuccessor(
                                RuntimeGenerationClaimedSuccessor {
                                    claim_operation_id: Some(command_id.clone()),
                                    expected_prior: claim.expected_prior.clone(),
                                    claimed_generation: claim.claimed_generation,
                                },
                            ),
                        ),
                    };
                }
            }
        }

        // A candidate that fits an active managed claim may not become an
        // unmanaged registration merely by dropping or changing spawn_origin.
        if matching_candidate_claim(authority_domain_id, report, claims, targets).is_some() {
            return identity_mismatch();
        }
    }

    if source_matches && report.spawn_origin.is_none() {
        if let Some(owner) = targets.owner_of(&candidate) {
            if let Some(target) = targets.get(owner) {
                if target
                    .current
                    .as_ref()
                    .is_some_and(|current| current.external_runtime.as_ref() == Some(&candidate))
                {
                    return current_disposition();
                }
                if let Some(tombstone) = target
                    .tombstones
                    .values()
                    .find(|tombstone| tombstone.external_runtime_ref == candidate)
                {
                    return tombstoned_disposition(tombstone.superseded_at_lsn);
                }
            }
        }

        let adapter_id = candidate.adapter_id.as_ref().expect("candidate validated");
        let runtime_id = candidate
            .runtime_session_id
            .as_ref()
            .expect("candidate validated");
        let generation = candidate.generation.as_ref().expect("candidate validated");
        if sessions
            .get_session(&super::SessionIdentity {
                adapter_id: adapter_id.clone(),
                deployment_scope: candidate.deployment_scope.clone(),
                runtime_session_id: runtime_id.clone(),
                session_generation: *generation,
            })
            .is_some()
        {
            return current_disposition();
        }
        if let Some(live) =
            sessions.get_live_session(adapter_id, &candidate.deployment_scope, runtime_id)
        {
            let managed_lineage = targets.records().any(|target| {
                target.adapter_id == *adapter_id
                    && target.deployment_scope == candidate.deployment_scope
                    && target
                        .current
                        .as_ref()
                        .and_then(|current| current.external_runtime.as_ref())
                        .is_some_and(|known| same_runtime_without_generation(known, &candidate))
            });
            if !managed_lineage && generation.value > live.identity.session_generation.value {
                // An unmanaged producer may report its own next runtime
                // generation. Active managed claims were fenced above; this
                // branch preserves authenticated non-spawn replacement.
                return current_disposition();
            }
        }
        if let Some(tombstone) = sessions.get_tombstone(
            adapter_id,
            &candidate.deployment_scope,
            runtime_id,
            generation,
        ) {
            return tombstoned_disposition(tombstone.superseded_at_lsn);
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
    }) || sessions.sessions().any(|record| {
        let known = ExternalRuntimeRef {
            adapter_id: Some(record.identity.adapter_id.clone()),
            deployment_scope: record.identity.deployment_scope.clone(),
            runtime_session_id: Some(record.identity.runtime_session_id.clone()),
            generation: Some(record.identity.session_generation),
        };
        same_runtime_without_generation(&known, &candidate)
    }) || sessions.tombstones().any(|tombstone| {
        tombstone.adapter_id == *candidate.adapter_id.as_ref().expect("candidate validated")
            && tombstone.deployment_scope == candidate.deployment_scope
            && tombstone.runtime_session_id
                == *candidate
                    .runtime_session_id
                    .as_ref()
                    .expect("candidate validated")
    });
    if same_native_lineage || !source_matches || report.spawn_origin.is_some() {
        identity_mismatch()
    } else {
        // An authenticated, unclaimed first report is the ordinary discovery
        // boundary. Classifying it Current means the shared fence admits it to
        // the existing source-order/identity validator; `Unknown` remains a
        // reject/quarantine disposition and is never an ordinary-ingress
        // exception.
        current_disposition()
    }
}

fn claim_matches_report_candidate(
    authority_domain_id: &AuthorityDomainId,
    record: &super::SpawnClaimRecord,
    report: &SessionReport,
    targets: &super::LogicalTargetRegistry,
    require_active: bool,
) -> bool {
    let claim = &record.claim;
    let target = claim
        .logical_target_id
        .as_ref()
        .and_then(|id| targets.get(id));
    let disposition_matches = if require_active {
        record.disposition == SpawnClaimDisposition::Active
    } else {
        matches!(
            record.disposition,
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation
        )
    };
    let target_matches = target.map_or_else(
        || {
            // A fresh claim reserves the stable logical id before the adapter
            // reports the deployment-scoped external runtime for generation 1.
            // The staging event creates that target from the authenticated
            // report; continuations must always match an existing exact prior.
            claim.expected_prior.is_none()
                && report.adapter_id.as_ref() == Some(&record.adapter_id)
                && !report.deployment_scope.is_empty()
        },
        |target| {
            target.current.as_ref() == claim.expected_prior.as_ref()
                && report.adapter_id.as_ref() == Some(&target.adapter_id)
                && report.deployment_scope == target.deployment_scope
        },
    );
    disposition_matches
        && claim.authority_domain_id.as_ref() == Some(authority_domain_id)
        && record.adapter_id == report.adapter_id.clone().unwrap_or_default()
        && target_matches
        && claim.claimed_generation == report.session_generation
}

fn matching_candidate_claim<'a>(
    authority_domain_id: &AuthorityDomainId,
    report: &SessionReport,
    claims: &'a SpawnClaimRegistry,
    targets: &super::LogicalTargetRegistry,
) -> Option<&'a super::SpawnClaimRecord> {
    let mut matching: Vec<_> = claims
        .records()
        .filter(|record| {
            claim_matches_report_candidate(authority_domain_id, record, report, targets, false)
        })
        .collect();
    matching.sort_by(|left, right| {
        left.claim
            .claim_operation_id
            .as_ref()
            .map(|id| id.value.as_bytes())
            .cmp(
                &right
                    .claim
                    .claim_operation_id
                    .as_ref()
                    .map(|id| id.value.as_bytes()),
            )
    });
    matching.into_iter().next()
}

fn current_disposition() -> RuntimeGenerationDisposition {
    RuntimeGenerationDisposition {
        disposition: Some(runtime_generation_disposition::Disposition::Current(
            RuntimeGenerationCurrent {},
        )),
    }
}

fn tombstoned_disposition(superseded_at_lsn: u64) -> RuntimeGenerationDisposition {
    RuntimeGenerationDisposition {
        disposition: Some(runtime_generation_disposition::Disposition::Tombstoned(
            RuntimeGenerationTombstoned {
                superseded_at_lsn: Some(patchbay_contracts::patchbay::Lsn {
                    value: superseded_at_lsn,
                }),
            },
        )),
    }
}

/// Classify an authenticated exact runtime target for non-report evidence.
/// Claimed successor output is deliberately not admitted here: only the exact
/// SessionReport shape may stage a successor.
pub fn classify_runtime_target(
    authority_domain_id: &AuthorityDomainId,
    external: &ExternalRuntimeRef,
    source: &RuntimeEvidenceSourceAttachment,
    adapters: &AdapterRegistry,
    sessions: &super::SessionRegistry,
) -> RuntimeGenerationDisposition {
    if validate_external_runtime(external, "runtime evidence target").is_err()
        || source.adapter_id.as_ref() != external.adapter_id.as_ref()
        || !source_matches_current_attachment(authority_domain_id, source, adapters)
    {
        return identity_mismatch();
    }
    let adapter_id = external.adapter_id.as_ref().expect("external validated");
    let runtime_id = external
        .runtime_session_id
        .as_ref()
        .expect("external validated");
    let generation = external.generation.as_ref().expect("external validated");
    if sessions
        .get_session(&super::SessionIdentity {
            adapter_id: adapter_id.clone(),
            deployment_scope: external.deployment_scope.clone(),
            runtime_session_id: runtime_id.clone(),
            session_generation: *generation,
        })
        .is_some()
    {
        return RuntimeGenerationDisposition {
            disposition: Some(runtime_generation_disposition::Disposition::Current(
                RuntimeGenerationCurrent {},
            )),
        };
    }
    if let Some(tombstone) = sessions.get_tombstone(
        adapter_id,
        &external.deployment_scope,
        runtime_id,
        generation,
    ) {
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
    let same_lineage = sessions.sessions().any(|record| {
        record.identity.adapter_id == *adapter_id
            && record.identity.deployment_scope == external.deployment_scope
            && record.identity.runtime_session_id == *runtime_id
    }) || sessions.tombstones().any(|tombstone| {
        tombstone.adapter_id == *adapter_id
            && tombstone.deployment_scope == external.deployment_scope
            && tombstone.runtime_session_id == *runtime_id
    });
    if same_lineage {
        identity_mismatch()
    } else {
        RuntimeGenerationDisposition {
            disposition: Some(runtime_generation_disposition::Disposition::Unknown(
                RuntimeGenerationUnknown {},
            )),
        }
    }
}

pub fn quarantined_runtime_candidate(
    authority_domain_id: &AuthorityDomainId,
    candidate: RuntimeEvidenceCandidate,
    disposition: RuntimeGenerationDisposition,
    reason: RuntimeEvidenceQuarantineReason,
    source: RuntimeEvidenceSourceAttachment,
    sessions: &super::SessionRegistry,
    claims: &SpawnClaimRegistry,
) -> Result<QuarantinedRuntimeEvidence, RuntimeEvidenceError> {
    let external = runtime_evidence_candidate_target(authority_domain_id, &candidate)?;
    quarantine_envelope(
        authority_domain_id,
        candidate,
        external,
        disposition,
        reason,
        source,
        (sessions, claims),
    )
}

pub fn quarantined_observation(
    authority_domain_id: &AuthorityDomainId,
    observation: Observation,
    disposition: RuntimeGenerationDisposition,
    reason: RuntimeEvidenceQuarantineReason,
    source: RuntimeEvidenceSourceAttachment,
    sessions: &super::SessionRegistry,
    claims: &SpawnClaimRegistry,
) -> Result<QuarantinedRuntimeEvidence, RuntimeEvidenceError> {
    quarantined_runtime_candidate(
        authority_domain_id,
        quarantined_runtime_evidence::Candidate::Observation(observation),
        disposition,
        reason,
        source,
        sessions,
        claims,
    )
}

pub fn quarantined_session_report(
    authority_domain_id: &AuthorityDomainId,
    report: SessionReport,
    disposition: RuntimeGenerationDisposition,
    reason: RuntimeEvidenceQuarantineReason,
    source: RuntimeEvidenceSourceAttachment,
    sessions: &super::SessionRegistry,
    claims: &SpawnClaimRegistry,
) -> Result<QuarantinedRuntimeEvidence, RuntimeEvidenceError> {
    quarantined_runtime_candidate(
        authority_domain_id,
        quarantined_runtime_evidence::Candidate::SessionReport(report),
        disposition,
        reason,
        source,
        sessions,
        claims,
    )
}

fn quarantine_envelope(
    authority_domain_id: &AuthorityDomainId,
    candidate: quarantined_runtime_evidence::Candidate,
    external: ExternalRuntimeRef,
    disposition: RuntimeGenerationDisposition,
    reason: RuntimeEvidenceQuarantineReason,
    source: RuntimeEvidenceSourceAttachment,
    projections: (&super::SessionRegistry, &SpawnClaimRegistry),
) -> Result<QuarantinedRuntimeEvidence, RuntimeEvidenceError> {
    let (sessions, claims) = projections;
    let classification = canonical_runtime_evidence_classification_context(
        authority_domain_id,
        &candidate,
        &external,
        disposition,
        sessions,
        claims,
    );
    let envelope = QuarantinedRuntimeEvidence {
        authority_domain_id: Some(authority_domain_id.clone()),
        candidate: Some(candidate),
        classification: Some(classification),
        reason: reason as i32,
        source_attachment: Some(source),
    };
    validate_quarantined_runtime_evidence(&envelope)?;
    Ok(envelope)
}

/// Reconstruct the complete durable classification context for one admitted
/// quarantine candidate. Storage uses this same constructor and requires exact
/// equality, so callers cannot forge an owner, current generation, tombstone,
/// or claim while retaining only the correct disposition.
#[must_use]
pub fn canonical_runtime_evidence_classification_context(
    authority_domain_id: &AuthorityDomainId,
    candidate: &quarantined_runtime_evidence::Candidate,
    external: &ExternalRuntimeRef,
    disposition: RuntimeGenerationDisposition,
    sessions: &super::SessionRegistry,
    claims: &SpawnClaimRegistry,
) -> RuntimeEvidenceClassificationContext {
    let targets = sessions.logical_targets();
    let logical_target_id = targets.owner_of(external).cloned();
    let classified_target = RuntimeGenerationRef {
        logical_target_id: logical_target_id.clone(),
        external_runtime: Some(external.clone()),
    };

    let related_logical_target = logical_target_id
        .as_ref()
        .and_then(|id| targets.get(id))
        .or_else(|| {
            targets.records().find(|target| {
                target
                    .current
                    .as_ref()
                    .and_then(|current| current.external_runtime.as_ref())
                    .is_some_and(|known| same_runtime_without_generation(known, external))
                    || target.tombstones.values().any(|known| {
                        same_runtime_without_generation(&known.external_runtime_ref, external)
                    })
            })
        });
    let current = related_logical_target
        .and_then(|target| target.current.clone())
        .or_else(|| {
            let adapter = external.adapter_id.as_ref()?;
            let runtime = external.runtime_session_id.as_ref()?;
            let live = sessions.get_live_session(adapter, &external.deployment_scope, runtime)?;
            let live_external = ExternalRuntimeRef {
                adapter_id: Some(live.identity.adapter_id.clone()),
                deployment_scope: live.identity.deployment_scope.clone(),
                runtime_session_id: Some(live.identity.runtime_session_id.clone()),
                generation: Some(live.identity.session_generation),
            };
            Some(RuntimeGenerationRef {
                logical_target_id: targets.owner_of(&live_external).cloned(),
                external_runtime: Some(live_external),
            })
        });

    let tombstone = related_logical_target
        .and_then(|target| {
            target
                .tombstones
                .values()
                .find(|known| known.external_runtime_ref == *external)
        })
        .map(|known| LogicalTargetTombstone {
            external_runtime_ref: Some(known.external_runtime_ref.clone()),
            superseded_at_lsn: Some(patchbay_contracts::patchbay::Lsn {
                value: known.superseded_at_lsn,
            }),
        })
        .or_else(|| {
            let adapter = external.adapter_id.as_ref()?;
            let runtime = external.runtime_session_id.as_ref()?;
            let generation = external.generation.as_ref()?;
            sessions
                .get_tombstone(adapter, &external.deployment_scope, runtime, generation)
                .map(|known| LogicalTargetTombstone {
                    external_runtime_ref: Some(external.clone()),
                    superseded_at_lsn: Some(patchbay_contracts::patchbay::Lsn {
                        value: known.superseded_at_lsn,
                    }),
                })
        });

    let correlated_claim = match candidate {
        quarantined_runtime_evidence::Candidate::Observation(observation) => {
            crate::acceptance::exact_command_correlation(&observation.correlations)
                .and_then(|id| claims.claim_for_operation(&id))
        }
        quarantined_runtime_evidence::Candidate::SessionReport(report) => {
            report_claim_operation(report)
                .and_then(|id| claims.claim_for_operation(id))
                .or_else(|| matching_candidate_claim(authority_domain_id, report, claims, targets))
        }
        quarantined_runtime_evidence::Candidate::DeliveryAcknowledgement(acknowledgement) => {
            acknowledgement
                .command_id
                .as_ref()
                .and_then(|id| claims.claim_for_operation(id))
        }
        quarantined_runtime_evidence::Candidate::TranscriptStatus(status) => status
            .observation
            .as_ref()
            .and_then(|observation| {
                crate::acceptance::exact_command_correlation(&observation.correlations)
            })
            .and_then(|id| claims.claim_for_operation(&id)),
        quarantined_runtime_evidence::Candidate::ElicitationMutation(mutation) => mutation
            .elicitation
            .as_ref()
            .and_then(|elicitation| {
                crate::acceptance::exact_command_correlation(&elicitation.correlations)
            })
            .and_then(|id| claims.claim_for_operation(&id)),
    };
    let active_claim = correlated_claim
        .filter(|record| {
            matches!(
                record.disposition,
                SpawnClaimDisposition::Active
                    | SpawnClaimDisposition::PoisonedPendingReconciliation
            )
        })
        .map(|record| record.claim.clone());

    RuntimeEvidenceClassificationContext {
        disposition: Some(disposition),
        classified_target: Some(classified_target),
        current,
        tombstone,
        active_claim,
    }
}

#[must_use]
pub fn quarantine_reason_for(
    disposition: &RuntimeGenerationDisposition,
) -> RuntimeEvidenceQuarantineReason {
    match disposition.disposition.as_ref() {
        Some(runtime_generation_disposition::Disposition::Tombstoned(_)) => {
            RuntimeEvidenceQuarantineReason::Tombstoned
        }
        Some(runtime_generation_disposition::Disposition::Unknown(_)) => {
            RuntimeEvidenceQuarantineReason::UnknownTarget
        }
        Some(runtime_generation_disposition::Disposition::IdentityMismatch(_)) => {
            RuntimeEvidenceQuarantineReason::IdentityMismatch
        }
        Some(runtime_generation_disposition::Disposition::Current(_)) => {
            RuntimeEvidenceQuarantineReason::StaleSourceOrder
        }
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_)) | None => {
            RuntimeEvidenceQuarantineReason::ClaimMismatch
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
    let domain = nonempty_domain(quarantined.authority_domain_id.as_ref())?;
    let candidate = quarantined_candidate_target(quarantined)?;
    let context = quarantined
        .classification
        .as_ref()
        .ok_or_else(|| malformed("quarantine has no classification context"))?;
    let classified_target = context
        .classified_target
        .as_ref()
        .ok_or_else(|| malformed("quarantine has no classified target"))?;
    validate_runtime_generation_ref(classified_target, "quarantine classified target")?;
    if classified_target.external_runtime.as_ref() != Some(&candidate) {
        return Err(fence(
            "quarantine candidate does not match its classified runtime target",
        ));
    }
    let classification = context
        .disposition
        .as_ref()
        .and_then(|value| value.disposition.as_ref())
        .ok_or_else(|| malformed("quarantine has no runtime classification"))?;
    let reason = RuntimeEvidenceQuarantineReason::try_from(quarantined.reason)
        .map_err(|_| malformed("quarantine reason is unknown"))?;
    let reason_matches = match (classification, reason) {
        (
            runtime_generation_disposition::Disposition::Current(_),
            RuntimeEvidenceQuarantineReason::StaleSourceOrder,
        ) => context.current.as_ref() == Some(classified_target),
        (
            runtime_generation_disposition::Disposition::Tombstoned(disposition),
            RuntimeEvidenceQuarantineReason::Tombstoned,
        ) => context.tombstone.as_ref().is_some_and(|tombstone| {
            tombstone.external_runtime_ref.as_ref() == Some(&candidate)
                && tombstone.superseded_at_lsn == disposition.superseded_at_lsn
                && tombstone.superseded_at_lsn.is_some_and(|lsn| lsn.value > 0)
        }),
        (
            runtime_generation_disposition::Disposition::Unknown(_),
            RuntimeEvidenceQuarantineReason::UnknownTarget,
        ) => true,
        (
            runtime_generation_disposition::Disposition::IdentityMismatch(_),
            RuntimeEvidenceQuarantineReason::IdentityMismatch,
        ) => true,
        (
            runtime_generation_disposition::Disposition::IdentityMismatch(_),
            RuntimeEvidenceQuarantineReason::ClaimMismatch,
        ) => context.active_claim.as_ref().is_some_and(|claim| {
            claim.authority_domain_id.as_ref() == Some(domain)
                && claim
                    .claim_operation_id
                    .as_ref()
                    .is_some_and(|id| !id.value.is_empty())
        }),
        (
            runtime_generation_disposition::Disposition::IdentityMismatch(_),
            RuntimeEvidenceQuarantineReason::StaleAttachment,
        ) => true,
        _ => false,
    };
    if !reason_matches {
        return Err(fence(
            "quarantine reason, disposition, candidate, and target context disagree",
        ));
    }
    if matches!(
        classification,
        runtime_generation_disposition::Disposition::ClaimedSuccessor(_)
    ) {
        return Err(fence(
            "an exact claimed successor must stage, not quarantine",
        ));
    }
    let source = quarantined
        .source_attachment
        .as_ref()
        .ok_or_else(|| malformed("quarantine has no source attachment"))?;
    if source.adapter_id.as_ref() != candidate.adapter_id.as_ref()
        || source
            .adapter_id
            .as_ref()
            .is_none_or(|id| id.value.is_empty())
        || source
            .adapter_generation
            .is_none_or(|generation| generation.value == 0)
        || source.attachment_event_id.as_ref().is_none_or(|event_id| {
            event_id.authority_domain_id.as_ref() != Some(domain)
                || event_id.lsn.is_none_or(|lsn| lsn.value == 0)
        })
    {
        return Err(malformed(
            "quarantine source attachment is incomplete or targets another runtime adapter",
        ));
    }
    Ok(())
}

/// Return the exact runtime target nested inside one generated ingress family.
pub fn runtime_evidence_candidate_target(
    authority_domain_id: &AuthorityDomainId,
    candidate: &RuntimeEvidenceCandidate,
) -> Result<ExternalRuntimeRef, RuntimeEvidenceError> {
    let domain = nonempty_domain(Some(authority_domain_id))?;
    let external = match candidate {
        quarantined_runtime_evidence::Candidate::Observation(observation) => {
            if observation.authority_domain_id.as_ref() != Some(domain) {
                return Err(fence("quarantined Observation belongs to another domain"));
            }
            external_from_scope(observation.target_scope.as_ref(), "quarantined Observation")?
        }
        quarantined_runtime_evidence::Candidate::SessionReport(report) => report_external(report)
            .ok_or_else(|| {
            malformed("quarantined SessionReport has malformed runtime identity")
        })?,
        quarantined_runtime_evidence::Candidate::DeliveryAcknowledgement(acknowledgement) => {
            if acknowledgement
                .command_id
                .as_ref()
                .is_none_or(|id| id.value.is_empty())
            {
                return Err(malformed(
                    "quarantined delivery acknowledgement has no command id",
                ));
            }
            let target = acknowledgement.target.as_ref().ok_or_else(|| {
                malformed("quarantined delivery acknowledgement has no runtime target")
            })?;
            validate_runtime_generation_ref(target, "quarantined delivery acknowledgement")?;
            target.external_runtime.clone().expect("validated runtime")
        }
        quarantined_runtime_evidence::Candidate::TranscriptStatus(status) => {
            let observation = status.observation.as_ref().ok_or_else(|| {
                malformed("quarantined transcript/status evidence has no Observation")
            })?;
            if observation.authority_domain_id.as_ref() != Some(domain) {
                return Err(fence(
                    "quarantined transcript/status Observation belongs to another domain",
                ));
            }
            external_from_scope(
                observation.target_scope.as_ref(),
                "quarantined transcript/status Observation",
            )?
        }
        quarantined_runtime_evidence::Candidate::ElicitationMutation(mutation) => {
            let elicitation = mutation
                .elicitation
                .as_ref()
                .ok_or_else(|| malformed("quarantined Elicitation mutation has no Elicitation"))?;
            if elicitation.authority_domain_id.as_ref() != Some(domain)
                || mutation.from_state == mutation.to_state
                || patchbay_contracts::patchbay::ElicitationState::try_from(mutation.from_state)
                    .is_err()
                || patchbay_contracts::patchbay::ElicitationState::try_from(mutation.to_state)
                    .is_err()
            {
                return Err(fence(
                    "quarantined Elicitation mutation has invalid domain or transition framing",
                ));
            }
            external_from_scope(
                elicitation.target_context.as_ref(),
                "quarantined Elicitation mutation",
            )?
        }
    };
    validate_external_runtime(&external, "runtime evidence candidate")?;
    Ok(external)
}

/// Return the exact runtime target nested inside an admitted quarantine family.
pub fn quarantined_candidate_target(
    quarantined: &QuarantinedRuntimeEvidence,
) -> Result<ExternalRuntimeRef, RuntimeEvidenceError> {
    let domain = nonempty_domain(quarantined.authority_domain_id.as_ref())?;
    let candidate = quarantined
        .candidate
        .as_ref()
        .ok_or_else(|| malformed("quarantine has no admitted generated candidate family"))?;
    runtime_evidence_candidate_target(domain, candidate)
}

/// Canonical runtime-session audit target for the nested candidate.
pub fn quarantined_candidate_scope(
    quarantined: &QuarantinedRuntimeEvidence,
) -> Result<TargetScope, RuntimeEvidenceError> {
    let external = quarantined_candidate_target(quarantined)?;
    Ok(TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: external.adapter_id,
        deployment_scope: external.deployment_scope,
        runtime_session_id: external.runtime_session_id,
        session_generation: external.generation,
        ..TargetScope::default()
    })
}

#[must_use]
pub fn quarantine_reason_code(reason: RuntimeEvidenceQuarantineReason) -> &'static str {
    match reason {
        RuntimeEvidenceQuarantineReason::Tombstoned => "runtime_evidence_tombstoned",
        RuntimeEvidenceQuarantineReason::UnknownTarget => "runtime_evidence_unknown_target",
        RuntimeEvidenceQuarantineReason::IdentityMismatch => "runtime_evidence_identity_mismatch",
        RuntimeEvidenceQuarantineReason::ClaimMismatch => "runtime_evidence_claim_mismatch",
        RuntimeEvidenceQuarantineReason::StaleAttachment => "runtime_evidence_stale_attachment",
        RuntimeEvidenceQuarantineReason::StaleSourceOrder => "runtime_evidence_stale_source_order",
        RuntimeEvidenceQuarantineReason::Unspecified => "runtime_evidence_unspecified",
    }
}

pub(crate) fn validate_spawn_promotion_result_order(
    promotion: &SpawnPromotionCommitted,
) -> Result<(), RuntimeEvidenceError> {
    let latest_lifecycle_lsn = promotion
        .lifecycle
        .iter()
        .map(|evidence| {
            event_lsn(
                evidence
                    .event_id
                    .as_ref()
                    .ok_or_else(|| malformed("promotion lifecycle entry has no event id"))?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| malformed("promotion has no lifecycle evidence"))?;
    let result_lsn = event_lsn(
        promotion
            .successful_result
            .as_ref()
            .and_then(|result| result.event_id.as_ref())
            .ok_or_else(|| malformed("promotion has no successful Result event id"))?,
    )?;
    if result_lsn <= latest_lifecycle_lsn {
        return Err(fence(
            "promotion Result did not qualify after its delivered/running lifecycle evidence",
        ));
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
    let latest_lifecycle_lsn = validate_lifecycle(&promotion.lifecycle, command_id, promotion_lsn)?;
    validate_spawn_promotion_result_order(promotion)?;
    let result = promotion
        .successful_result
        .as_ref()
        .ok_or_else(|| malformed("promotion has no successful result"))?;
    let result_lsn = event_lsn(
        result
            .event_id
            .as_ref()
            .ok_or_else(|| malformed("result has no event id"))?,
    )?;
    if result.command_id.as_ref() != Some(command_id)
        || FailureCode::try_from(result.failure_code).ok() != Some(FailureCode::Unspecified)
        || result_lsn <= latest_lifecycle_lsn
        || result_lsn >= promotion_lsn
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
    let operation = accepted_operation
        .operation
        .as_ref()
        .ok_or_else(|| malformed("accepted claim has no Operation"))?;
    let sender = operation
        .sender
        .as_ref()
        .ok_or_else(|| malformed("accepted spawn Operation has no sender"))?;
    let sender_actor = sender
        .actor_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| malformed("accepted spawn Operation has no sender actor"))?;
    if operation.authority_domain_id.as_ref() != Some(domain)
        || operation.command_id.as_ref() != Some(command_id)
        || OperationKind::try_from(operation.kind).ok() != Some(OperationKind::Spawn)
        || accepted_operation
            .authorizing_grant_id
            .as_ref()
            .is_none_or(|id| id.value.is_empty())
    {
        return Err(fence(
            "promotion is not bound to the exact accepted spawn Operation",
        ));
    }
    let accepted_target = operation
        .target_scope
        .as_ref()
        .ok_or_else(|| malformed("accepted spawn Operation has no target"))?;
    let promoted_external = promoted
        .external_runtime
        .as_ref()
        .expect("promoted runtime validated");
    if TargetScopeKind::try_from(accepted_target.kind).ok() != Some(TargetScopeKind::Adapter)
        || accepted_target.adapter_id != promoted_external.adapter_id
        || accepted_target.actor_id.is_some()
        || accepted_target.runtime_session_id.is_some()
        || accepted_target.session_generation.is_some()
        || !accepted_target.deployment_scope.is_empty()
        || !accepted_target.project_or_group.is_empty()
        || !accepted_target.legacy_audit_resource_id.is_empty()
        || accepted_target.resource.is_some()
    {
        return Err(fence(
            "promotion target is not the exact canonical accepted adapter target",
        ));
    }
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
    let committed_at = promotion
        .committed_at
        .as_ref()
        .ok_or_else(|| malformed("promotion has no committed_at"))?;
    let actual_kinds: HashSet<_> = descendant.allowed_operation_kinds.iter().copied().collect();
    let expected_kinds: HashSet<_> = DESCENDANT_GRANT_ALLOWED_KINDS
        .iter()
        .map(|kind| *kind as i32)
        .collect();
    if descendant.audit_id.as_ref() != Some(audit_id)
        || descendant.authority_domain_id.as_ref() != Some(domain)
        || descendant.grant_id.as_ref() != Some(&descendant_grant_id(domain, command_id))
        || descendant.subject_actor_id.as_ref() != Some(sender_actor)
        || descendant.subject_endpoint_id != sender.endpoint_id
        || !descendant.subject_endpoint_class.is_empty()
        || descendant.allowed_operation_kinds.len() != DESCENDANT_GRANT_ALLOWED_KINDS.len()
        || actual_kinds != expected_kinds
        || descendant.created_at.as_ref() != Some(committed_at)
        || descendant.expires_at.is_some()
        || descendant.revocation_generation.is_some()
        || descendant.revoked_at.is_some()
        || descendant.provenance.as_ref().is_none_or(|provenance| {
            provenance.spawn_operation_id.as_ref() != Some(command_id)
                || provenance.spawning_grant_id != authority.spawning_grant_id
                || provenance.continuation_authority != authority.continuation_authority
        })
    {
        return Err(fence(
            "promotion descendant is not exactly derived from the accepted Operation",
        ));
    }
    let descendant_target = descendant
        .target_scope
        .as_ref()
        .ok_or_else(|| malformed("promotion descendant has no target"))?;
    if TargetScopeKind::try_from(descendant_target.kind).ok()
        != Some(TargetScopeKind::RuntimeSession)
        || descendant_target.adapter_id != promoted_external.adapter_id
        || descendant_target.deployment_scope != promoted_external.deployment_scope
        || descendant_target.runtime_session_id != promoted_external.runtime_session_id
        || descendant_target.session_generation != promoted_external.generation
    {
        return Err(fence(
            "promotion descendant target differs from the promoted runtime",
        ));
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
) -> Result<u64, RuntimeEvidenceError> {
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
    Ok(previous_lsn)
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

#[must_use]
pub fn source_matches_current_attachment(
    authority_domain_id: &AuthorityDomainId,
    source: &RuntimeEvidenceSourceAttachment,
    adapters: &AdapterRegistry,
) -> bool {
    let Some(adapter_id) = source.adapter_id.as_ref() else {
        return false;
    };
    let Some(record) = adapters.get(adapter_id) else {
        return false;
    };
    record.registration.authority_domain_id.as_ref() == Some(authority_domain_id)
        && record.registration.adapter_id.as_ref() == Some(adapter_id)
        && record.registration.adapter_generation == source.adapter_generation
        && source.attachment_event_id.as_ref() == Some(&record.attach_event_id)
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

fn validate_runtime_generation_ref(
    target: &RuntimeGenerationRef,
    context: &str,
) -> Result<(), RuntimeEvidenceError> {
    if target
        .logical_target_id
        .as_ref()
        .is_some_and(|id| id.value.is_empty())
    {
        return Err(malformed(format!(
            "{context} has an empty logical target id"
        )));
    }
    validate_external_runtime(
        target
            .external_runtime
            .as_ref()
            .ok_or_else(|| malformed(format!("{context} has no external runtime")))?,
        context,
    )
}

fn validate_external_runtime(
    external: &ExternalRuntimeRef,
    context: &str,
) -> Result<(), RuntimeEvidenceError> {
    if external
        .adapter_id
        .as_ref()
        .is_none_or(|id| id.value.is_empty())
        || external.deployment_scope.is_empty()
        || external
            .runtime_session_id
            .as_ref()
            .is_none_or(|id| id.value.is_empty())
        || external
            .generation
            .is_none_or(|generation| generation.value == 0)
    {
        return Err(malformed(format!(
            "{context} has malformed external runtime identity"
        )));
    }
    Ok(())
}

fn external_from_scope(
    scope: Option<&TargetScope>,
    context: &str,
) -> Result<ExternalRuntimeRef, RuntimeEvidenceError> {
    let scope = scope.ok_or_else(|| malformed(format!("{context} has no target scope")))?;
    if TargetScopeKind::try_from(scope.kind).ok() != Some(TargetScopeKind::RuntimeSession)
        || scope.actor_id.is_some()
        || !scope.project_or_group.is_empty()
        || !scope.legacy_audit_resource_id.is_empty()
        || scope.resource.is_some()
    {
        return Err(fence(format!(
            "{context} does not carry one canonical runtime-session target"
        )));
    }
    let external = ExternalRuntimeRef {
        adapter_id: scope.adapter_id.clone(),
        deployment_scope: scope.deployment_scope.clone(),
        runtime_session_id: scope.runtime_session_id.clone(),
        generation: scope.session_generation,
    };
    validate_external_runtime(&external, context)?;
    Ok(external)
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
