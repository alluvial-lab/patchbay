//! Elicitation-slot state derived from the durable event log.
//!
//! This projection is an independent log consumer. Operation acceptance does
//! not know about [`ElicitationState`]: response operations pass through the
//! normal command lifecycle, and their terminal transition events close the
//! correlated Elicitation slot here.

use std::collections::HashMap;

use patchbay_contracts::patchbay::{
    typed_correlation, AcceptedOperation, ActorId, ApprovalDecision, ApprovalResponsePayload, AuthorityDomainId,
    CommandId, CommandTransition, Elicitation, ElicitationId, ElicitationState, Lsn, Operation,
    OperationKind, OperationState, PayloadContentType, ResponseContract, StoredEventKind,
    TypedCorrelation,
};
use prost::Message;

use crate::storage::{validate_next_replay_event, RecordedEvent, Storage};

use super::AcceptanceError;

/// Return whether an `OperationState` is terminal in the command lifecycle
/// registry (`docs/PROTOCOL.md` § Command lifecycle state). This mirrors
/// `acceptance::is_terminal` — the terminal-state set is a protocol fact, not
/// acceptance-owned. The elicitation layer needs it to detect terminal
/// response-Operation transitions without depending on the acceptance module.
#[must_use]
const fn operation_state_is_terminal(state: OperationState) -> bool {
    matches!(
        state,
        OperationState::Completed
            | OperationState::Rejected
            | OperationState::Failed
            | OperationState::Expired
            | OperationState::Cancelled
            | OperationState::Superseded
    )
}

/// The current state of one Elicitation slot.
///
/// This record is a derived projection. The durable Elicitation and command
/// transition events remain authoritative.
#[derive(Debug, Clone, PartialEq)]
pub struct ElicitationRecord {
    pub elicitation_id: ElicitationId,
    pub state: ElicitationState,
    /// The LSN of the first transition that terminalized the slot.
    pub terminal_lsn: Option<u64>,
    /// The response contract carried by the opening Elicitation event.
    pub contract: Option<ResponseContract>,
    /// The actor expected to answer this Elicitation, when specified.
    pub expected_responder_actor: Option<ActorId>,
    /// The response Operation that won terminalization, when one exists.
    /// Retained so an exact idempotent retry can pass validation and reach
    /// storage deduplication without admitting a different late candidate.
    pub winning_response: Option<Operation>,
}

/// An independent event-log consumer that projects Elicitation-slot state.
///
/// The layer deliberately owns no storage and has no acceptance-pipeline
/// dependency. Recovery and live-tail callers feed committed events through
/// [`Self::observe`]. Because the authority-domain log is delivered in LSN
/// order, the first correlated terminal response transition structurally wins.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ElicitationSlotLayer {
    slots: HashMap<ElicitationId, ElicitationRecord>,
    /// command_id → Operation, built from OPERATION events. Used to confirm
    /// a correlated terminal transition belongs to a response Operation and
    /// to retain the winning Operation for idempotent terminal retries.
    command_operations: HashMap<CommandId, Operation>,
}

