extern crate patchbay_test_support;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};

use patchbay_contracts::patchbay::{
    observation_request, session_state_event, spawn_claim_event, spawn_request, typed_correlation,
    AcceptedOperation, ActorEndpointRef, ActorId, AdapterCapability, AdapterId,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest,
    AuditEventKind, AuditRecord, AuthorityDomainId, CommandId, CommandTransition, DescendantGrant,
    DescendantGrantProvenance, DeviceId, EndpointId, EventId, ExternalEffectDisposition,
    ExternalRuntimeRef, FailureCode, FreshSpawn, Generation, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, IdempotencyKey, LogicalTargetCreated, LogicalTargetId, Lsn, Observation,
    ObservationKind, ObservationRequest, Operation, OperationKind, OperationState, OperatorRecord,
    PayloadContentType, PayloadEnvelope, PrincipalEnrollment, ReceiveRequest, ResourceId,
    ResourceIdentity, ResourceKind, Revocation, RuntimeGenerationRef, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionRegistered, SessionStateEvent,
    SpawnClaimAccepted, SpawnClaimEvent, SpawnExecutionEvidence, SpawnExecutionPhase,
    SpawnGenerationClaim, SpawnPromotionCommitted, SpawnRequest, SpawnTargetSpec, StoredEventKind,
    StoredEventPayload, SubmissionOutcome, SubmitRequest, TargetScope, TargetScopeKind, TimeWindow,
    TypedCorrelation, VerifyOperatorPasswordRequest,
};
use patchbay_core::{
    acceptance::SPAWN_REQUEST_SCHEMA,
    adapter,
    audit::{AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::{
        grant_authorizes, ingest_descendant_grant, ingest_grant, ingest_revocation,
        rebuild_from_log, AuthorityError, AuthorityRegistry, IssuerRef,
        DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::{
        AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedStorage, CoreGenerationStore,
        DedupOutcome, GrantAppendOutcome, GrantIdentityKey, RecordedEvent, RusqliteStorage,
        SpawnPromotionAppend, Storage, StorageError, StoredSnapshot, TargetKey,
    },
    time::TestClock,
};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    decision_gate::CoreDecisionGate,
    issuer::{
        OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER,
    },
    login_security::{LoginLimiter, StderrLoginAuditSink},
    rpc::{
        adapter_control_service_server::AdapterControlService,
        control_service_server::ControlService,
    },
    service::ControlServiceImpl,
    spawn_completion::{SpawnCompletionDriver, SpawnCompletionError},
    state::ProjectionState,
};
use prost::Message;
use prost_types::Timestamp;
use tokio::sync::Semaphore;
use tokio_stream::StreamExt;
use tonic::{Request, Response};

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
fn adapter_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        ..TargetScope::default()
    }
}

fn session_scope() -> TargetScope {
    session_scope_for(7)
}

fn session_scope_for(generation: u64) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "spawned-session".to_owned(),
        }),
        session_generation: Some(Generation { value: generation }),
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
    seed_parent_grant(storage).await;
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
                        idempotency_key: "spawn-1-key".to_owned(),
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
    let result = Observation {
        authority_domain_id: Some(domain()),
        kind: ObservationKind::Result as i32,
        correlations: vec![correlation()],
        target_scope: Some(fleet_scope()),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let mut result_audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 1_000,
            nanos: 0,
        },
        AuditEventKind::CommandRunning,
    );
    result_audit.command_id = Some(command_id());
    result_audit.target_scope = result.target_scope.clone();
    result_audit.reason_code = "spawn_completion_deferred".to_owned();
    let result_id = storage
        .append_spawn_result_deferred_audited(&domain(), result, result_audit)
        .await
        .unwrap()
        .source_event_id;
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

fn descendant_candidate(audit_id: EventId) -> DescendantGrant {
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
            continuation_authority: None,
        }),
        created_at: Some(Timestamp {
            seconds: 1_000,
            nanos: 0,
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        audit_id: Some(audit_id),
        ..DescendantGrant::default()
    }
}

async fn append_descendant<S>(storage: &S, audit_id: EventId) -> EventId
where
    S: Storage,
{
    let mut authority = AuthorityRegistry::new();
    ingest_descendant_grant(
        storage,
        &mut authority,
        &domain(),
        descendant_candidate(audit_id),
    )
    .await
    .unwrap()
}

async fn seed_parent_grant<S: Storage>(storage: &S) {
    seed_parent_grant_for(storage, fleet_scope()).await;
}

async fn seed_adapter_parent_grant<S: Storage>(storage: &S) {
    seed_parent_grant_for(storage, adapter_scope()).await;
}

async fn seed_parent_grant_for<S: Storage>(storage: &S, target_scope: TargetScope) {
    let mut authority = AuthorityRegistry::new();
    ingest_grant(
        storage,
        &mut authority,
        &domain(),
        Grant {
            grant_id: Some(parent_grant_id()),
            authority_domain_id: Some(domain()),
            subject_actor_id: Some(actor()),
            subject_endpoint_id: Some(endpoint()),
            target_scope: Some(target_scope),
            allowed_operation_kinds: vec![OperationKind::Spawn as i32],
            provenance: Some(GrantProvenance {
                reason: "spawn authority fixture".to_owned(),
                ..GrantProvenance::default()
            }),
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..Grant::default()
        },
    )
    .await
    .unwrap();
}

#[derive(Clone)]
struct ControlAuth {
    session_id: String,
    principal_id: String,
    principal_secret: String,
}

