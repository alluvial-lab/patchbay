use patchbay_contracts::patchbay::{
    AcceptedOperation, AdapterId, AuthorityDomainId, CommandId, CommandTransition, FailureCode,
    GrantId, Operation, OperationState, ResourceId, ResourceIdentity, ResourceKind,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::CommandIndex,
    adapter::fail_running_commands_for_adapter,
    storage::{RecordedEvent, RusqliteStorage, Storage},
};
use prost::Message;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn resource_scope(adapter: &str, kind: Option<&str>, id: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Resource as i32,
        resource: Some(ResourceIdentity {
            adapter_id: Some(AdapterId {
                value: adapter.to_owned(),
            }),
            resource_id: Some(ResourceId {
                value: id.to_owned(),
            }),
            resource_kind: kind.map(|value| ResourceKind {
                value: value.to_owned(),
            }),
        }),
        ..TargetScope::default()
    }
}

fn recorded(lsn: u64, kind: StoredEventKind, payload: Vec<u8>) -> RecordedEvent {
    RecordedEvent {
        event_id: patchbay_contracts::patchbay::EventId {
            authority_domain_id: Some(domain()),
            lsn: Some(patchbay_contracts::patchbay::Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload,
        },
    }
}

fn running_command(
    index: &mut CommandIndex,
    command: &str,
    scope: TargetScope,
    next_lsn: &mut u64,
) {
    let command_id = CommandId {
        value: command.to_owned(),
    };
    let operation = Operation {
        command_id: Some(command_id.clone()),
        authority_domain_id: Some(domain()),
        kind: patchbay_contracts::patchbay::OperationKind::Query as i32,
        target_scope: Some(scope),
        idempotency_key: format!("{command}-key"),
        ..Operation::default()
    };
    index
        .apply(&recorded(
            *next_lsn,
            StoredEventKind::Operation,
            AcceptedOperation {
                operation: Some(operation),
                authorizing_grant_id: Some(GrantId {
                    value: "grant".to_owned(),
                }),
            }
            .encode_to_vec(),
        ))
        .unwrap();
    *next_lsn += 1;
    for (from, to) in [
        (OperationState::Accepted, OperationState::Delivered),
        (OperationState::Delivered, OperationState::Running),
    ] {
        index
            .apply(&recorded(
                *next_lsn,
                StoredEventKind::CommandTransition,
                CommandTransition {
                    command_id: Some(command_id.clone()),
                    from_state: from as i32,
                    to_state: to as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            ))
            .unwrap();
        *next_lsn += 1;
    }
}

#[tokio::test]
async fn disconnect_fails_only_running_commands_with_a_canonical_matching_resource_adapter() {
    let mut index = CommandIndex::new();
    let mut lsn = 1;
    running_command(
        &mut index,
        "matching",
        resource_scope("adapter-a", Some("pool"), "shared"),
        &mut lsn,
    );
    running_command(
        &mut index,
        "other-adapter",
        resource_scope("adapter-b", Some("pool"), "shared"),
        &mut lsn,
    );
    running_command(
        &mut index,
        "malformed",
        resource_scope("adapter-a", None, "shared"),
        &mut lsn,
    );

    let storage = RusqliteStorage::open_in_memory().unwrap();
    let failed = fail_running_commands_for_adapter(
        &storage,
        &index,
        &domain(),
        &AdapterId {
            value: "adapter-a".to_owned(),
        },
    )
    .await
    .unwrap();
    assert_eq!(failed.len(), 1);

    let events = storage
        .read_after(&domain(), patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .unwrap();
    let transition = events
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
        .map(|event| CommandTransition::decode(event.payload.payload.as_slice()).unwrap())
        .expect("one failure transition");
    assert_eq!(
        transition.command_id,
        Some(CommandId {
            value: "matching".to_owned()
        })
    );
    assert_eq!(transition.from_state, OperationState::Running as i32);
    assert_eq!(transition.to_state, OperationState::Failed as i32);
    assert_eq!(
        transition.failure_code,
        FailureCode::ExecutionOutcomeUnknown as i32
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
            .count(),
        1
    );
}
