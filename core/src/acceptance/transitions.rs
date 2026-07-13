//! Canonical command lifecycle transitions and transition-event application.

use patchbay_contracts::patchbay::{CommandTransition, FailureCode, OperationState};

use super::{CommandRecord, OperationStateExt};

/// Errors detected while constructing or folding acceptance state.
#[derive(Debug, thiserror::Error)]
pub enum AcceptanceError {
    /// A durable record cannot form a valid in-memory command projection.
    #[error("corrupt command record: {0}")]
    CorruptRecord(String),

    /// The event sequence violates its encoded state or the protocol adjacency.
    #[error("corrupt command log: {0}")]
    CorruptLog(String),
}

/// Return whether the protocol permits a command transition from `from` to
/// `to`.
///
/// This match is the implementation single source of truth for the adjacency
/// in `docs/PROTOCOL.md` § "Command lifecycle state" and mirrors
/// `command_lifecycle.qnt::allowedTransition`. Terminal states, and the invalid
/// `Unspecified` state, have no outgoing transitions.
#[must_use]
pub const fn allowed_transition(from: OperationState, to: OperationState) -> bool {
    use OperationState::{
        Accepted, Cancelled, Completed, Delivered, Expired, Failed, Rejected, Running, Superseded,
    };

    match from {
        Accepted => matches!(
            to,
            Delivered | Rejected | Failed | Expired | Cancelled | Superseded
        ),
        Delivered => matches!(
            to,
            Running | Completed | Rejected | Failed | Expired | Cancelled | Superseded
        ),
        Running => matches!(to, Completed | Failed | Expired | Cancelled | Superseded),
        _ => false,
    }
}

/// Apply one durable command-transition event to an in-memory record.
///
/// All event fields are validated before the record is mutated. A mismatch or
/// invalid enum value indicates a corrupt/tampered log and fails fast. When a
/// transition enters a terminal state, `event_lsn` is retained as the unique
/// terminal transition's durable position.
pub fn apply_transition(
    record: &mut CommandRecord,
    transition: &CommandTransition,
    event_lsn: u64,
) -> Result<(), AcceptanceError> {
    if record.state.is_terminal() {
        return Err(AcceptanceError::CorruptLog(format!(
            "transition for already-terminal command {:?}",
            record.command_id
        )));
    }

    let from_state = OperationState::try_from(transition.from_state).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown from_state value {} for command {:?}",
            transition.from_state, record.command_id
        ))
    })?;
    let to_state = OperationState::try_from(transition.to_state).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown to_state value {} for command {:?}",
            transition.to_state, record.command_id
        ))
    })?;
    let failure_code = FailureCode::try_from(transition.failure_code).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown failure_code value {} for command {:?}",
            transition.failure_code, record.command_id
        ))
    })?;

    if from_state != record.state {
        return Err(AcceptanceError::CorruptLog(format!(
            "from_state mismatch for command {:?}: log says {from_state:?}, memory says {:?}",
            record.command_id, record.state
        )));
    }

    // Identity check: the transition must belong to this command. A
    // transition for a different command_id (or a missing command_id)
    // indicates a routing/corruption error — fail fast before mutation.
    let transition_cmd = transition.command_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptLog(format!(
            "transition for command {:?} is missing its own command_id",
            record.command_id
        ))
    })?;
    if transition_cmd != &record.command_id {
        return Err(AcceptanceError::CorruptLog(format!(
            "command_id mismatch: transition is for {:?}, record is for {:?}",
            transition_cmd, record.command_id
        )));
    }

    if !allowed_transition(record.state, to_state) {
        return Err(AcceptanceError::CorruptLog(format!(
            "disallowed transition {:?} -> {to_state:?} for command {:?}",
            record.state, record.command_id
        )));
    }

    record.state = to_state;
    if to_state.is_terminal() {
        record.terminal_lsn = Some(event_lsn);
        record.failure_code = (failure_code != FailureCode::Unspecified).then_some(failure_code);
    }

    Ok(())
}
