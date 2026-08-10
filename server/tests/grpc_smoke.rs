use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, ActorEndpointRef, ActorId,
    AdapterCapability, AdapterId, AdapterRegistration, AdapterSnapshotSupport,
    AdapterTargetCategory, AttachRequest, AuditEventKind, AuthorityDomainId, CommandId,
    CommandTransition, ControlSurfacePrincipalRecord, ControlSurfaceRevocation, DeviceId, EndpointId, EventId, EnterSecurityLockdownRequest, ExitSecurityLockdownRequest, FailureCode, Generation, Grant,
    GrantId, GrantProvenance, GrantRevocationPolicy, IdempotencyKey, LoadSnapshotRequest, Lsn,
    AuditQuery, DiagnosticsQuery, Operation, OperationKind, OperationState, OperatorRecord,
    RevokeAllOperatorSessionsRequest, RevokeControlSurfaceEndpointRequest,
    RevokeControlSurfacePrincipalRequest, RevokeGrantRequest,
    PayloadEnvelope, PrincipalEnrollment, PayloadContentType, QueryDiagnosticsRequest, RuntimeSessionId,
    ObservationRequest, ResourceCapability, ResourceFreshnessState, ResourceId, ResourceKind,
    ResourceProjectionContract, ResourceReport, ResourceReportMutation, ResourceSnapshot,
    ResourceSnapshotReport, ResourceStateUpsert, ResourceViewReport, SchemaDescriptor,
    SessionActivityState,
    SessionConnectivityState, SessionRegistered, SessionSnapshot, SessionState, SnapshotViewKind,
    StoredEventKind, StoredEventPayload, SubmissionOutcome,
    SubmitRequest, SubscribeRequest, TargetScope, TargetScopeKind, TimeWindow,
    VerifyOperatorPasswordRequest, diagnostics_query, query_diagnostics_response,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    authority::{
        events as authority_events, hash_principal_credential, ingest_grant, AuthorityRegistry,
    },
    session::events as session_events,
    time::{Clock, TestClock},
    storage::{
        AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedDedupOutcome, AuditedStorage,
        CoreGenerationStore, DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError,
        StoredSnapshot, TargetKey,
    },
};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    admin_service::{AdminServiceImpl, SetupSecret},
    decision_gate::CoreDecisionGate,
    login_security::{LoginLimiter, StderrLoginAuditSink},
    operator_session::OperatorSessionBinding,
    issuer::{
        OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER,
    },
    rpc::{
        adapter_control_service_server::AdapterControlService,
        admin_service_server::AdminService, control_service_client::ControlServiceClient,
        control_service_server::{ControlService, ControlServiceServer},
    },
    service::{
        map_storage_error_to_status, ControlServiceImpl, CoreSecretInterceptor, CORE_SECRET_HEADER,
    },
    snapshot::encode_session_checkpoint,
    state::ProjectionState,
};
use prost::Message;
use prost_types::Timestamp;
use tempfile::TempDir;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Channel, Code, Request};
use tonic_types::StatusExt;

const SECRET: &str = "test-core-secret";
const OPERATOR_ACTOR: &str = "operator-primary";
const PRINCIPAL_ID: &str = "web-principal";
const PRINCIPAL_SECRET: &str = "web-principal-secret";
const CONCURRENT_SUBMISSIONS: usize = 16;

#[derive(Clone)]
struct FailPostAppendReadOnceStorage {
    inner: RusqliteStorage,
    fail_post_append_read: bool,
    fail_next_read: Arc<AtomicBool>,
    fail_next_query_audit: Arc<AtomicBool>,
    fail_next_decision: Arc<AtomicBool>,
}

impl FailPostAppendReadOnceStorage {
    fn new(inner: RusqliteStorage) -> Self {
        Self {
            inner,
            fail_post_append_read: true,
            fail_next_read: Arc::new(AtomicBool::new(false)),
            fail_next_query_audit: Arc::new(AtomicBool::new(false)),
            fail_next_decision: Arc::new(AtomicBool::new(false)),
        }
    }

    fn failing_diagnostics_materialization(inner: RusqliteStorage) -> Self {
        Self {
            inner,
            fail_post_append_read: false,
            fail_next_read: Arc::new(AtomicBool::new(false)),
            fail_next_query_audit: Arc::new(AtomicBool::new(true)),
            fail_next_decision: Arc::new(AtomicBool::new(false)),
        }
    }

    fn failing_next_decision(inner: RusqliteStorage) -> Self {
        Self {
            inner,
            fail_post_append_read: false,
            fail_next_read: Arc::new(AtomicBool::new(false)),
            fail_next_query_audit: Arc::new(AtomicBool::new(false)),
            fail_next_decision: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl CoreGenerationStore for FailPostAppendReadOnceStorage {
    async fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> Result<Generation, StorageError> {
        self.inner
            .load_or_create_core_generation(authority_domain_id, candidate)
            .await
    }
}

impl Storage for FailPostAppendReadOnceStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        let outcome = self
            .inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await?;
        if self.fail_post_append_read && matches!(&outcome, DedupOutcome::Appended(_)) {
            self.fail_next_read.store(true, Ordering::SeqCst);
        }
        Ok(outcome)
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        if self.fail_next_read.swap(false, Ordering::SeqCst) {
            return Err(StorageError::ReadFailed {
                message: "injected catch-up read failure".to_owned(),
                retryable: true,
            });
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

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<EventId, StorageError> {
        self.inner.append_audit(authority_domain_id, audit).await
    }

    async fn append_decision(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<EventId, StorageError> {
        if self.fail_next_decision.swap(false, Ordering::SeqCst) {
            return Err(StorageError::WriteFailed { message: "injected decision failure".to_owned(), retryable: true });
        }
        self.inner
            .append_audited(authority_domain_id, source, audit)
            .await
            .map(|result| result.source_event_id)
    }

    async fn append_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        self.inner.append_audited(authority_domain_id, source, audit).await
    }

    async fn append_dedup_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<AuditedDedupOutcome, StorageError> {
        self.inner
            .append_dedup_audited(authority_domain_id, key, target, source, audit)
            .await
    }

    async fn query_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        spec: AuditPageSpec,
    ) -> Result<patchbay_contracts::patchbay::AuditPage, StorageError> {
        if self.fail_next_query_audit.swap(false, Ordering::SeqCst) {
            return Err(StorageError::ReadFailed {
                message: "injected diagnostics materialization failure".to_owned(),
                retryable: true,
            });
        }
        self.inner.query_audit(authority_domain_id, spec).await
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestAuth {
    session_id: String,
    session_generation: u64,
    principal_id: String,
    principal_secret: String,
}

struct TestServer {
    client: ControlServiceClient<Channel>,
    storage: RusqliteStorage,
    task: JoinHandle<()>,
    operator_session: TestAuth,
    _directory: TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn revoke_all_survives_restart_and_forces_a_higher_generation_login() {
    let mut server = start_server().await;
    let old_auth = server.operator_session.clone();
    let revoked = server
        .client
        .revoke_all_operator_sessions(authenticated_request(
            RevokeAllOperatorSessionsRequest {
                reason_code: "operator_requested".to_owned(),
            },
            SECRET,
            &old_auth,
        ))
        .await
        .expect("revoke-all must commit through the gRPC boundary")
        .into_inner();
    assert!(revoked.revoked_session_count > 0);
    assert_eq!(revoked.invalidated_through_generation.unwrap().value, old_auth.session_generation);

    server.task.abort();
    let (mut client, task, fresh_auth) = serve(server.storage.clone()).await;
    let old_session = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("old-after-restart", "old-after-restart")),
            },
            SECRET,
            &old_auth,
        ))
        .await
        .expect_err("opaque pre-restart session ids must not survive restart");
    assert_eq!(old_session.code(), Code::Unauthenticated);
    assert!(fresh_auth.session_generation > old_auth.session_generation);

    let accepted = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("fresh-after-restart", "fresh-after-restart")),
            },
            SECRET,
            &fresh_auth,
        ))
        .await
        .expect("fresh higher-generation login remains usable")
        .into_inner();
    assert_eq!(accepted.outcome, SubmissionOutcome::Accepted as i32);
    task.abort();
}

