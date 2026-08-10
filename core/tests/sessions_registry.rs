use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, EventId, Generation, Lsn, OperationKind, RuntimeSessionId,
    SecurityLockdownEntered, SecurityLockdownEvent, SecurityLockdownExited, SessionActivityChanged,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState,
    SessionGenerationBumped, SessionModelChanged, SessionRegistered, SessionRelabeled,
    SessionState, StoredEventKind, StoredEventPayload, TargetScope,
};
use prost::Message;

use patchbay_core::{
    acceptance::TargetResolver,
    security::events as security_events,
    session::{events, SessionError, SessionIdentity, SessionRegistry, SessionStateEvent},
    storage::RecordedEvent,
};

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn adapter() -> AdapterId {
    AdapterId {
        value: "pi".to_owned(),
    }
}

fn runtime() -> RuntimeSessionId {
    RuntimeSessionId {
        value: "runtime-1".to_owned(),
    }
}

fn generation(value: u64) -> Generation {
    Generation { value }
}

fn state(connectivity: SessionConnectivityState, activity: SessionActivityState) -> SessionState {
    SessionState {
        connectivity: connectivity as i32,
        activity: activity as i32,
    }
}

fn recorded(lsn: u64, event: &SessionStateEvent) -> RecordedEvent {
    recorded_payload(domain(), lsn, events::encode(event))
}

fn recorded_payload(
    authority_domain_id: AuthorityDomainId,
    lsn: u64,
    payload: StoredEventPayload,
) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(authority_domain_id),
            lsn: Some(Lsn { value: lsn }),
        },
        payload,
    }
}

fn registered_mutation(event: &mut SessionStateEvent) -> &mut SessionRegistered {
    let Some(patchbay_contracts::patchbay::session_state_event::Mutation::Registered(mutation)) =
        event.mutation.as_mut()
    else {
        panic!("registration fixture must contain SessionRegistered");
    };
    mutation
}

fn registration() -> SessionStateEvent {
    events::registered(
        domain(),
        SessionRegistered {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            initial_state: Some(state(
                SessionConnectivityState::Unknown,
                SessionActivityState::Unknown,
            )),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "main".to_owned(),
            model: "provider/model-1".to_owned(),
            spawn_origin: None,
        },
    )
}

fn identity(generation_value: u64) -> SessionIdentity {
    SessionIdentity {
        adapter_id: adapter(),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: runtime(),
        session_generation: generation(generation_value),
    }
}

fn payload_only_redelivery_mutant_accepts(
    applied: &StoredEventPayload,
    candidate: &StoredEventPayload,
) -> bool {
    applied.payload == candidate.payload
}

fn decoded_semantic_redelivery_mutant_accepts(
    applied: &StoredEventPayload,
    candidate: &StoredEventPayload,
) -> bool {
    if applied.kind != candidate.kind {
        return false;
    }
    match StoredEventKind::try_from(applied.kind) {
        Ok(StoredEventKind::SessionState) => {
            SessionStateEvent::decode(applied.payload.as_slice()).unwrap()
                == SessionStateEvent::decode(candidate.payload.as_slice()).unwrap()
        }
        Ok(StoredEventKind::SecurityLockdown) => {
            SecurityLockdownEvent::decode(applied.payload.as_slice()).unwrap()
                == SecurityLockdownEvent::decode(candidate.payload.as_slice()).unwrap()
        }
        _ => false,
    }
}

fn assert_applied_event_rejects_cross_outer_domain(
    mut registry: SessionRegistry,
    applied_event: &RecordedEvent,
) {
    let other = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };

    let exact_state = registry.clone();
    registry
        .observe(applied_event)
        .expect("the fixture must already be an exact applied-event redelivery");
    assert_eq!(
        registry, exact_state,
        "exact redelivery mutated the registry"
    );

    let mut candidate = applied_event.clone();
    candidate.event_id.authority_domain_id = Some(other.clone());
    assert_eq!(
        applied_event.event_id.authority_domain_id.as_ref(),
        Some(&domain())
    );
    assert_eq!(
        candidate.event_id.authority_domain_id.as_ref(),
        Some(&other)
    );
    assert_eq!(candidate.event_id.lsn, applied_event.event_id.lsn);
    assert_eq!(candidate.payload, applied_event.payload);

    let before = registry.clone();
    assert!(matches!(
        registry.observe(&candidate),
        Err(SessionError::AuthorityDomainMismatch { expected, actual })
            if expected == domain() && actual == other
    ));
    assert_eq!(
        registry, before,
        "a cross-outer-domain applied envelope mutated the registry"
    );
}

