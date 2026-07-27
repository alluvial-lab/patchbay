use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId, AdapterId,
    AuthorityDomainId, CommandId, CommandTransition, EventId, Generation, Lsn, Operation,
    OperationKind, OperationState, RuntimeSessionId, SessionRegistered, SessionStateEvent,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    authority::{AuthorityError, SpawnDescendantTail, DESCENDANT_GRANT_ALLOWED_KINDS},
    storage::RecordedEvent,
};
use prost::Message;

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn command(value: &str) -> CommandId {
    CommandId {
        value: value.to_owned(),
    }
}

fn actor(value: &str) -> ActorId {
    ActorId {
        value: value.to_owned(),
    }
}

fn spawn_event(lsn: u64, actor_id: &str) -> RecordedEvent {
    recorded(
        lsn,
        &domain("authority-main"),
        StoredEventKind::Operation,
        &AcceptedOperation {
            operation: Some(Operation {
                command_id: Some(command("spawn-1")),
                authority_domain_id: Some(domain("authority-main")),
                sender: Some(ActorEndpointRef {
                    actor_id: Some(actor(actor_id)),
                    ..ActorEndpointRef::default()
                }),
                kind: OperationKind::Spawn as i32,
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::FleetSupervisor as i32,
                    ..TargetScope::default()
                }),
                ..Operation::default()
            }),
            authorizing_grant_id: Some(patchbay_contracts::patchbay::GrantId { value: "spawn-grant".to_owned() }),
        },
    )
}

fn transition_event(lsn: u64, to_state: OperationState) -> RecordedEvent {
    recorded(
        lsn,
        &domain("authority-main"),
        StoredEventKind::CommandTransition,
        &CommandTransition {
            command_id: Some(command("spawn-1")),
            to_state: to_state as i32,
            from_state: OperationState::Running as i32,
            ..CommandTransition::default()
        },
    )
}

fn registration_event(lsn: u64, include_spawn_origin: bool) -> RecordedEvent {
    let spawn_origin = include_spawn_origin.then(|| TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(command("spawn-1"))),
    });
    recorded(
        lsn,
        &domain("authority-main"),
        StoredEventKind::SessionState,
        &SessionStateEvent {
            authority_domain_id: Some(domain("authority-main")),
            mutation: Some(session_state_event::Mutation::Registered(
                SessionRegistered {
                    adapter_id: Some(AdapterId {
                        value: "pi".to_owned(),
                    }),
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: Some(RuntimeSessionId {
                        value: "spawned-session".to_owned(),
                    }),
                    session_generation: Some(Generation { value: 7 }),
                    spawn_origin,
                    ..SessionRegistered::default()
                },
            )),
        },
    )
}

fn recorded<M: Message>(
    lsn: u64,
    authority_domain_id: &AuthorityDomainId,
    kind: StoredEventKind,
    message: &M,
) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(authority_domain_id.clone()),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: message.encode_to_vec(),
        },
    }
}

#[test]
fn completed_spawn_issues_exactly_once_in_all_six_arrival_orders() {
    let events = [
        spawn_event(1, "operator"),
        transition_event(2, OperationState::Completed),
        registration_event(3, true),
    ];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];

    for order in permutations {
        let mut tail = SpawnDescendantTail::new();
        let issuances: Vec<_> = order
            .into_iter()
            .filter_map(|index| tail.observe(&events[index]).unwrap())
            .collect();

        assert_eq!(
            issuances.len(),
            1,
            "arrival order {order:?} must issue exactly once"
        );
        let issuance = &issuances[0];
        assert_eq!(issuance.spawn_operation_id, command("spawn-1"));
        assert_eq!(issuance.subject_actor_id, actor("operator"));
        assert_eq!(issuance.authority_domain_id, domain("authority-main"));
        assert_eq!(
            issuance.allowed_operation_kinds,
            DESCENDANT_GRANT_ALLOWED_KINDS
        );
        assert_eq!(
            issuance.descendant_grant_id.value,
            "desc:authority-main:spawn-1"
        );
        assert_eq!(issuance.spawning_grant_id, Some(patchbay_contracts::patchbay::GrantId { value: "spawn-grant".to_owned() }));
        assert_eq!(issuance.audit_id, None);
        assert_eq!(
            issuance.spawned_session_scope,
            TargetScope {
                kind: TargetScopeKind::RuntimeSession as i32,
                adapter_id: Some(AdapterId {
                    value: "pi".to_owned(),
                }),
                runtime_session_id: Some(RuntimeSessionId {
                    value: "spawned-session".to_owned(),
                }),
                session_generation: Some(Generation { value: 7 }),
                deployment_scope: "machine-a".to_owned(),
                ..TargetScope::default()
            },
            "the grant target must be the registered session, not the spawn fleet target"
        );
    }
}