async fn seed_operator<S: Storage>(storage: &S) {
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::OperatorRecord as i32,
                payload: OperatorRecord {
                    actor_id: Some(actor()),
                    password_hash: "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g".to_owned(),
                    created_at: Some(Timestamp { seconds: 1, nanos: 0 }),
                    authority_domain_id: Some(domain()),
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();
}

async fn login_control<S>(service: &ControlServiceImpl<S>) -> ControlAuth
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    let login = service
        .verify_operator_password(Request::new(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(actor()),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(endpoint()),
                device_id: Some(device()),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .unwrap()
        .into_inner();
    let principal = login.principal.unwrap();
    ControlAuth {
        session_id: login.operator_session_id.unwrap().value,
        principal_id: principal.principal_id,
        principal_secret: principal.secret,
    }
}

fn authenticated_control<T>(message: T, auth: &ControlAuth) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert(OPERATOR_ID_HEADER, actor().value.parse().unwrap());
    request
        .metadata_mut()
        .insert(OPERATOR_SESSION_HEADER, auth.session_id.parse().unwrap());
    request
        .metadata_mut()
        .insert(PRINCIPAL_ID_HEADER, auth.principal_id.parse().unwrap());
    request.metadata_mut().insert(
        PRINCIPAL_SECRET_HEADER,
        auth.principal_secret.parse().unwrap(),
    );
    request
}

fn submitted_operation(
    command: &str,
    key: &str,
    kind: OperationKind,
    target_scope: TargetScope,
) -> Operation {
    let payload = (kind == OperationKind::Spawn).then(|| PayloadEnvelope {
        payload: SpawnRequest {
            intent: Some(spawn_request::Intent::Fresh(FreshSpawn {})),
            target_spec: Some(SpawnTargetSpec {
                shape: "session".to_owned(),
                ..SpawnTargetSpec::default()
            }),
        }
        .encode_to_vec(),
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: SPAWN_REQUEST_SCHEMA.to_owned(),
    });
    Operation {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: kind as i32,
        target_scope: Some(target_scope),
        idempotency_key: key.to_owned(),
        payload,
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

fn adapter_registration() -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        endpoint_id: Some(EndpointId {
            value: "pi-adapter-endpoint".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        adapter_generation: Some(Generation { value: 1 }),
        capability: Some(AdapterCapability {
            supported_operation_kinds: vec![OperationKind::Spawn as i32],
            streaming_support: true,
            session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
            session_replacement_support: true,
            target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    }
}

fn authenticated<T>(message: T, attachment_token: &str) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert(ADAPTER_ID_HEADER, "pi".parse().unwrap());
    request.metadata_mut().insert(
        ADAPTER_EVIDENCE_HEADER,
        "adapter-test-secret".parse().unwrap(),
    );
    request.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        attachment_token.parse().unwrap(),
    );
    request
}

fn attachment_token<T>(response: &Response<T>) -> String {
    response
        .metadata()
        .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

async fn attach_adapter<S>(service: &AdapterControlServiceImpl<S>) -> String
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    let response = service
        .attach(Request::new(AttachRequest {
            registration: Some(adapter_registration()),
            attachment_evidence: b"adapter-test-secret".to_vec(),
        }))
        .await
        .unwrap();
    attachment_token(&response)
}

async fn report_session<S>(
    service: &AdapterControlServiceImpl<S>,
    token: &str,
    generation: u64,
    spawn_origin: Option<TypedCorrelation>,
) -> EventId
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::SessionReport(
                    patchbay_contracts::patchbay::SessionReport {
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        deployment_scope: "machine-a".to_owned(),
                        runtime_session_id: Some(RuntimeSessionId {
                            value: "spawned-session".to_owned(),
                        }),
                        session_generation: Some(Generation { value: generation }),
                        connectivity: SessionConnectivityState::Live as i32,
                        activity: SessionActivityState::Idle as i32,
                        project: "patchbay".to_owned(),
                        cwd: "/work/patchbay".to_owned(),
                        name: "spawned".to_owned(),
                        model: "provider/model".to_owned(),
                        spawn_origin,
                        source_cursor: Some(
                            patchbay_contracts::patchbay::SessionReportSourceCursor {
                                adapter_generation: Some(Generation { value: 1 }),
                                revision: 1,
                            },
                        ),
                        continuation_context_status: 0,
                    },
                )),
            },
            token,
        ))
        .await
        .unwrap()
        .into_inner()
        .event_id
        .expect("session report has a durable event id")
}

async fn seed_spawn_claim<S: Storage>(storage: &S) {
    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    let claim_exists = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::SpawnClaim as i32)
        .filter_map(|event| SpawnClaimEvent::decode(event.payload.payload.as_slice()).ok())
        .any(|event| {
            matches!(
                event.mutation,
                Some(spawn_claim_event::Mutation::Accepted(accepted))
                    if accepted
                        .accepted_operation
                        .as_ref()
                        .and_then(|accepted| accepted.operation.as_ref())
                        .and_then(|operation| operation.command_id.as_ref())
                        == Some(&command_id())
            )
        });
    if claim_exists {
        return;
    }

    let logical_target_id = LogicalTargetId {
        value: "logical-spawn-1".to_owned(),
    };
    storage
        .append(
            &domain(),
            patchbay_core::session::events::encode(
                &patchbay_core::session::events::logical_target_created(
                    domain(),
                    LogicalTargetCreated {
                        logical_target_id: Some(logical_target_id.clone()),
                        adapter_id: adapter_scope().adapter_id,
                        deployment_scope: "machine-a".to_owned(),
                    },
                ),
            ),
        )
        .await
        .unwrap();
    let operation = Operation {
        command_id: Some(command_id()),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(actor()),
            endpoint_id: Some(endpoint()),
            device_id: Some(device()),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(adapter_scope()),
        idempotency_key: "spawn-1-key".to_owned(),
        payload: Some(PayloadEnvelope {
            payload: SpawnRequest {
                intent: Some(spawn_request::Intent::Fresh(FreshSpawn {})),
                target_spec: Some(SpawnTargetSpec {
                    shape: "session".to_owned(),
                    ..SpawnTargetSpec::default()
                }),
            }
            .encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: SPAWN_REQUEST_SCHEMA.to_owned(),
        }),
        ..Operation::default()
    };
    let accepted_operation = AcceptedOperation {
        operation: Some(operation.clone()),
        authorizing_grant_id: Some(parent_grant_id()),
    };
    let accepted_claim = SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation),
        claim: Some(SpawnGenerationClaim {
            authority_domain_id: Some(domain()),
            claim_operation_id: Some(command_id()),
            logical_target_id: Some(logical_target_id),
            expected_prior: None,
            claimed_generation: Some(Generation { value: 1 }),
        }),
        ..SpawnClaimAccepted::default()
    };
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        },
        AuditEventKind::CommandSubmissionAccepted,
    );
    audit.actor_id = Some(actor());
    audit.endpoint_id = Some(endpoint());
    audit.device_id = Some(device());
    audit.command_id = Some(command_id());
    audit.grant_id = Some(parent_grant_id());
    audit.target_scope = Some(adapter_scope());
    audit.reason_code = "operation_spawn".to_owned();
    storage
        .append_spawn_claim_accepted(
            &domain(),
            &IdempotencyKey {
                value: operation.idempotency_key.clone(),
            },
            &TargetKey::new("adapter:pi".to_owned()).unwrap(),
            accepted_claim,
            audit,
            operation.encode_to_vec(),
        )
        .await
        .unwrap();
}

