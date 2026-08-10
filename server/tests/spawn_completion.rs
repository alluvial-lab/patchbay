use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterId, AuditEventKind, AuditRecord, AuthorityDomainId, CommandId, CommandTransition,
    DescendantGrant, DescendantGrantProvenance, DeviceId, EndpointId, EventId, FailureCode,
    Generation, GrantId, GrantRevocationPolicy, IdempotencyKey, Lsn, Observation, ObservationKind,
    Operation, OperationKind, OperationState, RuntimeSessionId, SessionRegistered,
    SessionStateEvent, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
    TypedCorrelation,
};
use patchbay_core::{
    audit::{AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::{ingest_descendant_grant, AuthorityRegistry, DESCENDANT_GRANT_ALLOWED_KINDS},
    storage::{
        AuditRecordDraft, AuditedStorage, DedupOutcome, RecordedEvent, RusqliteStorage, Storage,
        StorageError, StoredSnapshot, TargetKey,
    },
    time::TestClock,
};
use patchbay_core_server::{
    decision_gate::CoreDecisionGate,
    spawn_completion::{SpawnCompletionDriver, SpawnCompletionError},
};
use prost::Message;
use prost_types::Timestamp;
use tokio::sync::Semaphore;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}
fn command_id() -> CommandId {
    CommandId {
        value: "spawn-1".to_owned(),
    }
}
fn actor() -> ActorId {
    ActorId {
        value: "verified-operator".to_owned(),
    }
}
fn endpoint() -> EndpointId {
    EndpointId {
        value: "verified-browser".to_owned(),
    }
}
fn device() -> DeviceId {
    DeviceId {
        value: "verified-laptop".to_owned(),
    }
}
fn parent_grant_id() -> GrantId {
    GrantId {
        value: "spawn-grant".to_owned(),
    }
}
fn correlation() -> TypedCorrelation {
    TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(command_id())),
    }
}
fn fleet_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    }
}
fn session_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "spawned-session".to_owned(),
        }),
        session_generation: Some(Generation { value: 7 }),
        ..TargetScope::default()
    }
}
fn clock() -> Arc<TestClock> {
    Arc::new(TestClock::new(Timestamp {
        seconds: 1_000,
        nanos: 0,
    }))
}

async fn all_events<S: Storage>(storage: &S) -> Vec<RecordedEvent> {
    storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
}

async fn seed_evidence<S: Storage>(storage: &S) -> EventId {
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: AcceptedOperation {
                    operation: Some(Operation {
                        command_id: Some(command_id()),
                        authority_domain_id: Some(domain()),
                        sender: Some(ActorEndpointRef {
                            actor_id: Some(actor()),
                            endpoint_id: Some(endpoint()),
                            device_id: Some(device()),
                            ..ActorEndpointRef::default()
                        }),
                        kind: OperationKind::Spawn as i32,
                        target_scope: Some(fleet_scope()),
                        ..Operation::default()
                    }),
                    authorizing_grant_id: Some(parent_grant_id()),
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(command_id()),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Delivered as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();
    let result_id = storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: Observation {
                    authority_domain_id: Some(domain()),
                    kind: ObservationKind::Result as i32,
                    correlations: vec![correlation()],
                    target_scope: Some(fleet_scope()),
                    failure_code: FailureCode::Unspecified as i32,
                    ..Observation::default()
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::SessionState as i32,
                payload: SessionStateEvent {
                    authority_domain_id: Some(domain()),
                    mutation: Some(session_state_event::Mutation::Registered(
                        SessionRegistered {
                            adapter_id: session_scope().adapter_id,
                            deployment_scope: "machine-a".to_owned(),
                            runtime_session_id: session_scope().runtime_session_id,
                            session_generation: Some(Generation { value: 7 }),
                            spawn_origin: Some(correlation()),
                            ..SessionRegistered::default()
                        },
                    )),
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();
    result_id
}

async fn append_completion_audit<S: Storage>(storage: &S, source: EventId) -> EventId {
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 1_000,
            nanos: 0,
        },
        AuditEventKind::CommandCompleted,
    );
    audit.actor_id = Some(actor());
    audit.endpoint_id = Some(endpoint());
    audit.device_id = Some(device());
    audit.command_id = Some(command_id());
    audit.grant_id = Some(parent_grant_id());
    audit.target_scope = Some(session_scope());
    audit.reason_code = "spawn_completion".to_owned();
    audit.source_event_id = Some(source);
    storage.append_audit(&domain(), audit).await.unwrap()
}

async fn append_descendant<S>(storage: &S, audit_id: EventId)
where
    S: Storage,
{
    let mut authority = AuthorityRegistry::new();
    ingest_descendant_grant(
        storage,
        &mut authority,
        &domain(),
        DescendantGrant {
            grant_id: Some(GrantId {
                value: "desc:authority-main:spawn-1".to_owned(),
            }),
            authority_domain_id: Some(domain()),
            subject_actor_id: Some(actor()),
            subject_endpoint_id: Some(endpoint()),
            target_scope: Some(session_scope()),
            allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
                .iter()
                .map(|kind| *kind as i32)
                .collect(),
            provenance: Some(DescendantGrantProvenance {
                spawn_operation_id: Some(command_id()),
                spawning_grant_id: Some(parent_grant_id()),
            }),
            created_at: Some(Timestamp {
                seconds: 1_000,
                nanos: 0,
            }),
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            audit_id: Some(audit_id),
            ..DescendantGrant::default()
        },
    )
    .await
    .unwrap();
}

fn production_audit<S>(storage: S) -> Arc<dyn AuditSink>
where
    S: Storage + Clone + 'static,
{
    Arc::new(RequiredAuditFanout::new(
        Arc::new(DurableAuditSink::new(storage, domain())),
        vec![],
    ))
}

fn completion_counts(events: &[RecordedEvent]) -> (usize, usize, usize) {
    let audits = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::AuditRecord as i32)
        .filter_map(|event| AuditRecord::decode(event.payload.payload.as_slice()).ok())
        .filter(|audit| audit.reason_code == "spawn_completion")
        .count();
    let grants = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::DescendantGrant as i32)
        .count();
    let transitions = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
        .filter_map(|event| CommandTransition::decode(event.payload.payload.as_slice()).ok())
        .filter(|transition| {
            transition.command_id.as_ref() == Some(&command_id())
                && transition.to_state == OperationState::Completed as i32
        })
        .count();
    (audits, grants, transitions)
}

