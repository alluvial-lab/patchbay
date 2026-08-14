//! Canonical command lifecycle transitions and transition-event application.

use patchbay_contracts::patchbay::{
    CommandTransition, FailureCode, GrantRevocationEffect, OperationState,
};

use crate::storage::StorageError;

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

    /// A transition event arrived for a command that is already terminal.
    ///
    /// This is NOT corruption — it is the race-produced duplicate that the
    /// first-durable-terminal-wins design expects. Under concurrency, two
    /// terminal candidates can both pass the in-memory `current_state` check
    /// (a TOCTOU window) and both append `COMMAND_TRANSITION` events. The
    /// first wins (TerminalFinality); the second is this variant. The replay
    /// fold catches it and skips the event rather than aborting recovery.
    /// The live `ingest_observation` path checks `current_state` before
    /// appending to prevent the common case, but the race window exists by
    /// design — the log is append-only, and TerminalFinality is a STATE
    /// property (enforced by `apply_transition`), not a LOG property.
    #[error("transition for already-terminal command {0}")]
    AlreadyTerminal(String),

    /// A target scope cannot be projected to a canonical idempotency key.
    #[error("invalid target scope: {0}")]
    InvalidTargetScope(String),

    /// The durable storage boundary failed outside normal dedup reconciliation.
    #[error(transparent)]
    Storage(#[from] StorageError),
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
/// Apply one durable revocation-policy effect through the same lifecycle
/// validator used by adapter transitions. The revocation LSN is the durable
/// terminal boundary for the affected command.
pub fn apply_grant_revocation_effect(
    record: &mut CommandRecord,
    effect: &GrantRevocationEffect,
    revocation_lsn: u64,
) -> Result<bool, AcceptanceError> {
    let command_id = effect.command_id.as_ref().ok_or_else(|| {
        AcceptanceError::CorruptLog("grant revocation effect is missing command_id".to_owned())
    })?;
    if command_id != &record.command_id {
        return Err(AcceptanceError::CorruptLog(format!(
            "grant revocation effect targets {:?}, record is {:?}",
            command_id, record.command_id
        )));
    }
    let from = OperationState::try_from(effect.from_state).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown revocation effect from_state {}",
            effect.from_state
        ))
    })?;
    let to = OperationState::try_from(effect.to_state).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown revocation effect to_state {}",
            effect.to_state
        ))
    })?;
    let failure = FailureCode::try_from(effect.failure_code).map_err(|_| {
        AcceptanceError::CorruptLog(format!(
            "unknown revocation effect failure_code {}",
            effect.failure_code
        ))
    })?;
    if record.state.is_terminal() {
        return Ok(false);
    }
    if from != record.state || !allowed_transition(from, to) {
        return Err(AcceptanceError::CorruptLog(format!(
            "invalid revocation effect {:?} -> {:?} for command {:?}",
            from, to, record.command_id
        )));
    }
    let expected_failure = match to {
        OperationState::Cancelled => FailureCode::Cancelled,
        OperationState::Rejected => FailureCode::AuthorizationDenied,
        _ => {
            return Err(AcceptanceError::CorruptLog(
                "revocation effect must terminalize command".to_owned(),
            ))
        }
    };
    if failure != expected_failure {
        return Err(AcceptanceError::CorruptLog(format!(
            "revocation effect for command {:?} has failure {:?}, expected {:?}",
            record.command_id, failure, expected_failure
        )));
    }
    record.state = to;
    record.terminal_lsn = Some(revocation_lsn);
    record.failure_code = Some(failure);
    Ok(true)
}

pub fn apply_transition(
    record: &mut CommandRecord,
    transition: &CommandTransition,
    event_lsn: u64,
) -> Result<(), AcceptanceError> {
    if record.state.is_terminal() {
        return Err(AcceptanceError::AlreadyTerminal(format!(
            "{:?}",
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