async fn seed_adapter_scoped_spawn<S: Storage>(storage: &S) {
    seed_spawn_claim(storage).await;
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
}

async fn acknowledge_delivery<S>(
    service: &AdapterControlServiceImpl<S>,
    token: &str,
    operation: &Operation,
) where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::Event(Observation {
                    authority_domain_id: Some(domain()),
                    kind: ObservationKind::Event as i32,
                    correlations: vec![TypedCorrelation {
                        r#ref: Some(typed_correlation::Ref::CommandId(
                            operation.command_id.clone().unwrap(),
                        )),
                    }],
                    target_scope: operation.target_scope.clone(),
                    payload: Some(PayloadEnvelope {
                        schema_ref: adapter::DELIVERY_ACKNOWLEDGEMENT_SCHEMA.to_owned(),
                        ..PayloadEnvelope::default()
                    }),
                    failure_code: FailureCode::Unspecified as i32,
                    ..Observation::default()
                })),
            },
            token,
        ))
        .await
        .unwrap();
}

fn spawn_result(failure_code: FailureCode) -> Observation {
    Observation {
        authority_domain_id: Some(domain()),
        kind: ObservationKind::Result as i32,
        correlations: vec![correlation(), correlation()],
        target_scope: Some(adapter_scope()),
        failure_code: failure_code as i32,
        ..Observation::default()
    }
}

fn managed_claim() -> SpawnGenerationClaim {
    SpawnGenerationClaim {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(command_id()),
        logical_target_id: Some(LogicalTargetId {
            value: "logical-spawn-1".to_owned(),
        }),
        expected_prior: None,
        claimed_generation: Some(Generation { value: 1 }),
    }
}

async fn report_progress<S>(
    service: &AdapterControlServiceImpl<S>,
    token: &str,
    phase: SpawnExecutionPhase,
) -> EventId
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    report_progress_for_claim(service, token, phase, managed_claim()).await
}

async fn report_progress_for_claim<S>(
    service: &AdapterControlServiceImpl<S>,
    token: &str,
    phase: SpawnExecutionPhase,
    exact_claim: SpawnGenerationClaim,
) -> EventId
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::SpawnExecutionEvidence(
                    SpawnExecutionEvidence {
                        authority_domain_id: Some(domain()),
                        exact_claim: Some(exact_claim.clone()),
                        phase: phase as i32,
                        external_effect_disposition: ExternalEffectDisposition::Identified as i32,
                        failure_code: FailureCode::Unspecified as i32,
                        external_runtime: Some(RuntimeGenerationRef {
                            logical_target_id: exact_claim.logical_target_id,
                            external_runtime: Some(ExternalRuntimeRef {
                                adapter_id: adapter_scope().adapter_id,
                                deployment_scope: "machine-a".to_owned(),
                                runtime_session_id: session_scope().runtime_session_id,
                                generation: Some(Generation { value: 1 }),
                            }),
                        }),
                        ..SpawnExecutionEvidence::default()
                    },
                )),
            },
            token,
        ))
        .await
        .unwrap()
        .into_inner()
        .event_id
        .expect("spawn progress evidence has a durable event id")
}

fn durable_claim(events: &[RecordedEvent]) -> SpawnGenerationClaim {
    events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::SpawnClaim as i32)
        .filter_map(|event| SpawnClaimEvent::decode(event.payload.payload.as_slice()).ok())
        .find_map(|event| match event.mutation {
            Some(spawn_claim_event::Mutation::Accepted(accepted))
                if accepted
                    .claim
                    .as_ref()
                    .and_then(|claim| claim.claim_operation_id.as_ref())
                    == Some(&command_id()) =>
            {
                accepted.claim
            }
            _ => None,
        })
        .expect("managed spawn has one durable exact claim")
}

async fn report_successful_spawn<S>(service: &AdapterControlServiceImpl<S>, token: &str) -> EventId
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::Event(spawn_result(
                    FailureCode::Unspecified,
                ))),
            },
            token,
        ))
        .await
        .unwrap()
        .into_inner()
        .event_id
        .expect("successful spawn result has a durable event id")
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
    let promotions = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::SpawnPromotionCommitted as i32)
        .filter_map(|event| SpawnPromotionCommitted::decode(event.payload.payload.as_slice()).ok())
        .filter(|promotion| {
            promotion
                .accepted_claim
                .as_ref()
                .and_then(|accepted| accepted.claim.as_ref())
                .and_then(|claim| claim.claim_operation_id.as_ref())
                == Some(&command_id())
        })
        .count();
    let grants = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::DescendantGrant as i32)
        .count()
        + promotions;
    let transitions = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
        .filter_map(|event| CommandTransition::decode(event.payload.payload.as_slice()).ok())
        .filter(|transition| {
            transition.command_id.as_ref() == Some(&command_id())
                && transition.to_state == OperationState::Completed as i32
        })
        .count()
        + promotions;
    (audits, grants, transitions)
}

#[derive(Clone)]
struct LoseFirstDescendantAppendAcknowledgement {
    inner: AuditedStorage<RusqliteStorage>,
    lose_next_append: Arc<AtomicBool>,
}