#[tokio::test]
async fn scope_revocations_reject_operations_and_subscriptions_before_acceptance() {
    let mut server = start_server().await;
    let first = server.operator_session.clone();
    let second = login_surface(&mut server.client, "second-endpoint", "second-device").await;
    let third = login_surface(&mut server.client, "third-endpoint", "third-device").await;
    let fourth = login_surface(&mut server.client, "fourth-endpoint", "fourth-device").await;

    let before_principal = operation_event_count(&server.storage).await;
    let principal_revocation = server
        .client
        .revoke_control_surface_principal(authenticated_request(
            RevokeControlSurfacePrincipalRequest {
                principal_id: first.principal_id.clone(),
                reason_code: "principal_lockdown".to_owned(),
            },
            SECRET,
            &second,
        ))
        .await
        .expect("a distinct endpoint can revoke the target principal")
        .into_inner();
    assert!(principal_revocation.revoked_session_count > 0);
    assert_eq!(operation_event_count(&server.storage).await, before_principal);
    let principal_repeat = server
        .client
        .revoke_control_surface_principal(authenticated_request(
            RevokeControlSurfacePrincipalRequest {
                principal_id: first.principal_id.clone(),
                reason_code: "principal_lockdown_repeat".to_owned(),
            },
            SECRET,
            &second,
        ))
        .await
        .expect("repeating principal revocation is idempotent")
        .into_inner();
    assert!(!principal_repeat.newly_revoked);
    assert_eq!(principal_repeat.revoked_session_count, 0);
    assert_scope_denied(&mut server.client, &first).await;
    assert_submit_accepted(&mut server.client, &second, "principal-unaffected").await;

    let before_endpoint = operation_event_count(&server.storage).await;
    let endpoint_revocation = server
        .client
        .revoke_control_surface_endpoint(authenticated_request(
            RevokeControlSurfaceEndpointRequest {
                reason_code: "endpoint_lockdown".to_owned(),
                target: Some(
                    patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::EndpointId(
                        EndpointId { value: "second-endpoint".to_owned() },
                    ),
                ),
            },
            SECRET,
            &third,
        ))
        .await
        .expect("a distinct endpoint can revoke the target endpoint")
        .into_inner();
    assert!(endpoint_revocation.revoked_session_count > 0);
    assert_eq!(operation_event_count(&server.storage).await, before_endpoint);
    let endpoint_repeat = server
        .client
        .revoke_control_surface_endpoint(authenticated_request(
            RevokeControlSurfaceEndpointRequest {
                reason_code: "endpoint_lockdown_repeat".to_owned(),
                target: Some(
                    patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::EndpointId(
                        EndpointId { value: "second-endpoint".to_owned() },
                    ),
                ),
            },
            SECRET,
            &third,
        ))
        .await
        .expect("repeating endpoint revocation is idempotent")
        .into_inner();
    assert!(!endpoint_repeat.newly_revoked);
    assert_eq!(endpoint_repeat.revoked_session_count, 0);
    assert_scope_denied(&mut server.client, &second).await;
    assert_submit_accepted(&mut server.client, &third, "endpoint-unaffected").await;

    let before_device = operation_event_count(&server.storage).await;
    let device_revocation = server
        .client
        .revoke_control_surface_endpoint(authenticated_request(
            RevokeControlSurfaceEndpointRequest {
                reason_code: "device_lockdown".to_owned(),
                target: Some(
                    patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::DeviceId(
                        DeviceId { value: "third-device".to_owned() },
                    ),
                ),
            },
            SECRET,
            &fourth,
        ))
        .await
        .expect("a distinct endpoint can revoke the target device")
        .into_inner();
    assert!(device_revocation.revoked_session_count > 0);
    assert_eq!(operation_event_count(&server.storage).await, before_device);
    assert_scope_denied(&mut server.client, &third).await;
    assert_submit_accepted(&mut server.client, &fourth, "device-unaffected").await;

    let missing_target = server
        .client
        .revoke_control_surface_principal(authenticated_request(
            RevokeControlSurfacePrincipalRequest {
                principal_id: "missing-principal".to_owned(),
                reason_code: "missing_target".to_owned(),
            },
            SECRET,
            &fourth,
        ))
        .await
        .expect_err("a valid issuer cannot revoke a missing target");
    assert_eq!(missing_target.code(), Code::NotFound);
    let denied_audits = server
        .storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::AuthorizationFailed],
                actor_id: None,
                endpoint_id: Some(EndpointId { value: "fourth-endpoint".to_owned() }),
                command_id: None,
                grant_id: None,
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["control_surface_principal_not_found".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .expect("denied target decisions must be queryable");
    assert_eq!(denied_audits.records.len(), 1);

    let principal_audits = server
        .storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::ControlSurfacePrincipalRevoked],
                actor_id: None,
                endpoint_id: Some(EndpointId { value: "second-endpoint".to_owned() }),
                command_id: None,
                grant_id: None,
                target: None,
                failure_codes: vec![],
                reason_codes: vec![],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .expect("repeated revocation audits must be queryable");
    assert_eq!(principal_audits.records.len(), 2);
    assert!(principal_audits.records.iter().all(|record| {
        record.endpoint_id.as_ref().map(|id| id.value.as_str()) == Some("second-endpoint")
            && record.target_scope.as_ref().map(|scope| scope.legacy_audit_resource_id.as_str())
                == Some(first.principal_id.as_str())
    }));

    let endpoint_audits = server
        .storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::ControlSurfaceEndpointRevoked],
                actor_id: None,
                endpoint_id: Some(EndpointId { value: "third-endpoint".to_owned() }),
                command_id: None,
                grant_id: None,
                target: None,
                failure_codes: vec![],
                reason_codes: vec![],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .expect("endpoint revocation attribution must be queryable");
    assert_eq!(endpoint_audits.records.len(), 2);
    assert!(endpoint_audits.records.iter().all(|record| {
        record.endpoint_id.as_ref().map(|id| id.value.as_str()) == Some("third-endpoint")
            && record.target_scope.as_ref().map(|scope| scope.legacy_audit_resource_id.as_str())
                == Some("second-endpoint")
    }));

    let mut auth_failure = authenticated_request(
        RevokeControlSurfaceEndpointRequest {
            reason_code: "auth_failure".to_owned(),
            target: Some(
                patchbay_contracts::patchbay::revoke_control_surface_endpoint_request::Target::EndpointId(
                    EndpointId { value: "fourth-endpoint".to_owned() },
                ),
            ),
        },
        SECRET,
        &fourth,
    );
    auth_failure.metadata_mut().insert(
        PRINCIPAL_SECRET_HEADER,
        "wrong-principal-secret".parse().expect("metadata is valid"),
    );
    let authentication_failure = server
        .client
        .revoke_control_surface_endpoint(auth_failure)
        .await
        .expect_err("invalid principal credentials must fail closed");
    assert_eq!(authentication_failure.code(), Code::Unauthenticated);
    let authentication_audits = server
        .storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::LoginFailed],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: None,
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["transport_principal_authentication_failed".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .expect("authentication failures must be queryable");
    assert!(!authentication_audits.records.is_empty());
}

#[tokio::test]
async fn stale_issuer_is_rejected_when_revocation_commits_before_acceptance() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let storage = RusqliteStorage::open(
        directory
            .path()
            .join("patchbay.sqlite3")
            .to_str()
            .expect("UTF-8 test path"),
    )
    .expect("test storage must open");
    seed_authority_and_session(&storage).await;
    let decision_gate = CoreDecisionGate::default();
    let (mut client, task, auth) = serve_with_gate(storage.clone(), decision_gate.clone()).await;

    let gate_guard = decision_gate.acquire().await;
    let revoked_principal_id = auth.principal_id.clone();
    let submit = tokio::spawn(async move {
        client
            .submit(authenticated_request(
                SubmitRequest {
                    operation: Some(operation("stale-issuer", "stale-issuer-key")),
                },
                SECRET,
                &auth,
            ))
            .await
    });
    tokio::task::yield_now().await;

    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::ControlSurfaceRevocation as i32,
                payload: ControlSurfaceRevocation {
                    authority_domain_id: Some(domain()),
                    verified_revoker: Some(ActorEndpointRef {
                        actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
                        endpoint_id: Some(EndpointId { value: "revoker-endpoint".to_owned() }),
                        device_id: Some(DeviceId { value: "revoker-device".to_owned() }),
                        endpoint_generation: Some(Generation { value: 1 }),
                    }),
                    occurred_at: Some(Timestamp { seconds: 3, nanos: 0 }),
                    reason_code: "race_test".to_owned(),
                    target: Some(
                        patchbay_contracts::patchbay::control_surface_revocation::Target::PrincipalId(
                            revoked_principal_id,
                        ),
                    ),
                }
                .encode_to_vec(),
            },
        )
        .await
        .expect("revocation must commit while the request waits for acceptance");
    drop(gate_guard);

    let error = submit
        .await
        .expect("submit task must complete")
        .expect_err("the cached pre-revocation issuer must not submit");
    assert_eq!(error.code(), Code::Unauthenticated);
    task.abort();
}

