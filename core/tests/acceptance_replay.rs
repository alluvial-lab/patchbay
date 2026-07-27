use patchbay_contracts::patchbay::{
    AcceptedOperation, AdapterId, AuthorityDomainId, CommandId, CommandTransition, FailureCode, Observation,
    Operation, OperationKind, OperationState, RuntimeSessionId, StoredEventKind,
    StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::acceptance::{rebuild_from_log, target_key_for, AcceptanceError, CommandIndex};
use patchbay_core::storage::{RusqliteStorage, Storage};
use prost::Message;

fn authority_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn operation(command: &str, idempotency_key: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        authority_domain_id: Some(authority_domain()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            runtime_session_id: Some(RuntimeSessionId {
                value: "session-1".to_owned(),
            }),
            deployment_scope: "local".to_owned(),
            ..TargetScope::default()
        }),
        idempotency_key: idempotency_key.to_owned(),
        ..Operation::default()
    }
}

fn transition(
    command: &str,
    from: OperationState,
    to: OperationState,
    failure_code: FailureCode,
) -> CommandTransition {
    CommandTransition {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        from_state: from as i32,
        to_state: to as i32,
        failure_code: failure_code as i32,
        ..CommandTransition::default()
    }
}

fn event_payload<M: Message>(kind: StoredEventKind, message: &M) -> StoredEventPayload {
    StoredEventPayload {
        kind: kind as i32,
        payload: message.encode_to_vec(),
    }
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

async fn append_transition(storage: &RusqliteStorage, transition: &CommandTransition) -> u64 {
    storage
        .append(
            &authority_domain(),
            event_payload(StoredEventKind::CommandTransition, transition),
        )
        .await
        .unwrap()
        .lsn
        .unwrap()
        .value
}

#[tokio::test]
async fn operation_and_transition_events_reconstruct_the_full_index() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let first = operation("command-1", "key-1");
    let second = operation("command-2", "key-2");

    assert_eq!(append_operation(&storage, &first).await, 1);
    assert_eq!(
        append_transition(
            &storage,
            &transition(
                "command-1",
                OperationState::Accepted,
                OperationState::Delivered,
                FailureCode::Unspecified,
            ),
        )
        .await,
        2
    );
    // Observation evidence must not independently derive command state.
    storage
        .append(
            &authority_domain(),
            event_payload(StoredEventKind::Observation, &Observation::default()),
        )
        .await
        .unwrap();
    assert_eq!(
        append_transition(
            &storage,
            &transition(
                "command-1",
                OperationState::Delivered,
                OperationState::Running,
                FailureCode::Unspecified,
            ),
        )
        .await,
        4
    );
    assert_eq!(
        append_transition(
            &storage,
            &transition(
                "command-1",
                OperationState::Running,
                OperationState::Failed,
                FailureCode::ExecutionFailed,
            ),
        )
        .await,
        5
    );
    assert_eq!(append_operation(&storage, &second).await, 6);

    let index = rebuild_from_log(&storage, &authority_domain())
        .await
        .unwrap();

    assert_eq!(index.len(), 2);
    let first_record = index
        .get_command(first.command_id.as_ref().unwrap())
        .expect("first command is indexed");
    assert_eq!(first_record.operation, first);
    assert_eq!(first_record.state, OperationState::Failed);
    assert_eq!(first_record.terminal_lsn, Some(5));
    assert_eq!(
        first_record.failure_code,
        Some(FailureCode::ExecutionFailed)
    );

    let second_record = index
        .get_command(second.command_id.as_ref().unwrap())
        .expect("second command is indexed");
    assert_eq!(second_record.state, OperationState::Accepted);
    assert_eq!(second_record.terminal_lsn, None);
}

#[tokio::test]
async fn replay_is_deterministic_and_secondary_lookup_resolves_the_same_record() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let accepted = operation("command-1", "key-1");
    append_operation(&storage, &accepted).await;
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Accepted,
            OperationState::Delivered,
            FailureCode::Unspecified,
        ),
    )
    .await;

    let first = rebuild_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let second = rebuild_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    assert_eq!(first, second);

    let target_key = target_key_for(&accepted).unwrap();
    let by_retry_scope = first
        .get_by_idempotency_key(&authority_domain(), "key-1", &target_key)
        .expect("secondary hash index resolves the command");
    let by_command_id = first
        .get_command(accepted.command_id.as_ref().unwrap())
        .expect("primary hash index resolves the command");
    assert_eq!(by_retry_scope, by_command_id);
}

