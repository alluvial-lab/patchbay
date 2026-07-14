use patchbay_contracts::patchbay::{
    session_state_event, AdapterId, AuthorityDomainId, Generation, Lsn, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionStateEvent, StoredEventKind,
};
use patchbay_core::{
    session::{
        ingest_session_report, rebuild_from_log, IngestResult, SessionError, SessionRegistry,
        SessionReport,
    },
    storage::{RecordedEvent, RusqliteStorage, Storage},
};
use prost::Message;

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

fn report(generation_value: u64) -> SessionReport {
    SessionReport {
        authority_domain_id: domain(),
        adapter_id: adapter(),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: runtime(),
        session_generation: generation(generation_value),
        connectivity: SessionConnectivityState::Unknown,
        activity: SessionActivityState::Unknown,
        project: "patchbay".to_owned(),
        cwd: "/work/patchbay".to_owned(),
        name: "main".to_owned(),
    }
}

async fn events(storage: &RusqliteStorage) -> Vec<RecordedEvent> {
    storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
}

fn decode(event: &RecordedEvent) -> SessionStateEvent {
    assert_eq!(
        StoredEventKind::try_from(event.payload.kind).unwrap(),
        StoredEventKind::SessionState
    );
    SessionStateEvent::decode(event.payload.payload.as_slice()).unwrap()
}

async fn register(
    storage: &RusqliteStorage,
    registry: &mut SessionRegistry,
    initial_report: SessionReport,
) {
    let result = ingest_session_report(storage, registry, initial_report)
        .await
        .unwrap();
    assert!(matches!(result, IngestResult::Registered { .. }));
    let committed = events(storage).await;
    registry.observe(committed.last().unwrap()).unwrap();
}

#[tokio::test]
async fn first_report_writes_registration() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let registry = SessionRegistry::new();

    let result = ingest_session_report(&storage, &registry, report(1))
        .await
        .unwrap();

    let IngestResult::Registered { event_id } = result else {
        panic!("first report must register the session");
    };
    assert_eq!(event_id.authority_domain_id, Some(domain()));
    assert_eq!(event_id.lsn, Some(Lsn { value: 1 }));

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 1);
    let event = decode(&committed[0]);
    assert_eq!(event.authority_domain_id, Some(domain()));
    let Some(session_state_event::Mutation::Registered(registered)) = event.mutation else {
        panic!("expected SessionRegistered mutation");
    };
    assert_eq!(registered.adapter_id, Some(adapter()));
    assert_eq!(registered.deployment_scope, "machine-a");
    assert_eq!(registered.runtime_session_id, Some(runtime()));
    assert_eq!(registered.session_generation, Some(generation(1)));
    assert_eq!(
        registered.initial_state.unwrap().connectivity(),
        SessionConnectivityState::Unknown
    );
}

#[tokio::test]
async fn newer_report_writes_one_generation_bump_and_tombstones_prior_generation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    let mut initial = report(1);
    initial.connectivity = SessionConnectivityState::Offline;
    initial.activity = SessionActivityState::Working;
    initial.project = "old-project".to_owned();
    initial.cwd = "/work/old".to_owned();
    initial.name = "old-name".to_owned();
    register(&storage, &mut registry, initial).await;

    let mut replacement = report(2);
    replacement.connectivity = SessionConnectivityState::Live;
    replacement.activity = SessionActivityState::Idle;
    replacement.project = "new-project".to_owned();
    replacement.cwd = "/work/new".to_owned();
    replacement.name = "new-name".to_owned();
    let result = ingest_session_report(&storage, &registry, replacement)
        .await
        .unwrap();

    let IngestResult::GenerationBumped {
        tombstone_event_id,
        new_generation_event_id,
        from_generation,
        to_generation,
    } = result
    else {
        panic!("newer generation must supersede the live generation");
    };
    assert_eq!(tombstone_event_id, new_generation_event_id);
    assert_eq!(from_generation, generation(1));
    assert_eq!(to_generation, generation(2));

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2, "a bump must append one event");
    let event = decode(&committed[1]);
    let Some(session_state_event::Mutation::GenerationBumped(bump)) = event.mutation else {
        panic!("expected SessionGenerationBumped mutation");
    };
    assert_eq!(bump.from_generation, Some(generation(1)));
    assert_eq!(bump.to_generation, Some(generation(2)));
    let bumped_state = bump.initial_state.expect("bump must carry the new state");
    assert_eq!(bumped_state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(bumped_state.activity(), SessionActivityState::Idle);
    assert_eq!(bump.project, "new-project");
    assert_eq!(bump.cwd, "/work/new");
    assert_eq!(bump.name, "new-name");

    let rebuilt = rebuild_from_log(&storage, &domain()).await.unwrap();
    let tombstone = rebuilt
        .get_tombstone(&adapter(), "machine-a", &runtime(), &generation(1))
        .expect("the bump event must fold into a tombstone");
    assert_eq!(tombstone.superseded_at_lsn, 2);
    let live = rebuilt
        .get_live_session(&adapter(), "machine-a", &runtime())
        .expect("the replacement generation must be live");
    assert_eq!(live.identity.session_generation, generation(2));
    assert_eq!(live.state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(live.state.activity(), SessionActivityState::Idle);
    assert_eq!(live.project, "new-project");
    assert_eq!(live.cwd, "/work/new");
    assert_eq!(live.name, "new-name");
}

#[tokio::test]
async fn equal_generation_connectivity_change_writes_delta() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.connectivity = SessionConnectivityState::Live;

    let result = ingest_session_report(&storage, &registry, changed)
        .await
        .unwrap();

    assert!(matches!(
        result,
        IngestResult::ConnectivityChanged {
            from: SessionConnectivityState::Unknown,
            to: SessionConnectivityState::Live,
            ..
        }
    ));
    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2);
    assert!(matches!(
        decode(&committed[1]).mutation,
        Some(session_state_event::Mutation::ConnectivityChanged(_))
    ));
}