#[tokio::test]
async fn grpc_seam_submits_streams_and_loads_snapshots() {
    let mut server = start_server().await;

    let unauthorized = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("unauthorized", "unauthorized")),
            },
            "wrong-secret",
            &server.operator_session,
        ))
        .await
        .expect_err("an invalid core secret must fail closed");
    assert_eq!(unauthorized.code(), Code::Unauthenticated);

    let mut missing_actor = authenticated_request(
        SubmitRequest {
            operation: Some(operation("missing-actor", "missing-actor")),
        },
        SECRET,
        &server.operator_session,
    );
    missing_actor.metadata_mut().remove(OPERATOR_ID_HEADER);
    let missing_actor = server
        .client
        .submit(missing_actor)
        .await
        .expect_err("operator actor metadata must be present");
    assert_eq!(missing_actor.code(), Code::Unauthenticated);

    let accepted = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("command-1", "key-1")),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("authorized submit must complete")
        .into_inner();
    assert_eq!(
        SubmissionOutcome::try_from(accepted.outcome).expect("known outcome"),
        SubmissionOutcome::Accepted
    );
    assert_eq!(
        OperationState::try_from(accepted.operation_state).expect("known state"),
        OperationState::Accepted
    );

    let mut subscription = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: Some(Lsn { value: 0 }),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("subscribe must establish")
        .into_inner();
    let mut saw_operation = false;
    while let Some(event) = subscription
        .message()
        .await
        .expect("stream message must decode")
    {
        let kind = event.payload.map(|payload| payload.kind);
        saw_operation |= kind == Some(StoredEventKind::Operation as i32);
    }
    assert!(
        saw_operation,
        "cursor-zero replay must include the operation"
    );

    let mut resumed = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: accepted.accepted_lsn,
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("cursor resume must establish")
        .into_inner();
    assert!(
        resumed
            .message()
            .await
            .expect("resumed stream must decode")
            .is_none(),
        "subscribe must return only events with LSN greater than the cursor and complete"
    );

    for invalid_view in [SnapshotViewKind::Unspecified as i32, 99] {
        let error = server
            .client
            .load_snapshot(authenticated_request(
                LoadSnapshotRequest {
                    authority_domain_id: Some(domain()),
                    at_or_before: None,
                    view_kind: invalid_view,
                },
                SECRET,
                &server.operator_session,
            ))
            .await
            .expect_err("unspecified/unknown snapshot view rejects");
        assert_eq!(error.code(), Code::InvalidArgument);
    }

    let materialized_snapshot = server
        .client
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: None,
                view_kind: patchbay_contracts::patchbay::SnapshotViewKind::Session as i32,
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("snapshot lookup must complete")
        .into_inner();
    // The service used to report `present: false` because production never
    // wrote durable checkpoints. A missing durable checkpoint now falls back
    // to the authoritative rebuilt session projection.
    assert!(materialized_snapshot.present);
    assert_eq!(
        materialized_snapshot.view_kind,
        patchbay_contracts::patchbay::SnapshotViewKind::Session as i32
    );
    let materialized = SessionSnapshot::decode(materialized_snapshot.snapshot_payload.as_slice())
        .expect("read-materialized snapshot must be valid protobuf");
    assert_eq!(materialized.authority_domain_id, Some(domain()));
    assert_eq!(materialized.sessions.len(), 1);
    assert_eq!(
        materialized.sessions[0]
            .runtime_session_id
            .as_ref()
            .map(|id| id.value.as_str()),
        Some("session-1")
    );
    assert_eq!(
        materialized.snapshot_lsn,
        materialized_snapshot
            .event_id
            .as_ref()
            .and_then(|event_id| event_id.lsn)
    );
    assert!(materialized.snapshot_lsn.as_ref().unwrap().value >= accepted.accepted_lsn.as_ref().unwrap().value);
    assert_eq!(
        materialized.sessions[0].state,
        Some(SessionState {
            connectivity: SessionConnectivityState::Live as i32,
            activity: SessionActivityState::Idle as i32,
        })
    );

    let accepted_lsn = accepted
        .accepted_lsn
        .expect("accepted result must carry LSN");
    server
        .storage
        .write_snapshot(&domain(), accepted_lsn, b"snapshot-v1".to_vec())
        .await
        .expect("snapshot fixture must write");
    let loaded_snapshot = server
        .client
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: None,
                view_kind: patchbay_contracts::patchbay::SnapshotViewKind::Session as i32,
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("latest snapshot lookup must complete")
        .into_inner();
    assert!(loaded_snapshot.present);
    assert_eq!(
        loaded_snapshot.view_kind,
        patchbay_contracts::patchbay::SnapshotViewKind::Session as i32
    );
    assert_ne!(loaded_snapshot.snapshot_payload, b"snapshot-v1");
    SessionSnapshot::decode(loaded_snapshot.snapshot_payload.as_slice())
        .expect("corrupt/older checkpoint is repaired by the current session projection");

    let resource_identity = patchbay_contracts::patchbay::ResourceIdentity {
        adapter_id: Some(AdapterId { value: "resource-adapter".into() }),
        resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
        resource_id: Some(ResourceId { value: "pool-1".into() }),
    };
    let adapter_service = AdapterControlServiceImpl::new(
        server.storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new("resource-evidence").unwrap(),
    )
    .await
    .unwrap();
    let attached = adapter_service
        .attach(Request::new(AttachRequest {
            registration: Some(AdapterRegistration {
                adapter_id: Some(AdapterId { value: "resource-adapter".into() }),
                endpoint_id: Some(EndpointId { value: "resource-endpoint".into() }),
                authority_domain_id: Some(domain()),
                adapter_generation: Some(Generation { value: 3 }),
                capability: Some(AdapterCapability {
                    target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
                    resource_capabilities: vec![ResourceCapability {
                        resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                        snapshot_support: AdapterSnapshotSupport::Partial as i32,
                        projection_contract: Some(ResourceProjectionContract {
                            target_category: AdapterTargetCategory::OperationalResource as i32,
                            payload_schema: Some(SchemaDescriptor {
                                schema_ref: "provider_pool.payload.v1".into(),
                                content_type: PayloadContentType::Protobuf as i32,
                            }),
                            projection_schema: Some(SchemaDescriptor {
                                schema_ref: "provider_pool.projection.v1".into(),
                                content_type: PayloadContentType::Json as i32,
                            }),
                        }),
                    }],
                    ..AdapterCapability::default()
                }),
                ..AdapterRegistration::default()
            }),
            attachment_evidence: b"resource-evidence".to_vec(),
        }))
        .await
        .expect("resource adapter attaches");
    let attachment_token = attached
        .metadata()
        .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let mut report = Request::new(ObservationRequest {
        authority_domain_id: Some(domain()),
        observation: Some(observation_request::Observation::ResourceReport(ResourceReport {
            adapter_id: Some(AdapterId { value: "resource-adapter".into() }),
            adapter_generation: Some(Generation { value: 3 }),
            report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
                views: vec![ResourceViewReport {
                    resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                    completeness: AdapterSnapshotSupport::Partial as i32,
                    mutations: vec![ResourceReportMutation {
                        identity: Some(resource_identity.clone()),
                        mutation: Some(resource_report_mutation::Mutation::Upsert(
                            ResourceStateUpsert {
                                resource_payload: Some(PayloadEnvelope {
                                    payload: vec![1],
                                    content_type: PayloadContentType::Protobuf as i32,
                                    schema_ref: "provider_pool.payload.v1".into(),
                                }),
                                projection_payload: Some(PayloadEnvelope {
                                    payload: br#"{\"state\":\"ok\"}"#.to_vec(),
                                    content_type: PayloadContentType::Json as i32,
                                    schema_ref: "provider_pool.projection.v1".into(),
                                }),
                            },
                        )),
                    }],
                }],
            })),
            observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
        })),
    });
    report.metadata_mut().insert(ADAPTER_ID_HEADER, "resource-adapter".parse().unwrap());
    report.metadata_mut().insert(ADAPTER_EVIDENCE_HEADER, "resource-evidence".parse().unwrap());
    report.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        attachment_token.parse().unwrap(),
    );
    adapter_service
        .ingest_observation(report)
        .await
        .expect("authenticated resource report appends");

    let restarted = ProjectionState::rebuild(&server.storage, &domain())
        .await
        .expect("resource projection survives restart");
    assert_eq!(
        TargetResolver::resolve(
            restarted.target_resolver(),
            &domain(),
            OperationKind::Query,
            &TargetScope {
                kind: TargetScopeKind::Resource as i32,
                resource: Some(resource_identity.clone()),
                ..TargetScope::default()
            },
        )
        .await,
        Ok(TargetBinding::Resource(
            patchbay_core::resource::ResourceIdentity::try_from_wire(&resource_identity).unwrap(),
        ))
    );
    let resource_response = server
        .client
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: Some(Lsn { value: 1 }),
                view_kind: SnapshotViewKind::Resource as i32,
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("resource snapshot loads")
        .into_inner();
    assert_eq!(resource_response.view_kind, SnapshotViewKind::Resource as i32);
    let resources = ResourceSnapshot::decode(resource_response.snapshot_payload.as_slice())
        .expect("resource response contains ResourceSnapshot");
    assert_eq!(resources.resources.len(), 1);
    assert_eq!(resources.resources[0].identity, Some(resource_identity));
    assert_eq!(
        resources.resources[0].freshness,
        ResourceFreshnessState::Current as i32
    );
    assert_eq!(
        resources.snapshot_lsn,
        resource_response.event_id.and_then(|event| event.lsn)
    );
}

