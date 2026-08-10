use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterId, AuditEventKind, AuditRecord, AuthorityDomainId, CommandId, CommandTransition,
    DescendantGrant, DescendantGrantProvenance, DeviceId, EndpointId, EventId, FailureCode,
    Generation, GrantId, GrantRevocationPolicy, Lsn, Observation, ObservationKind, Operation,
    OperationKind, OperationState, RuntimeSessionId, SessionGenerationBumped, SessionRegistered,
    SessionStateEvent, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
    TypedCorrelation,
};
use patchbay_core::{
    authority::{
        AuthorityError, DescendantGrantIssuance, SpawnCompletionAction, SpawnDescendantTail,
        DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::RecordedEvent,
};
use prost::Message;
use prost_types::Timestamp;

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.into(),
    }
}
fn command(value: &str) -> CommandId {
    CommandId {
        value: value.into(),
    }
}
fn actor(value: &str) -> ActorId {
    ActorId {
        value: value.into(),
    }
}
fn endpoint(value: &str) -> EndpointId {
    EndpointId {
        value: value.into(),
    }
}
fn device(value: &str) -> DeviceId {
    DeviceId {
        value: value.into(),
    }
}
fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.into(),
    }
}
fn event_id(lsn: u64) -> EventId {
    EventId {
        authority_domain_id: Some(domain("authority-main")),
        lsn: Some(Lsn { value: lsn }),
    }
}
fn command_correlation() -> TypedCorrelation {
    TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(command("spawn-1"))),
    }
}
fn spawn_target() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    }
}
fn session_target() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId { value: "pi".into() }),
        deployment_scope: "machine-a".into(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "spawned-session".into(),
        }),
        session_generation: Some(Generation { value: 7 }),
        ..TargetScope::default()
    }
}

fn recorded<M: Message>(lsn: u64, kind: StoredEventKind, message: &M) -> RecordedEvent {
    RecordedEvent {
        event_id: event_id(lsn),
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: message.encode_to_vec(),
        },
    }
}

fn spawn_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::Operation,
        &AcceptedOperation {
            operation: Some(Operation {
                command_id: Some(command("spawn-1")),
                authority_domain_id: Some(domain("authority-main")),
                sender: Some(ActorEndpointRef {
                    actor_id: Some(actor("operator")),
                    endpoint_id: Some(endpoint("browser")),
                    device_id: Some(device("laptop")),
                    ..ActorEndpointRef::default()
                }),
                kind: OperationKind::Spawn as i32,
                target_scope: Some(spawn_target()),
                ..Operation::default()
            }),
            authorizing_grant_id: Some(grant_id("spawn-grant")),
        },
    )
}

fn transition_event(lsn: u64, from: OperationState, to: OperationState) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::CommandTransition,
        &CommandTransition {
            command_id: Some(command("spawn-1")),
            from_state: from as i32,
            to_state: to as i32,
            failure_code: FailureCode::Unspecified as i32,
            ..CommandTransition::default()
        },
    )
}

fn result_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::Observation,
        &Observation {
            authority_domain_id: Some(domain("authority-main")),
            kind: ObservationKind::Result as i32,
            correlations: vec![command_correlation()],
            target_scope: Some(spawn_target()),
            failure_code: FailureCode::Unspecified as i32,
            ..Observation::default()
        },
    )
}

fn registration_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::SessionState,
        &SessionStateEvent {
            authority_domain_id: Some(domain("authority-main")),
            mutation: Some(session_state_event::Mutation::Registered(
                SessionRegistered {
                    adapter_id: Some(AdapterId { value: "pi".into() }),
                    deployment_scope: "machine-a".into(),
                    runtime_session_id: Some(RuntimeSessionId {
                        value: "spawned-session".into(),
                    }),
                    session_generation: Some(Generation { value: 7 }),
                    spawn_origin: Some(command_correlation()),
                    ..SessionRegistered::default()
                },
            )),
        },
    )
}

fn bump_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::SessionState,
        &SessionStateEvent {
            authority_domain_id: Some(domain("authority-main")),
            mutation: Some(session_state_event::Mutation::GenerationBumped(
                SessionGenerationBumped {
                    adapter_id: Some(AdapterId { value: "pi".into() }),
                    deployment_scope: "machine-a".into(),
                    runtime_session_id: Some(RuntimeSessionId {
                        value: "spawned-session".into(),
                    }),
                    from_generation: Some(Generation { value: 6 }),
                    to_generation: Some(Generation { value: 7 }),
                    spawn_origin: Some(command_correlation()),
                    ..SessionGenerationBumped::default()
                },
            )),
        },
    )
}