#[test]
fn registry_requires_and_exposes_a_non_empty_authority_domain() {
    assert!(matches!(
        SessionRegistry::new(AuthorityDomainId {
            value: String::new(),
        }),
        Err(SessionError::EmptyAuthorityDomain)
    ));

    let registry = SessionRegistry::new(domain()).unwrap();
    assert_eq!(registry.authority_domain_id(), &domain());
}

#[test]
fn malformed_owned_event_table_is_exactly_non_mutating() {
    let base = recorded(1, &registration());
    let mut cases = Vec::new();

    let mut candidate = base.clone();
    candidate.event_id.authority_domain_id = None;
    cases.push(("missing outer domain", candidate));

    let mut candidate = base.clone();
    candidate.event_id.authority_domain_id = Some(AuthorityDomainId {
        value: String::new(),
    });
    cases.push(("empty outer domain", candidate));

    let mut candidate = base.clone();
    candidate.event_id.authority_domain_id = Some(AuthorityDomainId {
        value: "authority-other".to_owned(),
    });
    cases.push(("wrong outer domain", candidate));

    let mut candidate = base.clone();
    candidate.event_id.lsn = None;
    cases.push(("missing LSN", candidate));

    let mut source = registration();
    source.authority_domain_id = Some(AuthorityDomainId {
        value: "authority-other".to_owned(),
    });
    cases.push(("inner/outer domain mismatch", recorded(1, &source)));

    let mut source = registration();
    source.mutation = None;
    cases.push(("missing mutation", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source).adapter_id = None;
    cases.push(("missing adapter id", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source).deployment_scope.clear();
    cases.push(("empty deployment scope", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source).runtime_session_id = None;
    cases.push(("missing runtime session id", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source).session_generation = None;
    cases.push(("missing generation", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source).initial_state = None;
    cases.push(("missing initial state", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source)
        .initial_state
        .as_mut()
        .unwrap()
        .connectivity = i32::MAX;
    cases.push(("unknown connectivity", recorded(1, &source)));

    let mut source = registration();
    registered_mutation(&mut source)
        .initial_state
        .as_mut()
        .unwrap()
        .activity = i32::MAX;
    cases.push(("unknown activity", recorded(1, &source)));

    for (name, candidate) in cases {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        let before = registry.clone();
        assert!(registry.observe(&candidate).is_err(), "{name} must reject");
        assert_eq!(registry, before, "{name} mutated the registry");
    }
}

#[test]
fn registration_creates_a_live_session() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let record = registry
        .get_session(&identity(1))
        .expect("registration must create the live generation");
    assert_eq!(
        record.state,
        state(
            SessionConnectivityState::Unknown,
            SessionActivityState::Unknown
        )
    );
    assert_eq!(record.project, "patchbay");
    assert_eq!(record.cwd, "/work/patchbay");
    assert_eq!(record.name, "main");
    assert_eq!(record.model, "provider/model-1");
    assert_eq!(record.last_authoritative_lsn, Some(1));
    assert!(!record.tombstoned);
}

#[tokio::test]
async fn folds_axis_changes_relabel_and_generation_bump() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let connectivity = events::connectivity_changed(
        domain(),
        SessionConnectivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionConnectivityState::Unknown as i32,
            to: SessionConnectivityState::Live as i32,
        },
    );
    registry.observe(&recorded(2, &connectivity)).unwrap();

    let activity = events::activity_changed(
        domain(),
        SessionActivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionActivityState::Unknown as i32,
            to: SessionActivityState::Working as i32,
        },
    );
    registry.observe(&recorded(3, &activity)).unwrap();

    let relabel = events::relabeled(
        domain(),
        SessionRelabeled {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            project: "patchbay-next".to_owned(),
            cwd: "/work/patchbay-next".to_owned(),
            name: "replacement".to_owned(),
        },
    );
    registry.observe(&recorded(4, &relabel)).unwrap();

    let bump = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
            initial_state: Some(state(
                SessionConnectivityState::Live,
                SessionActivityState::Working,
            )),
            project: "patchbay-next".to_owned(),
            cwd: "/work/patchbay-next".to_owned(),
            name: "replacement".to_owned(),
            model: "provider/model-2".to_owned(),
            spawn_origin: None,
        },
    );
    registry.observe(&recorded(5, &bump)).unwrap();

    assert!(registry.get_session(&identity(1)).is_none());
    let live = registry
        .get_session(&identity(2))
        .expect("generation bump must establish the new live generation");
    assert_eq!(live.state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(live.state.activity(), SessionActivityState::Working);
    assert_eq!(live.project, "patchbay-next");
    assert_eq!(live.cwd, "/work/patchbay-next");
    assert_eq!(live.name, "replacement");
    assert_eq!(live.model, "provider/model-2");
    assert_eq!(live.last_authoritative_lsn, Some(5));

    let tombstone = registry
        .get_tombstone(&adapter(), "machine-a", &runtime(), &generation(1))
        .expect("the prior generation must remain queryable");
    assert_eq!(tombstone.superseded_generation, generation(1));
    assert_eq!(tombstone.superseded_at_lsn, 5);
    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));

    let stale_target = TargetScope {
        kind: patchbay_contracts::patchbay::TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime()),
        session_generation: Some(generation(1)),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    };
    assert!(
        TargetResolver::resolve(&registry, &domain(), OperationKind::Instruct, &stale_target,)
            .await
            .is_err()
    );

    let live_target = TargetScope {
        kind: patchbay_contracts::patchbay::TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime()),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    };
    let binding =
        TargetResolver::resolve(&registry, &domain(), OperationKind::Instruct, &live_target)
            .await
            .expect("an unspecified generation binds the live generation");
    assert!(matches!(
        binding,
        patchbay_core::acceptance::TargetBinding::RuntimeSession {
            session_generation,
            ..
        } if session_generation == generation(2)
    ));
}

