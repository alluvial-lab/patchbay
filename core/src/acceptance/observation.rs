//! Adapter-to-core observation ingestion and command-state reflection.
//!
//! Observations are durable evidence in their own right. Status and result
//! observations may additionally produce a first-class command-transition
//! event, while observations for commands that are already terminal remain
//! audit-only records.

use patchbay_contracts::patchbay::{
    typed_correlation, AuthorityDomainId, CommandId, CommandTransition, EventId, FailureCode,
    Observation, ObservationKind, OperationKind, OperationState, StoredEventKind,
    StoredEventPayload, TargetScope, TypedCorrelation,
};
use prost::Message;

use crate::{acceptance::Clock, resource::ResourceIdentity, storage::Storage};

use super::{allowed_transition, AcceptanceError, OperationStateExt};

/// Read access to the live command-state projection.
///
/// The durable event log remains authoritative; this lookup is the hot-path
/// projection rebuilt by replay. Ingestion uses it to encode `from_state` and
/// to reject late candidates before they can become transition events.
/// A snapshot of a command's current state for the acceptance boundary.
///
/// Returned by [`CommandStateLookup`]. Carries the current `OperationState`
/// and the originating Operation's correlations (which may include an
/// `ElicitationId` for response Operations). The correlations flow into
/// derived `CommandTransition` events so the Elicitation-slot layer can
/// correlate a response terminal transition back to its Elicitation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSnapshot {
    pub state: OperationState,
    /// Generated kind carried by the accepted Operation.
    pub operation_kind: OperationKind,
    /// Exact target carried by the accepted Operation. Status/result evidence
    /// must bind to this target before it can append or derive a transition.
    pub target_scope: Option<TargetScope>,
    /// The originating Operation's correlations (e.g. ElicitationId for
    /// response Operations). Carried into derived transitions.
    pub correlations: Vec<TypedCorrelation>,
    pub terminal_lsn: Option<u64>,
}

/// Read-only lookup of a command's current in-memory state. The acceptance
/// pipeline uses this on the dedup-retry path (to return the existing command's
/// state, not a hardcoded `Accepted`), and observation ingestion uses it to
/// check terminality before emitting a transition and to carry the command's
/// correlations into the derived transition.
pub trait CommandStateLookup: Send + Sync {
    fn current_state(
        &self,
        command_id: &CommandId,
    ) -> impl std::future::Future<Output = Option<CommandSnapshot>> + Send;
}

/// A command transition implied by an adapter observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionCandidate {
    pub command_id: CommandId,
    pub to_state: OperationState,
    pub failure_code: FailureCode,
    pub correlations: Vec<TypedCorrelation>,
}

/// The durable outcome of ingesting one observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestResult {
    /// The observation was recorded and did not imply a state change.
    Recorded { event_id: EventId },
    /// The observation and its resulting command transition were recorded.
    Transitioned {
        observation_event_id: EventId,
        transition_event_id: EventId,
        to_state: OperationState,
    },
    /// A successful spawn result was recorded as durable evidence, while the
    /// descendant-completion driver retains terminalization authority.
    CompletionDeferred { observation_event_id: EventId },
    /// The observation was recorded, but its command was already terminal.
    StaleCandidate { observation_event_id: EventId },
}

