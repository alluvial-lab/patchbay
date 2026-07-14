use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, EventId, Generation, Lsn, RuntimeSessionId,
    SessionActivityChanged, SessionActivityState, SessionConnectivityChanged,
    SessionConnectivityState, SessionGenerationBumped, SessionRegistered, SessionRelabeled,
    SessionState, StoredEventKind, StoredEventPayload, TargetScope,
};
use patchbay_core::{
    acceptance::TargetResolver,
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
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain()),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: events::encode(event),
    }
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
fn registration_creates_a_live_session() {
    let mut registry = SessionRegistry::new();
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
    assert_eq!(record.last_authoritative_lsn, Some(1));
    assert!(!record.tombstoned);
}

#[test]
fn folds_axis_changes_relabel_and_generation_bump() {
    let mut registry = SessionRegistry::new();
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
    assert_eq!(live.last_authoritative_lsn, Some(5));

    let tombstone = registry
        .get_tombstone(&adapter(), "machine-a", &runtime(), &generation(1))
        .expect("the prior generation must remain queryable");
    assert_eq!(tombstone.superseded_generation, generation(1));
    assert_eq!(tombstone.superseded_at_lsn, 5);
    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));

    let stale_target = TargetScope {
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime()),
        session_generation: Some(generation(1)),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    };
    assert_eq!(registry.resolve(&stale_target), None);

    let live_target = TargetScope {
        adapter_id: Some(adapter()),
        runtime_session_id: Some(runtime()),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    };
    let binding = registry
        .resolve(&live_target)
        .expect("an unspecified generation binds the live generation");
    assert_eq!(binding.session_generation, generation(2));
}

#[tokio::test]
async fn tombstones_are_scoped_to_the_full_session_identity() {
    let mut registry = SessionRegistry::new();
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
        },
    );

    registry.observe(&recorded(1, &registration())).unwrap();
    registry.observe(&recorded(2, &registration_b)).unwrap();
    registry.observe(&recorded(3, &bump_a)).unwrap();

    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));
    assert!(!registry.is_tombstoned(&adapter_b, "machine-b", &runtime(), &generation(1)));

    let target_b = TargetScope {
        adapter_id: Some(adapter_b.clone()),
        runtime_session_id: Some(runtime()),
        session_generation: Some(generation(1)),
        deployment_scope: "machine-b".to_owned(),
        ..TargetScope::default()
    };
    let binding = TargetResolver::resolve(&registry, &domain(), &target_b)
        .await
        .expect("a same-runtime session under another adapter must remain resolvable");
    assert_eq!(binding.adapter_id, adapter_b);
    assert_eq!(binding.session_generation, generation(1));
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
    let bump = events::generation_bumped(
        domain(),
        SessionGenerationBumped {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            from_generation: Some(generation(1)),
            to_generation: Some(generation(2)),
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
        recorded(4, &bump),
        recorded(5, &activity),
    ];

    let mut registry = SessionRegistry::new();
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
fn disallowed_connectivity_transition_is_corrupt_log() {
    let mut registry = SessionRegistry::new();
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
    let mut registry = SessionRegistry::new();
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
fn non_session_events_are_ignored() {
    let mut registry = SessionRegistry::new();
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
