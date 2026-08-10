use std::sync::atomic::{AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AdapterId, AuthorityDomainId, CommandId, EventId,
    Generation, IdempotencyKey, Lsn, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState, SessionStateEvent, StoredEventKind, StoredEventPayload,
    TypedCorrelation,
};
use patchbay_core::{
    session::{
        ingest_session_report, mark_adapter_sessions_stale, rebuild_from_log, IngestResult,
        SessionError, SessionRegistry, SessionReport,
    },
    storage::{
        DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot,
        TargetKey,
    },
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
        model: "provider/model-1".to_owned(),
        spawn_origin: None,
    }
}

async fn events<S: Storage>(storage: &S) -> Vec<RecordedEvent> {
    events_in(storage, &domain()).await
}

async fn events_in<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Vec<RecordedEvent> {
    storage
        .read_after(authority_domain_id, Lsn { value: 0 })
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

async fn register<S: Storage>(
    storage: &S,
    registry: &mut SessionRegistry,
    initial_report: SessionReport,
) {
    let expected_adapter = initial_report.adapter_id.clone();
    let expected_scope = initial_report.deployment_scope.clone();
    let expected_runtime = initial_report.runtime_session_id.clone();
    let expected_generation = initial_report.session_generation;
    let result = ingest_session_report(storage, &mut *registry, initial_report)
        .await
        .unwrap();
    assert!(matches!(result, IngestResult::Registered { .. }));
    let live = registry
        .get_live_session(&expected_adapter, &expected_scope, &expected_runtime)
        .expect("successful registration must warm the supplied registry");
    assert_eq!(live.identity.session_generation, expected_generation);
}

/// A storage adapter that fails a configured append before delegating it.
///
/// The successful appends remain in the wrapped durable log, reproducing a
/// transient failure between sequential session deltas.
struct FailOnNthAppendStorage {
    inner: RusqliteStorage,
    append_count: AtomicUsize,
    fail_on_append: AtomicUsize,
}

impl FailOnNthAppendStorage {
    fn new() -> Self {
        Self {
            inner: RusqliteStorage::open_in_memory().unwrap(),
            append_count: AtomicUsize::new(0),
            fail_on_append: AtomicUsize::new(0),
        }
    }

    fn fail_on_append(&self, nth: usize) {
        self.append_count.store(0, Ordering::SeqCst);
        self.fail_on_append.store(nth, Ordering::SeqCst);
    }

    fn recover(&self) {
        self.fail_on_append.store(0, Ordering::SeqCst);
    }
}

impl Storage for FailOnNthAppendStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        let append_number = self.append_count.fetch_add(1, Ordering::SeqCst) + 1;
        if append_number == self.fail_on_append.load(Ordering::SeqCst) {
            return Err(StorageError::WriteFailed {
                message: "injected append failure".to_owned(),
                retryable: true,
            });
        }
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

#[tokio::test]
async fn first_report_writes_registration() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();

    let result = ingest_session_report(&storage, &mut registry, report(1))
        .await
        .unwrap();

    let IngestResult::Registered { event_id } = result else {
        panic!("first report must register the session");
    };
    assert_eq!(event_id.authority_domain_id, Some(domain()));
    assert_eq!(event_id.lsn, Some(Lsn { value: 1 }));
    let hot = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .expect("registered result must be immediately visible");
    assert_eq!(hot.identity.session_generation, generation(1));
    assert_eq!(hot.model, "provider/model-1");

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
    assert_eq!(registered.model, "provider/model-1");
    assert_eq!(
        registered.initial_state.unwrap().connectivity(),
        SessionConnectivityState::Unknown
    );
}

#[tokio::test]
async fn malformed_spawn_origin_is_rejected_before_session_append() {
    for origin in [
        TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
                value: String::new(),
            })),
        },
        TypedCorrelation { r#ref: None },
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut registry = SessionRegistry::new(domain()).unwrap();
        let mut candidate = report(1);
        candidate.spawn_origin = Some(origin);
        let error = ingest_session_report(&storage, &mut registry, candidate)
            .await
            .expect_err("malformed spawn_origin must fail before durability");
        assert!(matches!(error, SessionError::CorruptRecord(_)));
        assert!(events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn newer_report_writes_one_generation_bump_and_tombstones_prior_generation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
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
    replacement.spawn_origin = Some(TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
            value: "spawn-replacement".to_owned(),
        })),
    });
    let result = ingest_session_report(&storage, &mut registry, replacement)
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
    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(1)));
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .identity
            .session_generation,
        generation(2),
        "generation result must be immediately visible"
    );

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
    assert_eq!(bump.model, "provider/model-1");
    assert_eq!(
        bump.spawn_origin,
        Some(TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
                value: "spawn-replacement".to_owned(),
            })),
        })
    );

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
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.connectivity = SessionConnectivityState::Live;

    let result = ingest_session_report(&storage, &mut registry, changed)
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
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .state
            .connectivity(),
        SessionConnectivityState::Live
    );
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
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.activity = SessionActivityState::Working;

    let result = ingest_session_report(&storage, &mut registry, changed)
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
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .state
            .activity(),
        SessionActivityState::Working
    );
    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2);
    assert!(matches!(
        decode(&committed[1]).mutation,
        Some(session_state_event::Mutation::ActivityChanged(_))
    ));
}