/// Ingest an adapter-reported observation.
///
/// The implied lifecycle transition is validated before durability, then the
/// Observation, derived transition, and transition audit commit atomically. A
/// terminal, disallowed, or faulted candidate therefore cannot survive as a
/// replayable source-without-transition prefix that later acquires authority.
pub async fn ingest_observation<S, L>(
    storage: &S,
    state_lookup: &L,
    observation: Observation,
) -> Result<IngestResult, AcceptanceError>
where
    S: Storage,
    L: CommandStateLookup,
{
    let authority_domain_id = validate_authority_domain(&observation)?.clone();
    let observation_kind = ObservationKind::try_from(observation.kind).ok();
    let Some(candidate) = derive_transition(&observation) else {
        if matches!(observation_kind, Some(ObservationKind::Result))
            || (matches!(observation_kind, Some(ObservationKind::Status))
                && !is_resource_status_fact(&observation))
        {
            return Err(AcceptanceError::CorruptRecord(
                "status/result observation is missing one unambiguous, non-empty command correlation or carries an unknown failure code"
                    .to_owned(),
            ));
        }
        let observation_event_id = storage
            .append(
                &authority_domain_id,
                StoredEventPayload {
                    kind: StoredEventKind::Observation as i32,
                    payload: observation.encode_to_vec(),
                },
            )
            .await?;
        validate_event_id(&observation_event_id, &authority_domain_id, "observation")?;
        return Ok(IngestResult::Recorded {
            event_id: observation_event_id,
        });
    };

    // Resolve the command before persistence. The resulting decision is the
    // only authority allowed to select the audit kind; a raw Observation
    // envelope cannot distinguish a completion from a stale late result.
    let snapshot = match state_lookup.current_state(&candidate.command_id).await {
        Some(snapshot) => snapshot,
        None => {
            // Unknown runtime-targeted evidence is quarantined by authenticated
            // adapter ingress. This lower boundary has no source attachment and
            // therefore rejects without creating a raw replayable Observation.
            return Err(AcceptanceError::CorruptRecord(format!(
                "observation references unknown command {:?}",
                candidate.command_id
            )));
        }
    };
    if observation.target_scope != snapshot.target_scope {
        return Err(AcceptanceError::CorruptRecord(format!(
            "observation target does not match command {:?}",
            candidate.command_id
        )));
    }
    if snapshot.state.is_terminal() {
        if let Some(event_id) = storage
            .reconcile_observation_retry(&authority_domain_id, observation.clone())
            .await?
        {
            validate_event_id(&event_id, &authority_domain_id, "observation retry")?;
            return Ok(IngestResult::Recorded { event_id });
        }
        // This boundary does not possess authenticated attachment/generation
        // context, so it cannot truthfully construct QuarantinedRuntimeEvidence.
        // The production adapter ingress classifies runtime-targeted candidates
        // first and uses the dedicated typed quarantine append. Changed late
        // evidence fails closed rather than overwriting the canonical outcome.
        return Err(AcceptanceError::CorruptRecord(format!(
            "late terminal observation for command {:?} requires authenticated runtime quarantine",
            candidate.command_id
        )));
    }

    if snapshot.operation_kind == OperationKind::Spawn
        && candidate.to_state == OperationState::Completed
        && candidate.failure_code == FailureCode::Unspecified
        && matches!(
            snapshot.state,
            OperationState::Delivered | OperationState::Running
        )
    {
        // Preserve bounded, redacted operational evidence while keeping the
        // canonical command lifecycle non-terminal. `inspect-command` already
        // returns a bounded command-audit page, and AuditRecordDraft has no
        // arbitrary payload field, so the successful Result body cannot leak
        // through this diagnostic checkpoint.
        let mut audit = crate::storage::AuditRecordDraft::new(
            crate::acceptance::SystemClock.now(),
            patchbay_contracts::patchbay::AuditEventKind::CommandRunning,
        );
        audit.command_id = Some(candidate.command_id);
        audit.target_scope = observation.target_scope.clone();
        audit.reason_code = "spawn_completion_deferred".to_owned();
        let committed = storage
            .append_spawn_result_deferred_audited(&authority_domain_id, observation, audit)
            .await?;
        validate_event_id(
            &committed.source_event_id,
            &authority_domain_id,
            "observation",
        )?;
        validate_event_id(
            &committed.audit_event_id,
            &authority_domain_id,
            "deferred spawn audit",
        )?;
        return Ok(IngestResult::CompletionDeferred {
            observation_event_id: committed.source_event_id,
        });
    }

    // A repeated status/Result can only be an exact transport retry of the
    // canonical source that established the current state. Changed evidence is
    // not a new generic fact and must fail before durability.
    if snapshot.state == candidate.to_state {
        let observation_event_id = storage
            .reconcile_observation_retry(&authority_domain_id, observation)
            .await?
            .ok_or_else(|| {
                AcceptanceError::CorruptRecord(format!(
                    "changed repeated observation for command {:?} is not an exact durable retry",
                    candidate.command_id
                ))
            })?;
        validate_event_id(
            &observation_event_id,
            &authority_domain_id,
            "observation retry",
        )?;
        return Ok(IngestResult::Recorded {
            event_id: observation_event_id,
        });
    }

    if !allowed_transition(snapshot.state, candidate.to_state) {
        return Err(AcceptanceError::CorruptRecord(format!(
            "observation implies disallowed transition {:?} -> {:?} for command {:?}",
            snapshot.state, candidate.to_state, candidate.command_id
        )));
    }

    let transition = CommandTransition {
        command_id: Some(candidate.command_id.clone()),
        to_state: candidate.to_state as i32,
        from_state: snapshot.state as i32,
        failure_code: candidate.failure_code as i32,
        // Durable commit time belongs to a clock-aware persistence/composition
        // boundary. Observation ingestion has no clock port, so it must not
        // fabricate one.
        committed_at: None,
        // Merge the Observation's correlations with the command's own
        // correlations (which may include an ElicitationId for response
        // Operations). This is what lets the Elicitation-slot layer correlate
        // a response terminal transition back to its Elicitation — the
        // correlation flows from the Operation, through the transition, to the
        // slot layer. De-duplicate by reference equality.
        correlations: merge_correlations(&snapshot.correlations, &candidate.correlations),
    };
    let audit_kind = match candidate.to_state {
        OperationState::Delivered => patchbay_contracts::patchbay::AuditEventKind::CommandDelivered,
        OperationState::Running => patchbay_contracts::patchbay::AuditEventKind::CommandRunning,
        OperationState::Completed => patchbay_contracts::patchbay::AuditEventKind::CommandCompleted,
        OperationState::Rejected => patchbay_contracts::patchbay::AuditEventKind::CommandRejected,
        OperationState::Failed => patchbay_contracts::patchbay::AuditEventKind::CommandFailed,
        OperationState::Expired => patchbay_contracts::patchbay::AuditEventKind::CommandExpired,
        OperationState::Cancelled => patchbay_contracts::patchbay::AuditEventKind::CommandCancelled,
        OperationState::Superseded => {
            patchbay_contracts::patchbay::AuditEventKind::CommandSuperseded
        }
        OperationState::Accepted | OperationState::Unspecified => {
            return Err(AcceptanceError::CorruptRecord(
                "observation selected a non-transition state".to_owned(),
            ));
        }
    };
    let mut audit =
        crate::storage::AuditRecordDraft::new(crate::acceptance::SystemClock.now(), audit_kind);
    audit.command_id = Some(candidate.command_id.clone());
    audit.failure_code =
        (candidate.failure_code != FailureCode::Unspecified).then_some(candidate.failure_code);
    audit.reason_code = "command_state_transition".to_owned();
    let committed = storage
        .append_observation_transition_audited(&authority_domain_id, observation, transition, audit)
        .await?;
    validate_event_id(
        &committed.observation_event_id,
        &authority_domain_id,
        "observation",
    )?;
    validate_event_id(
        &committed.transition_event_id,
        &authority_domain_id,
        "command transition",
    )?;
    validate_event_id(
        &committed.audit_event_id,
        &authority_domain_id,
        "command transition audit",
    )?;

    Ok(IngestResult::Transitioned {
        observation_event_id: committed.observation_event_id,
        transition_event_id: committed.transition_event_id,
        to_state: candidate.to_state,
    })
}