impl ElicitationSlotLayer {
    /// Construct an empty Elicitation-slot projection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one committed event into the Elicitation-slot projection.
    ///
    /// The method is idempotent for re-delivered events. Once a slot is
    /// terminal, later correlated terminal responses are stale candidates and
    /// leave the winning state and LSN unchanged.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "unknown stored event kind {}",
                event.payload.kind
            ))
        })?;

        match kind {
            StoredEventKind::Elicitation => self.observe_elicitation(event),
            StoredEventKind::Operation => self.observe_operation(event),
            StoredEventKind::CommandTransition => self.observe_command_transition(event),
            StoredEventKind::Observation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::Revocation
            | StoredEventKind::SessionState
            | StoredEventKind::ResourceState
            | StoredEventKind::SpawnClaim
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::OperatorSessionRevocation
            | StoredEventKind::ControlSurfaceRevocation
            | StoredEventKind::SecurityLockdown
            | StoredEventKind::AuditRecord => Ok(()),
            StoredEventKind::Unspecified => Err(AcceptanceError::CorruptLog(
                "elicitation replay event kind is unspecified".to_owned(),
            )),
        }
    }

    /// Return the projected slot for an Elicitation id.
    #[must_use]
    pub fn get_slot(&self, id: &ElicitationId) -> Option<&ElicitationRecord> {
        self.slots.get(id)
    }

    fn observe_elicitation(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (event_domain, event_lsn) = event_identity(event)?;
        let elicitation =
            Elicitation::decode(event.payload.payload.as_slice()).map_err(|error| {
                AcceptanceError::CorruptRecord(format!(
                    "cannot decode elicitation at LSN {event_lsn}: {error}"
                ))
            })?;

        let elicitation_id = elicitation.elicitation_id.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "elicitation at LSN {event_lsn} is missing elicitation_id"
            ))
        })?;
        if elicitation_id.value.is_empty() {
            return Err(AcceptanceError::CorruptRecord(format!(
                "elicitation at LSN {event_lsn} has an empty elicitation_id"
            )));
        }

        let elicitation_domain = elicitation.authority_domain_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "elicitation {:?} at LSN {event_lsn} is missing authority_domain_id",
                elicitation_id
            ))
        })?;
        if elicitation_domain != event_domain {
            return Err(AcceptanceError::CorruptLog(format!(
                "elicitation authority domain {:?} does not match event authority domain {:?} at LSN {event_lsn}",
                elicitation_domain, event_domain
            )));
        }

        if let Some(recorded_lsn) = elicitation.recorded_lsn {
            if recorded_lsn.value != event_lsn {
                return Err(AcceptanceError::CorruptLog(format!(
                    "elicitation {:?} records LSN {}, but its event is at LSN {event_lsn}",
                    elicitation_id, recorded_lsn.value
                )));
            }
        }

        let state = ElicitationState::try_from(elicitation.state).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "elicitation {:?} at LSN {event_lsn} has unknown state {}",
                elicitation_id, elicitation.state
            ))
        })?;
        if state == ElicitationState::Unspecified {
            return Err(AcceptanceError::CorruptRecord(format!(
                "elicitation {:?} at LSN {event_lsn} has unspecified state",
                elicitation_id
            )));
        }

        // An event may be delivered again after later events have already
        // advanced the slot. Treat any already-known opening as a replayed
        // source event; it must never reset a terminal projection.
        if self.slots.contains_key(&elicitation_id) {
            return Ok(());
        }

        self.slots.insert(
            elicitation_id.clone(),
            ElicitationRecord {
                elicitation_id,
                state,
                terminal_lsn: is_terminal_state(state).then_some(event_lsn),
                contract: elicitation.response_contract,
                expected_responder_actor: elicitation.expected_responder_actor,
                winning_response: None,
            },
        );
        Ok(())
    }

    /// Fold an OPERATION event: record its command_id → OperationKind so that
    /// later correlated transitions can be confirmed as response Operations.
    fn observe_operation(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (_, event_lsn) = event_identity(event)?;
        let accepted = AcceptedOperation::decode(event.payload.payload.as_slice()).map_err(|error| {
            AcceptanceError::CorruptRecord(format!(
                "cannot decode accepted operation at LSN {event_lsn}: {error}"
            ))
        })?;
        let operation = accepted.operation.ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {event_lsn} has no operation"
            ))
        })?;
        let command_id = operation.command_id.clone().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "operation at LSN {event_lsn} is missing command_id"
            ))
        })?;
        OperationKind::try_from(operation.kind).map_err(|_| {
            AcceptanceError::CorruptRecord(format!(
                "operation for command {:?} at LSN {event_lsn} has unknown kind {}",
                command_id, operation.kind
            ))
        })?;
        // First-write-wins: a duplicate OPERATION event (should not happen
        // with dedup, but the log is authoritative) is idempotent here.
        self.command_operations
            .entry(command_id)
            .or_insert(operation);
        Ok(())
    }

    fn observe_command_transition(&mut self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        let (_, event_lsn) = event_identity(event)?;
        let transition =
            CommandTransition::decode(event.payload.payload.as_slice()).map_err(|error| {
                AcceptanceError::CorruptRecord(format!(
                    "cannot decode command transition at LSN {event_lsn}: {error}"
                ))
            })?;

        let Some(elicitation_id) = correlation_to_elicitation(&transition.correlations) else {
            return Ok(());
        };

        let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
            AcceptanceError::CorruptLog(format!(
                "response transition at LSN {event_lsn} has unknown to_state {}",
                transition.to_state
            ))
        })?;
        if !operation_state_is_terminal(to_state) {
            return Ok(());
        }

        // Confirm this transition belongs to a response Operation. A regular
        // command that happens to carry an ElicitationId correlation must NOT
        // terminalize the slot. Look up the originating Operation's kind.
        let command_id = transition.command_id.as_ref().ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "transition at LSN {event_lsn} correlates to elicitation {:?} but has no command_id",
                elicitation_id
            ))
        })?;
        let response_operation = self.command_operations.get(command_id).ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "transition at LSN {event_lsn} correlates to elicitation {:?} but its command {:?} was not seen as an OPERATION event",
                elicitation_id, command_id
            ))
        })?;
        let kind = OperationKind::try_from(response_operation.kind).map_err(|_| {
            AcceptanceError::CorruptLog(format!(
                "operation for command {:?} has unknown kind {}",
                command_id, response_operation.kind
            ))
        })?;
        if !matches!(
            kind,
            OperationKind::ApprovalResponse | OperationKind::ElicitationResponse
        ) {
            // Not a response Operation — a regular command carrying an
            // ElicitationId correlation. Do not terminalize the slot.
            return Ok(());
        }

        self.terminalize_slot(
            &elicitation_id,
            kind,
            to_state,
            event_lsn,
            response_operation.clone(),
        )
    }

    fn terminalize_slot(
        &mut self,
        elicitation_id: &ElicitationId,
        kind: OperationKind,
        response_state: OperationState,
        event_lsn: u64,
        response_operation: Operation,
    ) -> Result<(), AcceptanceError> {
        let slot = self.slots.get_mut(elicitation_id).ok_or_else(|| {
            AcceptanceError::CorruptLog(format!(
                "terminal response transition at LSN {event_lsn} references unknown elicitation {:?}",
                elicitation_id
            ))
        })?;

        if is_terminal_state(slot.state) {
            // First-answer-wins: this response is stale. The winning slot state
            // and terminal LSN are immutable.
            return Ok(());
        }

        // Only a Completed response terminalizes the slot. A
        // Rejected/Failed/Expired/Cancelled/Superseded response means the
        // response itself failed, so the slot stays pending and another
        // surface may answer. For a completed approval response, the typed
        // operator decision selects the terminal's valence.
        if response_state == OperationState::Completed {
            slot.state = if kind == OperationKind::ApprovalResponse {
                match decode_approval_decision(&response_operation)? {
                    ApprovalDecision::Approved => ElicitationState::Answered,
                    ApprovalDecision::Denied => ElicitationState::Declined,
                    decision => {
                        return Err(AcceptanceError::CorruptRecord(format!(
                            "approval response terminal transition carried non-committed decision {decision:?}"
                        )))
                    }
                }
            } else {
                ElicitationState::Answered
            };
            slot.terminal_lsn = Some(event_lsn);
            slot.winning_response = Some(response_operation);
        }
        // Non-Completed terminals leave the slot non-terminal.
        Ok(())
    }
}