#[test]
fn generation_bump_without_initial_state_is_corrupt_record() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();
    let bump = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
            initial_state: None,
            project: "new-project".to_owned(),
            cwd: "/work/new".to_owned(),
            name: "new-name".to_owned(),
            model: "provider/model-2".to_owned(),
            spawn_origin: None,
        },
    );

    assert!(matches!(
        registry.observe(&recorded(2, &bump)),
        Err(SessionError::CorruptRecord(message)) if message.contains("missing initial_state")
    ));
    assert!(!registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));
}

#[tokio::test]
async fn tombstones_are_scoped_to_the_full_session_identity() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let adapter_b = AdapterId {
        value: "other-adapter".to_owned(),
    };
    let registration_b = events::registered(
        domain(),
        SessionRegistered {
            adapter_id: Some(adapter_b.clone()),
            deployment_scope: "machine-b".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            initial_state: Some(state(
                SessionConnectivityState::Unknown,
                SessionActivityState::Unknown,
            )),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "other".to_owned(),
            model: "provider/model-other".to_owned(),
            spawn_origin: None,
        },
    );
    let bump_a = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
            initial_state: Some(state(
                SessionConnectivityState::Unknown,
                SessionActivityState::Unknown,
            )),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "main".to_owned(),
            model: "provider/model-2".to_owned(),
            spawn_origin: None,
        },
    );

    registry.observe(&recorded(1, &registration())).unwrap();
    registry.observe(&recorded(2, &registration_b)).unwrap();
    registry.observe(&recorded(3, &bump_a)).unwrap();

    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));
    assert!(!registry.is_tombstoned(&adapter_b, "machine-b", &runtime(), &generation(1)));

    let target_b = TargetScope {
        kind: patchbay_contracts::patchbay::TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter_b.clone()),
        runtime_session_id: Some(runtime()),
        session_generation: Some(generation(1)),
        deployment_scope: "machine-b".to_owned(),
        ..TargetScope::default()
    };
    let binding = TargetResolver::resolve(&registry, &domain(), OperationKind::Instruct, &target_b)
        .await
        .expect("a same-runtime session under another adapter must remain resolvable");
    assert!(matches!(
        binding,
        patchbay_core::acceptance::TargetBinding::RuntimeSession {
            adapter_id,
            session_generation,
            ..
        } if adapter_id == adapter_b && session_generation == generation(1)
    ));
}

