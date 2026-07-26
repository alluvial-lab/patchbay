use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use patchbay_contracts::patchbay::{
    observation_request, typed_correlation, AdapterCapability, AdapterDiagnosticPayload,
    AdapterDiagnosticReport, AdapterDiagnosticSeverity, AdapterRegistration,
    AdapterSnapshotSupport, AttachRequest, AuditEventKind, AuthorityDomainId, CommandId, EndpointId, FailureCode,
    Generation, IdempotencyKey, Lsn, Observation, ObservationKind, Operation, OperationKind,
    PayloadContentType, PayloadEnvelope, ReceiveRequest, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
    TypedCorrelation,
};
use patchbay_core::storage::{
    DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot, TargetKey,
};
use prost::Message;
use tokio::sync::Notify;
use tokio_stream::StreamExt;
use tonic::Request;

use super::*;

const EVIDENCE: &str = "adapter-test-secret";

#[derive(Clone)]
struct BlockingReadStorage {
    inner: RusqliteStorage,
    block_next_read: Arc<AtomicBool>,
    read_started: Arc<Notify>,
    release_read: Arc<Notify>,
}

impl BlockingReadStorage {
    fn new() -> Self {
        Self {
            inner: RusqliteStorage::open_in_memory().expect("storage opens"),
            block_next_read: Arc::new(AtomicBool::new(false)),
            read_started: Arc::new(Notify::new()),
            release_read: Arc::new(Notify::new()),
        }
    }

    fn block_next_read(&self) {
        self.block_next_read.store(true, Ordering::SeqCst);
    }

    async fn wait_for_blocked_read(&self) {
        self.read_started.notified().await;
    }

    fn release_blocked_read(&self) {
        self.release_read.notify_one();
    }
}

impl Storage for BlockingReadStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
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
        if self.block_next_read.swap(false, Ordering::SeqCst) {
            self.read_started.notify_one();
            self.release_read.notified().await;
        }
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
async fn authenticated_diagnostic_report_appends_source_and_audit_atomically() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;
    let response = service
        .report_diagnostics(authenticated_with_attachment_token(
            AdapterDiagnosticReport {
                authority_domain_id: Some(domain.clone()),
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::Adapter as i32,
                    adapter_id: Some(adapter_id()),
                    ..Default::default()
                }),
                observed_at: Some(prost_types::Timestamp { seconds: 2, nanos: 0 }),
                payload: Some(PayloadEnvelope {
                    payload: AdapterDiagnosticPayload {
                        code: "pi_adapter_started".into(),
                        severity: AdapterDiagnosticSeverity::Info as i32,
                        adapter_generation: Some(Generation { value: 1 }),
                        count: 1,
                        ..Default::default()
                    }
                    .encode_to_vec(),
                    content_type: PayloadContentType::Protobuf as i32,
                    schema_ref: "patchbay.AdapterDiagnosticPayload".into(),
                }),
                ..Default::default()
            },
            &attachment_token,
        ))
        .await
        .expect("report succeeds")
        .into_inner();
    assert!(response.accepted);
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.expect("events read");
    assert_eq!(events.iter().filter(|event| event.payload.kind == StoredEventKind::Observation as i32).count(), 2, "registration plus diagnostic source");
    let diagnostic_source = response.observation_event_id.expect("source id");
    let audit_id = response.audit_event_id.expect("audit id");
    let audit = events.iter().find(|event| event.event_id == audit_id).expect("audit event");
    let audit = patchbay_contracts::patchbay::AuditRecord::decode(audit.payload.payload.as_slice()).expect("audit decodes");
    assert_eq!(audit.kind, AuditEventKind::AdapterDiagnosticReported as i32);
    assert_eq!(audit.source_event_id, Some(diagnostic_source));
    assert_eq!(audit.reason_code, "pi_adapter_started");
    assert!(audit.adapter_diagnostic.is_some());
}