impl LoseFirstDescendantAppendAcknowledgement {
    fn new(inner: AuditedStorage<RusqliteStorage>) -> Self {
        Self {
            inner,
            lose_next_append: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl Storage for LoseFirstDescendantAppendAcknowledgement {
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
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn append_spawn_result_deferred_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        observation: Observation,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        self.inner
            .append_spawn_result_deferred_audited(authority_domain_id, observation, audit)
            .await
    }

    async fn reconcile_observation_retry(
        &self,
        authority_domain_id: &AuthorityDomainId,
        observation: Observation,
    ) -> Result<Option<EventId>, StorageError> {
        self.inner
            .reconcile_observation_retry(authority_domain_id, observation)
            .await
    }

    async fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<GrantAppendOutcome, StorageError> {
        let is_descendant = source.kind == StoredEventKind::DescendantGrant as i32;
        let outcome = self
            .inner
            .append_grant_audited(authority_domain_id, identity, source, audit)
            .await?;
        if is_descendant
            && matches!(outcome, GrantAppendOutcome::Appended(_))
            && self.lose_next_append.swap(false, Ordering::SeqCst)
        {
            return Err(StorageError::WriteFailed {
                message: "synthetic lost descendant append acknowledgement".to_owned(),
                retryable: true,
            });
        }
        Ok(outcome)
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

#[derive(Clone)]
struct PromotionAppendFault {
    inner: AuditedStorage<RusqliteStorage>,
    /// 1 fails before the atomic append; 2 loses acknowledgement after commit.
    mode: Arc<AtomicU8>,
}

impl PromotionAppendFault {
    fn new(inner: AuditedStorage<RusqliteStorage>, mode: u8) -> Self {
        Self {
            inner,
            mode: Arc::new(AtomicU8::new(mode)),
        }
    }
}

impl Storage for PromotionAppendFault {
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
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn append_spawn_promotion_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        promotion: SpawnPromotionCommitted,
        audit: AuditRecordDraft,
    ) -> Result<SpawnPromotionAppend, StorageError> {
        match self.mode.swap(0, Ordering::SeqCst) {
            1 => Err(StorageError::WriteFailed {
                message: "synthetic crash before promotion transaction".to_owned(),
                retryable: true,
            }),
            2 => {
                self.inner
                    .append_spawn_promotion_audited(authority_domain_id, promotion, audit)
                    .await?;
                Err(StorageError::WriteFailed {
                    message: "synthetic lost promotion commit acknowledgement".to_owned(),
                    retryable: true,
                })
            }
            _ => {
                self.inner
                    .append_spawn_promotion_audited(authority_domain_id, promotion, audit)
                    .await
            }
        }
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

async fn seed_ready_managed_promotion(storage: &AuditedStorage<RusqliteStorage>) {
    seed_parent_grant(storage).await;
    let gate = CoreDecisionGate::default();
    let service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&service).await;
    {
        let _guard = gate.acquire().await;
        seed_adapter_scoped_spawn(storage).await;
    }
    report_progress(&service, &token, SpawnExecutionPhase::ExternalIdentityKnown).await;
    report_progress(&service, &token, SpawnExecutionPhase::HandshakeReconciling).await;
    report_session(&service, &token, 1, Some(correlation())).await;
    report_successful_spawn(&service, &token).await;
    report_progress(
        &service,
        &token,
        SpawnExecutionPhase::SuccessEvidenceReported,
    )
    .await;
}

#[tokio::test]
async fn managed_promotion_crash_prefix_is_neither_or_complete_and_replays_once() {
    for mode in [1, 2] {
        let inner = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
        seed_ready_managed_promotion(&inner).await;
        let storage = PromotionAppendFault::new(inner.clone(), mode);
        let error = match SpawnCompletionDriver::bootstrap(
            storage.clone(),
            domain(),
            CoreDecisionGate::default(),
            production_audit(storage),
            clock(),
        )
        .await
        {
            Ok(_) => panic!("synthetic promotion crash must interrupt the first driver"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SpawnCompletionError::Storage(StorageError::WriteFailed {
                retryable: true,
                ..
            })
        ));
        let crashed = all_events(&inner).await;
        let expected = if mode == 1 { (0, 0, 0) } else { (1, 1, 1) };
        assert_eq!(completion_counts(&crashed), expected);
        assert_eq!(
            crashed
                .iter()
                .filter(|event| {
                    matches!(
                        StoredEventKind::try_from(event.payload.kind).ok(),
                        Some(StoredEventKind::SpawnPromotionCommitted)
                    )
                })
                .count(),
            usize::from(mode == 2),
            "the crash prefix contains neither the promotion nor a partial source"
        );

        let restarted = SpawnCompletionDriver::bootstrap(
            inner.clone(),
            domain(),
            CoreDecisionGate::default(),
            production_audit(inner.clone()),
            clock(),
        )
        .await
        .expect("restart commits or replays the one complete promotion");
        drop(restarted);
        assert_eq!(completion_counts(&all_events(&inner).await), (1, 1, 1));
        ProjectionState::rebuild(&inner, &domain())
            .await
            .expect("the complete authority-bearing promotion replays in every projection");
    }
}

#[tokio::test]
async fn managed_driver_suppresses_promotion_after_accepted_authority_revocation() {
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_ready_managed_promotion(&storage).await;
    let mut authority = rebuild_from_log(&storage, &domain()).await.unwrap();
    ingest_revocation(
        &storage,
        &mut authority,
        &domain(),
        Revocation {
            authority_domain_id: Some(domain()),
            grant_id: Some(parent_grant_id()),
            revoked_by: Some(ActorEndpointRef {
                actor_id: Some(actor()),
                endpoint_id: Some(endpoint()),
                device_id: Some(device()),
                ..ActorEndpointRef::default()
            }),
            revoked_at: Some(Timestamp {
                seconds: 999,
                nanos: 0,
            }),
            revocation_generation: Some(Generation { value: 1 }),
            accepted_operation_policy: GrantRevocationPolicy::Continue as i32,
            reason: "promotion_authority_revoked".to_owned(),
            ..Revocation::default()
        },
    )
    .await
    .unwrap();

    let driver = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .expect("revoked accepted authority suppresses promotion without crashing the owner");
    drop(driver);
    assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
    ProjectionState::rebuild(&storage, &domain())
        .await
        .expect("the staged, unpromoted prefix remains replayable for reconciliation");
}

#[tokio::test]
async fn committed_descendant_with_lost_ack_repairs_without_duplicate_grant_or_creation_audit() {
    let inner = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_evidence(&inner).await;
    let storage = LoseFirstDescendantAppendAcknowledgement::new(inner.clone());
    let error = match SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(storage),
        clock(),
    )
    .await
    {
        Ok(_) => panic!("lost descendant acknowledgement must interrupt the first driver"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        SpawnCompletionError::Authority(AuthorityError::Storage(StorageError::WriteFailed {
            retryable: true,
            ..
        }))
    ));
    let ambiguous_prefix = all_events(&inner).await;
    assert_eq!(completion_counts(&ambiguous_prefix), (1, 1, 0));
    let descendant_source_id = ambiguous_prefix
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::DescendantGrant as i32)
        .unwrap()
        .event_id
        .clone();

    let restarted = SpawnCompletionDriver::bootstrap(
        inner.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(inner.clone()),
        clock(),
    )
    .await
    .unwrap();
    drop(restarted);
    assert_eq!(completion_counts(&all_events(&inner).await), (1, 1, 1));

    let audits = inner
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantCreated],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(GrantId {
                    value: "desc:authority-main:spawn-1".to_owned(),
                }),
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["descendant_grant_created".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
    assert_eq!(
        audits.records[0].source_event_id,
        Some(descendant_source_id)
    );
}

#[tokio::test]
async fn migrated_v4_complete_prefix_with_duplicate_descendant_bootstraps_and_retries_earliest() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v4-duplicate-descendant.sqlite3");
    let storage = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    let source = seed_evidence(&storage).await;
    let audit_id = append_completion_audit(&storage, source).await;
    let candidate = descendant_candidate(audit_id);
    let mut authority = AuthorityRegistry::new();
    let earliest_id =
        ingest_descendant_grant(&storage, &mut authority, &domain(), candidate.clone())
            .await
            .unwrap();
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
    let complete_prefix = all_events(&storage).await;
    let earliest_source = complete_prefix
        .iter()
        .find(|event| event.event_id == earliest_id)
        .unwrap()
        .payload
        .clone();
    drop(storage);
    tokio::task::yield_now().await;

    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute(
            "INSERT INTO events (lsn, authority_domain_id, kind, payload)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                complete_prefix.len() as i64 + 1,
                domain().value,
                earliest_source.kind,
                earliest_source.encode_to_vec()
            ],
        )
        .unwrap();
        db.execute_batch(
            "DROP TABLE staged_successor_reconciliations;
             DROP TABLE grant_identities;
             PRAGMA user_version = 4;",
        )
        .unwrap();
    }

    let migrated = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    let migrated_prefix = all_events(&migrated).await;
    assert_eq!(completion_counts(&migrated_prefix), (1, 2, 1));

    let driver = SpawnCompletionDriver::bootstrap(
        migrated.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(migrated.clone()),
        clock(),
    )
    .await
    .expect("the complete migrated prefix must bootstrap as quiescent");
    drop(driver);
    assert_eq!(all_events(&migrated).await, migrated_prefix);

    let mut fresh = AuthorityRegistry::new();
    let retry_id = ingest_descendant_grant(&migrated, &mut fresh, &domain(), candidate)
        .await
        .expect("the legacy duplicate must retry through the earliest identity");
    assert_eq!(retry_id, earliest_id);
    assert_eq!(all_events(&migrated).await, migrated_prefix);
    assert_eq!(fresh, rebuild_from_log(&migrated, &domain()).await.unwrap());

    let audits = migrated
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantCreated],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(GrantId {
                    value: "desc:authority-main:spawn-1".to_owned(),
                }),
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["descendant_grant_created".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
    assert_eq!(audits.records[0].source_event_id, Some(earliest_id));
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

    async fn append_spawn_result_deferred_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        observation: Observation,
        audit: AuditRecordDraft,
    ) -> Result<AuditedAppend, StorageError> {
        self.inner
            .append_spawn_result_deferred_audited(authority_domain_id, observation, audit)
            .await
    }

    async fn reconcile_observation_retry(
        &self,
        authority_domain_id: &AuthorityDomainId,
        observation: Observation,
    ) -> Result<Option<EventId>, StorageError> {
        self.inner
            .reconcile_observation_retry(authority_domain_id, observation)
            .await
    }

    async fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<GrantAppendOutcome, StorageError> {
        self.inner
            .append_grant_audited(authority_domain_id, identity, source, audit)
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

async fn run_live_adapter_case(
    report_first: bool,
) -> (AuditedStorage<RusqliteStorage>, AuthorityRegistry) {
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_parent_grant(&storage).await;
    let gate = CoreDecisionGate::default();
    let driver = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        gate.clone(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .unwrap();
    let service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&service).await;
    let driver_task = tokio::spawn(driver.run());

    {
        let _guard = gate.acquire().await;
        seed_adapter_scoped_spawn(&storage).await;
    }
    report_progress(&service, &token, SpawnExecutionPhase::ExternalIdentityKnown).await;
    report_progress(&service, &token, SpawnExecutionPhase::HandshakeReconciling).await;
    if report_first {
        report_session(&service, &token, 1, Some(correlation())).await;
        assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
        report_successful_spawn(&service, &token).await;
    } else {
        report_successful_spawn(&service, &token).await;
        assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
        report_session(&service, &token, 1, Some(correlation())).await;
    }
    report_progress(
        &service,
        &token,
        SpawnExecutionPhase::SuccessEvidenceReported,
    )
    .await;

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if completion_counts(&all_events(&storage).await) == (1, 1, 1) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("continuous driver completes committed spawn facts");
    driver_task.abort();

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
    let authority = rebuild_from_log(&storage, &domain()).await.unwrap();
    (storage, authority)
}

#[tokio::test]
async fn managed_evidence_retries_complete_once_and_restart_as_a_replayable_prefix() {
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_parent_grant(&storage).await;
    let gate = CoreDecisionGate::default();
    let service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&service).await;
    {
        let _guard = gate.acquire().await;
        seed_adapter_scoped_spawn(&storage).await;
    }

    report_progress(&service, &token, SpawnExecutionPhase::ExternalIdentityKnown).await;
    report_progress(&service, &token, SpawnExecutionPhase::HandshakeReconciling).await;
    let earliest_result = report_successful_spawn(&service, &token).await;
    let result_retry_before_staging = report_successful_spawn(&service, &token).await;
    assert_eq!(
        result_retry_before_staging, earliest_result,
        "exact Result retries reuse the canonical durable source"
    );
    let mut driver = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        gate.clone(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .expect("result evidence without a staged successor remains deferred");
    assert_eq!(
        completion_counts(&all_events(&storage).await),
        (0, 0, 0),
        "the driver cannot promote before exact staged successor evidence"
    );

    let staged = report_session(&service, &token, 1, Some(correlation())).await;
    let staged_retry = report_session(&service, &token, 1, Some(correlation())).await;
    assert_eq!(
        staged_retry, staged,
        "SessionReport retries reuse the one staged-successor record"
    );
    let result_retry_after_staging = report_successful_spawn(&service, &token).await;
    assert_eq!(result_retry_after_staging, earliest_result);
    report_progress(
        &service,
        &token,
        SpawnExecutionPhase::SuccessEvidenceReported,
    )
    .await;

    let before_completion = all_events(&storage).await;
    assert_eq!(
        before_completion
            .iter()
            .filter(|event| {
                event.payload.kind == StoredEventKind::SpawnSuccessorEvidenceStaged as i32
            })
            .count(),
        1
    );
    assert_eq!(
        before_completion
            .iter()
            .filter(|event| {
                event.payload.kind == StoredEventKind::Observation as i32
                    && Observation::decode(event.payload.payload.as_slice())
                        .ok()
                        .is_some_and(|observation| {
                            observation.kind == ObservationKind::Result as i32
                                && observation.correlations == vec![correlation(), correlation()]
                        })
            })
            .count(),
        1
    );

    driver
        .catch_up_to_quiescence()
        .await
        .expect("completion driver accepts exact durable retries");
    drop(driver);
    assert_eq!(completion_counts(&all_events(&storage).await), (1, 1, 1));
    assert_eq!(
        report_successful_spawn(&service, &token).await,
        earliest_result,
        "terminal exact Result retry still returns the canonical source"
    );

    let post_promotion_retry = report_session(&service, &token, 1, Some(correlation())).await;
    assert_eq!(
        post_promotion_retry, staged,
        "a late transport retry still reconciles to the original staged fact"
    );
    ProjectionState::rebuild(&storage, &domain())
        .await
        .expect("all projections restart from the retry-bearing prefix");
    let restarted = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .expect("completion driver restart remains quiescent");
    drop(restarted);
    assert_eq!(completion_counts(&all_events(&storage).await), (1, 1, 1));
}

#[tokio::test]
async fn authenticated_failed_result_retry_reuses_canonical_source_across_driver_restart() {
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    seed_parent_grant(&storage).await;
    let gate = CoreDecisionGate::default();
    let service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&service).await;
    {
        let _guard = gate.acquire().await;
        seed_adapter_scoped_spawn(&storage).await;
    }

    let submit = |observation| {
        service.ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::Event(observation)),
            },
            &token,
        ))
    };
    let first = submit(spawn_result(FailureCode::ExecutionFailed))
        .await
        .expect("first authenticated failed Result commits")
        .into_inner()
        .event_id
        .expect("failed Result has a canonical source id");
    let before_retry = all_events(&storage).await;
    let retry = submit(spawn_result(FailureCode::ExecutionFailed))
        .await
        .expect("exact authenticated failed Result retry reconciles")
        .into_inner()
        .event_id;
    assert_eq!(retry, Some(first));
    assert_eq!(all_events(&storage).await, before_retry);

    let changed_error = submit(spawn_result(FailureCode::ExecutionOutcomeUnknown))
        .await
        .expect_err("changed terminal evidence must remain rejected");
    assert_eq!(changed_error.code(), tonic::Code::Internal);
    assert_eq!(all_events(&storage).await, before_retry);
    assert_eq!(completion_counts(&before_retry), (0, 0, 0));

    ProjectionState::rebuild(&storage, &domain())
        .await
        .expect("failed retry prefix rebuilds every projection");
    let driver = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        gate,
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .expect("completion driver remains quiescent on canonical failure evidence");
    drop(driver);
    assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
    let restarted = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .expect("completion driver restart remains quiescent");
    drop(restarted);
    assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
}