#[test]
fn reobserving_a_committed_prefix_is_idempotent() {
    let connectivity = events::connectivity_changed(
        domain(),
        SessionConnectivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionConnectivityState::Unknown as i32,
            to: SessionConnectivityState::Live as i32,
        },
    );
    let relabel = events::relabeled(
        domain(),
        SessionRelabeled {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            project: "renamed".to_owned(),
            cwd: "/work/renamed".to_owned(),
            name: "renamed".to_owned(),
        },
    );
    let model = events::model_changed(
        domain(),
        SessionModelChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: "provider/model-1".to_owned(),
            to: "provider/model-2".to_owned(),
        },
    );
    let bump = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
            initial_state: Some(state(
                SessionConnectivityState::Live,
                SessionActivityState::Unknown,
            )),
            project: "renamed".to_owned(),
            cwd: "/work/renamed".to_owned(),
            name: "renamed".to_owned(),
            model: "provider/model-2".to_owned(),
            spawn_origin: None,
        },
    );
    let activity = events::activity_changed(
        domain(),
        SessionActivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(2)),
            from: SessionActivityState::Unknown as i32,
            to: SessionActivityState::Idle as i32,
        },
    );
    let events = [
        recorded(1, &registration()),
        recorded(2, &connectivity),
        recorded(3, &relabel),
        recorded(4, &model),
        recorded(5, &bump),
        recorded(6, &activity),
    ];

    let mut registry = SessionRegistry::new(domain()).unwrap();
    for event in &events {
        registry.observe(event).unwrap();
    }
    let once = registry.clone();
    for event in &events {
        registry.observe(event).unwrap();
    }

    assert_eq!(registry, once);
}

#[test]
fn exact_envelope_equality_kills_payload_only_and_decoded_semantic_mutants() {
    let registration_event = recorded(1, &registration());
    let mut applied = SessionRegistry::new(domain()).unwrap();
    applied.observe(&registration_event).unwrap();

    let changed_owned_kind = RecordedEvent {
        event_id: registration_event.event_id.clone(),
        payload: StoredEventPayload {
            kind: StoredEventKind::SecurityLockdown as i32,
            payload: registration_event.payload.payload.clone(),
        },
    };
    let changed_to_sibling_kind = RecordedEvent {
        event_id: registration_event.event_id.clone(),
        payload: StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: registration_event.payload.payload.clone(),
        },
    };
    for (name, candidate) in [
        ("another owned kind", &changed_owned_kind),
        ("a sibling kind", &changed_to_sibling_kind),
    ] {
        assert_ne!(candidate.payload.kind, registration_event.payload.kind);
        assert!(
            payload_only_redelivery_mutant_accepts(&registration_event.payload, &candidate.payload),
            "the {name} fixture must expose a payload-only equality mutant"
        );
    }

    let mut semantically_equal_reencoding = registration_event.clone();
    // Unknown field 127 with varint value 1 is valid Protobuf framing. Prost
    // drops it while decoding, so the decoded SessionStateEvent is unchanged.
    semantically_equal_reencoding
        .payload
        .payload
        .extend_from_slice(&[0xf8, 0x07, 0x01]);
    assert_ne!(
        semantically_equal_reencoding.payload.payload,
        registration_event.payload.payload
    );
    assert!(
        decoded_semantic_redelivery_mutant_accepts(
            &registration_event.payload,
            &semantically_equal_reencoding.payload
        ),
        "the valid alternate encoding must expose a decoded-semantic equality mutant"
    );

    for (name, candidate) in [
        ("kind-only change to another owned kind", changed_owned_kind),
        (
            "kind-only change to a sibling kind",
            changed_to_sibling_kind,
        ),
        (
            "bytes-only semantically equal re-encoding",
            semantically_equal_reencoding,
        ),
    ] {
        let mut registry = applied.clone();
        let before = registry.clone();
        assert!(
            matches!(
                registry.observe(&candidate),
                Err(SessionError::CorruptLog(_))
            ),
            "{name} was mistaken for exact redelivery"
        );
        assert_eq!(registry, before, "{name} mutated the registry");
    }
}

#[test]
fn unseen_and_duplicate_owned_events_reject_without_mutation() {
    let registration_event = recorded(1, &registration());
    let mut applied = SessionRegistry::new(domain()).unwrap();
    applied.observe(&registration_event).unwrap();

    let mut high_water = SessionRegistry::new(domain()).unwrap();
    high_water.observe(&recorded(2, &registration())).unwrap();
    let before = high_water.clone();
    assert!(matches!(
        high_water.observe(&recorded(1, &registration())),
        Err(SessionError::CorruptLog(_))
    ));
    assert_eq!(high_water, before, "unseen older event mutated state");

    let mut duplicate = applied;
    let before = duplicate.clone();
    assert!(matches!(
        duplicate.observe(&recorded(2, &registration())),
        Err(SessionError::CorruptLog(_))
    ));
    assert_eq!(duplicate, before, "new-LSN registration mutated state");

    let bump = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
            initial_state: Some(state(
                SessionConnectivityState::Unknown,
                SessionActivityState::Unknown,
            )),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "main".to_owned(),
            model: "provider/model-2".to_owned(),
            spawn_origin: None,
        },
    );
    let mut duplicate_generation = SessionRegistry::new(domain()).unwrap();
    duplicate_generation
        .observe(&recorded(1, &registration()))
        .unwrap();
    duplicate_generation.observe(&recorded(2, &bump)).unwrap();
    let before = duplicate_generation.clone();
    assert!(matches!(
        duplicate_generation.observe(&recorded(3, &bump)),
        Err(SessionError::CorruptLog(_))
    ));
    assert_eq!(duplicate_generation, before);
}