#[tokio::test]
async fn transition_for_unknown_command_is_corrupt_log() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_transition(
        &storage,
        &transition(
            "missing-command",
            OperationState::Accepted,
            OperationState::Delivered,
            FailureCode::Unspecified,
        ),
    )
    .await;

    let error = rebuild_from_log(&storage, &authority_domain())
        .await
        .expect_err("an unknown command transition must fail replay");

    assert!(matches!(error, AcceptanceError::CorruptLog(_)));
    assert!(error.to_string().contains("transition for unknown command"));
}

#[tokio::test]
async fn from_state_mismatch_is_corrupt_log_without_becoming_a_valid_transition() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    append_operation(&storage, &operation("command-1", "key-1")).await;
    // Accepted -> Delivered would otherwise be valid. Encoding Delivered as
    // from_state makes the mismatch itself the reason replay rejects it.
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Delivered,
            OperationState::Delivered,
            FailureCode::Unspecified,
        ),
    )
    .await;

    let error = rebuild_from_log(&storage, &authority_domain())
        .await
        .expect_err("a from_state mismatch must fail replay");

    assert!(matches!(error, AcceptanceError::CorruptLog(_)));
    assert!(error.to_string().contains("from_state mismatch"));
}

#[tokio::test]
async fn duplicate_operation_identity_is_corrupt_log() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let accepted = operation("command-1", "key-1");
    append_operation(&storage, &accepted).await;
    append_operation(&storage, &accepted).await;

    let error = rebuild_from_log(&storage, &authority_domain())
        .await
        .expect_err("duplicate command identity must fail replay");

    assert!(matches!(error, AcceptanceError::CorruptLog(_)));
    assert!(error.to_string().contains("duplicate operation"));
}

#[tokio::test]
async fn duplicate_terminal_transition_is_skipped_not_corruption() {
    // The TOCTOU race: under concurrency, two terminal candidates can both
    // pass the in-memory current_state check and both append COMMAND_TRANSITION
    // events. The first wins (TerminalFinality); the second is a race-produced
    // duplicate. The replay fold catches AlreadyTerminal and SKIPS the event
    // (it's not corruption — it's the expected first-durable-terminal-wins
    // outcome), rather than aborting recovery.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let accepted = operation("command-1", "key-1");
    append_operation(&storage, &accepted).await;
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Accepted,
            OperationState::Delivered,
            FailureCode::Unspecified,
        ),
    )
    .await;
    let first_terminal_lsn = append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Delivered,
            OperationState::Completed,
            FailureCode::Unspecified,
        ),
    )
    .await;
    // A second terminal transition (the race-produced duplicate).
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Completed,
            OperationState::Failed,
            FailureCode::ExecutionFailed,
        ),
    )
    .await;

    let rebuilt = rebuild_from_log(&storage, &authority_domain())
        .await
        .expect("duplicate terminal transition should be skipped, not abort recovery");
    let record = rebuilt
        .get_command(accepted.command_id.as_ref().unwrap())
        .expect("command reconstructed");
    // The FIRST terminal (Completed) wins; the second (Failed) is skipped.
    assert_eq!(record.state, OperationState::Completed);
    assert_eq!(record.terminal_lsn, Some(first_terminal_lsn));
    assert_eq!(record.failure_code, None);
}

#[tokio::test]
async fn rebuild_ignores_snapshots_and_replays_full_log() {
    // v0.1.0: snapshot checkpointing is deferred because the storage snapshot
    // slot has no projection discriminator. rebuild_from_log always replays
    // from LSN 0. This test verifies that even if a snapshot is written to
    // the authority-domain slot (by some other code path), rebuild_from_log
    // ignores it and reconstructs the full index from the complete event log.
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let accepted = operation("command-1", "key-1");
    append_operation(&storage, &accepted).await;
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Accepted,
            OperationState::Delivered,
            FailureCode::Unspecified,
        ),
    )
    .await;
    append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Delivered,
            OperationState::Running,
            FailureCode::Unspecified,
        ),
    )
    .await;
    let terminal_lsn = append_transition(
        &storage,
        &transition(
            "command-1",
            OperationState::Running,
            OperationState::Completed,
            FailureCode::Unspecified,
        ),
    )
    .await;

    // Rebuild — should reconstruct the full lifecycle from LSN 0.
    let rebuilt = rebuild_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    let record = rebuilt
        .get_command(accepted.command_id.as_ref().unwrap())
        .expect("full replay restored the accepted command");
    assert_eq!(record.state, OperationState::Completed);
    assert_eq!(record.terminal_lsn, Some(terminal_lsn));
    assert_eq!(record.failure_code, None);

    // Determinism: rebuild again, same result.
    let rebuilt_again = rebuild_from_log(&storage, &authority_domain())
        .await
        .unwrap();
    assert_eq!(rebuilt, rebuilt_again);
}

#[test]
fn new_index_is_empty() {
    let index = CommandIndex::new();
    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
}