#[tokio::test]
async fn equal_generation_model_change_writes_delta_and_rebuilds() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.model = "provider/model-2".to_owned();

    let result = ingest_session_report(&storage, &mut registry, changed)
        .await
        .unwrap();
    assert!(matches!(
        result,
        IngestResult::ModelChanged { ref from, ref to, .. }
            if from == "provider/model-1" && to == "provider/model-2"
    ));
    let committed = events(&storage).await;
    assert!(matches!(
        decode(&committed[1]).mutation,
        Some(session_state_event::Mutation::ModelChanged(_))
    ));
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .model,
        "provider/model-2",
        "model result must be immediately visible"
    );
    let warm = registry.clone();
    registry.observe(&committed[1]).unwrap();
    assert_eq!(registry, warm, "exact caller redelivery must be inert");
    assert_eq!(
        rebuild_from_log(&storage, &domain())
            .await
            .unwrap()
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .model,
        "provider/model-2"
    );
}

#[tokio::test]
async fn equal_generation_metadata_change_writes_relabel() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;
    let mut changed = report(1);
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();

    let result = ingest_session_report(&storage, &mut registry, changed)
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
    let hot = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .unwrap();
    assert_eq!(hot.project, "patchbay-next");
    assert_eq!(hot.cwd, "/work/patchbay-next");
    assert_eq!(hot.name, "replacement");
}

#[tokio::test]
async fn identical_equal_generation_report_is_idempotent() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;

    let result = ingest_session_report(&storage, &mut registry, report(1))
        .await
        .unwrap();

    assert_eq!(result, IngestResult::NoChange);
    assert_eq!(events(&storage).await.len(), 1);
}

#[tokio::test]
async fn lower_generation_is_rejected_without_writing() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(2)).await;

    let error = ingest_session_report(&storage, &mut registry, report(1))
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
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;

    let mut changed = report(1);
    changed.connectivity = SessionConnectivityState::Live;
    changed.activity = SessionActivityState::Working;
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();
    changed.model = "provider/model-2".to_owned();

    let result = ingest_session_report(&storage, &mut registry, changed)
        .await
        .unwrap();
    let IngestResult::DeltasApplied { event_ids } = result else {
        panic!("a multi-field report must return the combined result");
    };
    assert_eq!(event_ids.len(), 4);

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 5);
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
        Some(session_state_event::Mutation::ModelChanged(_))
    ));
    assert!(matches!(
        decode(&committed[4]).mutation,
        Some(session_state_event::Mutation::Relabeled(_))
    ));

    let rebuilt = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert_eq!(
        registry, rebuilt,
        "hot multi-delta fold must equal cold replay"
    );
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
async fn multi_delta_retry_after_partial_failure_warms_registry_and_replays() {
    let storage = FailOnNthAppendStorage::new();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1)).await;

    let mut changed = report(1);
    changed.connectivity = SessionConnectivityState::Live;
    changed.activity = SessionActivityState::Working;
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();
    changed.model = "provider/model-2".to_owned();

    storage.fail_on_append(2);
    let error = ingest_session_report(&storage, &mut registry, changed.clone())
        .await
        .expect_err("the injected second append must fail");
    assert!(matches!(
        error,
        SessionError::Storage(StorageError::WriteFailed { .. })
    ));

    let partially_applied = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .expect("the first committed delta must warm the hot projection");
    assert_eq!(
        partially_applied.state.connectivity(),
        SessionConnectivityState::Live
    );
    assert_eq!(
        partially_applied.state.activity(),
        SessionActivityState::Unknown,
        "the failed activity delta must not be projected"
    );
    assert_eq!(
        events(&storage).await.len(),
        2,
        "only registration and connectivity persist"
    );

    storage.recover();
    let retry = ingest_session_report(&storage, &mut registry, changed)
        .await
        .expect("retry must append only the remaining deltas");
    let IngestResult::DeltasApplied { event_ids } = retry else {
        panic!("retry must apply the remaining activity, model, and metadata deltas");
    };
    assert_eq!(event_ids.len(), 3);

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 5, "retry must not duplicate connectivity");
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
        Some(session_state_event::Mutation::ModelChanged(_))
    ));
    assert!(matches!(
        decode(&committed[4]).mutation,
        Some(session_state_event::Mutation::Relabeled(_))
    ));

    let rebuilt = rebuild_from_log(&storage, &domain())
        .await
        .expect("the partial-failure retry log must remain replayable");
    let live = rebuilt
        .get_live_session(&adapter(), "machine-a", &runtime())
        .expect("replay must restore the live session");
    assert_eq!(live.state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(live.state.activity(), SessionActivityState::Working);
    assert_eq!(live.project, "patchbay-next");
    assert_eq!(live.cwd, "/work/patchbay-next");
    assert_eq!(live.name, "replacement");
}