#[test]
fn rejected_event_does_not_claim_its_replay_identity() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let malformed_source = events::connectivity_changed(
        domain(),
        SessionConnectivityChanged {
            adapter_id: None,
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionConnectivityState::Unknown as i32,
            to: SessionConnectivityState::Live as i32,
        },
    );
    let malformed_event = recorded(2, &malformed_source);
    let before = registry.clone();
    assert!(matches!(
        registry.observe(&malformed_event),
        Err(SessionError::CorruptRecord(_))
    ));
    assert_eq!(registry, before);

    let corrected = events::connectivity_changed(
        domain(),
        SessionConnectivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionConnectivityState::Unknown as i32,
            to: SessionConnectivityState::Live as i32,
        },
    );
    registry.observe(&recorded(2, &corrected)).unwrap();
    assert_eq!(
        registry
            .get_session(&identity(1))
            .unwrap()
            .state
            .connectivity(),
        SessionConnectivityState::Live
    );
}

#[test]
fn rejected_new_lsn_semantic_conflicts_are_non_mutating_and_do_not_claim_their_lsn() {
    struct Case {
        name: &'static str,
        registry: SessionRegistry,
        rejected: RecordedEvent,
        corrected: RecordedEvent,
    }

    let generation_bump = |from: u64, to: u64| {
        events::generation_bumped(
            domain(),
            SessionGenerationBumped {
                adapter_id: Some(adapter()),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: Some(runtime()),
                from_generation: Some(generation(from)),
                to_generation: Some(generation(to)),
                initial_state: Some(state(
                    SessionConnectivityState::Unknown,
                    SessionActivityState::Unknown,
                )),
                project: "patchbay".to_owned(),
                cwd: "/work/patchbay".to_owned(),
                name: "main".to_owned(),
                model: format!("provider/model-{to}"),
                spawn_origin: None,
            },
        )
    };
    let connectivity_change =
        |generation_value: u64, from: SessionConnectivityState, to: SessionConnectivityState| {
            events::connectivity_changed(
                domain(),
                SessionConnectivityChanged {
                    adapter_id: Some(adapter()),
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: Some(runtime()),
                    session_generation: Some(generation(generation_value)),
                    from: from as i32,
                    to: to as i32,
                },
            )
        };
    let activity_change = |from: SessionActivityState, to: SessionActivityState| {
        events::activity_changed(
            domain(),
            SessionActivityChanged {
                adapter_id: Some(adapter()),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: Some(runtime()),
                session_generation: Some(generation(1)),
                from: from as i32,
                to: to as i32,
            },
        )
    };

    let stale_generation_delta = {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        registry.observe(&recorded(1, &registration())).unwrap();
        registry
            .observe(&recorded(2, &generation_bump(1, 2)))
            .unwrap();
        assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));
        Case {
            name: "stale/tombstoned generation delta versus live generation",
            registry,
            rejected: recorded(
                3,
                &connectivity_change(
                    1,
                    SessionConnectivityState::Unknown,
                    SessionConnectivityState::Live,
                ),
            ),
            corrected: recorded(
                3,
                &connectivity_change(
                    2,
                    SessionConnectivityState::Unknown,
                    SessionConnectivityState::Live,
                ),
            ),
        }
    };

    let wrong_bump_from_generation = {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        registry.observe(&recorded(1, &registration())).unwrap();
        Case {
            name: "generation bump with wrong from_generation",
            registry,
            rejected: recorded(2, &generation_bump(0, 2)),
            corrected: recorded(2, &generation_bump(1, 2)),
        }
    };

    let false_connectivity_prior = {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        registry.observe(&recorded(1, &registration())).unwrap();
        Case {
            name: "connectivity delta with false prior state",
            registry,
            rejected: recorded(
                2,
                &connectivity_change(
                    1,
                    SessionConnectivityState::Stale,
                    SessionConnectivityState::Live,
                ),
            ),
            corrected: recorded(
                2,
                &connectivity_change(
                    1,
                    SessionConnectivityState::Unknown,
                    SessionConnectivityState::Live,
                ),
            ),
        }
    };

    let false_activity_prior = {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        registry.observe(&recorded(1, &registration())).unwrap();
        Case {
            name: "activity delta with false prior state",
            registry,
            rejected: recorded(
                2,
                &activity_change(SessionActivityState::Idle, SessionActivityState::Working),
            ),
            corrected: recorded(
                2,
                &activity_change(SessionActivityState::Unknown, SessionActivityState::Working),
            ),
        }
    };

    for Case {
        name,
        mut registry,
        rejected,
        corrected,
    } in [
        stale_generation_delta,
        wrong_bump_from_generation,
        false_connectivity_prior,
        false_activity_prior,
    ] {
        assert_eq!(
            rejected.event_id, corrected.event_id,
            "{name} must be repaired at the same durable identity"
        );
        assert_ne!(
            rejected.payload, corrected.payload,
            "{name} must use a genuinely corrected envelope"
        );

        let before = registry.clone();
        assert!(
            matches!(
                registry.observe(&rejected),
                Err(SessionError::CorruptLog(_))
            ),
            "{name} must reject"
        );
        assert_eq!(
            registry, before,
            "{name} mutated state or claimed the rejected LSN"
        );

        registry
            .observe(&corrected)
            .unwrap_or_else(|error| panic!("{name} poisoned corrected same-LSN replay: {error}"));
        assert_ne!(
            registry, before,
            "{name} correction returned success without applying the event"
        );
    }
}