#[tokio::test]
async fn transition_fault_cannot_strand_authenticated_result_and_legacy_strand_fences_promotion() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("spawn-result-transition-fault.sqlite3");
    let storage = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    seed_parent_grant(&storage).await;
    let gate = CoreDecisionGate::default();
    let service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&service).await;
    {
        let _guard = gate.acquire().await;
        seed_adapter_scoped_spawn(&storage).await;
    }
    report_progress(&service, &token, SpawnExecutionPhase::ExternalIdentityKnown).await;
    report_progress(&service, &token, SpawnExecutionPhase::HandshakeReconciling).await;
    report_successful_spawn(&service, &token).await;
    let before_fault = all_events(&storage).await;

    let trigger_sql = format!(
        "CREATE TRIGGER abort_spawn_result_transition\n\
         BEFORE INSERT ON events\n\
         WHEN NEW.kind = {}\n\
         BEGIN SELECT RAISE(ABORT, 'injected transition failure'); END;",
        StoredEventKind::CommandTransition as i32
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch(&trigger_sql).unwrap();

    let route_error = service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::Event(spawn_result(
                    FailureCode::ExecutionFailed,
                ))),
            },
            &token,
        ))
        .await
        .expect_err("the injected transition failure must fail authenticated ingress");
    assert_eq!(route_error.code(), tonic::Code::Internal);
    assert_eq!(
        all_events(&storage).await,
        before_fault,
        "the dedicated transaction must roll back Result, transition, and audit together"
    );

    // Reproduce the pre-fix source-without-transition prefix through explicit
    // low-level writes. Production authenticated ingress above cannot create
    // it, but replay must still fence one already present in durable history.
    let mut stranded_failure = spawn_result(FailureCode::ExecutionFailed);
    stranded_failure.sender = Some(ActorEndpointRef {
        actor_id: Some(ActorId {
            value: "pi".to_owned(),
        }),
        endpoint_id: adapter_registration().endpoint_id,
        ..ActorEndpointRef::default()
    });
    let stranded_payload = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: stranded_failure.encode_to_vec(),
    };
    connection
        .execute(
            "INSERT INTO events (authority_domain_id, kind, payload) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                domain().value,
                StoredEventKind::Observation as i32,
                stranded_payload.encode_to_vec()
            ],
        )
        .expect("backend-only SQL fixture inserts the historical stranded source");
    let stranded_event_id = EventId {
        authority_domain_id: Some(domain()),
        lsn: Some(Lsn {
            value: connection.last_insert_rowid() as u64,
        }),
    };
    let transition_error = storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(command_id()),
                    from_state: OperationState::Delivered as i32,
                    to_state: OperationState::Failed as i32,
                    failure_code: FailureCode::ExecutionFailed as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
        )
        .await
        .expect_err("the emulated separate transition insert must fault");
    assert!(matches!(transition_error, StorageError::WriteFailed { .. }));
    let stranded_prefix = all_events(&storage).await;
    assert_eq!(stranded_prefix.len(), before_fault.len() + 1);
    assert_eq!(stranded_prefix.last().unwrap().event_id, stranded_event_id);

    connection
        .execute_batch("DROP TRIGGER abort_spawn_result_transition;")
        .unwrap();
    report_session(&service, &token, 1, Some(correlation())).await;
    let driver_error = match SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        CoreDecisionGate::default(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    {
        Ok(_) => panic!("a conflicting stranded Result must not promote"),
        Err(error) => error,
    };
    assert!(matches!(
        driver_error,
        SpawnCompletionError::CorruptLog(message)
            if message.contains("conflicting Result evidence")
    ));
    assert_eq!(completion_counts(&all_events(&storage).await), (0, 0, 0));
}