#[tokio::test]
async fn diagnostics_query_uses_query_lifecycle_and_replays_result() {
    let mut server = start_audited_server().await;
    let query = DiagnosticsQuery {
        query: Some(diagnostics_query::Query::Audit(AuditQuery {
            limit: Some(1),
            ..AuditQuery::default()
        })),
    };
    let request = QueryDiagnosticsRequest {
        operation: Some(diagnostic_query_operation("diagnostic-query", "diagnostic-query-key", query)),
    };
    let first = server
        .client
        .query_diagnostics(authenticated_request(
            request.clone(),
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("authorized diagnostics query must complete")
        .into_inner();
    let submission = first.submission.expect("query has submission");
    assert_eq!(submission.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(submission.operation_state, OperationState::Completed as i32);
    let result_event_id = first.result_event_id.clone().expect("query has durable result");
    assert!(matches!(
        first.result,
        Some(query_diagnostics_response::Result::Audit(_))
    ));

    let retry = server
        .client
        .query_diagnostics(authenticated_request(
            request,
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("exact query retry must complete")
        .into_inner();
    assert_eq!(retry.result_event_id, Some(result_event_id));
    assert!(matches!(
        retry.result,
        Some(query_diagnostics_response::Result::Audit(_))
    ));
}

#[tokio::test]
async fn lockdown_entry_decision_failure_leaves_posture_and_operations_usable() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let inner = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&inner).await;
    let storage = FailPostAppendReadOnceStorage::failing_next_decision(inner);
    let (mut client, task, auth) = serve(storage).await;

    let enter = client
        .enter_security_lockdown(authenticated_request(
            EnterSecurityLockdownRequest {
                authority_domain_id: Some(domain()),
                reason_code: "injected_write_failure".to_owned(),
            },
            SECRET,
            &auth,
        ))
        .await
        .expect_err("failed source/audit transaction must not report entry success");
    assert_eq!(enter.code(), Code::Unavailable);

    let submitted = client
        .submit(authenticated_request(
            SubmitRequest { operation: Some(operation("post-failure-command", "post-failure-key")) },
            SECRET,
            &auth,
        ))
        .await
        .expect("entry failure must leave the old authority usable")
        .into_inner();
    assert_eq!(submitted.outcome, SubmissionOutcome::Accepted as i32);
    task.abort();
}

#[tokio::test]
async fn lockdown_exit_decision_failure_keeps_the_posture_active() {
    let inner = RusqliteStorage::open_in_memory().expect("storage opens");
    seed_authority_and_session(&inner).await;
    let storage = FailPostAppendReadOnceStorage::new(inner);
    let control = ControlServiceImpl::new_with_security(
        storage.clone(),
        domain(),
        Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await
    .expect("control service initializes");
    let login = control
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId { value: "exit-test".to_owned() }),
                device_id: Some(DeviceId { value: "exit-host".to_owned() }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .expect("login succeeds")
        .into_inner();
    let principal = login.principal.expect("login principal");
    let auth = TestAuth {
        session_id: login.operator_session_id.expect("login session").value,
        session_generation: login.operator_session_generation.expect("login generation").value,
        principal_id: principal.principal_id,
        principal_secret: principal.secret,
    };
    control
        .enter_security_lockdown(authenticated_request(
            EnterSecurityLockdownRequest { authority_domain_id: Some(domain()), reason_code: "exit_failure".to_owned() },
            SECRET,
            &auth,
        ))
        .await
        .expect("entry succeeds before exit failure injection");
    storage.fail_next_decision.store(true, Ordering::SeqCst);
    let admin = AdminServiceImpl::new(
        control.clone(),
        SetupSecret::new("unused".to_owned(), Duration::from_secs(60)),
    );
    let exit = admin
        .exit_security_lockdown(Request::new(ExitSecurityLockdownRequest {
            authority_domain_id: Some(domain()),
            reason_code: "exit_failure".to_owned(),
        }))
        .await
        .expect_err("failed exit source/audit transaction must not reopen operations");
    assert_eq!(exit.code(), Code::Unavailable);
    assert!(control.projection_state().lockdown_state().await.active);
}

#[tokio::test]
async fn lockdown_and_submit_race_is_ordered_by_the_shared_decision_gate() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let storage = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&storage).await;
    let (client, task, auth) = serve_with_gate(storage.clone(), CoreDecisionGate::default()).await;
    let mut entry_client = client.clone();
    let mut submit_client = client.clone();
    let entry_auth = auth.clone();
    let submit_auth = auth.clone();
    let (entry, submit) = tokio::join!(
        entry_client.enter_security_lockdown(authenticated_request(
            EnterSecurityLockdownRequest {
                authority_domain_id: Some(domain()),
                reason_code: "race_entry".to_owned(),
            },
            SECRET,
            &entry_auth,
        )),
        submit_client.submit(authenticated_request(
            SubmitRequest { operation: Some(operation("race-command", "race-key")) },
            SECRET,
            &submit_auth,
        )),
    );
    let entry = entry.expect("lockdown writer must not fail in the race").into_inner();
    assert!(entry.lockdown.expect("entry posture").active);
    match submit {
        Ok(response) => {
            let result = response.into_inner();
            assert!(
                result.outcome == SubmissionOutcome::Accepted as i32
                    || (result.outcome == SubmissionOutcome::Rejected as i32
                        && result.reason_code == "security_lockdown_active"),
                "submit must either commit before entry or receive the canonical lockdown rejection"
            );
        }
        Err(status) => assert_eq!(status.code(), Code::Unauthenticated),
    }
    let events = storage.read_after(&domain(), Lsn { value: 0 }).await.expect("race log is readable");
    let entry_lsn = events
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::SecurityLockdown as i32)
        .and_then(|event| event.event_id.lsn.as_ref())
        .expect("lockdown source event")
        .value;
    if let Some(operation_lsn) = events
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .and_then(|event| event.event_id.lsn.as_ref())
        .map(|lsn| lsn.value)
    {
        assert!(operation_lsn < entry_lsn, "accepted submit must commit before lockdown");
    }
    task.abort();
}

#[tokio::test]
async fn lockdown_rejects_every_operation_kind_and_query_retry_before_typed_validation() {
    let mut server = start_audited_server().await;
    let exact_query = QueryDiagnosticsRequest {
        operation: Some(diagnostic_query_operation(
            "lockdown-query-retry",
            "lockdown-query-retry-key",
            DiagnosticsQuery {
                query: Some(diagnostics_query::Query::Audit(AuditQuery { limit: Some(1), ..Default::default() })),
            },
        )),
    };
    let before = server
        .client
        .query_diagnostics(authenticated_request(exact_query.clone(), SECRET, &server.operator_session))
        .await
        .expect("query before lockdown must complete")
        .into_inner();
    assert_eq!(before.submission.expect("query submission").outcome, SubmissionOutcome::Accepted as i32);

    let mut enter = Request::new(patchbay_contracts::patchbay::EnterSecurityLockdownRequest {
        authority_domain_id: Some(domain()),
        reason_code: "test_lockdown".to_owned(),
    });
    // The helper uses the same compound issuer headers as Submit.
    enter.metadata_mut().insert(CORE_SECRET_HEADER, SECRET.parse().unwrap());
    enter.metadata_mut().insert(OPERATOR_SESSION_HEADER, server.operator_session.session_id.parse().unwrap());
    enter.metadata_mut().insert(OPERATOR_ID_HEADER, OPERATOR_ACTOR.parse().unwrap());
    enter.metadata_mut().insert(PRINCIPAL_ID_HEADER, server.operator_session.principal_id.parse().unwrap());
    enter.metadata_mut().insert(PRINCIPAL_SECRET_HEADER, server.operator_session.principal_secret.parse().unwrap());
    server
        .client
        .enter_security_lockdown(enter)
        .await
        .expect("lockdown entry must be authorized");

    let fresh = login_surface(&mut server.client, "lockdown-read", "lockdown-host").await;
    let malformed = QueryDiagnosticsRequest {
        operation: Some(diagnostic_query_operation(
            "lockdown-malformed-query",
            "lockdown-malformed-query-key",
            DiagnosticsQuery::default(),
        )),
    };
    let malformed_result = server
        .client
        .query_diagnostics(authenticated_request(malformed, SECRET, &fresh))
        .await
        .expect("lockdown rejection is a protocol result")
        .into_inner()
        .submission
        .expect("malformed query has a rejection result");
    assert_eq!(malformed_result.failure_code, FailureCode::AuthorizationDenied as i32);
    assert_eq!(malformed_result.reason_code, "security_lockdown_active");

    let retry_result = server
        .client
        .query_diagnostics(authenticated_request(exact_query, SECRET, &fresh))
        .await
        .expect("exact query retry is rejected while locked")
        .into_inner()
        .submission
        .expect("retry has a rejection result");
    assert_eq!(retry_result.failure_code, FailureCode::AuthorizationDenied as i32);
    assert_eq!(retry_result.reason_code, "security_lockdown_active");

    let kinds = [
        OperationKind::Spawn,
        OperationKind::Attach,
        OperationKind::Instruct,
        OperationKind::Cancel,
        OperationKind::Interrupt,
        OperationKind::Query,
        OperationKind::ApprovalResponse,
        OperationKind::ElicitationResponse,
        OperationKind::Reconfigure,
        OperationKind::SessionManagement,
    ];
    for (index, kind) in kinds.into_iter().enumerate() {
        let mut candidate = operation(&format!("lockdown-kind-{index}"), &format!("lockdown-kind-key-{index}"));
        candidate.kind = kind as i32;
        let result = server
            .client
            .submit(authenticated_request(SubmitRequest { operation: Some(candidate) }, SECRET, &fresh))
            .await
            .expect("lockdown operation rejection is a protocol result")
            .into_inner();
        assert_eq!(result.outcome, SubmissionOutcome::Rejected as i32, "{kind:?}");
        assert_eq!(result.failure_code, FailureCode::AuthorizationDenied as i32, "{kind:?}");
        assert_eq!(result.reason_code, "security_lockdown_active", "{kind:?}");
    }
    assert_eq!(operation_event_count(&server.storage).await, 1, "lockdown rejects must append no command events");
}

#[tokio::test]
async fn diagnostics_materialization_failure_terminalizes_and_retry_reconciles() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let inner = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&inner).await;
    let storage = FailPostAppendReadOnceStorage::failing_diagnostics_materialization(inner.clone());
    let (mut client, task, operator_session) = serve(storage).await;
    let query = DiagnosticsQuery {
        query: Some(diagnostics_query::Query::Audit(AuditQuery {
            limit: Some(1),
            ..AuditQuery::default()
        })),
    };
    let request = QueryDiagnosticsRequest {
        operation: Some(diagnostic_query_operation(
            "diagnostics-materialization-failure",
            "diagnostics-materialization-failure-key",
            query,
        )),
    };

    let first = client
        .query_diagnostics(authenticated_request(
            request.clone(),
            SECRET,
            &operator_session,
        ))
        .await
        .expect("materialization failure must remain a durable query result")
        .into_inner();
    let first_submission = first.submission.expect("failed query has submission");
    assert_eq!(first_submission.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(first_submission.operation_state, OperationState::Failed as i32);
    assert_eq!(first_submission.failure_code, FailureCode::ExecutionFailed as i32);

    let transitions = inner
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("durable query log must remain readable")
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
        .map(|event| CommandTransition::decode(event.payload.payload.as_slice()).expect("transition must decode"))
        .filter(|transition| {
            transition
                .command_id
                .as_ref()
                .is_some_and(|id| id.value == "diagnostics-materialization-failure")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        transitions.iter().map(|transition| transition.to_state).collect::<Vec<_>>(),
        vec![
            OperationState::Delivered as i32,
            OperationState::Failed as i32,
        ],
        "materialization failure must durably terminalize the delivered query",
    );
    assert_eq!(
        transitions[1].failure_code,
        FailureCode::ExecutionFailed as i32,
        "the transition uses the canonical accepted-operation failure code",
    );

    let failure_audit = inner
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::CommandFailed],
                actor_id: None,
                endpoint_id: None,
                command_id: Some(CommandId {
                    value: "diagnostics-materialization-failure".to_owned(),
                }),
                grant_id: None,
                target: None,
                failure_codes: vec![FailureCode::ExecutionFailed],
                reason_codes: vec!["diagnostics_materialization_failed".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .expect("failure audit must be queryable");
    assert_eq!(failure_audit.records.len(), 1);
    let failure_audit_record = &failure_audit.records[0];
    assert_eq!(failure_audit_record.reason_code, "diagnostics_materialization_failed");
    assert_eq!(failure_audit_record.failure_code, FailureCode::ExecutionFailed as i32);
    assert!(failure_audit_record.correlation_id.is_empty());

    let retry = client
        .query_diagnostics(authenticated_request(
            request,
            SECRET,
            &operator_session,
        ))
        .await
        .expect("retry must reconcile the durable failed terminal state")
        .into_inner();
    let retry_submission = retry.submission.expect("retry has submission");
    assert_eq!(retry_submission.outcome, SubmissionOutcome::Accepted as i32);
    assert!(retry_submission.deduplicated);
    assert_eq!(retry_submission.operation_state, OperationState::Failed as i32);
    assert_eq!(retry_submission.failure_code, FailureCode::ExecutionFailed as i32);
    assert!(retry.result.is_none(), "a failed query has no materialized result");

    task.abort();
}

#[tokio::test]
async fn grant_subject_uses_verified_actor_not_operator_session() {
    let mut server = start_server().await;
    assert_ne!(server.operator_session.session_id, OPERATOR_ACTOR);

    let result = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("actor-bound-command", "actor-bound-key")),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("a grant bound to the verified actor must authorize the opaque session")
        .into_inner();

    assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
    assert!(!result.deduplicated);
}