#[test]
fn old_security_redeliveries_do_not_cancel_later_lockdown_posture() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let entered = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                reason_code: "first_entry".to_owned(),
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    registry.observe(&entered).unwrap();
    assert!(registry.lockdown_active());

    let exited = recorded_payload(
        domain(),
        3,
        security_events::encode(&security_events::exited(
            domain(),
            SecurityLockdownExited {
                reason_code: "first_exit".to_owned(),
                ..SecurityLockdownExited::default()
            },
        )),
    );
    registry.observe(&exited).unwrap();
    assert!(!registry.lockdown_active());

    let after_exit = registry.clone();
    registry.observe(&entered).unwrap();
    assert_eq!(
        registry, after_exit,
        "redelivering the old entry must not reactivate lockdown"
    );
    assert!(
        !registry.lockdown_active(),
        "the old entry must remain inert before any compensating redelivery"
    );

    let reentered = recorded_payload(
        domain(),
        4,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                reason_code: "second_entry".to_owned(),
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    registry.observe(&reentered).unwrap();
    assert!(registry.lockdown_active());

    let after_reentry = registry.clone();
    registry.observe(&exited).unwrap();
    assert_eq!(
        registry, after_reentry,
        "redelivering the old exit must not clear the later entry"
    );
    assert!(
        registry.lockdown_active(),
        "the old exit must remain inert before any compensating redelivery"
    );
}

#[test]
fn lockdown_exact_envelope_equality_kills_payload_only_and_decoded_semantic_mutants() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let applied_event = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                reason_code: "exact_envelope_fixture".to_owned(),
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    registry.observe(&applied_event).unwrap();
    let applied = registry.clone();
    registry.observe(&applied_event).unwrap();
    assert_eq!(
        registry, applied,
        "the exact lockdown envelope must be inert"
    );

    let kind_only_change = RecordedEvent {
        event_id: applied_event.event_id.clone(),
        payload: StoredEventPayload {
            kind: StoredEventKind::SessionState as i32,
            payload: applied_event.payload.payload.clone(),
        },
    };
    assert_ne!(kind_only_change.payload.kind, applied_event.payload.kind);
    assert_eq!(
        kind_only_change.payload.payload,
        applied_event.payload.payload
    );
    assert!(
        payload_only_redelivery_mutant_accepts(&applied_event.payload, &kind_only_change.payload),
        "the lockdown kind-only fixture must expose a payload-only equality mutant"
    );

    let mut semantically_equal_reencoding = applied_event.clone();
    // Unknown field 127 with varint value 1 is valid Protobuf framing. Prost
    // drops it while decoding, so the decoded SecurityLockdownEvent is equal.
    semantically_equal_reencoding
        .payload
        .payload
        .extend_from_slice(&[0xf8, 0x07, 0x01]);
    assert_eq!(
        semantically_equal_reencoding.payload.kind,
        applied_event.payload.kind
    );
    assert_ne!(
        semantically_equal_reencoding.payload.payload,
        applied_event.payload.payload
    );
    assert!(
        decoded_semantic_redelivery_mutant_accepts(
            &applied_event.payload,
            &semantically_equal_reencoding.payload,
        ),
        "the valid lockdown re-encoding must expose a decoded-semantic equality mutant"
    );

    for (name, candidate) in [
        ("kind-only same-bytes change", kind_only_change),
        (
            "same-kind decoded-equal unknown-field re-encoding",
            semantically_equal_reencoding,
        ),
    ] {
        let mut candidate_registry = applied.clone();
        let before = candidate_registry.clone();
        assert!(
            matches!(
                candidate_registry.observe(&candidate),
                Err(SessionError::CorruptLog(message))
                    if message.contains("conflicting durable envelopes")
            ),
            "{name} was mistaken for exact lockdown redelivery"
        );
        assert_eq!(
            candidate_registry, before,
            "{name} mutated the lockdown projection"
        );
    }
}

