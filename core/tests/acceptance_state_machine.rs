use patchbay_contracts::patchbay::{
    CommandId, CommandTransition, FailureCode, Operation, OperationState,
};
use patchbay_core::acceptance::{
    allowed_transition, apply_transition, is_terminal, AcceptanceError, CommandRecord,
    OperationStateExt,
};

const STATES: [OperationState; 9] = [
    OperationState::Accepted,
    OperationState::Delivered,
    OperationState::Running,
    OperationState::Completed,
    OperationState::Rejected,
    OperationState::Failed,
    OperationState::Expired,
    OperationState::Cancelled,
    OperationState::Superseded,
];

const TERMINAL_STATES: [OperationState; 6] = [
    OperationState::Completed,
    OperationState::Rejected,
    OperationState::Failed,
    OperationState::Expired,
    OperationState::Cancelled,
    OperationState::Superseded,
];

const ALLOWED_TRANSITIONS: [(OperationState, OperationState); 18] = [
    (OperationState::Accepted, OperationState::Delivered),
    (OperationState::Accepted, OperationState::Rejected),
    (OperationState::Accepted, OperationState::Failed),
    (OperationState::Accepted, OperationState::Expired),
    (OperationState::Accepted, OperationState::Cancelled),
    (OperationState::Accepted, OperationState::Superseded),
    (OperationState::Delivered, OperationState::Running),
    (OperationState::Delivered, OperationState::Completed),
    (OperationState::Delivered, OperationState::Rejected),
    (OperationState::Delivered, OperationState::Failed),
    (OperationState::Delivered, OperationState::Expired),
    (OperationState::Delivered, OperationState::Cancelled),
    (OperationState::Delivered, OperationState::Superseded),
    (OperationState::Running, OperationState::Completed),
    (OperationState::Running, OperationState::Failed),
    (OperationState::Running, OperationState::Expired),
    (OperationState::Running, OperationState::Cancelled),
    (OperationState::Running, OperationState::Superseded),
];

fn operation() -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: "command-1".to_owned(),
        }),
        ..Operation::default()
    }
}

fn record_at(state: OperationState) -> CommandRecord {
    let mut record = CommandRecord::new(operation(), 1).expect("test operation has a command id");
    record.state = state;
    record
}

fn transition(
    from_state: OperationState,
    to_state: OperationState,
    failure_code: FailureCode,
) -> CommandTransition {
    CommandTransition {
        command_id: Some(CommandId {
            value: "command-1".to_owned(),
        }),
        from_state: from_state as i32,
        to_state: to_state as i32,
        failure_code: failure_code as i32,
        ..CommandTransition::default()
    }
}

/// A transition with a caller-specified command_id (for identity-mismatch tests).
fn transition_for(
    cmd: &str,
    from_state: OperationState,
    to_state: OperationState,
) -> CommandTransition {
    CommandTransition {
        command_id: Some(CommandId {
            value: cmd.to_owned(),
        }),
        from_state: from_state as i32,
        to_state: to_state as i32,
        ..CommandTransition::default()
    }
}

#[test]
fn allowed_transition_matches_every_protocol_table_cell() {
    for from in STATES {
        for to in STATES {
            let expected = ALLOWED_TRANSITIONS.contains(&(from, to));
            assert_eq!(
                allowed_transition(from, to),
                expected,
                "unexpected protocol adjacency result for {from:?} -> {to:?}"
            );
        }
    }
}

#[test]
fn every_terminal_state_rejects_transitions_out() {
    for terminal_state in TERMINAL_STATES {
        let mut record = record_at(terminal_state);
        let event = transition(
            terminal_state,
            OperationState::Delivered,
            FailureCode::Unspecified,
        );

        let result = apply_transition(&mut record, &event, 99);

        assert!(
            matches!(result, Err(AcceptanceError::AlreadyTerminal(_))),
            "{terminal_state:?} accepted a transition out"
        );
        assert_eq!(record.state, terminal_state);
    }
}

#[test]
fn accepted_to_completed_is_rejected() {
    let mut record = record_at(OperationState::Accepted);
    let event = transition(
        OperationState::Accepted,
        OperationState::Completed,
        FailureCode::Unspecified,
    );

    let result = apply_transition(&mut record, &event, 10);

    assert!(matches!(result, Err(AcceptanceError::CorruptLog(_))));
    assert_eq!(record.state, OperationState::Accepted);
    assert_eq!(record.terminal_lsn, None);
}

