use patchbay_contracts::patchbay::{
    observation_request, typed_correlation, AdapterCapability, AdapterRegistration,
    AdapterSnapshotSupport, AttachRequest, AuthorityDomainId, CommandId, EndpointId, FailureCode,
    Generation, Lsn, Observation, ObservationKind, Operation, OperationKind, PayloadEnvelope,
    ReceiveRequest, RuntimeSessionId, SessionActivityState, SessionConnectivityState,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::storage::{RusqliteStorage, Storage};
use prost::Message;
use tokio_stream::StreamExt;
use tonic::Request;

use super::*;

const EVIDENCE: &str = "adapter-test-secret";

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
        .expect("attach succeeds")
        .into_inner();
    assert!(attached.accepted);
    assert!(attached.attach_event_id.is_some());

    let report = session_report(SessionConnectivityState::Live);
    service
        .ingest_observation(authenticated(ObservationRequest {
            authority_domain_id: Some(domain.clone()),
            observation: Some(observation_request::Observation::SessionReport(report)),
        }))
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
        .receive_deliveries(authenticated(ReceiveRequest {
            adapter_id: Some(adapter_id()),
            cursor: Some(Lsn { value: 0 }),
        }))
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
    assert!(deliveries.next().await.is_none(), "tail completes cleanly");
}

#[tokio::test]
async fn delivered_command_is_redelivered_and_reacknowledged_without_double_transition() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = attached_service(storage.clone(), domain.clone()).await;
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

    let mut first_tail = receive_from_start(&service).await;
    assert!(first_tail.next().await.unwrap().is_ok());
    assert!(first_tail.next().await.is_none());

    // Treat the first successful core call as a response lost after the
    // delivered checkpoint committed: the simulated adapter does not execute.
    service
        .ingest_observation(authenticated(delivery_acknowledgement(
            domain.clone(),
            &operation,
        )))
        .await
        .expect("first acknowledgement commits delivered");
    let mut executions = 0;

    let mut redelivery_tail = receive_from_start(&service).await;
    let redelivery = redelivery_tail
        .next()
        .await
        .expect("delivered command is re-offered")
        .expect("redelivery is valid");
    assert_eq!(redelivery.operation, Some(operation.clone()));
    assert!(redelivery_tail.next().await.is_none());

    service
        .ingest_observation(authenticated(delivery_acknowledgement(
            domain.clone(),
            &operation,
        )))
        .await
        .expect("delivered command re-acknowledges idempotently");
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
    let service = attached_service(storage.clone(), domain.clone()).await;
    report_session(&service, domain.clone(), SessionConnectivityState::Live).await;

    let tail = receive_from_start(&service).await;
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

    report_session(&service, domain.clone(), SessionConnectivityState::Live).await;
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
async fn clean_delivery_tail_completion_does_not_mark_sessions_stale() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = attached_service(storage.clone(), domain.clone()).await;
    report_session(&service, domain.clone(), SessionConnectivityState::Live).await;

    let mut tail = receive_from_start(&service).await;
    assert!(tail.next().await.is_none(), "empty durable tail completes");
    drop(tail);
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    let rebuilt = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("session log rebuilds");
    assert_eq!(
        rebuilt
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains registered")
            .state
            .connectivity(),
        SessionConnectivityState::Live
    );
}

async fn attached_service(
    storage: RusqliteStorage,
    domain: AuthorityDomainId,
) -> AdapterControlServiceImpl<RusqliteStorage> {
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).expect("valid evidence"),
    )
    .await
    .expect("service initializes");
    service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(domain)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach succeeds");
    service
}

async fn receive_from_start(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
) -> DeliveryStream {
    service
        .receive_deliveries(authenticated(ReceiveRequest {
            adapter_id: Some(adapter_id()),
            cursor: Some(Lsn { value: 0 }),
        }))
        .await
        .expect("delivery stream opens")
        .into_inner()
}

async fn report_session(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    domain: AuthorityDomainId,
    connectivity: SessionConnectivityState,
) {
    service
        .ingest_observation(authenticated(ObservationRequest {
            authority_domain_id: Some(domain),
            observation: Some(observation_request::Observation::SessionReport(
                session_report(connectivity),
            )),
        }))
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
