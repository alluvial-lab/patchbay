//! Elicitation-slot state derived from the durable event log.
//!
//! This projection is an independent log consumer. Operation acceptance does
//! not know about [`ElicitationState`]: response operations pass through the
//! normal command lifecycle, and their terminal transition events close the
//! correlated Elicitation slot here.

use std::collections::HashMap;

use patchbay_contracts::patchbay::{
    typed_correlation, AuthorityDomainId, CommandTransition, Elicitation, ElicitationId,
    ElicitationState, Lsn, OperationState, StoredEventKind, TypedCorrelation,
};
use prost::Message;

use crate::storage::{RecordedEvent, Storage};

use super::{AcceptanceError, OperationStateExt};

/// The current state of one Elicitation slot.
///
/// This record is a derived projection. The durable Elicitation and command
/// transition events remain authoritative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElicitationRecord {
    pub elicitation_id: ElicitationId,
    pub state: ElicitationState,
    /// The LSN of the first transition that terminalized the slot.
    pub terminal_lsn: Option<u64>,
}

/// An independent event-log consumer that projects Elicitation-slot state.
///
/// The layer deliberately owns no storage and has no acceptance-pipeline
/// dependency. Recovery and live-tail callers feed committed events through
/// [`Self::observe`]. Because the authority-domain log is delivered in LSN
/// order, the first correlated terminal response transition structurally wins.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ElicitationSlotLayer {
    slots: HashMap<ElicitationId, ElicitationRecord>,
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
            StoredEventKind::CommandTransition => self.observe_command_transition(event),
            StoredEventKind::Operation
            | StoredEventKind::Observation
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::Revocation
            | StoredEventKind::SessionState
            | StoredEventKind::Unspecified => Ok(()),
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
            },
        );
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
        if !to_state.is_terminal() {
            return Ok(());
        }

        self.terminalize_slot(&elicitation_id, event_lsn)
    }

    fn terminalize_slot(
        &mut self,
        elicitation_id: &ElicitationId,
        event_lsn: u64,
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

        // v0.1.0 maps any successfully terminal response-operation lifecycle to
        // Answered. Distinguishing explicit denial as Declined belongs to the
        // response-contract validation layer rather than failure-code guessing.
        slot.state = ElicitationState::Answered;
        slot.terminal_lsn = Some(event_lsn);
        Ok(())
    }
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
        let (event_domain, event_lsn) = event_identity(&event)?;
        if event_domain != authority_domain_id {
            return Err(AcceptanceError::CorruptLog(format!(
                "recovery event belongs to authority domain {:?}, expected {:?}",
                event_domain, authority_domain_id
            )));
        }
        if event_lsn <= previous_lsn {
            return Err(AcceptanceError::CorruptLog(format!(
                "recovery event LSN {event_lsn} is not after previous LSN {previous_lsn}"
            )));
        }

        layer.observe(&event)?;
        previous_lsn = event_lsn;
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