#[tokio::test]
async fn equal_generation_activity_change_writes_delta() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.activity = SessionActivityState::Working;

    let result = ingest_session_report(&storage, &registry, changed)
        .await
        .unwrap();

    assert!(matches!(
        result,
        IngestResult::ActivityChanged {
            from: SessionActivityState::Unknown,
            to: SessionActivityState::Working,
            ..
        }
    ));
    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2);
    assert!(matches!(
        decode(&committed[1]).mutation,
        Some(session_state_event::Mutation::ActivityChanged(_))
    ));
}

#[tokio::test]
async fn equal_generation_metadata_change_writes_relabel() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();

    let result = ingest_session_report(&storage, &registry, changed)
        .await
        .unwrap();

    assert!(matches!(result, IngestResult::Relabeled { .. }));
    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2);
    let Some(session_state_event::Mutation::Relabeled(relabel)) = decode(&committed[1]).mutation
    else {
        panic!("expected SessionRelabeled mutation");
    };
    assert_eq!(relabel.project, "patchbay-next");
    assert_eq!(relabel.cwd, "/work/patchbay-next");
    assert_eq!(relabel.name, "replacement");
}

#[tokio::test]
async fn identical_equal_generation_report_is_idempotent() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(1)).await;

    let result = ingest_session_report(&storage, &registry, report(1))
        .await
        .unwrap();

    assert_eq!(result, IngestResult::NoChange);
    assert_eq!(events(&storage).await.len(), 1);
}

#[tokio::test]
async fn lower_generation_is_rejected_without_writing() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(2)).await;

    let error = ingest_session_report(&storage, &registry, report(1))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SessionError::StaleGeneration { live, reported }
            if live == generation(2) && reported == generation(1)
    ));
    assert_eq!(events(&storage).await.len(), 1);
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .identity
            .session_generation,
        generation(2)
    );
}

#[tokio::test]
async fn one_report_persists_every_changed_axis_and_metadata() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    register(&storage, &mut registry, report(1)).await;

    let mut changed = report(1);
    changed.connectivity = SessionConnectivityState::Live;
    changed.activity = SessionActivityState::Working;
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();

    let result = ingest_session_report(&storage, &registry, changed)
        .await
        .unwrap();
    let IngestResult::DeltasApplied { event_ids } = result else {
        panic!("a multi-field report must return the combined result");
    };
    assert_eq!(event_ids.len(), 3);

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 4);
    assert!(matches!(
        decode(&committed[1]).mutation,
        Some(session_state_event::Mutation::ConnectivityChanged(_))
    ));
    assert!(matches!(
        decode(&committed[2]).mutation,
        Some(session_state_event::Mutation::ActivityChanged(_))
    ));
    assert!(matches!(
        decode(&committed[3]).mutation,
        Some(session_state_event::Mutation::Relabeled(_))
    ));

    let rebuilt = rebuild_from_log(&storage, &domain()).await.unwrap();
    let live = rebuilt
        .get_live_session(&adapter(), "machine-a", &runtime())
        .unwrap();
    assert_eq!(live.state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(live.state.activity(), SessionActivityState::Working);
    assert_eq!(live.project, "patchbay-next");
    assert_eq!(live.cwd, "/work/patchbay-next");
    assert_eq!(live.name, "replacement");
}

#[tokio::test]
async fn empty_identity_fields_are_rejected_before_write_and_valid_reports_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let registry = SessionRegistry::new();

    let mut empty_adapter = report(1);
    empty_adapter.adapter_id.value.clear();
    let mut empty_scope = report(1);
    empty_scope.deployment_scope.clear();
    let mut empty_runtime = report(1);
    empty_runtime.runtime_session_id.value.clear();

    for (field, invalid) in [
        ("adapter_id", empty_adapter),
        ("deployment_scope", empty_scope),
        ("runtime_session_id", empty_runtime),
    ] {
        let error = ingest_session_report(&storage, &registry, invalid)
            .await
            .unwrap_err();
        assert!(
            matches!(error, SessionError::CorruptRecord(message) if message.contains(field)),
            "expected an empty {field} error"
        );
        assert!(events(&storage).await.is_empty());
    }

    ingest_session_report(&storage, &registry, report(1))
        .await
        .expect("a valid report must be accepted");
    let rebuilt = rebuild_from_log(&storage, &domain())
        .await
        .expect("every accepted report must produce a replayable log");
    assert!(rebuilt
        .get_live_session(&adapter(), "machine-a", &runtime())
        .is_some());
}

#[tokio::test]
async fn empty_authority_domain_is_rejected_before_lookup_or_write() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let registry = SessionRegistry::new();
    let mut invalid = report(1);
    invalid.authority_domain_id.value.clear();

    let error = ingest_session_report(&storage, &registry, invalid)
        .await
        .unwrap_err();

    assert!(
        matches!(error, SessionError::CorruptRecord(message) if message.contains("authority_domain_id is empty"))
    );
    assert!(events(&storage).await.is_empty());
}

#[tokio::test]
async fn disallowed_axis_transition_is_rejected_before_writing() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new();
    let mut initial = report(1);
    initial.connectivity = SessionConnectivityState::Live;
    register(&storage, &mut registry, initial).await;
    let mut invalid = report(1);
    invalid.connectivity = SessionConnectivityState::Unknown;

    let error = ingest_session_report(&storage, &registry, invalid)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SessionError::InvalidTransition { from, to }
            if from == "Live" && to == "Unknown"
    ));
    assert_eq!(events(&storage).await.len(), 1);
}