fn is_resource_status_fact(observation: &Observation) -> bool {
    if !observation.correlations.is_empty()
        || FailureCode::try_from(observation.failure_code).ok() != Some(FailureCode::Unspecified)
    {
        return false;
    }
    let Some(target) = observation.target_scope.as_ref() else {
        return false;
    };
    let Ok(identity) = ResourceIdentity::try_from_scope(target) else {
        return false;
    };
    !identity.adapter_id().value.trim().is_empty()
        && !identity.resource_kind().value.trim().is_empty()
        && !identity.resource_id().value.trim().is_empty()
}

/// Map an observation to the command transition it implies.
///
/// Event, delta, and unspecified observations carry evidence without changing
/// command state. Status means running. Result means completed when no failure
/// is present, rejected for protocol-mandated adapter semantic refusals
/// (`unsupported_command` and `delivery_rejected`), and failed for
/// execution/delivery errors. The original failure code is retained so policy
/// refusal and outcome ambiguity remain
/// distinguishable.
#[must_use]
pub fn derive_transition(observation: &Observation) -> Option<TransitionCandidate> {
    let observation_kind = ObservationKind::try_from(observation.kind).ok()?;
    let (to_state, failure_code) = match observation_kind {
        ObservationKind::Status => (OperationState::Running, FailureCode::Unspecified),
        ObservationKind::Result => {
            let failure_code = FailureCode::try_from(observation.failure_code).ok()?;
            match failure_code {
                FailureCode::Unspecified => (OperationState::Completed, failure_code),
                FailureCode::UnsupportedCommand | FailureCode::DeliveryRejected => {
                    (OperationState::Rejected, failure_code)
                }
                _ => (OperationState::Failed, failure_code),
            }
        }
        ObservationKind::Unspecified | ObservationKind::Event | ObservationKind::Delta => {
            return None;
        }
    };

    Some(TransitionCandidate {
        command_id: exact_command_correlation(&observation.correlations)?,
        to_state,
        failure_code,
        correlations: observation.correlations.clone(),
    })
}