fn audit_event(lsn: u64, source: EventId) -> RecordedEvent {
    let id = event_id(lsn);
    recorded(
        lsn,
        StoredEventKind::AuditRecord,
        &AuditRecord {
            audit_event_id: Some(id),
            occurred_at: Some(Timestamp {
                seconds: 10,
                nanos: 0,
            }),
            kind: AuditEventKind::CommandCompleted as i32,
            actor_id: Some(actor("operator")),
            endpoint_id: Some(endpoint("browser")),
            device_id: Some(device("laptop")),
            command_id: Some(command("spawn-1")),
            grant_id: Some(grant_id("spawn-grant")),
            target_scope: Some(session_target()),
            failure_code: FailureCode::Unspecified as i32,
            reason_code: "spawn_completion".into(),
            source_event_id: Some(source),
            ..AuditRecord::default()
        },
    )
}

fn descendant_from(issuance: &DescendantGrantIssuance) -> DescendantGrant {
    DescendantGrant {
        grant_id: Some(issuance.descendant_grant_id.clone()),
        authority_domain_id: Some(issuance.authority_domain_id.clone()),
        subject_actor_id: Some(issuance.subject_actor_id.clone()),
        subject_endpoint_id: issuance.subject_endpoint_id.clone(),
        target_scope: Some(issuance.spawned_session_scope.clone()),
        allowed_operation_kinds: issuance
            .allowed_operation_kinds
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        provenance: Some(DescendantGrantProvenance {
            spawn_operation_id: Some(issuance.spawn_operation_id.clone()),
            spawning_grant_id: Some(issuance.spawning_grant_id.clone()),
        }),
        created_at: Some(issuance.created_at),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        audit_id: Some(issuance.audit_id.clone()),
        ..DescendantGrant::default()
    }
}

fn observe_all(tail: &mut SpawnDescendantTail, events: &[RecordedEvent]) {
    for event in events {
        tail.observe(event).unwrap();
    }
}

#[test]
fn registration_and_generation_bump_produce_the_same_ordered_actions() {
    for session_event in [registration_event(4), bump_event(4)] {
        let mut tail = SpawnDescendantTail::new();
        observe_all(
            &mut tail,
            &[
                spawn_event(1),
                transition_event(2, OperationState::Accepted, OperationState::Delivered),
                result_event(3),
                session_event,
            ],
        );

        let SpawnCompletionAction::RecordAudit(audit) = tail.next_action().unwrap().unwrap() else {
            panic!("audit must be first");
        };
        assert_eq!(audit.completion_source_event_id, event_id(3));
        assert_eq!(audit.spawning_grant_id, grant_id("spawn-grant"));
        assert_eq!(audit.subject_actor_id, actor("operator"));
        assert_eq!(audit.subject_endpoint_id, Some(endpoint("browser")));
        assert_eq!(audit.subject_device_id, Some(device("laptop")));
        assert_eq!(audit.spawned_session_scope, session_target());

        tail.observe(&audit_event(5, event_id(3))).unwrap();
        let SpawnCompletionAction::IssueDescendantGrant(issuance) =
            tail.next_action().unwrap().unwrap()
        else {
            panic!("grant must follow audit");
        };
        assert_eq!(
            issuance.descendant_grant_id.value,
            "desc:authority-main:spawn-1"
        );
        assert_eq!(
            issuance.allowed_operation_kinds,
            DESCENDANT_GRANT_ALLOWED_KINDS
        );
        assert_eq!(issuance.audit_id, event_id(5));

        tail.observe(&recorded(
            6,
            StoredEventKind::DescendantGrant,
            &descendant_from(&issuance),
        ))
        .unwrap();
        let SpawnCompletionAction::CommitCompleted(commit) = tail.next_action().unwrap().unwrap()
        else {
            panic!("completion must be last");
        };
        assert_eq!(commit.from_state, OperationState::Delivered);
        assert_eq!(commit.spawn_operation_id, command("spawn-1"));
    }
}