#[tokio::test]
async fn adapter_attaches_reports_session_and_receives_targeted_operation() {
    let directory = tempfile::tempdir().expect("temp directory");
    let database = directory.path().join("core.sqlite3");
    let storage =
        RusqliteStorage::open(database.to_str().expect("utf8 path")).expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");

    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(domain.clone())),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach succeeds");
    let attachment_token = attachment_token(&attached);
    let attached = attached.into_inner();
    assert!(attached.accepted);
    assert!(attached.attach_event_id.is_some());

    let report = session_report(SessionConnectivityState::Live);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(report)),
            },
            &attachment_token,
        ))
        .await
        .expect("session report succeeds");

    let operation = Operation {
        command_id: Some(CommandId {
            value: "command-1".into(),
        }),
        authority_domain_id: Some(domain.clone()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id()),
            deployment_scope: "machine-a".into(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "session-1".into(),
            }),
            session_generation: Some(Generation { value: 1 }),
            ..Default::default()
        }),
        idempotency_key: "command-1-key".into(),
        ..Default::default()
    };
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: operation.encode_to_vec(),
            },
        )
        .await
        .expect("operation appends");

    let mut deliveries = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &attachment_token,
        ))
        .await
        .expect("delivery stream opens")
        .into_inner();
    let delivery = deliveries
        .next()
        .await
        .expect("one delivery")
        .expect("valid delivery");
    assert_eq!(
        delivery.operation.expect("operation").kind,
        OperationKind::Instruct as i32
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(20), deliveries.next())
            .await
            .is_err(),
        "an idle delivery subscription remains pending"
    );
}

#[tokio::test]
async fn concurrent_conflicting_model_reports_leave_a_replayable_log() {
    let storage = BlockingReadStorage::new();
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;

    let mut initial = session_report(SessionConnectivityState::Live);
    initial.model = "provider/model-a".into();
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(initial)),
            },
            &attachment_token,
        ))
        .await
        .expect("initial session report succeeds");

    storage.block_next_read();
    let first_service = service.clone();
    let first_domain = domain.clone();
    let first_token = attachment_token.clone();
    let first = tokio::spawn(async move {
        let mut report = session_report(SessionConnectivityState::Live);
        report.model = "provider/model-b".into();
        first_service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(first_domain),
                    observation: Some(observation_request::Observation::SessionReport(report)),
                },
                &first_token,
            ))
            .await
    });
    storage.wait_for_blocked_read().await;

    let second_service = service.clone();
    let second_domain = domain.clone();
    let second_token = attachment_token.clone();
    let second = tokio::spawn(async move {
        let mut report = session_report(SessionConnectivityState::Live);
        report.model = "provider/model-c".into();
        second_service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(second_domain),
                    observation: Some(observation_request::Observation::SessionReport(report)),
                },
                &second_token,
            ))
            .await
    });

    tokio::task::yield_now().await;
    storage.release_blocked_read();
    first
        .await
        .expect("first report task joins")
        .expect("first model report succeeds");
    second
        .await
        .expect("second report task joins")
        .expect("second model report succeeds");

    let replayed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("concurrent model-report log remains replayable");
    assert_eq!(
        replayed
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains live")
            .model,
        "provider/model-c"
    );
}

#[tokio::test]
async fn newer_attachment_fences_stale_adapter_process() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");

    let stale_token = attach_generation(&service, domain.clone(), 1).await;
    let current_token = attach_generation(&service, domain, 2).await;

    let stale = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &stale_token,
        ))
        .await
        .err()
        .expect("superseded attachment token must be rejected");
    assert_eq!(stale.code(), tonic::Code::Unauthenticated);

    service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &current_token,
        ))
        .await
        .expect("current attachment token remains valid");
}

#[tokio::test]
async fn post_attach_rpc_without_attachment_token_is_rejected() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, _attachment_token) = attached_service(storage, domain).await;

    let status = service
        .receive_deliveries(authenticated(ReceiveRequest {
            adapter_id: Some(adapter_id()),
            cursor: Some(Lsn { value: 0 }),
        }))
        .await
        .err()
        .expect("missing attachment token must be rejected");
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn rebuilt_core_forgets_attachment_tokens_until_adapter_reattaches() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (before_restart, stale_token) = attached_service(storage.clone(), domain.clone()).await;
    drop(before_restart);

    let after_restart = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service rebuilds");
    let status = after_restart
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &stale_token,
        ))
        .await
        .err()
        .expect("in-memory attachment tokens must not survive restart");
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    let refreshed_token = attach_generation(&after_restart, domain, 2).await;
    after_restart
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &refreshed_token,
        ))
        .await
        .expect("reattachment restores access");
}

