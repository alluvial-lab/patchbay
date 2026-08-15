use std::sync::atomic::{AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AdapterId, AuthorityDomainId, CommandId,
    ContinuationContextStatus, EventId, Generation, IdempotencyKey, Lsn, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionReportSourceCursor, SessionStateEvent,
    StoredEventKind, StoredEventPayload, TypedCorrelation,
};
use patchbay_core::{
    session::{
        ingest_session_report as ingest_session_report_in_domain, mark_adapter_sessions_stale,
        rebuild_from_log, IngestResult, SessionError, SessionRegistry, SessionReport,
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

fn cursor(adapter_generation: u64, revision: u64) -> SessionReportSourceCursor {
    SessionReportSourceCursor {
        adapter_generation: Some(generation(adapter_generation)),
        revision,
    }
}

fn report(session_generation: u64, adapter_generation: u64, revision: u64) -> SessionReport {
    SessionReport {
        adapter_id: Some(adapter()),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(runtime()),
        session_generation: Some(generation(session_generation)),
        connectivity: SessionConnectivityState::Unknown as i32,
        activity: SessionActivityState::Unknown as i32,
        project: "patchbay".to_owned(),
        cwd: "/work/patchbay".to_owned(),
        name: "main".to_owned(),
        model: "provider/model-1".to_owned(),
        spawn_origin: None,
        source_cursor: Some(cursor(adapter_generation, revision)),
        continuation_context_status: 0,
    }
}

async fn ingest<S: Storage>(
    storage: &S,
    registry: &mut SessionRegistry,
    report: SessionReport,
) -> Result<IngestResult, SessionError> {
    ingest_session_report_in_domain(storage, registry, &domain(), report).await
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
    let expected_adapter = initial_report.adapter_id.clone().unwrap();
    let expected_scope = initial_report.deployment_scope.clone();
    let expected_runtime = initial_report.runtime_session_id.clone().unwrap();
    let expected_generation = initial_report.session_generation.unwrap();
    let result = ingest(storage, registry, initial_report).await.unwrap();
    assert!(matches!(result, IngestResult::Registered { .. }));
    let live = registry
        .get_live_session(&expected_adapter, &expected_scope, &expected_runtime)
        .expect("successful registration must warm the supplied registry");
    assert_eq!(live.identity.session_generation, expected_generation);
}

/// A storage adapter that fails a configured append before delegating it.
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
async fn first_report_writes_registration_with_source_cursor() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();

    let result = ingest(&storage, &mut registry, report(1, 4, 1))
        .await
        .unwrap();

    let IngestResult::Registered { event_id } = result else {
        panic!("first report must register the session");
    };
    assert_eq!(event_id.lsn, Some(Lsn { value: 1 }));
    let hot = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .unwrap();
    assert_eq!(hot.last_source_cursor, Some(cursor(4, 1)));

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 1);
    let Some(session_state_event::Mutation::Registered(registered)) =
        decode(&committed[0]).mutation
    else {
        panic!("expected SessionRegistered mutation");
    };
    assert_eq!(registered.source_cursor, Some(cursor(4, 1)));
    assert_eq!(registered.model, "provider/model-1");
}

#[tokio::test]
async fn equal_generation_report_is_one_atomic_event_and_hot_equals_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1, 4, 1)).await;

    let mut changed = report(1, 4, 2);
    changed.connectivity = SessionConnectivityState::Live as i32;
    changed.activity = SessionActivityState::Working as i32;
    changed.project = "patchbay-next".to_owned();
    changed.cwd = "/work/patchbay-next".to_owned();
    changed.name = "replacement".to_owned();
    changed.model = "provider/model-2".to_owned();
    let result = ingest(&storage, &mut registry, changed.clone())
        .await
        .unwrap();

    assert!(matches!(result, IngestResult::ReportApplied { .. }));
    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2, "one report must append one mutation");
    let Some(session_state_event::Mutation::ReportApplied(applied)) =
        decode(&committed[1]).mutation
    else {
        panic!("expected SessionReportApplied mutation");
    };
    assert_eq!(applied.previous_source_cursor, Some(cursor(4, 1)));
    assert_eq!(applied.report, Some(changed));

    let live = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .unwrap();
    assert_eq!(live.state.connectivity(), SessionConnectivityState::Live);
    assert_eq!(live.state.activity(), SessionActivityState::Working);
    assert_eq!(live.project, "patchbay-next");
    assert_eq!(live.model, "provider/model-2");
    assert_eq!(live.last_source_cursor, Some(cursor(4, 2)));

    let warm = registry.clone();
    registry.observe(&committed[1]).unwrap();
    assert_eq!(registry, warm, "exact envelope replay must be inert");
    assert_eq!(
        rebuild_from_log(&storage, &domain()).await.unwrap(),
        registry
    );
}