#[tokio::test]
async fn crash_prefixes_repair_to_one_audit_grant_and_terminal_transition() {
    for prefix in 0..=3 {
        let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
        let source = seed_evidence(&storage).await;
        let audit_id = if prefix >= 1 {
            Some(append_completion_audit(&storage, source).await)
        } else {
            None
        };
        if prefix >= 2 {
            append_descendant(&storage, audit_id.clone().unwrap()).await;
        }
        if prefix >= 3 {
            storage
                .append(
                    &domain(),
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: CommandTransition {
                            command_id: Some(command_id()),
                            from_state: OperationState::Delivered as i32,
                            to_state: OperationState::Completed as i32,
                            failure_code: FailureCode::Unspecified as i32,
                            ..CommandTransition::default()
                        }
                        .encode_to_vec(),
                    },
                )
                .await
                .unwrap();
        }

        let gate = CoreDecisionGate::default();
        let driver = SpawnCompletionDriver::bootstrap(
            storage.clone(),
            domain(),
            gate,
            production_audit(storage.clone()),
            clock(),
        )
        .await
        .unwrap();
        drop(driver);
        assert_eq!(completion_counts(&all_events(&storage).await), (1, 1, 1));

        let restarted = SpawnCompletionDriver::bootstrap(
            storage.clone(),
            domain(),
            CoreDecisionGate::default(),
            production_audit(storage.clone()),
            clock(),
        )
        .await
        .unwrap();
        drop(restarted);
        assert_eq!(completion_counts(&all_events(&storage).await), (1, 1, 1));
    }
}

#[tokio::test]
async fn non_durable_audit_sink_fails_closed_before_grant_or_completion() {
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_evidence(&storage).await;
    let error = match SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        Arc::new(StderrAuditSink),
        clock(),
    )
    .await
    {
        Ok(_) => panic!("diagnostic-only audit cannot complete a spawn"),
        Err(error) => error,
    };
    assert!(matches!(error, SpawnCompletionError::Audit(_)));
    assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
}

#[derive(Clone)]
struct BlockingTransitionStorage {
    inner: AuditedStorage<RusqliteStorage>,
    reached: Arc<Semaphore>,
    release: Arc<Semaphore>,
    blocked: Arc<AtomicBool>,
}

impl Storage for BlockingTransitionStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        let is_completion = if payload.kind == StoredEventKind::CommandTransition as i32 {
            CommandTransition::decode(payload.payload.as_slice())
                .ok()
                .is_some_and(|transition| transition.to_state == OperationState::Completed as i32)
        } else {
            false
        };
        if is_completion && !self.blocked.swap(true, Ordering::SeqCst) {
            self.reached.add_permits(1);
            self.release
                .acquire()
                .await
                .expect("release semaphore open")
                .forget();
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

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<EventId, StorageError> {
        self.inner.append_audit(authority_domain_id, audit).await
    }
}

#[tokio::test]
async fn shared_gate_hides_the_committed_audit_and_grant_prefix() {
    let inner = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    let storage = BlockingTransitionStorage {
        inner: inner.clone(),
        reached: Arc::new(Semaphore::new(0)),
        release: Arc::new(Semaphore::new(0)),
        blocked: Arc::new(AtomicBool::new(false)),
    };
    seed_evidence(&storage).await;
    let gate = CoreDecisionGate::default();
    let task_gate = gate.clone();
    let task_storage = storage.clone();
    let task = tokio::spawn(async move {
        SpawnCompletionDriver::bootstrap(
            task_storage.clone(),
            domain(),
            task_gate,
            production_audit(task_storage),
            clock(),
        )
        .await
    });

    storage
        .reached
        .acquire()
        .await
        .expect("driver reaches final transition")
        .forget();
    assert_eq!(completion_counts(&all_events(&inner).await), (1, 1, 0));

    let blocked_reader =
        tokio::time::timeout(std::time::Duration::from_millis(20), gate.acquire()).await;
    assert!(
        blocked_reader.is_err(),
        "reader must wait behind the shared gate"
    );

    storage.release.add_permits(1);
    let driver = task.await.unwrap().unwrap();
    drop(driver);
    assert_eq!(completion_counts(&all_events(&inner).await), (1, 1, 1));
    let _reader = gate.acquire().await;
}