#[tokio::test]
async fn live_staged_promotion_preserves_verified_authority_and_two_lever_revocation() {
    for report_first in [false, true] {
        let (storage, mut authority) = run_live_adapter_case(report_first).await;
        let descendant_id = GrantId {
            value: "desc:authority-main:spawn-1".to_owned(),
        };
        let descendant = authority
            .get_grant(&descendant_id)
            .expect("live completion projects descendant grant")
            .clone();
        assert_eq!(descendant.subject_actor_id, actor());
        assert_eq!(descendant.subject_endpoint_id, Some(endpoint()));
        let issuer_actor = actor();
        let issuer_endpoint = endpoint();
        let issuer_domain = domain();
        let issuer = IssuerRef {
            actor: &issuer_actor,
            endpoint: Some(&issuer_endpoint),
            authority_domain_id: &issuer_domain,
        };
        assert!(grant_authorizes(
            &descendant,
            &issuer,
            OperationKind::Instruct,
            &session_scope_for(1),
        ));

        let events = all_events(&storage).await;
        let completion_audit_id = events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::AuditRecord as i32)
            .filter_map(|event| AuditRecord::decode(event.payload.payload.as_slice()).ok())
            .find(|audit| audit.reason_code == "spawn_completion")
            .and_then(|audit| audit.audit_event_id)
            .unwrap();
        let durable_grant = events
            .iter()
            .find(|event| event.payload.kind == StoredEventKind::SpawnPromotionCommitted as i32)
            .and_then(|event| {
                SpawnPromotionCommitted::decode(event.payload.payload.as_slice())
                    .ok()?
                    .authority?
                    .descendant_grant
            })
            .unwrap();
        assert_eq!(durable_grant.audit_id, Some(completion_audit_id));
        assert_eq!(durable_grant.subject_actor_id, Some(actor()));

        ingest_revocation(
            &storage,
            &mut authority,
            &domain(),
            Revocation {
                authority_domain_id: Some(domain()),
                grant_id: Some(parent_grant_id()),
                revoked_by: Some(ActorEndpointRef {
                    actor_id: Some(actor()),
                    endpoint_id: Some(endpoint()),
                    device_id: Some(device()),
                    ..ActorEndpointRef::default()
                }),
                revoked_at: Some(Timestamp {
                    seconds: 2_000,
                    nanos: 0,
                }),
                revocation_generation: Some(Generation { value: 1 }),
                accepted_operation_policy: GrantRevocationPolicy::Continue as i32,
                reason: "parent_revoked".to_owned(),
                ..Revocation::default()
            },
        )
        .await
        .unwrap();
        let descendant = authority.get_grant(&descendant_id).unwrap();
        assert!(grant_authorizes(
            descendant,
            &issuer,
            OperationKind::Instruct,
            &session_scope_for(1),
        ));

        ingest_revocation(
            &storage,
            &mut authority,
            &domain(),
            Revocation {
                authority_domain_id: Some(domain()),
                grant_id: Some(descendant_id.clone()),
                revoked_by: Some(ActorEndpointRef {
                    actor_id: Some(actor()),
                    endpoint_id: Some(endpoint()),
                    device_id: Some(device()),
                    ..ActorEndpointRef::default()
                }),
                revoked_at: Some(Timestamp {
                    seconds: 2_001,
                    nanos: 0,
                }),
                revocation_generation: Some(Generation { value: 1 }),
                accepted_operation_policy: GrantRevocationPolicy::Continue as i32,
                reason: "descendant_revoked".to_owned(),
                ..Revocation::default()
            },
        )
        .await
        .unwrap();
        assert!(!grant_authorizes(
            authority.get_grant(&descendant_id).unwrap(),
            &issuer,
            OperationKind::Instruct,
            &session_scope_for(1),
        ));
    }
}