#[tokio::test]
async fn delivered_command_is_redelivered_and_reacknowledged_without_double_transition() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let operation = targeted_operation(domain.clone(), "command-redelivery");
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: operation.encode_to_vec(),
            },
        )
        .await
        .expect("operation appends");

    let mut first_tail = receive_from_start(&service, &attachment_token).await;
    assert!(first_tail.next().await.unwrap().is_ok());

    // Treat the first successful core call as a response lost after the
    // delivered checkpoint committed: the simulated adapter does not execute.
    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("first acknowledgement commits delivered");
    drop(first_tail);
    let mut executions = 0;

    let mut redelivery_tail = receive_from_start(&service, &attachment_token).await;
    let redelivery = redelivery_tail
        .next()
        .await
        .expect("delivered command is re-offered")
        .expect("redelivery is valid");
    assert_eq!(redelivery.operation, Some(operation.clone()));

    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("delivered command re-acknowledges idempotently");
    drop(redelivery_tail);
    executions += 1;
    assert_eq!(executions, 1, "adapter begins execution exactly once");

    let events = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events remain readable");
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
            .count(),
        1,
        "a re-ack must not append delivered -> delivered"
    );
}

#[tokio::test]
async fn abnormal_delivery_stream_drop_marks_adapter_sessions_stale() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        &attachment_token,
    )
    .await;

    let tail = receive_from_start(&service, &attachment_token).await;
    drop(tail); // no terminal `None`: models transport loss / process death

    let mut became_stale = false;
    for _ in 0..100 {
        let rebuilt = session::rebuild_from_log(&storage, &domain)
            .await
            .expect("session log rebuilds");
        let connectivity = rebuilt
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains registered")
            .state
            .connectivity();
        if connectivity == SessionConnectivityState::Stale {
            became_stale = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        became_stale,
        "abnormal stream drop did not durably mark the session stale"
    );

    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        &attachment_token,
    )
    .await;
    let refreshed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("session log rebuilds after reconnect report");
    assert_eq!(
        refreshed
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains registered")
            .state
            .connectivity(),
        SessionConnectivityState::Live,
        "a fresh adapter report restores authoritative liveness"
    );
}

#[tokio::test]
async fn idle_delivery_subscription_receives_an_operation_accepted_later() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let mut subscription = receive_from_start(&service, &attachment_token).await;

    assert!(
        tokio::time::timeout(Duration::from_millis(20), subscription.next())
            .await
            .is_err(),
        "empty subscription must not complete"
    );

    let operation = targeted_operation(domain.clone(), "command-after-open");
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: operation.encode_to_vec(),
            },
        )
        .await
        .expect("operation appends");

    let delivered = tokio::time::timeout(Duration::from_secs(1), subscription.next())
        .await
        .expect("subscription observes the durable tail")
        .expect("subscription remains open")
        .expect("delivery is valid");
    assert_eq!(delivered.operation, Some(operation));
}

#[tokio::test]
async fn obsolete_stream_drop_is_inert_but_current_stream_drop_marks_stale() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        &attachment_token,
    )
    .await;

    let obsolete = receive_from_start(&service, &attachment_token).await;
    let current = receive_from_start(&service, &attachment_token).await;
    drop(obsolete);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        session_connectivity(&storage, &domain).await,
        SessionConnectivityState::Live,
        "the newer stream epoch fences the obsolete drop"
    );

    drop(current);
    wait_for_connectivity(&storage, &domain, SessionConnectivityState::Stale).await;
}

