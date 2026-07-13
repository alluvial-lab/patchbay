use patchbay_contracts::patchbay::{
    session_state_event, AdapterId, AuthorityDomainId, Generation, Lsn, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionStateEvent, StoredEventKind,
};
use patchbay_core::{
    session::{ingest_session_report, IngestResult, SessionError, SessionRegistry, SessionReport},
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
    register(&storage, &mut registry, report(1)).await;

    let result = ingest_session_report(&storage, &registry, report(2))
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

    registry.observe(&committed[1]).unwrap();
    let tombstone = registry
        .get_tombstone(&runtime(), &generation(1))
        .expect("the bump event must fold into a tombstone");
    assert_eq!(tombstone.superseded_at_lsn, 2);
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