fn validate_authority_domain(
    observation: &Observation,
) -> Result<&AuthorityDomainId, AcceptanceError> {
    let authority_domain_id = observation.authority_domain_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord("observation is missing authority_domain_id".to_owned())
    })?;
    if authority_domain_id.value.is_empty() {
        return Err(AcceptanceError::CorruptRecord(
            "observation authority_domain_id is empty".to_owned(),
        ));
    }
    Ok(authority_domain_id)
}

/// Qualify one exact command correlation from a typed-correlation list.
///
/// The wire list may repeat the same correlation during transport or adapter
/// composition. Identical non-empty `CommandId` references collapse to one
/// logical correlation; conflicting or empty command ids do not qualify.
/// Other typed references remain available to their owning correlation layer
/// and do not make the command reference ambiguous.
#[must_use]
pub fn exact_command_correlation(correlations: &[TypedCorrelation]) -> Option<CommandId> {
    let mut command_id: Option<&CommandId> = None;
    for correlation in correlations {
        let Some(typed_correlation::Ref::CommandId(candidate)) = correlation.r#ref.as_ref() else {
            continue;
        };
        if candidate.value.is_empty() {
            return None;
        }
        match command_id {
            None => command_id = Some(candidate),
            Some(existing) if existing == candidate => {}
            Some(_) => return None,
        }
    }
    command_id.cloned()
}

fn validate_event_id(
    event_id: &EventId,
    expected_domain: &AuthorityDomainId,
    record_kind: &str,
) -> Result<(), AcceptanceError> {
    match event_id.authority_domain_id.as_ref() {
        Some(actual_domain) if actual_domain == expected_domain => {}
        Some(actual_domain) => {
            return Err(AcceptanceError::CorruptRecord(format!(
                "storage returned {record_kind} event for domain {:?}, expected {:?}",
                actual_domain, expected_domain
            )));
        }
        None => {
            return Err(AcceptanceError::CorruptRecord(format!(
                "storage returned {record_kind} event without authority_domain_id"
            )));
        }
    }
    if event_id.lsn.is_none() {
        return Err(AcceptanceError::CorruptRecord(format!(
            "storage returned {record_kind} event without an LSN"
        )));
    }
    Ok(())
}

/// Merge two correlation lists, de-duplicating by typed-reference equality.
/// The command's own correlations (e.g. an ElicitationId on a response
/// Operation) take precedence; the Observation's correlations are appended
/// if not already present. This is what lets an Elicitation correlation flow
/// from the originating Operation through the derived CommandTransition to
/// the Elicitation-slot layer.
fn merge_correlations(
    command_correlations: &[TypedCorrelation],
    observation_correlations: &[TypedCorrelation],
) -> Vec<TypedCorrelation> {
    let mut merged = command_correlations.to_vec();
    for obs_corr in observation_correlations {
        if !merged
            .iter()
            .any(|existing| correlation_eq(existing, obs_corr))
        {
            merged.push(obs_corr.clone());
        }
    }
    merged
}

/// Structural equality for TypedCorrelation (compares the oneof ref).
fn correlation_eq(a: &TypedCorrelation, b: &TypedCorrelation) -> bool {
    match (a.r#ref.as_ref(), b.r#ref.as_ref()) {
        (
            Some(typed_correlation::Ref::CommandId(x)),
            Some(typed_correlation::Ref::CommandId(y)),
        ) => x == y,
        (
            Some(typed_correlation::Ref::MessageId(x)),
            Some(typed_correlation::Ref::MessageId(y)),
        ) => x == y,
        (Some(typed_correlation::Ref::ReplyId(x)), Some(typed_correlation::Ref::ReplyId(y))) => {
            x == y
        }
        (Some(typed_correlation::Ref::EventId(x)), Some(typed_correlation::Ref::EventId(y))) => {
            x == y
        }
        (
            Some(typed_correlation::Ref::ElicitationId(x)),
            Some(typed_correlation::Ref::ElicitationId(y)),
        ) => x == y,
        _ => false,
    }
}