#[tokio::test]
async fn revoke_grant_rpc_is_authorized_durable_and_audited() {
    let mut server = start_server().await;

    let missing = server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "missing-grant".to_owned() }),
                reason: "test_missing".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect_err("missing grants must be denied");
    assert_eq!(missing.code(), Code::PermissionDenied);

    server
        .storage
        .append(
            &domain(),
            authority_events::grant(
                domain(),
                Grant {
                    grant_id: Some(GrantId { value: "foreign-grant".to_owned() }),
                    authority_domain_id: Some(domain()),
                    subject_actor_id: Some(ActorId { value: "another-actor".to_owned() }),
                    target_scope: Some(target_scope()),
                    allowed_operation_kinds: vec![OperationKind::Cancel as i32],
                    provenance: Some(GrantProvenance { reason: "foreign fixture".to_owned(), ..GrantProvenance::default() }),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    ..Grant::default()
                },
            ),
        )
        .await
        .expect("foreign grant fixture must append");
    let foreign = server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "foreign-grant".to_owned() }),
                reason: "test_foreign".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect_err("foreign grants must be denied");
    assert_eq!(foreign.code(), Code::PermissionDenied);

    server
        .storage
        .append(
            &domain(),
            authority_events::grant(
                domain(),
                Grant {
                    grant_id: Some(GrantId { value: "endpoint-grant".to_owned() }),
                    authority_domain_id: Some(domain()),
                    subject_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
                    subject_endpoint_id: Some(EndpointId { value: "different-endpoint".to_owned() }),
                    target_scope: Some(target_scope()),
                    allowed_operation_kinds: vec![OperationKind::Cancel as i32],
                    provenance: Some(GrantProvenance { reason: "endpoint fixture".to_owned(), ..GrantProvenance::default() }),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    ..Grant::default()
                },
            ),
        )
        .await
        .expect("endpoint grant fixture must append");
    let endpoint = server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "endpoint-grant".to_owned() }),
                reason: "test_endpoint".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect_err("endpoint-mismatched grants must be denied");
    assert_eq!(endpoint.code(), Code::PermissionDenied);

    let denied_audits = server
        .storage
        .query_audit(&domain(), AuditPageSpec {
            kinds: vec![AuditEventKind::AuthorizationFailed],
            actor_id: None,
            endpoint_id: None,
            command_id: None,
            grant_id: None,
            target: None,
            failure_codes: vec![],
            reason_codes: vec!["grant_revocation_authorization_denied".to_owned()],
            occurred_from: None,
            occurred_before: None,
            before_lsn: None,
            limit: 50,
        })
        .await
        .expect("denied revocation must be queryable");
    assert_eq!(denied_audits.records.len(), 3);
    assert!(denied_audits.records.iter().all(|record| record.grant_id.is_none()));

    let revoked = server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "operator-grant".to_owned() }),
                reason: "test_revocation".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("the verified subject may revoke its grant")
        .into_inner();
    assert!(revoked.changed);
    assert!(!revoked.already_revoked);
    assert_eq!(revoked.applied_policy, GrantRevocationPolicy::Continue as i32);
    assert!(revoked.revocation_event_id.is_some());
    let success_audits = server
        .storage
        .query_audit(&domain(), AuditPageSpec {
            kinds: vec![AuditEventKind::GrantRevoked],
            actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
            endpoint_id: Some(EndpointId { value: "grpc-smoke-login".to_owned() }),
            command_id: None,
            grant_id: Some(GrantId { value: "operator-grant".to_owned() }),
            target: None,
            failure_codes: vec![],
            reason_codes: vec!["grant_revoked".to_owned()],
            occurred_from: None,
            occurred_before: None,
            before_lsn: None,
            limit: 10,
        })
        .await
        .expect("successful revocation audit must be queryable");
    assert_eq!(success_audits.records.len(), 1);

    let rejected = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("after-revocation", "after-revocation-key")),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("authorization denial is a typed submission result")
        .into_inner();
    assert_eq!(rejected.outcome, SubmissionOutcome::Rejected as i32);
    assert_eq!(rejected.failure_code, FailureCode::AuthorizationDenied as i32);
}