#[test]
fn conflicting_same_lsn_security_envelope_rejects_without_mutation() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let entered = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    registry.observe(&entered).unwrap();

    let conflicting_exit = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::exited(
            domain(),
            SecurityLockdownExited::default(),
        )),
    );
    assert_ne!(entered.payload, conflicting_exit.payload);

    let before = registry.clone();
    assert!(matches!(
        registry.observe(&conflicting_exit),
        Err(SessionError::CorruptLog(message))
            if message.contains("conflicting durable envelopes")
    ));
    assert_eq!(
        registry, before,
        "a conflicting lockdown envelope mutated the registry"
    );
    assert!(registry.lockdown_active());
}

#[test]
fn malformed_security_event_does_not_poison_corrected_same_lsn() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let mut malformed_source = security_events::entered(
        domain(),
        SecurityLockdownEntered {
            affected_runtime_session_count: 1,
            ..SecurityLockdownEntered::default()
        },
    );
    malformed_source.transition = None;
    let malformed = recorded_payload(domain(), 2, security_events::encode(&malformed_source));
    let corrected = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    assert_ne!(malformed.payload, corrected.payload);

    let before = registry.clone();
    assert!(matches!(
        registry.observe(&malformed),
        Err(SessionError::CorruptRecord(message)) if message.contains("has no transition")
    ));
    assert_eq!(
        registry, before,
        "a malformed lockdown event claimed its replay identity"
    );

    registry
        .observe(&corrected)
        .expect("a corrected lockdown envelope must still claim the same LSN");
    assert!(registry.lockdown_active());
    assert_eq!(
        registry
            .get_session(&identity(1))
            .unwrap()
            .last_authoritative_lsn,
        Some(2)
    );
}

#[test]
fn rejected_lockdown_semantics_are_non_mutating_and_do_not_claim_their_lsn() {
    let other_domain = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let inner_domain_conflict = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            other_domain,
            SecurityLockdownEntered {
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    let affected_count_mismatch = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                affected_runtime_session_count: 0,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    let exit_while_inactive = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::exited(
            domain(),
            SecurityLockdownExited::default(),
        )),
    );

    for (name, rejected) in [
        (
            "outer domain correct with conflicting inner domain",
            inner_domain_conflict,
        ),
        (
            "affected runtime-session count mismatch",
            affected_count_mismatch,
        ),
        ("exit while lockdown is inactive", exit_while_inactive),
    ] {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        registry.observe(&recorded(1, &registration())).unwrap();
        let corrected = recorded_payload(
            domain(),
            2,
            security_events::encode(&security_events::entered(
                domain(),
                SecurityLockdownEntered {
                    affected_runtime_session_count: 1,
                    ..SecurityLockdownEntered::default()
                },
            )),
        );
        assert_eq!(
            rejected.event_id.authority_domain_id.as_ref(),
            Some(&domain()),
            "{name} must reach the inner lockdown validator from the correct outer domain"
        );
        assert_eq!(rejected.event_id, corrected.event_id, "{name}");
        assert_ne!(rejected.payload, corrected.payload, "{name}");

        let before = registry.clone();
        assert!(
            matches!(
                registry.observe(&rejected),
                Err(SessionError::CorruptLog(_))
            ),
            "{name} must reject"
        );
        assert_eq!(registry, before, "{name} mutated or claimed its LSN");

        registry
            .observe(&corrected)
            .unwrap_or_else(|error| panic!("{name} poisoned corrected same-LSN replay: {error}"));
        assert!(registry.lockdown_active(), "{name}");
        assert_eq!(
            registry
                .get_session(&identity(1))
                .unwrap()
                .last_authoritative_lsn,
            Some(2),
            "{name}"
        );
    }
}

