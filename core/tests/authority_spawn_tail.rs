use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterId, AuditEventKind, AuditRecord, AuthorityDomainId, CommandId, CommandTransition,
    DescendantGrant, DescendantGrantProvenance, DeviceId, EndpointId, EventId, FailureCode,
    Generation, Grant, GrantId, GrantProvenance, GrantRevocationEffect, GrantRevocationPolicy, Lsn,
    Observation, ObservationKind, Operation, OperationKind, OperationState, Revocation,
    RuntimeSessionId, SessionGenerationBumped, SessionRegistered, SessionStateEvent,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    acceptance::CommandIndex,
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

fn parent_grant_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventKind::Grant,
        &Grant {
            grant_id: Some(grant_id("spawn-grant")),
            authority_domain_id: Some(domain("authority-main")),
            subject_actor_id: Some(actor("operator")),
            subject_endpoint_id: Some(endpoint("browser")),
            target_scope: Some(spawn_target()),
            allowed_operation_kinds: vec![OperationKind::Spawn as i32],
            provenance: Some(GrantProvenance {
                reason: "spawn authority fixture".into(),
                ..GrantProvenance::default()
            }),
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..Grant::default()
        },
    )
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
                idempotency_key: "spawn-1-key".into(),
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
    for session_event in [registration_event(5), bump_event(5)] {
        let mut tail = SpawnDescendantTail::new();
        observe_all(
            &mut tail,
            &[
                parent_grant_event(1),
                spawn_event(2),
                transition_event(3, OperationState::Accepted, OperationState::Delivered),
                result_event(4),
                session_event,
            ],
        );

        let SpawnCompletionAction::RecordAudit(audit) = tail.next_action().unwrap().unwrap() else {
            panic!("audit must be first");
        };
        assert_eq!(audit.completion_source_event_id, event_id(4));
        assert_eq!(audit.spawning_grant_id, grant_id("spawn-grant"));
        assert_eq!(audit.subject_actor_id, actor("operator"));
        assert_eq!(audit.subject_endpoint_id, Some(endpoint("browser")));
        assert_eq!(audit.subject_device_id, Some(device("laptop")));
        assert_eq!(audit.spawned_session_scope, session_target());

        tail.observe(&audit_event(6, event_id(4))).unwrap();
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
        assert_eq!(issuance.audit_id, event_id(6));

        tail.observe(&recorded(
            7,
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
fn relevant_fact_lsn_orders_converge_and_durable_progress_survives_redelivery() {
    for facts in [
        vec![
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            result_event(4),
            registration_event(5),
        ],
        vec![
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            registration_event(4),
            result_event(5),
        ],
    ] {
        let mut tail = SpawnDescendantTail::new();
        observe_all(&mut tail, &facts);
        assert!(matches!(
            tail.next_action().unwrap(),
            Some(SpawnCompletionAction::RecordAudit(_))
        ));
        let result_lsn = facts
            .iter()
            .find(|event| event.payload.kind == StoredEventKind::Observation as i32)
            .and_then(|event| event.event_id.lsn)
            .unwrap()
            .value;
        let audit = audit_event(6, event_id(result_lsn));
        tail.observe(&audit).unwrap();
        let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) =
            tail.next_action().unwrap()
        else {
            panic!("expected grant action");
        };
        let grant = recorded(
            7,
            StoredEventKind::DescendantGrant,
            &descendant_from(&issuance),
        );
        tail.observe(&grant).unwrap();
        tail.observe(&audit).unwrap();
        tail.observe(&grant).unwrap();
        assert!(matches!(
            tail.next_action().unwrap(),
            Some(SpawnCompletionAction::CommitCompleted(_))
        ));
    }
}

#[test]
fn byte_identical_legacy_descendant_redelivery_is_inert_but_changed_bytes_corrupt() {
    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            result_event(4),
            registration_event(5),
            audit_event(6, event_id(4)),
        ],
    );
    let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) =
        tail.next_action().unwrap()
    else {
        panic!("expected grant action");
    };
    let source = recorded(
        7,
        StoredEventKind::DescendantGrant,
        &descendant_from(&issuance),
    );
    tail.observe(&source).unwrap();

    let mut exact_legacy_redelivery = source.clone();
    exact_legacy_redelivery.event_id = event_id(8);
    tail.observe(&exact_legacy_redelivery)
        .expect("a byte-identical later source is legacy redelivery");
    assert!(matches!(
        tail.next_action().unwrap(),
        Some(SpawnCompletionAction::CommitCompleted(_))
    ));

    let mut changed_bytes = source;
    changed_bytes.event_id = event_id(9);
    // Append a valid unknown protobuf field. The decoded grant is unchanged,
    // so this catches implementations that compare only semantic fields rather
    // than the canonical stored source envelope.
    changed_bytes.payload.payload.extend_from_slice(&[0xf8, 0x07, 0x01]);
    assert!(matches!(
        tail.observe(&changed_bytes),
        Err(AuthorityError::CorruptLog(_))
    ));
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
                parent_grant_event(1),
                spawn_event(2),
                transition_event(3, OperationState::Accepted, OperationState::Delivered),
                result_event(4),
                registration_event(5),
                transition_event(6, OperationState::Delivered, terminal),
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
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            result_event(4),
            transition_event(5, OperationState::Delivered, OperationState::Completed),
            registration_event(6),
        ],
    );
    let Some(SpawnCompletionAction::RecordAudit(audit)) = tail.next_action().unwrap() else {
        panic!("legacy completion must be repaired");
    };
    assert_eq!(audit.completion_source_event_id, event_id(5));
    tail.observe(&audit_event(7, event_id(5))).unwrap();
    let Some(SpawnCompletionAction::IssueDescendantGrant(issuance)) = tail.next_action().unwrap()
    else {
        panic!("legacy completion must issue missing grant");
    };
    tail.observe(&recorded(
        8,
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
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            result_event(4),
            registration_event(5),
        ],
    );
    let mut wrong = audit_event(6, event_id(4));
    let mut record = AuditRecord::decode(wrong.payload.payload.as_slice()).unwrap();
    record.actor_id = Some(actor("spoofed"));
    wrong.payload.payload = record.encode_to_vec();
    assert!(matches!(
        tail.observe(&wrong),
        Err(AuthorityError::CorruptLog(_))
    ));

    let mut conflict = SpawnDescendantTail::new();
    observe_all(
        &mut conflict,
        &[
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            registration_event(4),
        ],
    );
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
fn accepted_state_and_preseed_success_never_arm_descendant_authority() {
    let mut accepted_only = SpawnDescendantTail::new();
    observe_all(
        &mut accepted_only,
        &[
            parent_grant_event(1),
            spawn_event(2),
            result_event(3),
            transition_event(4, OperationState::Accepted, OperationState::Delivered),
            registration_event(5),
        ],
    );
    assert_eq!(accepted_only.next_action().unwrap(), None);

    let mut preseed = SpawnDescendantTail::new();
    preseed.observe(&result_event(1)).unwrap();
    assert_eq!(preseed.next_action().unwrap(), None);
}

