use patchbay_contracts::patchbay::{
    typed_correlation, AcceptedOperation, AuthorityDomainId, CommandId, CommandTransition, Elicitation, ElicitationId,
    ElicitationState, FailureCode, Lsn, Operation, OperationKind, OperationState, StoredEventKind,
    StoredEventPayload, TypedCorrelation,
};
use patchbay_core::acceptance::{rebuild_slots_from_log, ElicitationRecord, ElicitationSlotLayer};
use patchbay_core::storage::{RecordedEvent, RusqliteStorage, Storage};
use prost::Message;

fn authority_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn elicitation_id() -> ElicitationId {
    ElicitationId {
        value: "elicitation-1".to_owned(),
    }
}

fn elicitation(state: ElicitationState) -> Elicitation {
    Elicitation {
        elicitation_id: Some(elicitation_id()),
        authority_domain_id: Some(authority_domain()),
        state: state as i32,
        ..Elicitation::default()
    }
}

fn elicitation_correlation() -> TypedCorrelation {
    TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::ElicitationId(elicitation_id())),
    }
}

fn transition(command: &str, to_state: OperationState) -> CommandTransition {
    CommandTransition {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        from_state: OperationState::Delivered as i32,
        to_state: to_state as i32,
        failure_code: FailureCode::Unspecified as i32,
        correlations: vec![elicitation_correlation()],
        ..CommandTransition::default()
    }
}

/// A response Operation (ElicitationResponse kind) correlated to the test Elicitation.
fn response_operation(command: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        authority_domain_id: Some(authority_domain()),
        kind: OperationKind::ElicitationResponse as i32,
        correlations: vec![elicitation_correlation()],
        ..Operation::default()
    }
}

/// A non-response Operation (Instruct kind) that happens to carry an
/// ElicitationId correlation — must NOT terminalize the slot.
fn non_response_operation(command: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        authority_domain_id: Some(authority_domain()),
        kind: OperationKind::Instruct as i32,
        correlations: vec![elicitation_correlation()],
        ..Operation::default()
    }
}

fn event_payload<M: Message>(kind: StoredEventKind, message: &M) -> StoredEventPayload {
    StoredEventPayload {
        kind: kind as i32,
        payload: message.encode_to_vec(),
    }
}

async fn append_elicitation(storage: &RusqliteStorage, state: ElicitationState) -> u64 {
    storage
        .append(
            &authority_domain(),
            event_payload(StoredEventKind::Elicitation, &elicitation(state)),
        )
        .await
        .unwrap()
        .lsn
        .unwrap()
        .value
}

async fn append_transition(
    storage: &RusqliteStorage,
    command: &str,
    to_state: OperationState,
) -> u64 {
    storage
        .append(
            &authority_domain(),
            event_payload(
                StoredEventKind::CommandTransition,
                &transition(command, to_state),
            ),
        )
        .await
        .unwrap()
        .lsn
        .unwrap()
        .value
}

async fn append_operation(storage: &RusqliteStorage, operation: &Operation) -> u64 {
    storage
        .append(
            &authority_domain(),
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: AcceptedOperation {
                    operation: Some(operation.clone()),
                    authorizing_grant_id: Some(patchbay_contracts::patchbay::GrantId { value: "test-grant".to_owned() }),
                }.encode_to_vec(),
            },
        )
        .await
        .unwrap()
        .lsn
        .unwrap()
        .value
}

/// Append a response OPERATION event + its COMMAND_TRANSITION event.
async fn append_response_op_and_transition(
    storage: &RusqliteStorage,
    command: &str,
    to_state: OperationState,
) -> u64 {
    append_operation(storage, &response_operation(command)).await;
    append_transition(storage, command, to_state).await
}

async fn all_events(storage: &RusqliteStorage) -> Vec<RecordedEvent> {
    storage
        .read_after(&authority_domain(), Lsn { value: 0 })
        .await
        .unwrap()
}

fn assert_answered(record: &ElicitationRecord, terminal_lsn: u64) {
    assert_eq!(record.elicitation_id, elicitation_id());
    assert_eq!(record.state, ElicitationState::Answered);
    assert_eq!(record.terminal_lsn, Some(terminal_lsn));
}

#[tokio::test]
async fn terminal_response_transition_terminalizes_the_correlated_slot() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    let terminal_lsn = append_response_op_and_transition(
        &storage,
        "response-command-1",
        OperationState::Completed,
    )
    .await;

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer
        .get_slot(&elicitation_id())
        .expect("the Elicitation event opens a projected slot");

    assert_answered(slot, terminal_lsn);
    assert_eq!(
        slot.winning_response,
        Some(response_operation("response-command-1")),
        "the projection retains the winning response for exact terminal retries"
    );
}