#[tokio::test]
async fn stream_loss_fails_running_once_and_leaves_delivered_redeliverable() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let running = targeted_operation(domain.clone(), "command-running");
    let delivered = targeted_operation(domain.clone(), "command-delivered");
    for operation in [&running, &delivered] {
        storage
            .append(
                &domain,
                StoredEventPayload {
                    kind: StoredEventKind::Operation as i32,
                    payload: operation.encode_to_vec(),
                },
            )
            .await
            .expect("operation appends");
    }

    let mut subscription = receive_from_start(&service, &attachment_token).await;
    for operation in [&running, &delivered] {
        subscription
            .next()
            .await
            .expect("delivery")
            .expect("valid delivery");
        service
            .ingest_observation(authenticated_with_attachment_token(
                delivery_acknowledgement(domain.clone(), operation),
                &attachment_token,
            ))
            .await
            .expect("delivery acknowledgement");
    }
    service
        .ingest_observation(authenticated_with_attachment_token(
            lifecycle_observation(
                domain.clone(),
                &running,
                ObservationKind::Status,
                FailureCode::Unspecified,
            ),
            &attachment_token,
        ))
        .await
        .expect("running observation");

    drop(subscription);
    wait_for_command_state(&storage, &domain, "command-running", OperationState::Failed).await;

    let rebuilt = acceptance::rebuild_from_log(&storage, &domain)
        .await
        .expect("command log rebuilds");
    let running_record = rebuilt
        .get_command(&CommandId {
            value: "command-running".into(),
        })
        .expect("running command remains indexed");
    assert_eq!(
        running_record.failure_code,
        Some(FailureCode::ExecutionOutcomeUnknown)
    );
    assert_eq!(
        rebuilt
            .get_command(&CommandId {
                value: "command-delivered".into(),
            })
            .expect("delivered command remains indexed")
            .state,
        OperationState::Delivered
    );

    service
        .ingest_observation(authenticated_with_attachment_token(
            lifecycle_observation(
                domain.clone(),
                &running,
                ObservationKind::Result,
                FailureCode::Unspecified,
            ),
            &attachment_token,
        ))
        .await
        .expect("late completion is accepted as audit evidence");
    let after_late = acceptance::rebuild_from_log(&storage, &domain)
        .await
        .expect("command log rebuilds after late terminal");
    assert_eq!(
        after_late
            .get_command(&CommandId {
                value: "command-running".into(),
            })
            .expect("command")
            .state,
        OperationState::Failed,
        "first durable terminal outcome remains final"
    );

    let mut redelivery = receive_from_start(&service, &attachment_token).await;
    let operation = redelivery
        .next()
        .await
        .expect("delivered command is re-offered")
        .expect("redelivery is valid")
        .operation
        .expect("redelivery carries operation");
    assert_eq!(operation.command_id, delivered.command_id);
}

async fn session_connectivity(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> SessionConnectivityState {
    session::rebuild_from_log(storage, domain)
        .await
        .expect("session log rebuilds")
        .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
        .expect("session remains registered")
        .state
        .connectivity()
}

async fn wait_for_connectivity(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
    expected: SessionConnectivityState,
) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if session_connectivity(storage, domain).await == expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("session connectivity reaches expected state");
}

async fn wait_for_command_state(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
    command_id: &str,
    expected: OperationState,
) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let commands = acceptance::rebuild_from_log(storage, domain)
                .await
                .expect("command log rebuilds");
            if commands
                .get_command(&CommandId {
                    value: command_id.into(),
                })
                .is_some_and(|record| record.state == expected)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("command reaches expected state");
}

async fn attached_service(
    storage: RusqliteStorage,
    domain: AuthorityDomainId,
) -> (AdapterControlServiceImpl<RusqliteStorage>, String) {
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain, 1).await;
    (service, attachment_token)
}

async fn receive_from_start(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    attachment_token: &str,
) -> DeliveryStream {
    service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            attachment_token,
        ))
        .await
        .expect("delivery stream opens")
        .into_inner()
}