#[tokio::test]
async fn unchanged_newer_report_still_advances_the_watermark_atomically() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1, 4, 1)).await;

    let result = ingest(&storage, &mut registry, report(1, 4, 2))
        .await
        .unwrap();

    assert!(matches!(result, IngestResult::ReportApplied { .. }));
    assert_eq!(events(&storage).await.len(), 2);
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .last_source_cursor,
        Some(cursor(4, 2))
    );
}

#[tokio::test]
async fn stale_source_cursors_are_loud_and_exactly_non_mutating() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1, 4, 1)).await;
    ingest(&storage, &mut registry, report(1, 4, 3))
        .await
        .unwrap();

    for stale in [report(1, 4, 3), report(1, 4, 2), report(1, 3, 99)] {
        let before = registry.clone();
        let before_events = events(&storage).await;
        let error = ingest(&storage, &mut registry, stale).await.unwrap_err();
        assert!(matches!(error, SessionError::StaleSourceCursor { .. }));
        assert_eq!(registry, before);
        assert_eq!(events(&storage).await, before_events);
    }

    ingest(&storage, &mut registry, report(1, 5, 1))
        .await
        .expect("a newer authenticated adapter generation resets local revision");
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .last_source_cursor,
        Some(cursor(5, 1))
    );
}

#[tokio::test]
async fn runtime_generation_order_precedes_source_order() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(2, 8, 12)).await;

    let before = registry.clone();
    let error = ingest(&storage, &mut registry, report(1, 99, 999))
        .await
        .unwrap_err();
    assert!(matches!(error, SessionError::StaleGeneration { .. }));
    assert_eq!(registry, before);

    let result = ingest(&storage, &mut registry, report(3, 8, 1))
        .await
        .unwrap();
    assert!(matches!(
        result,
        IngestResult::GenerationBumped {
            from_generation: Generation { value: 2 },
            to_generation: Generation { value: 3 },
            ..
        }
    ));
    assert!(registry.is_tombstoned(&adapter(), "machine-a", &runtime(), &generation(2)));
    assert_eq!(
        registry
            .get_live_session(&adapter(), "machine-a", &runtime())
            .unwrap()
            .last_source_cursor,
        Some(cursor(8, 1))
    );
}

#[tokio::test]
async fn stale_order_is_checked_before_state_transition_derivation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let mut initial = report(1, 4, 1);
    initial.connectivity = SessionConnectivityState::Live as i32;
    register(&storage, &mut registry, initial).await;

    let mut stale_invalid = report(1, 4, 1);
    stale_invalid.connectivity = SessionConnectivityState::Unknown as i32;
    assert!(matches!(
        ingest(&storage, &mut registry, stale_invalid)
            .await
            .unwrap_err(),
        SessionError::StaleSourceCursor { .. }
    ));

    let mut fresh_invalid = report(1, 4, 2);
    fresh_invalid.connectivity = SessionConnectivityState::Unknown as i32;
    assert!(matches!(
        ingest(&storage, &mut registry, fresh_invalid)
            .await
            .unwrap_err(),
        SessionError::InvalidTransition { .. }
    ));
    assert_eq!(events(&storage).await.len(), 1);
}