#[tokio::test]
async fn first_terminal_response_lsn_wins_and_later_response_is_stale() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    let winning_lsn = append_response_op_and_transition(
        &storage,
        "response-command-1",
        OperationState::Completed,
    )
    .await;
    // A second response (different command) for the same Elicitation.
    append_operation(&storage, &response_operation("response-command-2")).await;
    let stale_lsn = append_transition(&storage, "response-command-2", OperationState::Failed).await;
    assert!(winning_lsn < stale_lsn);

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer.get_slot(&elicitation_id()).unwrap();

    // The later response is stale: it neither rewrites the winner nor moves
    // the slot itself to ElicitationState::Stale.
    assert_answered(slot, winning_lsn);
}

#[tokio::test]
async fn replay_and_live_log_consumer_reconstruct_identical_slot_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    // A response operation may advance through non-terminal command states;
    // only its terminal transition closes the Elicitation slot.
    append_operation(&storage, &response_operation("response-command-1")).await;
    append_transition(&storage, "response-command-1", OperationState::Running).await;
    let terminal_lsn =
        append_transition(&storage, "response-command-1", OperationState::Completed).await;

    // The live path demonstrates the decoupling API: the slot layer receives
    // only RecordedEvent references and never calls the acceptance pipeline.
    let events = all_events(&storage).await;
    let mut live = ElicitationSlotLayer::new();
    for event in &events {
        live.observe(event).unwrap();
    }

    let rebuilt = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    assert_eq!(live, rebuilt);
    assert_answered(rebuilt.get_slot(&elicitation_id()).unwrap(), terminal_lsn);
}

#[tokio::test]
async fn reobserving_the_same_events_is_idempotent() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    let terminal_lsn = append_response_op_and_transition(
        &storage,
        "response-command-1",
        OperationState::Completed,
    )
    .await;
    let events = all_events(&storage).await;

    let mut layer = ElicitationSlotLayer::new();
    for event in &events {
        layer.observe(event).unwrap();
    }
    let after_first_pass = layer.clone();

    for event in &events {
        layer.observe(event).unwrap();
    }

    assert_eq!(layer, after_first_pass);
    assert_answered(layer.get_slot(&elicitation_id()).unwrap(), terminal_lsn);
}

#[tokio::test]
async fn non_terminal_response_transition_leaves_the_slot_open() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    append_operation(&storage, &response_operation("response-command-1")).await;
    append_transition(&storage, "response-command-1", OperationState::Running).await;

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer.get_slot(&elicitation_id()).unwrap();

    assert_eq!(slot.state, ElicitationState::Opened);
    assert_eq!(slot.terminal_lsn, None);
}

#[tokio::test]
async fn opening_event_uses_the_generated_elicitation_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Pending).await;

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer.get_slot(&elicitation_id()).unwrap();

    assert_eq!(slot.state, ElicitationState::Pending);
    assert_eq!(slot.terminal_lsn, None);
}

#[tokio::test]
async fn non_response_command_does_not_terminalize_the_slot() {
    // A regular command (Instruct) that happens to carry an ElicitationId
    // correlation must NOT terminalize the slot. Only response Operations
    // (ApprovalResponse / ElicitationResponse) do.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    append_operation(&storage, &non_response_operation("regular-command-1")).await;
    append_transition(&storage, "regular-command-1", OperationState::Completed).await;

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer.get_slot(&elicitation_id()).unwrap();

    // The slot stays non-terminal — the Completed transition was for a
    // non-response command, not a response Operation.
    assert_eq!(slot.state, ElicitationState::Opened);
    assert_eq!(slot.terminal_lsn, None);
}

#[tokio::test]
async fn failed_response_leaves_the_slot_pending() {
    // A response Operation that fails (Rejected/Failed) does NOT answer the
    // Elicitation. The slot stays pending — another surface may answer.
    // Operator denial is a completed typed `DENIED` decision (→ Declined),
    // not a machine Rejected/Failed response; those never terminalize the slot.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_elicitation(&storage, ElicitationState::Opened).await;
    append_response_op_and_transition(&storage, "response-command-1", OperationState::Failed).await;

    let layer = rebuild_slots_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let slot = layer.get_slot(&elicitation_id()).unwrap();

    assert_eq!(slot.state, ElicitationState::Opened);
    assert_eq!(slot.terminal_lsn, None);
}