#[tokio::test]
async fn post_commit_fold_failure_is_fail_closed_and_requires_rebuild() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let existing = patchbay_core::session::events::registered(
        domain(),
        patchbay_contracts::patchbay::SessionRegistered {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(runtime()),
            session_generation: Some(generation(1)),
            initial_state: Some(patchbay_contracts::patchbay::SessionState {
                connectivity: SessionConnectivityState::Unknown as i32,
                activity: SessionActivityState::Unknown as i32,
            }),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "main".to_owned(),
            model: "provider/model-1".to_owned(),
            spawn_origin: None,
        },
    );
    registry
        .observe(&RecordedEvent {
            event_id: EventId {
                authority_domain_id: Some(domain()),
                lsn: Some(Lsn { value: 10 }),
            },
            payload: patchbay_core::session::events::encode(&existing),
        })
        .unwrap();
    let before = registry.clone();
    let mut candidate = report(1);
    candidate.runtime_session_id = RuntimeSessionId {
        value: "runtime-2".to_owned(),
    };

    let error = ingest_session_report(&storage, &mut registry, candidate)
        .await
        .expect_err("the stale projection must reject the committed older identity");

    assert!(matches!(error, SessionError::CorruptLog(_)));
    assert_eq!(registry, before, "failed fold must not partially mutate");
    assert_eq!(
        events(&storage).await.len(),
        1,
        "the append committed before the fold failed"
    );
    let rebuilt = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert!(rebuilt
        .get_live_session(
            &adapter(),
            "machine-a",
            &RuntimeSessionId {
                value: "runtime-2".to_owned(),
            },
        )
        .is_some());
}

#[tokio::test]
async fn empty_identity_fields_are_rejected_before_write_and_valid_reports_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();

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
        let error = ingest_session_report(&storage, &mut registry, invalid)
            .await
            .unwrap_err();
        assert!(
            matches!(error, SessionError::CorruptRecord(message) if message.contains(field)),
            "expected an empty {field} error"
        );
        assert!(events(&storage).await.is_empty());
    }

    ingest_session_report(&storage, &mut registry, report(1))
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
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let mut invalid = report(1);
    invalid.authority_domain_id.value.clear();

    let error = ingest_session_report(&storage, &mut registry, invalid)
        .await
        .unwrap_err();

    assert!(
        matches!(error, SessionError::CorruptRecord(message) if message.contains("authority_domain_id is empty"))
    );
    assert!(events(&storage).await.is_empty());
}

#[tokio::test]
async fn cross_domain_report_rejects_before_lookup_mutation_or_either_log_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let other_domain = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let mut candidate = report(1);
    candidate.authority_domain_id = other_domain.clone();
    let before = registry.clone();

    let error = ingest_session_report(&storage, &mut registry, candidate)
        .await
        .expect_err("a report cannot cross the registry domain");

    assert!(matches!(
        error,
        SessionError::AuthorityDomainMismatch { expected, actual }
            if expected == domain() && actual == other_domain
    ));
    assert_eq!(registry, before);
    assert!(events_in(&storage, &domain()).await.is_empty());
    assert!(events_in(&storage, &other_domain).await.is_empty());
}

#[tokio::test]
async fn cross_domain_adapter_stale_reconciliation_appends_to_neither_domain() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    ingest_session_report(&storage, &mut registry, report(1))
        .await
        .unwrap();
    let other_domain = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let registry_before = registry.clone();
    let main_before = events_in(&storage, &domain()).await;
    let other_before = events_in(&storage, &other_domain).await;

    let error = mark_adapter_sessions_stale(&storage, &mut registry, &other_domain, &adapter())
        .await
        .expect_err("adapter stale reconciliation cannot cross the registry domain");

    assert!(matches!(
        error,
        SessionError::AuthorityDomainMismatch { expected, actual }
            if expected == domain() && actual == other_domain
    ));
    assert_eq!(registry, registry_before);
    assert_eq!(events_in(&storage, &domain()).await, main_before);
    assert_eq!(events_in(&storage, &other_domain).await, other_before);
}

#[tokio::test]
async fn disallowed_axis_transition_is_rejected_before_writing() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let mut initial = report(1);
    initial.connectivity = SessionConnectivityState::Live;
    register(&storage, &mut registry, initial).await;
    let mut invalid = report(1);
    invalid.connectivity = SessionConnectivityState::Unknown;

    let error = ingest_session_report(&storage, &mut registry, invalid)
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SessionError::InvalidTransition { from, to }
            if from == "Live" && to == "Unknown"
    ));
    assert_eq!(events(&storage).await.len(), 1);
}
