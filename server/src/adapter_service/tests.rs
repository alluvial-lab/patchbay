use patchbay_contracts::patchbay::{
    observation_request, AdapterCapability, AdapterRegistration, AdapterSnapshotSupport,
    AttachRequest, AuthorityDomainId, CommandId, EndpointId, Generation, Lsn, Operation,
    OperationKind, ReceiveRequest, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
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

    let report = patchbay_contracts::patchbay::SessionReport {
        adapter_id: Some(adapter_id()),
        deployment_scope: "machine-a".into(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "session-1".into(),
        }),
        session_generation: Some(Generation { value: 1 }),
        connectivity: SessionConnectivityState::Live as i32,
        activity: SessionActivityState::Idle as i32,
        project: "patchbay".into(),
        cwd: "/work/patchbay".into(),
        name: "test".into(),
        spawn_origin: None,
    };
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