#[test]
fn revocation_command_effect_terminal_suppresses_issuance_after_staged_audit() {
    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            parent_grant_event(1),
            spawn_event(2),
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
            result_event(4),
            registration_event(5),
            audit_event(6, event_id(4)),
        ],
    );
    assert!(matches!(
        tail.next_action().unwrap(),
        Some(SpawnCompletionAction::IssueDescendantGrant(_))
    ));
    tail.observe(&recorded(
        7,
        StoredEventKind::Revocation,
        &Revocation {
            authority_domain_id: Some(domain("authority-main")),
            grant_id: Some(grant_id("spawn-grant")),
            revocation_generation: Some(Generation { value: 1 }),
            accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
            command_effects: vec![GrantRevocationEffect {
                command_id: Some(command("spawn-1")),
                from_state: OperationState::Delivered as i32,
                to_state: OperationState::Cancelled as i32,
                failure_code: FailureCode::Cancelled as i32,
            }],
            ..Revocation::default()
        },
    ))
    .unwrap();
    assert_eq!(tail.next_action().unwrap(), None);
}

#[test]
fn require_reauthorization_effect_wins_before_success_evidence() {
    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            parent_grant_event(1),
            spawn_event(2),
            recorded(
                3,
                StoredEventKind::Revocation,
                &Revocation {
                    authority_domain_id: Some(domain("authority-main")),
                    grant_id: Some(grant_id("spawn-grant")),
                    revocation_generation: Some(Generation { value: 1 }),
                    accepted_operation_policy: GrantRevocationPolicy::RequireReauthorization as i32,
                    command_effects: vec![GrantRevocationEffect {
                        command_id: Some(command("spawn-1")),
                        from_state: OperationState::Accepted as i32,
                        to_state: OperationState::Rejected as i32,
                        failure_code: FailureCode::AuthorizationDenied as i32,
                    }],
                    ..Revocation::default()
                },
            ),
            result_event(4),
            registration_event(5),
        ],
    );
    assert_eq!(tail.next_action().unwrap(), None);
}