#[test]
fn non_completed_terminal_states_do_not_issue() {
    for state in [
        OperationState::Rejected,
        OperationState::Failed,
        OperationState::Cancelled,
    ] {
        let mut tail = SpawnDescendantTail::new();
        for event in [
            spawn_event(1, "operator"),
            transition_event(2, state),
            registration_event(3, true),
        ] {
            assert_eq!(tail.observe(&event).unwrap(), None, "state {state:?}");
        }
    }
}

#[test]
fn registration_without_spawn_origin_does_not_issue() {
    let mut tail = SpawnDescendantTail::new();
    for event in [
        spawn_event(1, "operator"),
        transition_event(2, OperationState::Completed),
        registration_event(3, false),
    ] {
        assert_eq!(tail.observe(&event).unwrap(), None);
    }
}

#[test]
fn replaying_the_complete_triple_does_not_duplicate_an_issuance() {
    let events = [
        spawn_event(1, "operator"),
        transition_event(2, OperationState::Completed),
        registration_event(3, true),
    ];
    let mut tail = SpawnDescendantTail::new();

    let first_pass_count = events
        .iter()
        .filter_map(|event| tail.observe(event).unwrap())
        .count();
    let replay_count = events
        .iter()
        .filter_map(|event| tail.observe(event).unwrap())
        .count();

    assert_eq!(first_pass_count, 1);
    assert_eq!(replay_count, 0);
}

#[test]
fn conflicting_spawn_duplicate_is_corrupt_log() {
    let mut tail = SpawnDescendantTail::new();
    tail.observe(&spawn_event(1, "operator")).unwrap();

    assert!(matches!(
        tail.observe(&spawn_event(2, "different-operator")),
        Err(AuthorityError::CorruptLog(message))
            if message.contains("conflicting records")
    ));
}

#[test]
fn decoded_and_tail_domains_are_enforced() {
    let mut tail = SpawnDescendantTail::new();
    tail.observe(&spawn_event(1, "operator")).unwrap();

    let cross_domain_transition = recorded(
        2,
        &domain("authority-other"),
        StoredEventKind::CommandTransition,
        &CommandTransition {
            command_id: Some(command("spawn-1")),
            to_state: OperationState::Completed as i32,
            ..CommandTransition::default()
        },
    );
    assert!(matches!(
        tail.observe(&cross_domain_transition),
        Err(AuthorityError::CorruptLog(message)) if message.contains("bound")
    ));

    let mismatched_operation = recorded(
        3,
        &domain("authority-main"),
        StoredEventKind::Operation,
        &AcceptedOperation {
            operation: Some(Operation {
                command_id: Some(command("spawn-2")),
                authority_domain_id: Some(domain("authority-other")),
                kind: OperationKind::Spawn as i32,
                ..Operation::default()
            }),
            authorizing_grant_id: Some(patchbay_contracts::patchbay::GrantId { value: "spawn-grant".to_owned() }),
        },
    );
    assert!(matches!(
        tail.observe(&mismatched_operation),
        Err(AuthorityError::CorruptLog(message)) if message.contains("does not match")
    ));
}