#[test]
fn applied_session_state_cross_outer_domain_is_not_exact_redelivery() {
    let applied_event = recorded(1, &registration());
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&applied_event).unwrap();

    assert_eq!(
        applied_event.payload.kind,
        StoredEventKind::SessionState as i32
    );
    assert_applied_event_rejects_cross_outer_domain(registry, &applied_event);
}

#[test]
fn applied_security_lockdown_cross_outer_domain_is_not_exact_redelivery() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();
    let applied_event = recorded_payload(
        domain(),
        2,
        security_events::encode(&security_events::entered(
            domain(),
            SecurityLockdownEntered {
                affected_runtime_session_count: 1,
                ..SecurityLockdownEntered::default()
            },
        )),
    );
    registry.observe(&applied_event).unwrap();

    assert_eq!(
        applied_event.payload.kind,
        StoredEventKind::SecurityLockdown as i32
    );
    assert_applied_event_rejects_cross_outer_domain(registry, &applied_event);
}

#[test]
fn cross_domain_session_and_security_events_are_non_mutating() {
    let other = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let mut other_registration = registration();
    other_registration.authority_domain_id = Some(other.clone());
    let session_event = recorded_payload(other.clone(), 1, events::encode(&other_registration));
    let security_event = recorded_payload(
        other.clone(),
        1,
        security_events::encode(&security_events::entered(
            other.clone(),
            SecurityLockdownEntered::default(),
        )),
    );

    for candidate in [session_event, security_event] {
        let mut registry = SessionRegistry::new(domain()).unwrap();
        let before = registry.clone();
        assert!(matches!(
            registry.observe(&candidate),
            Err(SessionError::AuthorityDomainMismatch { expected, actual })
                if expected == domain() && actual == other
        ));
        assert_eq!(registry, before);
    }
}

#[test]
fn disallowed_connectivity_transition_is_corrupt_log() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();
    let invalid = events::connectivity_changed(
        domain(),
        SessionConnectivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionConnectivityState::Unknown as i32,
            to: SessionConnectivityState::Unknown as i32,
        },
    );

    assert!(matches!(
        registry.observe(&recorded(2, &invalid)),
        Err(SessionError::CorruptLog(_))
    ));
}

#[test]
fn disallowed_activity_transition_is_corrupt_log() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();
    let invalid = events::activity_changed(
        domain(),
        SessionActivityChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: SessionActivityState::Unknown as i32,
            to: SessionActivityState::Unknown as i32,
        },
    );

    assert!(matches!(
        registry.observe(&recorded(2, &invalid)),
        Err(SessionError::CorruptLog(_))
    ));
}

#[test]
fn model_change_preserves_identity_and_rejects_mismatched_prior_value() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    registry.observe(&recorded(1, &registration())).unwrap();

    let changed = events::model_changed(
        domain(),
        SessionModelChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: "provider/model-1".to_owned(),
            to: "provider/model-2".to_owned(),
        },
    );
    registry.observe(&recorded(2, &changed)).unwrap();

    let record = registry.get_session(&identity(1)).unwrap();
    assert_eq!(record.model, "provider/model-2");
    assert_eq!(record.identity, identity(1));
    assert_eq!(record.last_authoritative_lsn, Some(2));

    let invalid = events::model_changed(
        domain(),
        SessionModelChanged {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            from: "provider/model-1".to_owned(),
            to: "provider/model-3".to_owned(),
        },
    );
    assert!(matches!(
        registry.observe(&recorded(3, &invalid)),
        Err(SessionError::CorruptLog(message)) if message.contains("expects prior model")
    ));
    assert_eq!(
        registry.get_session(&identity(1)).unwrap().model,
        "provider/model-2"
    );
}

#[test]
fn non_session_events_are_ignored() {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let event = RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain()),
            lsn: Some(Lsn { value: 1 }),
        },
        payload: StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: vec![0xff],
        },
    };

    registry.observe(&event).unwrap();
    assert!(registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .is_none());
}