fn decode_approval_decision(operation: &Operation) -> Result<ApprovalDecision, AcceptanceError> {
    let envelope = operation.payload.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord(
            "completed approval response Operation is missing its payload".to_owned(),
        )
    })?;
    if envelope.content_type != PayloadContentType::Protobuf as i32 {
        return Err(AcceptanceError::CorruptRecord(
            "completed approval response payload content_type is not PAYLOAD_CONTENT_TYPE_PROTOBUF"
                .to_owned(),
        ));
    }
    let payload =
        ApprovalResponsePayload::decode(envelope.payload.as_slice()).map_err(|error| {
            AcceptanceError::CorruptRecord(format!(
                "cannot decode completed ApprovalResponsePayload: {error}"
            ))
        })?;
    ApprovalDecision::try_from(payload.decision).map_err(|_| {
        AcceptanceError::CorruptRecord(format!(
            "completed approval response has unknown decision {}",
            payload.decision
        ))
    })
}

/// Rebuild an Elicitation-slot projection by replaying one authority domain.
///
/// v0.1.0 replays from LSN 0 because the shared snapshot slot has no projection
/// discriminator. This matches command-index recovery and avoids allowing one
/// projection's snapshot to hide another projection's earlier events.
pub async fn rebuild_slots_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<ElicitationSlotLayer, AcceptanceError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut layer = ElicitationSlotLayer::new();
    let mut previous_lsn = 0u64;

    for event in events {
        let validated = validate_next_replay_event(authority_domain_id, previous_lsn, &event)
            .map_err(|error| {
                error.map(
                    AcceptanceError::CorruptRecord,
                    AcceptanceError::CorruptLog,
                )
            })?;
        layer.observe(&event)?;
        previous_lsn = validated.lsn;
    }

    Ok(layer)
}

/// Extract the first typed Elicitation-id correlation, if present.
#[must_use]
pub fn correlation_to_elicitation(correlations: &[TypedCorrelation]) -> Option<ElicitationId> {
    correlations.iter().find_map(|correlation| {
        if let Some(typed_correlation::Ref::ElicitationId(id)) = correlation.r#ref.as_ref() {
            Some(id.clone())
        } else {
            None
        }
    })
}

