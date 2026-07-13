//! Adapter-to-core observation ingestion and command-state reflection.
//!
//! Observations are durable evidence in their own right. Status and result
//! observations may additionally produce a first-class command-transition
//! event, while observations for commands that are already terminal remain
//! audit-only records.

use patchbay_contracts::patchbay::{
    typed_correlation, AuthorityDomainId, CommandId, CommandTransition, EventId, FailureCode,
    Observation, ObservationKind, OperationState, StoredEventKind, StoredEventPayload,
    TypedCorrelation,
};
use prost::Message;

use crate::storage::Storage;

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
    /// The observation was recorded, but its command was already terminal.
    StaleCandidate { observation_event_id: EventId },
}

/// Ingest an adapter-reported observation.
///
/// The raw observation is appended before any derived state transition. A
/// terminal command turns every derived candidate into an audit-only stale
/// result, so no transition out of terminal state reaches the durable log.
pub async fn ingest_observation<S, L>(
    storage: &S,
    state_lookup: &L,
    observation: Observation,
) -> Result<IngestResult, AcceptanceError>
where
    S: Storage,
    L: CommandStateLookup,
{
    let authority_domain_id = validate_authority_domain(&observation)?;
    let observation_kind = ObservationKind::try_from(observation.kind).ok();
    let observation_payload = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: observation.encode_to_vec(),
    };
    let observation_event_id = storage
        .append(authority_domain_id, observation_payload)
        .await?;
    validate_event_id(&observation_event_id, authority_domain_id, "observation")?;

    let Some(candidate) = derive_transition(&observation) else {
        if matches!(
            observation_kind,
            Some(ObservationKind::Status | ObservationKind::Result)
        ) {
            return Err(AcceptanceError::CorruptRecord(
                "status/result observation is missing one unambiguous, non-empty command correlation or carries an unknown failure code"
                    .to_owned(),
            ));
        }
        return Ok(IngestResult::Recorded {
            event_id: observation_event_id,
        });
    };

    let snapshot = state_lookup
        .current_state(&candidate.command_id)
        .await
        .ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "observation references unknown command {:?}",
                candidate.command_id
            ))
        })?;

    if snapshot.state.is_terminal() {
        return Ok(IngestResult::StaleCandidate {
            observation_event_id,
        });
    }

    // Repeated status reports are useful evidence but do not represent a new
    // lifecycle transition.
    if snapshot.state == candidate.to_state {
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
        command_id: Some(candidate.command_id),
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
    let transition_payload = StoredEventPayload {
        kind: StoredEventKind::CommandTransition as i32,
        payload: transition.encode_to_vec(),
    };
    let transition_event_id = storage
        .append(authority_domain_id, transition_payload)
        .await?;
    validate_event_id(
        &transition_event_id,
        authority_domain_id,
        "command transition",
    )?;

    Ok(IngestResult::Transitioned {
        observation_event_id,
        transition_event_id,
        to_state: candidate.to_state,
    })
}

/// Map an observation to the command transition it implies.
///
/// Event, delta, and unspecified observations carry evidence without changing
/// command state. Status means running. Result means completed when no failure
/// is present and failed otherwise; the original failure code is retained so
/// `execution_outcome_unknown` remains distinguishable from a known failure.
#[must_use]
pub fn derive_transition(observation: &Observation) -> Option<TransitionCandidate> {
    let observation_kind = ObservationKind::try_from(observation.kind).ok()?;
    let (to_state, failure_code) = match observation_kind {
        ObservationKind::Status => (OperationState::Running, FailureCode::Unspecified),
        ObservationKind::Result => {
            let failure_code = FailureCode::try_from(observation.failure_code).ok()?;
            if failure_code == FailureCode::Unspecified {
                (OperationState::Completed, failure_code)
            } else {
                (OperationState::Failed, failure_code)
            }
        }
        ObservationKind::Unspecified | ObservationKind::Event | ObservationKind::Delta => {
            return None;
        }
    };

    Some(TransitionCandidate {
        command_id: correlated_command_id(&observation.correlations)?,
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

fn correlated_command_id(correlations: &[TypedCorrelation]) -> Option<CommandId> {
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