#[tokio::test]
async fn malformed_report_fields_fail_before_append() {
    let mut invalid_reports = Vec::new();

    let mut missing_cursor = report(1, 4, 1);
    missing_cursor.source_cursor = None;
    invalid_reports.push(("source_cursor", missing_cursor));

    let mut missing_adapter_generation = report(1, 4, 1);
    missing_adapter_generation
        .source_cursor
        .as_mut()
        .unwrap()
        .adapter_generation = None;
    invalid_reports.push(("adapter_generation", missing_adapter_generation));

    let mut zero_adapter_generation = report(1, 4, 1);
    zero_adapter_generation
        .source_cursor
        .as_mut()
        .unwrap()
        .adapter_generation = Some(Generation { value: 0 });
    invalid_reports.push(("positive adapter_generation", zero_adapter_generation));

    let mut zero_revision = report(1, 4, 1);
    zero_revision.source_cursor.as_mut().unwrap().revision = 0;
    invalid_reports.push(("revision is zero", zero_revision));

    let mut unspecified_connectivity = report(1, 4, 1);
    unspecified_connectivity.connectivity = SessionConnectivityState::Unspecified as i32;
    invalid_reports.push(("connectivity is unspecified", unspecified_connectivity));

    let mut unknown_activity = report(1, 4, 1);
    unknown_activity.activity = i32::MAX;
    invalid_reports.push(("unknown activity", unknown_activity));

    let mut unknown_context = report(1, 4, 1);
    unknown_context.continuation_context_status = i32::MAX;
    invalid_reports.push(("unknown continuation context", unknown_context));

    let mut continuation_only_context = report(1, 4, 1);
    continuation_only_context.continuation_context_status =
        ContinuationContextStatus::Resumed as i32;
    invalid_reports.push(("continuation-only context", continuation_only_context));

    let mut missing_adapter = report(1, 4, 1);
    missing_adapter.adapter_id = None;
    invalid_reports.push(("adapter_id", missing_adapter));

    let mut zero_session_generation = report(1, 4, 1);
    zero_session_generation.session_generation = Some(Generation { value: 0 });
    invalid_reports.push(("positive session_generation", zero_session_generation));

    let mut empty_runtime = report(1, 4, 1);
    empty_runtime
        .runtime_session_id
        .as_mut()
        .unwrap()
        .value
        .clear();
    invalid_reports.push(("runtime_session_id", empty_runtime));

    for (message_fragment, invalid) in invalid_reports {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut registry = SessionRegistry::new(domain()).unwrap();
        let before = registry.clone();
        let error = ingest(&storage, &mut registry, invalid).await.unwrap_err();
        assert!(
            matches!(error, SessionError::CorruptRecord(ref message) if message.contains(message_fragment)),
            "unexpected validation error for {message_fragment}: {error:?}"
        );
        assert_eq!(registry, before);
        assert!(events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn ordinary_ingress_rejects_every_spawn_origin_before_append() {
    for origin in [
        TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
                value: "valid-managed-origin".to_owned(),
            })),
        },
        TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
                value: String::new(),
            })),
        },
        TypedCorrelation { r#ref: None },
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut registry = SessionRegistry::new(domain()).unwrap();
        let mut candidate = report(1, 4, 1);
        candidate.spawn_origin = Some(origin);
        let error = ingest(&storage, &mut registry, candidate)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SessionError::CorruptRecord(ref message) if message.contains("ordinary session report ingress rejects spawn_origin")
        ));
        assert!(events(&storage).await.is_empty());
    }
}

#[tokio::test]
async fn report_append_failure_cannot_leave_a_partial_report() {
    let storage = FailOnNthAppendStorage::new();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    register(&storage, &mut registry, report(1, 4, 1)).await;
    let before = registry.clone();

    let mut changed = report(1, 4, 2);
    changed.connectivity = SessionConnectivityState::Live as i32;
    changed.activity = SessionActivityState::Working as i32;
    changed.model = "provider/model-2".to_owned();
    storage.fail_on_append(1);
    let error = ingest(&storage, &mut registry, changed.clone())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        SessionError::Storage(StorageError::WriteFailed { .. })
    ));
    assert_eq!(registry, before);
    assert_eq!(events(&storage).await.len(), 1);

    storage.recover();
    ingest(&storage, &mut registry, changed).await.unwrap();
    assert_eq!(events(&storage).await.len(), 2);
    assert_eq!(
        rebuild_from_log(&storage, &domain()).await.unwrap(),
        registry
    );
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
            source_cursor: None,
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
    let mut candidate = report(1, 4, 1);
    candidate.runtime_session_id = Some(RuntimeSessionId {
        value: "runtime-2".to_owned(),
    });

    let error = ingest(&storage, &mut registry, candidate)
        .await
        .expect_err("the stale projection must reject the committed older event identity");
    assert!(matches!(error, SessionError::CorruptLog(_)));
    assert_eq!(registry, before);
    assert_eq!(events(&storage).await.len(), 1, "append committed first");
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
async fn explicit_authority_domain_cannot_cross_the_bound_registry() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let other_domain = AuthorityDomainId {
        value: "authority-other".to_owned(),
    };
    let before = registry.clone();

    let error =
        ingest_session_report_in_domain(&storage, &mut registry, &other_domain, report(1, 4, 1))
            .await
            .unwrap_err();
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
async fn disconnect_delta_preserves_adapter_source_cursor() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let mut live = report(1, 4, 7);
    live.connectivity = SessionConnectivityState::Live as i32;
    register(&storage, &mut registry, live).await;

    mark_adapter_sessions_stale(&storage, &mut registry, &domain(), &adapter())
        .await
        .unwrap();

    let record = registry
        .get_live_session(&adapter(), "machine-a", &runtime())
        .unwrap();
    assert_eq!(record.state.connectivity(), SessionConnectivityState::Stale);
    assert_eq!(record.last_source_cursor, Some(cursor(4, 7)));
    assert_eq!(
        rebuild_from_log(&storage, &domain()).await.unwrap(),
        registry
    );
}