#[tokio::test]
async fn last_recovery_authority_grant_refusal_is_typed_and_audited() {
    let mut server = start_server().await;
    let error = server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "operator-lockdown-grant".to_owned() }),
                reason: "test_last_recovery_grant".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect_err("the last recovery-capable authority grant must be retained");
    assert_eq!(error.code(), Code::FailedPrecondition);
    assert!(error.message().contains("last_recovery_authority_grant"));

    let audits = server
        .storage
        .query_audit(&domain(), AuditPageSpec {
            kinds: vec![AuditEventKind::AuthorizationFailed],
            actor_id: None,
            endpoint_id: None,
            command_id: None,
            grant_id: Some(GrantId { value: "operator-lockdown-grant".to_owned() }),
            target: None,
            failure_codes: vec![FailureCode::AuthorizationDenied],
            reason_codes: vec!["last_recovery_authority_grant".to_owned()],
            occurred_from: None,
            occurred_before: None,
            before_lsn: None,
            limit: 10,
        })
        .await
        .expect("last-grant refusal must be queryable");
    assert_eq!(audits.records.len(), 1);
}

#[tokio::test]
async fn grant_expiry_is_enforced_at_rpc_boundary_and_audited() {
    let storage = RusqliteStorage::open_in_memory().expect("test storage must open");
    seed_authority_and_session(&storage).await;
    storage
        .append(
            &domain(),
            authority_events::grant(
                domain(),
                Grant {
                    grant_id: Some(GrantId { value: "expired-grant".to_owned() }),
                    authority_domain_id: Some(domain()),
                    subject_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
                    target_scope: Some(target_scope()),
                    allowed_operation_kinds: vec![OperationKind::Cancel as i32],
                    expires_at: Some(Timestamp { seconds: 100, nanos: 0 }),
                    provenance: Some(GrantProvenance { reason: "expired grant fixture".to_owned(), ..GrantProvenance::default() }),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    ..Grant::default()
                },
            ),
        )
        .await
        .expect("expired grant fixture must append");
    let clock: Arc<dyn Clock> = Arc::new(TestClock::new(Timestamp { seconds: 100, nanos: 0 }));
    let (mut client, task, operator_session) = serve_with_clock(storage.clone(), clock).await;
    let before = operation_event_count(&storage).await;

    let mut expired_operation = operation("expired-grant-command", "expired-grant-key");
    expired_operation.kind = OperationKind::Cancel as i32;
    let result = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(expired_operation),
            },
            SECRET,
            &operator_session,
        ))
        .await
        .expect("expired grant is a typed rejection")
        .into_inner();
    assert_eq!(result.outcome, SubmissionOutcome::Rejected as i32);
    assert_eq!(result.failure_code, FailureCode::Expired as i32);
    assert_eq!(result.reason_code, "grant_expired");
    assert_eq!(result.decision_grant_id.as_ref().map(|id| id.value.as_str()), Some("expired-grant"));
    assert_eq!(operation_event_count(&storage).await, before);

    let audits = storage
        .query_audit(&domain(), AuditPageSpec {
            kinds: vec![AuditEventKind::GrantExpired],
            actor_id: None,
            endpoint_id: None,
            command_id: None,
            grant_id: Some(GrantId { value: "expired-grant".to_owned() }),
            target: None,
            failure_codes: vec![],
            reason_codes: vec!["grant_expired".to_owned()],
            occurred_from: None,
            occurred_before: None,
            before_lsn: None,
            limit: 50,
        })
        .await
        .expect("expired grant audit must be queryable");
    assert_eq!(audits.records.len(), 1);
    task.abort();
}

#[tokio::test]
async fn subscribe_denies_initial_and_resume_establishment_after_query_grant_revocation() {
    let mut server = start_server().await;
    let accepted = server
        .client
        .submit(authenticated_request(
            SubmitRequest { operation: Some(operation("subscription-cursor", "subscription-cursor-key")) },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("fixture command must be accepted")
        .into_inner();

    let mut initial = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest { authority_domain_id: Some(domain()), cursor: Some(Lsn { value: 0 }) },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("live Query grant establishes the initial stream")
        .into_inner();
    while initial.message().await.expect("initial stream must decode").is_some() {}

    server
        .client
        .revoke_grant(authenticated_request(
            RevokeGrantRequest {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId { value: "operator-query-grant".to_owned() }),
                reason: "test_query_revocation".to_owned(),
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect("the verified subject may revoke its Query grant");

    let resumed = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: accepted.accepted_lsn,
            },
            SECRET,
            &server.operator_session,
        ))
        .await
        .expect_err("resume must re-check the now-revoked Query grant");
    assert_eq!(resumed.code(), Code::PermissionDenied);
}

#[tokio::test]
async fn rpc_rejects_expired_and_not_yet_valid_operations_without_append() {
    let mut server = start_server().await;
    let operation_count_before = operation_event_count(&server.storage).await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_secs() as i64;

    let mut expired = operation("expired-command", "expired-key");
    expired.validity_window = Some(TimeWindow {
        starts_at: Some(Timestamp {
            seconds: now - 60,
            nanos: 0,
        }),
        expires_at: Some(Timestamp {
            seconds: now - 1,
            nanos: 0,
        }),
    });
    expired.submitted_at = Some(Timestamp {
        seconds: now - 30,
        nanos: 0,
    });

    let mut not_yet_valid = operation("future-command", "future-key");
    not_yet_valid.validity_window = Some(TimeWindow {
        starts_at: Some(Timestamp {
            seconds: now + 60,
            nanos: 0,
        }),
        expires_at: Some(Timestamp {
            seconds: now + 120,
            nanos: 0,
        }),
    });
    not_yet_valid.submitted_at = Some(Timestamp {
        seconds: now + 60,
        nanos: 0,
    });

    for (operation, expected_failure) in [
        (expired, FailureCode::Expired),
        (not_yet_valid, FailureCode::ValidationFailed),
    ] {
        let result = server
            .client
            .submit(authenticated_request(
                SubmitRequest {
                    operation: Some(operation),
                },
                SECRET,
                &server.operator_session,
            ))
            .await
            .expect("validity rejection is an RPC submission result")
            .into_inner();
        assert_eq!(result.outcome, SubmissionOutcome::Rejected as i32);
        assert_eq!(result.failure_code, expected_failure as i32);
        assert!(result.accepted_lsn.is_none());
    }

    assert_eq!(
        operation_event_count(&server.storage).await,
        operation_count_before,
        "invalid windows must not append or become delivery candidates"
    );
}

#[tokio::test]
async fn core_generation_checkpoint_survives_reopen_and_mismatch_repairs() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("checkpoint-restart.sqlite3");
    let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    seed_authority_and_session(&storage).await;
    let service = ControlServiceImpl::new(storage.clone(), domain()).await.unwrap();
    let checkpoint = service
        .projection_state()
        .materialize_session_snapshot(
            domain(),
            Timestamp { seconds: 500, nanos: 0 },
        )
        .await;
    let checkpoint_lsn = checkpoint.snapshot_lsn.unwrap();
    let persisted_generation = checkpoint.core_generation.unwrap();
    let checkpoint_payload = checkpoint.encode_to_vec();
    let stored_checkpoint = encode_session_checkpoint(&checkpoint);
    storage
        .write_snapshot(&domain(), checkpoint_lsn, stored_checkpoint)
        .await
        .unwrap();
    drop(service);
    drop(storage);
    tokio::task::yield_now().await;

    let reopened = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    let restarted = ControlServiceImpl::new(reopened.clone(), domain()).await.unwrap();
    assert_eq!(
        restarted.projection_state().core_generation(),
        &persisted_generation
    );
    let operator_session = restarted
        .projection_state()
        .issue_operator_session(OperatorSessionBinding {
            actor_id: ActorId { value: OPERATOR_ACTOR.to_owned() },
            endpoint_id: EndpointId { value: "patchbay-web-server".to_owned() },
            device_id: DeviceId { value: "web-host".to_owned() },
            endpoint_generation: Generation { value: 1 },
        })
        .await;
    let auth = TestAuth {
        session_id: operator_session.id.value,
        session_generation: operator_session.session_generation.value,
        principal_id: PRINCIPAL_ID.to_owned(),
        principal_secret: PRINCIPAL_SECRET.to_owned(),
    };

    let compatible = restarted
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: None,
                view_kind: SnapshotViewKind::Session as i32,
            },
            SECRET,
            &auth,
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(compatible.snapshot_payload, checkpoint_payload);

    let mut stale = checkpoint.clone();
    stale.snapshot_lsn = Some(Lsn { value: 1 });
    let stale_payload = stale.encode_to_vec();
    reopened
        .write_snapshot(
            &domain(),
            Lsn { value: 1 },
            encode_session_checkpoint(&stale),
        )
        .await
        .unwrap();
    let repaired_stale = restarted
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: Some(Lsn { value: 1 }),
                view_kind: SnapshotViewKind::Session as i32,
            },
            SECRET,
            &auth,
        ))
        .await
        .unwrap()
        .into_inner();
    assert_ne!(repaired_stale.snapshot_payload, stale_payload);
    assert_eq!(
        SessionSnapshot::decode(repaired_stale.snapshot_payload.as_slice())
            .unwrap()
            .snapshot_lsn,
        Some(checkpoint_lsn)
    );

    for incompatible_generation in [
        None,
        Some(Generation {
            value: if persisted_generation.value == 1 { 2 } else { 1 },
        }),
    ] {
        let mut incompatible = checkpoint.clone();
        incompatible.core_generation = incompatible_generation;
        let incompatible_payload = incompatible.encode_to_vec();
        reopened
            .write_snapshot(
                &domain(),
                checkpoint_lsn,
                encode_session_checkpoint(&incompatible),
            )
            .await
            .unwrap();
        let repaired = restarted
            .load_snapshot(authenticated_request(
                LoadSnapshotRequest {
                    authority_domain_id: Some(domain()),
                    at_or_before: None,
                    view_kind: SnapshotViewKind::Session as i32,
                },
                SECRET,
                &auth,
            ))
            .await
            .unwrap()
            .into_inner();
        assert_ne!(repaired.snapshot_payload, incompatible_payload);
        let repaired = SessionSnapshot::decode(repaired.snapshot_payload.as_slice()).unwrap();
        assert_eq!(repaired.core_generation, Some(persisted_generation));
        assert_eq!(repaired.snapshot_lsn, Some(checkpoint_lsn));
    }
}