/// Return whether an Elicitation state is terminal in the protocol registry.
#[must_use]
pub const fn is_terminal_state(state: ElicitationState) -> bool {
    matches!(
        state,
        ElicitationState::Answered
            | ElicitationState::Declined
            | ElicitationState::Expired
            | ElicitationState::Cancelled
            | ElicitationState::Withdrawn
            | ElicitationState::Superseded
            | ElicitationState::Stale
    )
}

fn event_identity(event: &RecordedEvent) -> Result<(&AuthorityDomainId, u64), AcceptanceError> {
    let authority_domain_id = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptRecord("event has no authority domain".to_owned())
    })?;
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| AcceptanceError::CorruptRecord("event has no LSN".to_owned()))?;
    Ok((authority_domain_id, lsn.value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patchbay_contracts::patchbay::PayloadEnvelope;

    fn elicitation_id() -> ElicitationId {
        ElicitationId {
            value: "elicitation-approval-1".to_owned(),
        }
    }

    fn layer_with_open_slot() -> ElicitationSlotLayer {
        let id = elicitation_id();
        ElicitationSlotLayer {
            slots: HashMap::from([(
                id.clone(),
                ElicitationRecord {
                    elicitation_id: id,
                    state: ElicitationState::Opened,
                    terminal_lsn: None,
                    contract: None,
                    expected_responder_actor: None,
                    winning_response: None,
                },
            )]),
            command_operations: HashMap::new(),
        }
    }

    fn approval_operation(decision: ApprovalDecision) -> Operation {
        Operation {
            kind: OperationKind::ApprovalResponse as i32,
            payload: Some(PayloadEnvelope {
                payload: ApprovalResponsePayload {
                    decision: decision as i32,
                }
                .encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
                ..PayloadEnvelope::default()
            }),
            ..Operation::default()
        }
    }

    #[test]
    fn terminalize_slot_maps_completed_responses_by_kind_and_decision() {
        let cases = [
            (
                "completed approval APPROVED",
                OperationKind::ApprovalResponse,
                approval_operation(ApprovalDecision::Approved),
                OperationState::Completed,
                ElicitationState::Answered,
                true,
            ),
            (
                "completed approval DENIED (load-bearing decision mapping)",
                OperationKind::ApprovalResponse,
                approval_operation(ApprovalDecision::Denied),
                OperationState::Completed,
                ElicitationState::Declined,
                true,
            ),
            (
                "completed question remains payload-opaque",
                OperationKind::ElicitationResponse,
                Operation {
                    kind: OperationKind::ElicitationResponse as i32,
                    ..Operation::default()
                },
                OperationState::Completed,
                ElicitationState::Answered,
                true,
            ),
            (
                "machine rejection does not terminalize",
                OperationKind::ApprovalResponse,
                approval_operation(ApprovalDecision::Denied),
                OperationState::Rejected,
                ElicitationState::Opened,
                false,
            ),
        ];

        for (name, kind, operation, response_state, expected_state, terminalized) in cases {
            let mut layer = layer_with_open_slot();
            layer
                .terminalize_slot(
                    &elicitation_id(),
                    kind,
                    response_state,
                    42,
                    operation.clone(),
                )
                .unwrap();
            let slot = layer.get_slot(&elicitation_id()).unwrap();
            assert_eq!(slot.state, expected_state, "case {name}");
            assert_eq!(slot.terminal_lsn, terminalized.then_some(42), "case {name}");
            assert_eq!(
                slot.winning_response,
                terminalized.then_some(operation),
                "case {name}"
            );
        }
    }

    #[test]
    fn completed_approval_with_corrupt_payload_fails_closed() {
        let mut layer = layer_with_open_slot();
        let operation = Operation {
            kind: OperationKind::ApprovalResponse as i32,
            payload: Some(PayloadEnvelope {
                payload: vec![0xff],
                content_type: PayloadContentType::Protobuf as i32,
                ..PayloadEnvelope::default()
            }),
            ..Operation::default()
        };

        let error = layer
            .terminalize_slot(
                &elicitation_id(),
                OperationKind::ApprovalResponse,
                OperationState::Completed,
                42,
                operation,
            )
            .expect_err("corrupt approval payload must not select a terminal state");
        assert!(matches!(error, AcceptanceError::CorruptRecord(_)));
        let slot = layer.get_slot(&elicitation_id()).unwrap();
        assert_eq!(slot.state, ElicitationState::Opened);
        assert_eq!(slot.terminal_lsn, None);
    }
}