#[test]
fn adapter_scoped_spawn_rejects_cross_adapter_registration() {
    let mut accepted = spawn_event(2);
    let mut accepted_message =
        AcceptedOperation::decode(accepted.payload.payload.as_slice()).unwrap();
    accepted_message.operation.as_mut().unwrap().target_scope = Some(TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId { value: "pi".into() }),
        ..TargetScope::default()
    });
    accepted.payload.payload = accepted_message.encode_to_vec();

    let mut cross_adapter = registration_event(4);
    let mut session = SessionStateEvent::decode(cross_adapter.payload.payload.as_slice()).unwrap();
    let Some(session_state_event::Mutation::Registered(registered)) = session.mutation.as_mut()
    else {
        unreachable!()
    };
    registered.adapter_id = Some(AdapterId {
        value: "other-adapter".into(),
    });
    cross_adapter.payload.payload = session.encode_to_vec();

    let mut tail = SpawnDescendantTail::new();
    observe_all(
        &mut tail,
        &[
            parent_grant_event(1),
            accepted,
            transition_event(3, OperationState::Accepted, OperationState::Delivered),
        ],
    );
    assert!(matches!(
        tail.observe(&cross_adapter),
        Err(AuthorityError::CorruptLog(_))
    ));
}

#[test]
fn restart_uses_one_exact_duplicate_correlation_qualification_for_redelivery_and_completion() {
    let mut duplicate = result_event(4);
    let mut observation = Observation::decode(duplicate.payload.payload.as_slice()).unwrap();
    observation.correlations.push(command_correlation());
    duplicate.payload.payload = observation.encode_to_vec();
    let events = [
        parent_grant_event(1),
        spawn_event(2),
        transition_event(3, OperationState::Accepted, OperationState::Delivered),
        duplicate,
        registration_event(5),
    ];

    // Fresh projections over the same durable restart prefix must agree: the
    // successful result both suppresses unsafe redelivery and remains eligible
    // for the descendant-completion writer. A correlation cannot qualify for
    // only one side of that decision.
    let mut commands = CommandIndex::new();
    let mut tail = SpawnDescendantTail::new();
    for event in &events {
        commands.apply(event).unwrap();
        tail.observe(event).unwrap();
    }
    assert!(commands.has_deferred_spawn_success(&command("spawn-1")));
    assert!(matches!(
        tail.next_action().unwrap(),
        Some(SpawnCompletionAction::RecordAudit(_))
    ));
}