#[tokio::test]
async fn retry_reconciles_a_commit_after_post_append_catch_up_failure() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let inner = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&inner).await;
    let storage = FailPostAppendReadOnceStorage::new(inner.clone());
    let (mut client, task, operator_session) = serve(storage).await;

    let first = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("recoverable-command", "recoverable-key")),
            },
            SECRET,
            &operator_session,
        ))
        .await
        .expect_err("the injected post-append read failure must fail the first response");
    assert_eq!(first.code(), Code::Unavailable);
    assert!(first.get_error_details().retry_info().is_some());

    let retried = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("recoverable-command", "recoverable-key")),
            },
            SECRET,
            &operator_session,
        ))
        .await
        .expect("retry must reconcile the durable append before dedup lookup")
        .into_inner();
    assert_eq!(retried.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(retried.operation_state, OperationState::Accepted as i32);
    assert!(retried.deduplicated);

    let operation_count = inner
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("durable log must remain readable")
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .count();
    assert_eq!(operation_count, 1, "retry must not append a second command");

    task.abort();
}

#[tokio::test]
async fn concurrent_submits_complete_without_deadlock() {
    let server = start_server().await;
    let mut tasks = Vec::with_capacity(CONCURRENT_SUBMISSIONS);
    for index in 0..CONCURRENT_SUBMISSIONS {
        let mut client = server.client.clone();
        let operator_session = server.operator_session.clone();
        tasks.push(tokio::spawn(async move {
            client
                .submit(authenticated_request(
                    SubmitRequest {
                        operation: Some(operation(
                            &format!("concurrent-command-{index}"),
                            &format!("concurrent-key-{index}"),
                        )),
                    },
                    SECRET,
                    &operator_session,
                ))
                .await
                .map(|response| response.into_inner())
        }));
    }

    let results = tokio::time::timeout(Duration::from_secs(10), async {
        let mut results = Vec::with_capacity(CONCURRENT_SUBMISSIONS);
        for task in tasks {
            results.push(task.await.expect("submit task must not panic"));
        }
        results
    })
    .await
    .expect("parallel submissions must not deadlock");

    for result in results {
        let result = result.expect("parallel submit must complete");
        assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
    }
}

#[test]
fn binary_refuses_to_start_without_a_secret() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_patchbay-core-server"))
        .env_remove("PATCHBAY_CORE_SECRET")
        .output()
        .expect("server binary must be executable");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to start without it"),
        "startup failure must explain the missing trust root"
    );
}

#[test]
fn startup_secret_and_storage_error_mapping_fail_safe() {
    assert!(CoreSecretInterceptor::new("").is_err());

    let unavailable = map_storage_error_to_status(StorageError::Unavailable("offline".into()));
    assert_eq!(unavailable.code(), Code::Unavailable);
    assert!(unavailable.get_error_details().retry_info().is_some());

    assert_eq!(
        map_storage_error_to_status(StorageError::IdempotencyConflict).code(),
        Code::FailedPrecondition
    );
    assert_eq!(
        map_storage_error_to_status(StorageError::CorruptRecord("bad".into())).code(),
        Code::Internal
    );
}

async fn login_surface(
    client: &mut ControlServiceClient<Channel>,
    endpoint_id: &str,
    device_id: &str,
) -> TestAuth {
    let login = client
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId { value: endpoint_id.to_owned() }),
                device_id: Some(DeviceId { value: device_id.to_owned() }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .expect("test surface login must succeed")
        .into_inner();
    let principal = login.principal.expect("test login returns a principal");
    TestAuth {
        session_id: login.operator_session_id.expect("test login returns a session").value,
        session_generation: login
            .operator_session_generation
            .expect("test login returns a session generation")
            .value,
        principal_id: principal.principal_id,
        principal_secret: principal.secret,
    }
}

async fn assert_submit_accepted(
    client: &mut ControlServiceClient<Channel>,
    auth: &TestAuth,
    key: &str,
) {
    let result = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation(key, key)),
            },
            SECRET,
            auth,
        ))
        .await
        .expect("unrelated scope must remain usable")
        .into_inner();
    assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
}

async fn assert_scope_denied(client: &mut ControlServiceClient<Channel>, auth: &TestAuth) {
    let submit = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("revoked-scope", "revoked-scope")),
            },
            SECRET,
            auth,
        ))
        .await
        .expect_err("revoked scope must reject before operation acceptance");
    assert_eq!(submit.code(), Code::Unauthenticated);
    let subscribe = client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: Some(Lsn { value: 0 }),
            },
            SECRET,
            auth,
        ))
        .await
        .expect_err("revoked scope must reject subscription establishment");
    assert_eq!(subscribe.code(), Code::Unauthenticated);
}

async fn operation_event_count(storage: &RusqliteStorage) -> usize {
    storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("durable log must remain readable")
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .count()
}

async fn start_server() -> TestServer {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let storage = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&storage).await;
    let (client, task, operator_session) = serve(storage.clone()).await;

    TestServer {
        client,
        storage,
        task,
        operator_session,
        _directory: directory,
    }
}

async fn start_audited_server() -> TestServer {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let storage = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&storage).await;
    let (client, task, operator_session) = serve(AuditedStorage::new(storage.clone())).await;

    TestServer {
        client,
        storage,
        task,
        operator_session,
        _directory: directory,
    }
}

async fn serve<S>(storage: S) -> (ControlServiceClient<Channel>, JoinHandle<()>, TestAuth)
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    serve_with_clock(storage, Arc::new(patchbay_core::time::SystemClock)).await
}