#[test]
fn from_state_mismatch_is_rejected_without_mutation() {
    // The mismatch must be the ONLY reason for rejection: the to_state must
    // be ALLOWED from the record's actual state, so that without the
    // mismatch guard the transition would apply. An Accepted record + a
    // Delivered->Delivered event: Accepted->Delivered is allowed, so only
    // the from_state mismatch (Delivered != Accepted) prevents mutation.
    // This is the non-vacuous test — removing the guard would let it through.
    let mut record = record_at(OperationState::Accepted);
    let event = transition(
        OperationState::Delivered,
        OperationState::Delivered,
        FailureCode::Unspecified,
    );

    let result = apply_transition(&mut record, &event, 10);

    assert!(matches!(result, Err(AcceptanceError::CorruptLog(_))));
    assert_eq!(record.state, OperationState::Accepted);
    assert_eq!(record.terminal_lsn, None);
    assert_eq!(record.failure_code, None);
}

#[test]
fn command_id_mismatch_is_rejected_without_mutation() {
    // A transition for a different command must not mutate this record,
    // even if the from/to states are valid for it.
    let mut record = record_at(OperationState::Accepted);
    let event = transition_for(
        "command-2",
        OperationState::Accepted,
        OperationState::Delivered,
    );

    let result = apply_transition(&mut record, &event, 10);

    assert!(matches!(result, Err(AcceptanceError::CorruptLog(_))));
    assert_eq!(record.state, OperationState::Accepted);
    assert_eq!(record.terminal_lsn, None);
}

#[test]
fn transition_without_command_id_is_rejected() {
    let mut record = record_at(OperationState::Accepted);
    let mut event = transition(
        OperationState::Accepted,
        OperationState::Delivered,
        FailureCode::Unspecified,
    );
    event.command_id = None;

    let result = apply_transition(&mut record, &event, 10);

    assert!(matches!(result, Err(AcceptanceError::CorruptLog(_))));
    assert_eq!(record.state, OperationState::Accepted);
}

#[test]
fn terminal_classification_matches_the_protocol_registry() {
    for state in STATES {
        let expected = TERMINAL_STATES.contains(&state);
        assert_eq!(is_terminal(state), expected, "free function: {state:?}");
        assert_eq!(state.is_terminal(), expected, "extension trait: {state:?}");
    }
    assert!(!is_terminal(OperationState::Unspecified));
}

#[test]
fn every_allowed_transition_applies_and_sets_terminal_metadata() {
    for (index, (from, to)) in ALLOWED_TRANSITIONS.into_iter().enumerate() {
        let event_lsn = 100 + index as u64;
        let mut record = record_at(from);
        let event = transition(from, to, FailureCode::Unspecified);

        apply_transition(&mut record, &event, event_lsn)
            .unwrap_or_else(|error| panic!("{from:?} -> {to:?} failed: {error}"));

        assert_eq!(record.state, to);
        assert_eq!(
            record.terminal_lsn,
            is_terminal(to).then_some(event_lsn),
            "terminal LSN for {from:?} -> {to:?}"
        );
        assert_eq!(record.failure_code, None);
    }
}

#[test]
fn terminal_transition_retains_a_concrete_failure_code() {
    let mut record = record_at(OperationState::Running);
    let event = transition(
        OperationState::Running,
        OperationState::Failed,
        FailureCode::ExecutionFailed,
    );

    apply_transition(&mut record, &event, 41).expect("running -> failed is allowed");

    assert_eq!(record.state, OperationState::Failed);
    assert_eq!(record.terminal_lsn, Some(41));
    assert_eq!(record.failure_code, Some(FailureCode::ExecutionFailed));
}

#[test]
fn command_record_new_initializes_at_accepted_and_preserves_operation() {
    let op = operation();
    let record = CommandRecord::new(op.clone(), 1).expect("valid operation");

    assert_eq!(record.command_id, *op.command_id.as_ref().unwrap());
    assert_eq!(record.state, OperationState::Accepted);
    assert_eq!(record.terminal_lsn, None);
    assert_eq!(record.failure_code, None);
    assert_eq!(record.operation, op);
}

#[test]
fn command_record_rejects_an_accepted_operation_without_identity() {
    let error = CommandRecord::new(Operation::default(), 7)
        .expect_err("an accepted operation must have a command id");

    assert!(matches!(error, AcceptanceError::CorruptRecord(_)));
}