async fn report_session(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    domain: AuthorityDomainId,
    connectivity: SessionConnectivityState,
    attachment_token: &str,
) {
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain),
                observation: Some(observation_request::Observation::SessionReport(
                    session_report(connectivity),
                )),
            },
            attachment_token,
        ))
        .await
        .expect("session report succeeds");
}

fn session_report(
    connectivity: SessionConnectivityState,
) -> patchbay_contracts::patchbay::SessionReport {
    patchbay_contracts::patchbay::SessionReport {
        adapter_id: Some(adapter_id()),
        deployment_scope: "machine-a".into(),
        runtime_session_id: Some(runtime_session_id()),
        session_generation: Some(Generation { value: 1 }),
        connectivity: connectivity as i32,
        activity: SessionActivityState::Idle as i32,
        project: "patchbay".into(),
        cwd: "/work/patchbay".into(),
        name: "test".into(),
        model: "provider/model".into(),
        spawn_origin: None,
    }
}

fn targeted_operation(domain: AuthorityDomainId, command: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.into(),
        }),
        authority_domain_id: Some(domain),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id()),
            deployment_scope: "machine-a".into(),
            runtime_session_id: Some(runtime_session_id()),
            session_generation: Some(Generation { value: 1 }),
            ..Default::default()
        }),
        idempotency_key: format!("{command}-key"),
        ..Default::default()
    }
}

fn lifecycle_observation(
    domain: AuthorityDomainId,
    operation: &Operation,
    kind: ObservationKind,
    failure_code: FailureCode,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::Event(Observation {
            authority_domain_id: Some(domain),
            kind: kind as i32,
            target_scope: operation.target_scope.clone(),
            failure_code: failure_code as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(
                    operation.command_id.clone().expect("command id"),
                )),
            }],
            ..Default::default()
        })),
    }
}

fn delivery_acknowledgement(
    domain: AuthorityDomainId,
    operation: &Operation,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::Event(Observation {
            authority_domain_id: Some(domain),
            kind: ObservationKind::Event as i32,
            target_scope: operation.target_scope.clone(),
            payload: Some(PayloadEnvelope {
                schema_ref: adapter::DELIVERY_ACKNOWLEDGEMENT_SCHEMA.to_owned(),
                ..Default::default()
            }),
            failure_code: FailureCode::Unspecified as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(
                    operation.command_id.clone().expect("command id"),
                )),
            }],
            ..Default::default()
        })),
    }
}

fn registration(domain: AuthorityDomainId) -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(adapter_id()),
        endpoint_id: Some(EndpointId {
            value: "pi-adapter-endpoint".into(),
        }),
        authority_domain_id: Some(domain),
        adapter_generation: Some(Generation { value: 1 }),
        capability: Some(AdapterCapability {
            supported_operation_kinds: vec![OperationKind::Instruct as i32],
            streaming_support: true,
            snapshot_support: AdapterSnapshotSupport::Partial as i32,
            cancellation_support: true,
            session_replacement_support: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn adapter_id() -> AdapterId {
    AdapterId { value: "pi".into() }
}

fn runtime_session_id() -> RuntimeSessionId {
    RuntimeSessionId {
        value: "session-1".into(),
    }
}

async fn attach_generation<S>(
    service: &AdapterControlServiceImpl<S>,
    domain: AuthorityDomainId,
    generation: u64,
) -> String
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let mut registration = registration(domain);
    registration.adapter_generation = Some(Generation { value: generation });
    let response = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach succeeds");
    attachment_token(&response)
}

fn attachment_token<T>(response: &Response<T>) -> String {
    response
        .metadata()
        .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .expect("attach response carries attachment token")
        .to_str()
        .expect("attachment token is ASCII")
        .to_owned()
}

fn authenticated_with_attachment_token<T>(message: T, attachment_token: &str) -> Request<T> {
    let mut request = authenticated(message);
    request.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        attachment_token.parse().expect("metadata"),
    );
    request
}

fn authenticated<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert(ADAPTER_ID_HEADER, "pi".parse().expect("metadata"));
    request
        .metadata_mut()
        .insert(ADAPTER_EVIDENCE_HEADER, EVIDENCE.parse().expect("metadata"));
    request
}