async fn serve_with_gate<S>(
    storage: S,
    decision_gate: CoreDecisionGate,
) -> (ControlServiceClient<Channel>, JoinHandle<()>, TestAuth)
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    let service = ControlServiceImpl::new_with_security_and_decision_gate(
        storage,
        domain(),
        Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
        decision_gate,
    )
    .await
    .expect("service projections must rebuild");
    let interceptor = CoreSecretInterceptor::new(SECRET).expect("test secret is valid");
    let service = ControlServiceServer::with_interceptor(service, interceptor);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral listener must bind");
    let address = listener.local_addr().expect("listener has address");
    let incoming = TcpListenerStream::new(listener);
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(incoming)
            .await
            .expect("test server must run");
    });
    let mut client = ControlServiceClient::connect(format!("http://{address}"))
        .await
        .expect("test client must connect");
    let login = client
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId { value: "grpc-smoke-login".to_owned() }),
                device_id: Some(DeviceId { value: "grpc-smoke-host".to_owned() }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .expect("test login must issue a core-owned session")
        .into_inner();
    let session_generation = login
        .operator_session_generation
        .expect("test login returns a session generation")
        .value;
    let operator_session = login
        .operator_session_id
        .expect("test login returns a session")
        .value;
    let principal = login
        .principal
        .expect("test login returns a principal credential");
    (
        client,
        task,
        TestAuth {
            session_id: operator_session,
            session_generation,
            principal_id: principal.principal_id,
            principal_secret: principal.secret,
        },
    )
}

async fn serve_with_clock<S>(
    storage: S,
    clock: Arc<dyn Clock>,
) -> (ControlServiceClient<Channel>, JoinHandle<()>, TestAuth)
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    let service = ControlServiceImpl::new_with_clock(storage, domain(), clock)
        .await
        .expect("service projections must rebuild");
    let interceptor = CoreSecretInterceptor::new(SECRET).expect("test secret is valid");
    let service = ControlServiceServer::with_interceptor(service, interceptor);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral listener must bind");
    let address = listener.local_addr().expect("listener has address");
    let incoming = TcpListenerStream::new(listener);
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(incoming)
            .await
            .expect("test server must run");
    });
    let mut client = ControlServiceClient::connect(format!("http://{address}"))
        .await
        .expect("test client must connect");
    let login = client
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId {
                value: OPERATOR_ACTOR.to_owned(),
            }),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId {
                    value: "grpc-smoke-login".to_owned(),
                }),
                device_id: Some(DeviceId {
                    value: "grpc-smoke-host".to_owned(),
                }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .expect("test login must issue a core-owned session")
        .into_inner();
    let session_generation = login
        .operator_session_generation
        .expect("test login returns a session generation")
        .value;
    let operator_session = login
        .operator_session_id
        .expect("test login returns a session")
        .value;
    let principal = login
        .principal
        .expect("test login returns a principal credential");
    (
        client,
        task,
        TestAuth {
            session_id: operator_session,
            session_generation,
            principal_id: principal.principal_id,
            principal_secret: principal.secret,
        },
    )
}

async fn seed_authority_and_session(storage: &RusqliteStorage) {
    let target = target_scope();
    let mut authority = AuthorityRegistry::new();
    let grant = Grant {
        grant_id: Some(GrantId {
            value: "operator-grant".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId {
            value: OPERATOR_ACTOR.to_owned(),
        }),
        target_scope: Some(target.clone()),
        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
        provenance: Some(GrantProvenance {
            reason: "gRPC smoke fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    ingest_grant(storage, &mut authority, &domain(), grant)
        .await
        .expect("grant fixture must append");
    let query_grant = Grant {
        grant_id: Some(GrantId { value: "operator-query-grant".to_owned() }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::AuthorityDomain as i32,
            ..TargetScope::default()
        }),
        allowed_operation_kinds: vec![OperationKind::Query as i32],
        provenance: Some(GrantProvenance { reason: "diagnostics fixture".to_owned(), ..GrantProvenance::default() }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    ingest_grant(storage, &mut authority, &domain(), query_grant)
        .await
        .expect("query grant fixture must append");
    let lockdown_grant = Grant {
        grant_id: Some(GrantId { value: "operator-lockdown-grant".to_owned() }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
        target_scope: Some(TargetScope { kind: TargetScopeKind::AuthorityDomain as i32, ..TargetScope::default() }),
        allowed_operation_kinds: vec![OperationKind::SessionManagement as i32],
        provenance: Some(GrantProvenance { reason: "lockdown fixture".to_owned(), ..GrantProvenance::default() }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    ingest_grant(storage, &mut authority, &domain(), lockdown_grant)
        .await
        .expect("lockdown grant fixture must append");

    let operator = OperatorRecord {
        actor_id: Some(ActorId {
            value: OPERATOR_ACTOR.to_owned(),
        }),
        password_hash: "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g".to_owned(),
        created_at: Some(prost_types::Timestamp { seconds: 1, nanos: 0 }),
        authority_domain_id: Some(domain()),
    };
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::OperatorRecord as i32,
                payload: prost::Message::encode_to_vec(&operator),
            },
        )
        .await
        .expect("operator fixture must append");
    let principal = ControlSurfacePrincipalRecord {
        principal_id: PRINCIPAL_ID.to_owned(),
        operator_actor_id: Some(ActorId {
            value: OPERATOR_ACTOR.to_owned(),
        }),
        endpoint_id: Some(EndpointId {
            value: "patchbay-web-server".to_owned(),
        }),
        device_id: Some(DeviceId {
            value: "web-host".to_owned(),
        }),
        endpoint_generation: Some(Generation { value: 1 }),
        credential_hash: hash_principal_credential(PRINCIPAL_SECRET),
        created_at: Some(prost_types::Timestamp {
            seconds: 2,
            nanos: 0,
        }),
        authority_domain_id: Some(domain()),
    };
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::ControlSurfacePrincipal as i32,
                payload: prost::Message::encode_to_vec(&principal),
            },
        )
        .await
        .expect("principal fixture must append");

    let registration = session_events::registered(
        domain(),
        SessionRegistered {
            adapter_id: target.adapter_id,
            deployment_scope: target.deployment_scope,
            runtime_session_id: target.runtime_session_id,
            session_generation: target.session_generation,
            initial_state: Some(SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            }),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "smoke".to_owned(),
            model: "provider/model".to_owned(),
            spawn_origin: None,
        },
    );
    storage
        .append(&domain(), session_events::encode(&registration))
        .await
        .expect("session fixture must append");
}

fn core_request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(
        CORE_SECRET_HEADER,
        SECRET.parse().expect("test secret is valid metadata"),
    );
    request
}

fn authenticated_request<T>(message: T, secret: &str, operator_session: &TestAuth) -> Request<T> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(
        CORE_SECRET_HEADER,
        secret.parse().expect("test secret is valid metadata"),
    );
    request.metadata_mut().insert(
        OPERATOR_SESSION_HEADER,
        operator_session
            .session_id
            .parse()
            .expect("test operator session is valid metadata"),
    );
    request.metadata_mut().insert(
        OPERATOR_ID_HEADER,
        OPERATOR_ACTOR
            .parse()
            .expect("test operator actor is valid metadata"),
    );
    request.metadata_mut().insert(
        PRINCIPAL_ID_HEADER,
        operator_session
            .principal_id
            .parse()
            .expect("test principal id is valid metadata"),
    );
    request.metadata_mut().insert(
        PRINCIPAL_SECRET_HEADER,
        operator_session
            .principal_secret
            .parse()
            .expect("test principal secret is valid metadata"),
    );
    request
}

fn diagnostic_query_operation(command_id: &str, idempotency_key: &str, query: DiagnosticsQuery) -> Operation {
    Operation {
        command_id: Some(CommandId { value: command_id.to_owned() }),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: OperationKind::Query as i32,
        target_scope: Some(TargetScope { kind: TargetScopeKind::AuthorityDomain as i32, ..TargetScope::default() }),
        idempotency_key: idempotency_key.to_owned(),
        payload: Some(PayloadEnvelope {
            payload: query.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: "patchbay.DiagnosticsQuery".to_owned(),
        }),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp { seconds: 1, nanos: 0 }),
            expires_at: Some(Timestamp { seconds: 253_402_300_799, nanos: 0 }),
        }),
        submitted_at: Some(Timestamp { seconds: 1, nanos: 0 }),
        ..Operation::default()
    }
}

fn operation(command_id: &str, idempotency_key: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command_id.to_owned(),
        }),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(target_scope()),
        idempotency_key: idempotency_key.to_owned(),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 253_402_300_799,
                nanos: 0,
            }),
        }),
        submitted_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        ..Operation::default()
    }
}

fn target_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        runtime_session_id: Some(RuntimeSessionId {
            value: "session-1".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    }
}

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}