#[test]
fn relevant_fact_arrival_orders_converge_and_durable_progress_survives_redelivery() {
    let facts = [
        spawn_event(1),
        transition_event(2, OperationState::Accepted, OperationState::Running),
        result_event(3),
        registration_event(4),
    ];
    let orders = [[0, 1, 2, 3], [3, 2, 1, 0], [2, 0, 3, 1], [1, 3, 0, 2]];
    for order in orders {
        let mut tail = SpawnDescendantTail::new();
        for index in order {
            tail.observe(&facts[index]).unwrap();
        }
        assert!(matches!(
            tail.next_action().unwrap(),
            Some(SpawnCompletionAction::RecordAudit(_))
        ));
        tail.observe(&audit_event(5, event_id(3))).unwrap();
        let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) =
            tail.next_action().unwrap()
        else {
            panic!("expected grant action");
        };
        let grant = recorded(
            6,
            StoredEventKind::DescendantGrant,
            &descendant_from(&issuance),
        );
        tail.observe(&grant).unwrap();
        tail.observe(&audit_event(5, event_id(3))).unwrap();
        tail.observe(&grant).unwrap();
        assert!(matches!(
            tail.next_action().unwrap(),
            Some(SpawnCompletionAction::CommitCompleted(_))
        ));
    }
}

#[test]
fn failed_or_competing_terminal_spawn_never_requests_authority() {
    for terminal in [
        OperationState::Rejected,
        OperationState::Failed,
        OperationState::Expired,
        OperationState::Cancelled,
        OperationState::Superseded,
    ] {
        let mut tail = SpawnDescendantTail::new();
        observe_all(
            &mut tail,
            &[
                spawn_event(1),
                transition_event(2, OperationState::Accepted, OperationState::Delivered),
                result_event(3),
                registration_event(4),
                transition_event(5, OperationState::Delivered, terminal),
            ],
        );
        assert_eq!(tail.next_action().unwrap(), None, "terminal={terminal:?}");
    }
}

#[test]
fn legacy_completed_transition_repairs_audit_and_grant_but_not_terminal() {
    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            spawn_event(1),
            transition_event(2, OperationState::Accepted, OperationState::Delivered),
            result_event(3),
            transition_event(4, OperationState::Delivered, OperationState::Completed),
            registration_event(5),
        ],
    );
    let Some(SpawnCompletionAction::RecordAudit(audit)) = tail.next_action().unwrap() else {
        panic!("legacy completion must be repaired");
    };
    assert_eq!(audit.completion_source_event_id, event_id(4));
    tail.observe(&audit_event(6, event_id(4))).unwrap();
    let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) = tail.next_action().unwrap()
    else {
        panic!("legacy completion must issue missing grant");
    };
    tail.observe(&recorded(
        7,
        StoredEventKind::DescendantGrant,
        &descendant_from(&issuance),
    ))
    .unwrap();
    assert_eq!(tail.next_action().unwrap(), None);
}

#[test]
fn wrong_verified_audit_or_conflicting_session_target_fails_closed() {
    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            spawn_event(1),
            transition_event(2, OperationState::Accepted, OperationState::Delivered),
            result_event(3),
            registration_event(4),
        ],
    );
    let mut wrong = audit_event(5, event_id(3));
    let mut record = AuditRecord::decode(wrong.payload.payload.as_slice()).unwrap();
    record.actor_id = Some(actor("spoofed"));
    wrong.payload.payload = record.encode_to_vec();
    tail.observe(&wrong).unwrap();
    assert!(matches!(
        tail.next_action(),
        Err(AuthorityError::CorruptLog(_))
    ));

    let mut conflict = SpawnDescendantTail::new();
    conflict.observe(&registration_event(4)).unwrap();
    let mut bumped = bump_event(5);
    let mut state = SessionStateEvent::decode(bumped.payload.payload.as_slice()).unwrap();
    let Some(session_state_event::Mutation::GenerationBumped(ref mut mutation)) = state.mutation
    else {
        unreachable!()
    };
    mutation.to_generation = Some(Generation { value: 8 });
    bumped.payload.payload = state.encode_to_vec();
    assert!(matches!(
        conflict.observe(&bumped),
        Err(AuthorityError::CorruptLog(_))
    ));
}

#[test]
fn non_qualifying_duplicate_command_correlation_is_inert() {
    let mut duplicate = result_event(3);
    let mut observation = Observation::decode(duplicate.payload.payload.as_slice()).unwrap();
    observation.correlations.push(command_correlation());
    duplicate.payload.payload = observation.encode_to_vec();
    let mut tail = SpawnDescendantTail::new();
    tail.observe(&duplicate).unwrap();
    assert_eq!(tail.next_action().unwrap(), None);
}
