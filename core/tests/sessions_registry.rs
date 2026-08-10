use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, EventId, Generation, Lsn, OperationKind, RuntimeSessionId,
    SecurityLockdownEntered, SecurityLockdownExited, SessionActivityChanged, SessionActivityState,
    SessionConnectivityChanged, SessionConnectivityState, SessionGenerationBumped,
    SessionModelChanged, SessionRegistered, SessionRelabeled, SessionState, StoredEventKind,
    StoredEventPayload, TargetScope,
};
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
fn conflicting_and_unseen_owned_events_reject_without_mutation() {
    let registration_event = recorded(1, &registration());
    let mut applied = SessionRegistry::new(domain()).unwrap();
    applied.observe(&registration_event).unwrap();

    let mut changed_bytes = registration_event.clone();
    changed_bytes.payload.payload.push(0xff);
    let changed_kind = RecordedEvent {
        event_id: registration_event.event_id.clone(),
        payload: StoredEventPayload {
            kind: StoredEventKind::SecurityLockdown as i32,
            payload: Vec::new(),
        },
    };
    for (name, candidate) in [
        ("changed bytes", changed_bytes),
        ("changed owned kind", changed_kind),
    ] {
        let mut registry = applied.clone();
        let before = registry.clone();
        assert!(matches!(
            registry.observe(&candidate),
            Err(SessionError::CorruptLog(_))
        ));
        assert_eq!(registry, before, "{name} mutated the registry");
    }

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
fn exact_security_redelivery_is_inert_even_after_a_later_transition() {
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
    assert!(registry.lockdown_active());

    let exited = recorded_payload(
        domain(),
        3,
        security_events::encode(&security_events::exited(
            domain(),
            SecurityLockdownExited::default(),
        )),
    );
    registry.observe(&exited).unwrap();
    assert!(!registry.lockdown_active());

    let after_exit = registry.clone();
    registry.observe(&entered).unwrap();
    registry.observe(&exited).unwrap();
    assert_eq!(registry, after_exit);
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