#[tokio::test]
async fn adapter_scoped_delivery_result_report_restart_and_descendant_submit() {
    for generation_bump in [false] {
        run_real_adapter_scoped_submit_case(generation_bump).await;
    }
}

async fn run_real_adapter_scoped_submit_case(generation_bump: bool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(if generation_bump {
        "spawn-bump-e2e.sqlite3"
    } else {
        "spawn-registration-e2e.sqlite3"
    });
    let storage = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    seed_adapter_parent_grant(&storage).await;
    seed_operator(&storage).await;

    let gate = CoreDecisionGate::default();
    let driver = SpawnCompletionDriver::bootstrap(
        storage.clone(),
        domain(),
        gate.clone(),
        production_audit(storage.clone()),
        clock(),
    )
    .await
    .unwrap();
    let control = ControlServiceImpl::new_with_security_and_decision_gate(
        storage.clone(),
        domain(),
        Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
        gate.clone(),
    )
    .await
    .unwrap();
    let auth = login_control(&control).await;
    let adapter_service = AdapterControlServiceImpl::new_with_decision_gate(
        storage.clone(),
        domain(),
        AdapterEvidenceVerifier::new([("pi", "adapter-test-secret")]).unwrap(),
        gate.clone(),
    )
    .await
    .unwrap();
    let token = attach_adapter(&adapter_service).await;
    let mut driver_task = tokio::spawn(driver.run());
    if generation_bump {
        report_session(&adapter_service, &token, 6, None).await;
    }

    let spawn = submitted_operation(
        "spawn-1",
        "spawn-1-key",
        OperationKind::Spawn,
        adapter_scope(),
    );
    let submitted = control
        .submit(authenticated_control(
            SubmitRequest {
                operation: Some(spawn.clone()),
            },
            &auth,
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(submitted.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(submitted.operation_state, OperationState::Accepted as i32);
    assert_eq!(submitted.failure_code, FailureCode::Unspecified as i32);
    assert_eq!(submitted.decision_grant_id, Some(parent_grant_id()));
    // Claim acceptance is the preceding contract leaf; seed its durable output
    // explicitly so this integration remains scoped to Leaf 6.
    seed_spawn_claim(&storage).await;
    let integration_claim = durable_claim(&all_events(&storage).await);

    let incompatible_targets = [
        ("spawn-existing-runtime", session_scope()),
        (
            "spawn-existing-resource",
            TargetScope {
                kind: TargetScopeKind::Resource as i32,
                resource: Some(ResourceIdentity {
                    adapter_id: adapter_scope().adapter_id,
                    resource_kind: Some(ResourceKind {
                        value: "runtime-pool".to_owned(),
                    }),
                    resource_id: Some(ResourceId {
                        value: "pool-1".to_owned(),
                    }),
                }),
                ..TargetScope::default()
            },
        ),
    ];
    for (command, target) in incompatible_targets {
        let rejected = control
            .submit(authenticated_control(
                SubmitRequest {
                    operation: Some(submitted_operation(
                        command,
                        &format!("{command}-key"),
                        OperationKind::Spawn,
                        target,
                    )),
                },
                &auth,
            ))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(rejected.outcome, SubmissionOutcome::Rejected as i32);
        assert_eq!(rejected.failure_code, FailureCode::TargetNotFound as i32);
    }
    let accepted_command_ids: Vec<_> = all_events(&storage)
        .await
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .map(|event| AcceptedOperation::decode(event.payload.payload.as_slice()).unwrap())
        .filter_map(|accepted| accepted.operation?.command_id)
        .map(|id| id.value)
        .collect();
    assert!(!accepted_command_ids
        .iter()
        .any(|id| id == "spawn-existing-runtime"));
    assert!(!accepted_command_ids
        .iter()
        .any(|id| id == "spawn-existing-resource"));

    let mut deliveries = adapter_service
        .receive_deliveries(authenticated(
            ReceiveRequest {
                adapter_id: adapter_scope().adapter_id,
                cursor: Some(Lsn { value: 0 }),
            },
            &token,
        ))
        .await
        .unwrap()
        .into_inner();
    let delivered = tokio::time::timeout(Duration::from_secs(2), deliveries.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let delivered_operation = delivered
        .operation
        .expect("delivery carries the accepted spawn");
    assert_eq!(delivered_operation.command_id, spawn.command_id);
    assert_eq!(delivered_operation.kind, OperationKind::Spawn as i32);
    assert_eq!(delivered_operation.target_scope, Some(adapter_scope()));
    acknowledge_delivery(&adapter_service, &token, &delivered_operation).await;
    report_progress_for_claim(
        &adapter_service,
        &token,
        SpawnExecutionPhase::ExternalIdentityKnown,
        integration_claim.clone(),
    )
    .await;
    report_progress_for_claim(
        &adapter_service,
        &token,
        SpawnExecutionPhase::HandshakeReconciling,
        integration_claim.clone(),
    )
    .await;
    let deferred_source = report_successful_spawn(&adapter_service, &token).await;

    let deferred_audits = storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![],
                actor_id: None,
                endpoint_id: None,
                command_id: Some(command_id()),
                grant_id: None,
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["spawn_completion_deferred".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(deferred_audits.records.len(), 1);
    let deferred_audit = &deferred_audits.records[0];
    assert_eq!(deferred_audit.kind, AuditEventKind::CommandRunning as i32);
    assert_eq!(deferred_audit.source_event_id, Some(deferred_source));
    assert_eq!(deferred_audit.command_id, Some(command_id()));
    assert_eq!(deferred_audit.failure_code, FailureCode::Unspecified as i32);
    assert!(deferred_audit.adapter_diagnostic.is_none());

    let as_of = all_events(&storage)
        .await
        .last()
        .and_then(|event| event.event_id.lsn)
        .expect("deferred prefix has an LSN")
        .value;
    let diagnostics = control
        .projection_state()
        .diagnostics_at(&storage, &domain(), as_of)
        .await
        .unwrap();
    let deferred_inspection = diagnostics.inspect_command(&command_id()).unwrap();
    assert_eq!(
        deferred_inspection.current_state,
        OperationState::Delivered as i32,
        "deferred success evidence must not present terminal success"
    );
    assert!(deferred_inspection.terminal_event_id.is_none());

    report_session(&adapter_service, &token, 1, Some(correlation())).await;
    report_progress_for_claim(
        &adapter_service,
        &token,
        SpawnExecutionPhase::SuccessEvidenceReported,
        integration_claim,
    )
    .await;

    tokio::select! {
        result = &mut driver_task => panic!("spawn completion driver exited early: {result:?}"),
        result = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if completion_counts(&all_events(&storage).await) == (1, 1, 1) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }) => result.expect("adapter-scoped accepted spawn completes with descendant authority"),
    }
    driver_task.abort();
    let _ = driver_task.await;
    drop(deliveries);
    drop(adapter_service);
    drop(control);
    drop(storage);

    let reopened = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    let restart_gate = CoreDecisionGate::default();
    let restarted_driver = SpawnCompletionDriver::bootstrap(
        reopened.clone(),
        domain(),
        restart_gate.clone(),
        production_audit(reopened.clone()),
        clock(),
    )
    .await
    .unwrap();
    drop(restarted_driver);
    assert_eq!(completion_counts(&all_events(&reopened).await), (1, 1, 1));

    let restarted_control = ControlServiceImpl::new_with_security_and_decision_gate(
        reopened,
        domain(),
        Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
        restart_gate,
    )
    .await
    .unwrap();
    let restarted_auth = login_control(&restarted_control).await;
    let subsequent = restarted_control
        .submit(authenticated_control(
            SubmitRequest {
                operation: Some(submitted_operation(
                    "instruct-after-spawn",
                    "instruct-after-spawn-key",
                    OperationKind::Instruct,
                    session_scope_for(1),
                )),
            },
            &restarted_auth,
        ))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(subsequent.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(
        subsequent.decision_grant_id,
        Some(GrantId {
            value: "desc:authority-main:spawn-1".to_owned(),
        })
    );
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
