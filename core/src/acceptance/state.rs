//! In-memory command state derived from the durable event log.

use patchbay_contracts::patchbay::{CommandId, FailureCode, GrantId, Operation, OperationState};

use super::AcceptanceError;

/// The current state of an accepted command.
///
/// This record is a derived in-memory projection. The accepted [`Operation`]
/// and subsequent transition events in the durable log remain authoritative.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandRecord {
    pub command_id: CommandId,
    pub operation: Operation,
    pub state: OperationState,
    /// Authorizing-grant provenance. Durable population is deferred until the
    /// accepted Operation event carries acceptance metadata.
    pub grant_id: Option<GrantId>,
    /// The LSN of the transition into a terminal state, if one has committed.
    pub terminal_lsn: Option<u64>,
    pub failure_code: Option<FailureCode>,
}

impl CommandRecord {
    /// Build the initial projection for a durably accepted operation with its
    /// verified authorizing-grant provenance.
    pub fn new_accepted(
        operation: Operation,
        grant_id: GrantId,
        accept_lsn: u64,
    ) -> Result<Self, AcceptanceError> {
        if grant_id.value.is_empty() {
            return Err(AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {accept_lsn} has an empty grant_id"
            )));
        }
        let command_id = operation.command_id.clone().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {accept_lsn} is missing command_id"
            ))
        })?;
        Ok(Self {
            command_id,
            operation,
            state: OperationState::Accepted,
            grant_id: Some(grant_id),
            terminal_lsn: None,
            failure_code: None,
        })
    }

    /// Build the initial projection for a legacy in-memory test operation.
    /// Durable replay uses [`Self::new_accepted`] and rejects absent grant
    /// provenance.

    ///
    /// `accept_lsn` identifies the source event in corruption diagnostics. The
    /// record retains only the terminal LSN because the accepted operation's
    /// event remains the authority for its acceptance LSN.
    pub fn new(operation: Operation, accept_lsn: u64) -> Result<Self, AcceptanceError> {
        let command_id = operation.command_id.clone().ok_or_else(|| {
            AcceptanceError::CorruptRecord(format!(
                "accepted operation at LSN {accept_lsn} is missing command_id"
            ))
        })?;

        Ok(Self {
            command_id,
            operation,
            state: OperationState::Accepted,
            grant_id: None,
            terminal_lsn: None,
            failure_code: None,
        })
    }
}

/// Return whether `state` is one of the protocol's six terminal states.
///
/// `Unspecified` is invalid at the protocol boundary, but it is deliberately
/// classified as non-terminal here; boundary validation is responsible for
/// rejecting it.
#[must_use]
pub const fn is_terminal(state: OperationState) -> bool {
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

/// Ergonomic terminal-state query for the generated [`OperationState`] type.
///
/// Rust does not permit this crate to add an inherent implementation to a type
/// generated in `patchbay-contracts`, so this extension trait provides the
/// intended `state.is_terminal()` spelling while delegating to the canonical
/// free function.
pub trait OperationStateExt {
    fn is_terminal(&self) -> bool;
}

impl OperationStateExt for OperationState {
    fn is_terminal(&self) -> bool {
        is_terminal(*self)
    }
}
