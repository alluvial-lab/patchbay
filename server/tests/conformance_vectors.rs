extern crate patchbay_test_support;
use std::{collections::BTreeMap, env, fs, path::PathBuf, sync::Arc};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, AcceptedOperation,
    ActorEndpointRef, ActorId, AdapterCapability, AdapterId, AdapterRegistration, AuditEventKind,
    AuditRecord,
    AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest, AuthorityDomainId, CommandId,
    DeviceId, EndpointId, FailureCode, Generation, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, LoadSnapshotRequest, Lsn, Observation, ObservationKind,
    ObservationRequest, Operation, OperationKind, OperationState, OperatorRecord,
    PayloadContentType, PayloadEnvelope, PrincipalEnrollment, ReceiveRequest, ResourceCapability,
    ResourceId, ResourceIdentity, ResourceKind, ResourceProjectionContract, ResourceReport,
    ResourceReportMutation, ResourceSnapshot, ResourceSnapshotReport, ResourceStateUnknown, ResourceStateUpsert,
    ResourceViewReport, RuntimeSessionId, SchemaDescriptor, SessionActivityState,
    SessionConnectivityState, SessionRegistered, SessionReport, SessionReportSourceCursor,
    SessionSnapshot, SessionState, SnapshotViewKind,
    StoredEventKind, StoredEventPayload, SubmissionOutcome, SubmitRequest, TargetScope,
    TargetScopeKind, TimeWindow, VerifyOperatorPasswordRequest, ViewRevision,
};
use patchbay_core::{
    authority::events as authority_events,
    resource::{ingest_resource_report, ResourceRegistry, ResourceReportMode, ValidatedResourceReport},
    session::{self, events as session_events},
    storage::{CoreGenerationStore, RusqliteStorage, Storage},
    time::TestClock,
};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    issuer::{OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER},
    rpc::{adapter_control_service_server::AdapterControlService, control_service_server::ControlService},
    service::ControlServiceImpl,
    snapshot::{decode_compatible_session_checkpoint, encode_session_checkpoint},
    state::ProjectionState,
};
use prost::Message;
use prost_types::Timestamp;
use serde::Deserialize;
use serde_json::Value;
use tonic::{Code, Request, Response};

#[cfg(feature = "conformance-fault-injection")]
use patchbay_core_server::adapter_service::AdapterServiceConformanceFault;

const RUNNER: &str = "rust-server";
const EVIDENCE: &str = "conformance-adapter-evidence";
const OPERATOR_ACTOR: &str = "conformance-operator";
const OPERATOR_PASSWORD: &str = "correct-password";

#[derive(Debug, Deserialize)]
struct ConformanceVector {
    vector_id: String,
    property_id: String,
    promotion_status: String,
    #[serde(default)]
    implementation_checks: Vec<ImplementationCheck>,
    #[serde(default)]
    mutation_witnesses: Vec<MutationWitness>,
    input: Value,
    expected_outcome: Value,
}

#[derive(Debug, Deserialize)]
struct ImplementationCheck { runner: String, case: String }
#[derive(Debug, Deserialize)]
struct MutationWitness { mutation_id: String, runner: String }
#[derive(Debug, Deserialize)]
struct RequestedCheck { vector_id: String, case: String }
#[derive(Debug, Deserialize)]
struct RequestedMutation { vector_id: String, mutation_id: String }

fn vectors() -> BTreeMap<String, ConformanceVector> {
    let vector_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/vectors");
    let mut files = fs::read_dir(vector_dir)
        .expect("conformance vector directory must be readable")
        .map(|entry| entry.expect("vector directory entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect::<Vec<_>>();
    files.sort();
    files.into_iter().map(|path| {
        let vector: ConformanceVector = serde_json::from_slice(&fs::read(&path).expect("conformance vector must be readable"))
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        (vector.vector_id.clone(), vector)
    }).collect()
}

fn requests() -> Vec<RequestedCheck> {
    env::var("PATCHBAY_CONFORMANCE_REQUESTS").ok()
        .map(|raw| serde_json::from_str(&raw).expect("requested checks must be valid JSON"))
        .unwrap_or_default()
}

fn mutation_requests() -> Vec<RequestedMutation> {
    env::var("PATCHBAY_CONFORMANCE_MUTATIONS").ok()
        .map(|raw| serde_json::from_str(&raw).expect("requested mutations must be valid JSON"))
        .unwrap_or_default()
}

fn string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    value.pointer(pointer).and_then(Value::as_str).ok_or_else(|| format!("missing string field {pointer}"))
}

fn boolean(value: &Value, pointer: &str) -> Result<bool, String> {
    value.pointer(pointer).and_then(Value::as_bool).ok_or_else(|| format!("missing boolean field {pointer}"))
}

fn unsigned(value: &Value, pointer: &str) -> Result<u64, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing unsigned field {pointer}"))
}

fn tuple(value: &Value, pointer: &str) -> Result<ResourceIdentity, String> {
    let values = value.pointer(pointer).and_then(Value::as_array).ok_or_else(|| format!("missing identity tuple {pointer}"))?;
    if values.len() != 3 { return Err(format!("identity tuple {pointer} must have three fields")); }
    Ok(ResourceIdentity {
        adapter_id: Some(AdapterId { value: values[0].as_str().ok_or("adapter id must be string")?.to_owned() }),
        resource_kind: Some(ResourceKind { value: values[1].as_str().ok_or("resource kind must be string")?.to_owned() }),
        resource_id: Some(ResourceId { value: values[2].as_str().ok_or("resource id must be string")?.to_owned() }),
    })
}

fn registration(domain: &AuthorityDomainId, adapter_id: &AdapterId, kind: &ResourceKind, generation: u64) -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(adapter_id.clone()),
        endpoint_id: Some(EndpointId { value: format!("{}-endpoint", adapter_id.value) }),
        authority_domain_id: Some(domain.clone()),
        adapter_generation: Some(Generation { value: generation }),
        capability: Some(AdapterCapability {
            target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
            resource_capabilities: vec![ResourceCapability {
                resource_kind: Some(kind.clone()),
                snapshot_support: AdapterSnapshotSupport::Partial as i32,
                projection_contract: Some(ResourceProjectionContract {
                    target_category: AdapterTargetCategory::OperationalResource as i32,
                    payload_schema: Some(SchemaDescriptor {
                        schema_ref: format!("{}.payload.v1", kind.value),
                        content_type: PayloadContentType::Protobuf as i32,
                    }),
                    projection_schema: Some(SchemaDescriptor {
                        schema_ref: format!("{}.projection.v1", kind.value),
                        content_type: PayloadContentType::Json as i32,
                    }),
                }),
            }],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    }
}

fn session_registration(
    domain: &AuthorityDomainId,
    adapter_id: &AdapterId,
    generation: u64,
) -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(adapter_id.clone()),
        endpoint_id: Some(EndpointId {
            value: format!("{}-endpoint", adapter_id.value),
        }),
        authority_domain_id: Some(domain.clone()),
        adapter_generation: Some(Generation { value: generation }),
        capability: Some(AdapterCapability {
            supported_operation_kinds: vec![OperationKind::Instruct as i32],
            streaming_support: true,
            session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
            cancellation_support: true,
            session_replacement_support: true,
            target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    }
}

fn attachment_token<T>(response: &Response<T>) -> Result<String, String> {
    response.metadata().get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .ok_or("attach response omitted attachment token")?
        .to_str().map(str::to_owned).map_err(|error| error.to_string())
}

fn authenticated<T>(message: T, adapter_id: &str, token: &str) -> Result<Request<T>, String> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(ADAPTER_ID_HEADER, adapter_id.parse().map_err(|error| format!("bad adapter metadata: {error}"))?);
    request.metadata_mut().insert(ADAPTER_EVIDENCE_HEADER, EVIDENCE.parse().map_err(|error| format!("bad evidence metadata: {error}"))?);
    request.metadata_mut().insert(ADAPTER_ATTACHMENT_TOKEN_HEADER, token.parse().map_err(|error| format!("bad token metadata: {error}"))?);
    Ok(request)
}

fn authenticated_control<T>(message: T, actor_id: &str, session_id: &str, principal_id: &str, principal_secret: &str) -> Result<Request<T>, String> {
    let mut request = Request::new(message);
    for (header, value) in [
        (OPERATOR_SESSION_HEADER, session_id),
        (OPERATOR_ID_HEADER, actor_id),
        (PRINCIPAL_ID_HEADER, principal_id),
        (PRINCIPAL_SECRET_HEADER, principal_secret),
    ] {
        request.metadata_mut().insert(header, value.parse().map_err(|error| format!("bad control metadata {header}: {error}"))?);
    }
    Ok(request)
}

fn observation(vector: &ConformanceVector, domain: &AuthorityDomainId, identity: ResourceIdentity) -> Result<ObservationRequest, String> {
    let payload_claim = vector.input.pointer("/payload_claim").ok_or("missing payload claim")?;
    Ok(ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::Event(Observation {
            authority_domain_id: Some(domain.clone()),
            sender: Some(ActorEndpointRef {
                actor_id: Some(ActorId { value: string(&vector.input, "/forged_sender_actor")?.to_owned() }),
                ..ActorEndpointRef::default()
            }),
            kind: ObservationKind::Event as i32,
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Resource as i32,
                resource: Some(identity),
                ..TargetScope::default()
            }),
            payload: Some(PayloadEnvelope {
                payload: serde_json::to_vec(payload_claim).map_err(|error| error.to_string())?,
                content_type: PayloadContentType::Json as i32,
                schema_ref: "adapter.claimed.authority.v1".to_owned(),
            }),
            ..Observation::default()
        })),
    })
}

fn case_pointer(case_name: &str, suffix: &str) -> String {
    format!("/{case_name}{suffix}")
}

fn operation_target(vector: &ConformanceVector, case_name: &str) -> Result<TargetScope, String> {
    let pointer = case_pointer(case_name, "/operation/target_scope");
    match string(&vector.input, &format!("{pointer}/kind"))? {
        "TARGET_SCOPE_KIND_RUNTIME_SESSION" => Ok(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(AdapterId {
                value: string(&vector.input, &format!("{pointer}/adapter_id/value"))?.to_owned(),
            }),
            deployment_scope: string(&vector.input, &format!("{pointer}/deployment_scope"))?
                .to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: string(
                    &vector.input,
                    &format!("{pointer}/runtime_session_id/value"),
                )?
                .to_owned(),
            }),
            session_generation: Some(Generation {
                value: vector
                    .input
                    .pointer(&format!("{pointer}/session_generation/value"))
                    .and_then(Value::as_u64)
                    .ok_or("missing session generation")?,
            }),
            ..TargetScope::default()
        }),
        "TARGET_SCOPE_KIND_RESOURCE" => Ok(TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(ResourceIdentity {
                adapter_id: Some(AdapterId {
                    value: string(
                        &vector.input,
                        &format!("{pointer}/resource/adapter_id/value"),
                    )?
                    .to_owned(),
                }),
                resource_kind: Some(ResourceKind {
                    value: string(
                        &vector.input,
                        &format!("{pointer}/resource/resource_kind/value"),
                    )?
                    .to_owned(),
                }),
                resource_id: Some(ResourceId {
                    value: string(
                        &vector.input,
                        &format!("{pointer}/resource/resource_id/value"),
                    )?
                    .to_owned(),
                }),
            }),
            ..TargetScope::default()
        }),
        kind => Err(format!("unsupported conformance target kind {kind}")),
    }
}

fn operation(
    vector: &ConformanceVector,
    case_name: &str,
    authority_domain_id: &AuthorityDomainId,
) -> Result<Operation, String> {
    let pointer = case_pointer(case_name, "/operation");
    let kind = match string(&vector.input, &format!("{pointer}/kind"))? {
        "OPERATION_KIND_INSTRUCT" => OperationKind::Instruct,
        "OPERATION_KIND_QUERY" => OperationKind::Query,
        kind => return Err(format!("unsupported conformance OperationKind {kind}")),
    };
    for suffix in [
        "/submitted_at",
        "/validity_window/starts_at",
        "/validity_window/expires_at",
    ] {
        let _ = string(&vector.input, &format!("{pointer}{suffix}"))?;
    }
    Ok(Operation {
        command_id: Some(CommandId {
            value: string(&vector.input, &format!("{pointer}/command_id/value"))?.to_owned(),
        }),
        authority_domain_id: Some(authority_domain_id.clone()),
        sender: Some(ActorEndpointRef {
            actor_id: vector
                .input
                .pointer(&format!("{pointer}/sender/actor_id/value"))
                .and_then(Value::as_str)
                .map(|value| ActorId {
                    value: value.to_owned(),
                }),
            ..ActorEndpointRef::default()
        }),
        kind: kind as i32,
        target_scope: Some(operation_target(vector, case_name)?),
        idempotency_key: string(&vector.input, &format!("{pointer}/idempotency_key"))?.to_owned(),
        payload: Some(PayloadEnvelope::default()),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp {
                seconds: 99,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 101,
                nanos: 0,
            }),
        }),
        submitted_at: Some(Timestamp {
            seconds: 100,
            nanos: 0,
        }),
        ..Operation::default()
    })
}

fn operation_actor(vector: &ConformanceVector, case_name: &str) -> Result<String, String> {
    let pointer = if case_name == "session_case" {
        "/session_case/operation/sender/actor_id/value"
    } else {
        "/resource_case/verified_issuer/actor_id"
    };
    Ok(string(&vector.input, pointer)?.to_owned())
}

fn operation_grant(
    vector: &ConformanceVector,
    case_name: &str,
    authority_domain_id: &AuthorityDomainId,
    target: &TargetScope,
) -> Result<Grant, String> {
    let (grant_id, kind, endpoint) = if case_name == "session_case" {
        let grant_target = "/session_case/preconditions/matching_grant/target_scope";
        if string(
            &vector.input,
            "/session_case/preconditions/matching_grant/allowed_operation_kinds/0",
        )? != "OPERATION_KIND_INSTRUCT"
            || string(&vector.input, &format!("{grant_target}/kind"))?
                != "TARGET_SCOPE_KIND_RUNTIME_SESSION"
            || string(&vector.input, &format!("{grant_target}/adapter_id/value"))?
                != target
                    .adapter_id
                    .as_ref()
                    .map_or("", |id| id.value.as_str())
            || string(&vector.input, &format!("{grant_target}/deployment_scope"))?
                != target.deployment_scope
            || string(
                &vector.input,
                &format!("{grant_target}/runtime_session_id/value"),
            )? != target
                .runtime_session_id
                .as_ref()
                .map_or("", |id| id.value.as_str())
            || vector
                .input
                .pointer(&format!("{grant_target}/session_generation/value"))
                .and_then(Value::as_u64)
                != target.session_generation.map(|generation| generation.value)
        {
            return Err("session matching grant differs from the requested target/kind".to_owned());
        }
        (
            string(
                &vector.input,
                "/session_case/preconditions/matching_grant/grant_id/value",
            )?,
            OperationKind::Instruct,
            Some(EndpointId {
                value: string(
                    &vector.input,
                    "/session_case/operation/sender/endpoint_id/value",
                )?
                .to_owned(),
            }),
        )
    } else {
        if string(
            &vector.input,
            "/resource_case/matching_grant/operation_kind",
        )? != "OPERATION_KIND_QUERY"
            || tuple(&vector.input, "/resource_case/matching_grant/target")?
                != target
                    .resource
                    .clone()
                    .ok_or("resource target missing identity")?
        {
            return Err(
                "resource matching grant differs from the requested target/kind".to_owned(),
            );
        }
        (
            string(&vector.input, "/resource_case/matching_grant/grant_id")?,
            OperationKind::Query,
            None,
        )
    };
    Ok(Grant {
        grant_id: Some(GrantId {
            value: grant_id.to_owned(),
        }),
        authority_domain_id: Some(authority_domain_id.clone()),
        subject_actor_id: Some(ActorId {
            value: operation_actor(vector, case_name)?,
        }),
        subject_endpoint_id: endpoint,
        target_scope: Some(target.clone()),
        allowed_operation_kinds: vec![kind as i32],
        provenance: Some(GrantProvenance {
            reason: "server conformance vector".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    })
}

async fn seed_operator(
    storage: &RusqliteStorage,
    authority_domain_id: &AuthorityDomainId,
    actor_id: &str,
) -> Result<(), String> {
    storage.append(
        authority_domain_id,
        StoredEventPayload {
            kind: StoredEventKind::OperatorRecord as i32,
            payload: OperatorRecord {
                actor_id: Some(ActorId { value: actor_id.to_owned() }),
                password_hash: "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g".to_owned(),
                created_at: Some(Timestamp { seconds: 1, nanos: 0 }),
                authority_domain_id: Some(authority_domain_id.clone()),
            }.encode_to_vec(),
        },
    ).await.map_err(|error| error.to_string())?;
    Ok(())
}

async fn seed_operation_target(
    storage: &RusqliteStorage,
    authority_domain_id: &AuthorityDomainId,
    target: &TargetScope,
) -> Result<(), String> {
    if TargetScopeKind::try_from(target.kind).ok() == Some(TargetScopeKind::RuntimeSession) {
        let registration = session_events::registered(
            authority_domain_id.clone(),
            SessionRegistered {
                adapter_id: target.adapter_id.clone(),
                deployment_scope: target.deployment_scope.clone(),
                runtime_session_id: target.runtime_session_id.clone(),
                session_generation: target.session_generation,
                initial_state: Some(SessionState {
                    connectivity: SessionConnectivityState::Live as i32,
                    activity: SessionActivityState::Idle as i32,
                }),
                project: "conformance".to_owned(),
                cwd: "/conformance".to_owned(),
                name: "session-vector".to_owned(),
                model: "provider/model".to_owned(),
                spawn_origin: None,
                source_cursor: None,
            },
        );
        storage
            .append(authority_domain_id, session_events::encode(&registration))
            .await
            .map_err(|error| error.to_string())?;
    } else {
        let identity = target
            .resource
            .clone()
            .ok_or("resource target missing identity")?;
        let adapter_id = identity
            .adapter_id
            .clone()
            .ok_or("resource identity missing adapter")?;
        let resource_kind = identity
            .resource_kind
            .clone()
            .ok_or("resource identity missing kind")?;
        let mut registry = ResourceRegistry::new();
        ingest_resource_report(
            storage,
            &mut registry,
            ValidatedResourceReport {
                authority_domain_id: authority_domain_id.clone(),
                adapter_id,
                adapter_generation: Generation { value: 1 },
                mode: ResourceReportMode::Delta,
                views: vec![ResourceViewReport {
                    resource_kind: Some(resource_kind),
                    completeness: AdapterSnapshotSupport::Partial as i32,
                    mutations: vec![ResourceReportMutation {
                        identity: Some(identity),
                        mutation: Some(resource_report_mutation::Mutation::Upsert(
                            ResourceStateUpsert {
                                resource_payload: Some(PayloadEnvelope {
                                    payload: vec![1],
                                    content_type: PayloadContentType::Protobuf as i32,
                                    schema_ref: "resource.schema".to_owned(),
                                }),
                                projection_payload: Some(PayloadEnvelope {
                                    payload: vec![2],
                                    content_type: PayloadContentType::Json as i32,
                                    schema_ref: "projection.schema".to_owned(),
                                }),
                            },
                        )),
                    }],
                }],
                observed_at: Timestamp {
                    seconds: 100,
                    nanos: 0,
                },
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn server_operation_case(
    vector: &ConformanceVector,
    case_name: &str,
    with_grant: bool,
) -> Result<(), String> {
    let authority_domain_id = AuthorityDomainId {
        value: string(
            &vector.input,
            &case_pointer(case_name, "/operation/authority_domain_id/value"),
        )?
        .to_owned(),
    };
    let actor_id = operation_actor(vector, case_name)?;
    let target = operation_target(vector, case_name)?;
    if case_name == "resource_case"
        && tuple(&vector.input, "/resource_case/registered_resource")?
            != target
                .resource
                .clone()
                .ok_or("resource target missing identity")?
    {
        return Err("registered resource differs from Operation target".to_owned());
    }
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    seed_operator(&storage, &authority_domain_id, &actor_id).await?;
    seed_operation_target(&storage, &authority_domain_id, &target).await?;
    if with_grant {
        storage
            .append(
                &authority_domain_id,
                authority_events::grant(
                    authority_domain_id.clone(),
                    operation_grant(vector, case_name, &authority_domain_id, &target)?,
                ),
            )
            .await
            .map_err(|error| error.to_string())?;
    } else {
        let grants_pointer = if case_name == "session_case" {
            "/session_case/available_grants"
        } else {
            "/resource_case/available_grants"
        };
        if vector
            .input
            .pointer(grants_pointer)
            .and_then(Value::as_array)
            .is_none_or(|grants| !grants.is_empty())
        {
            return Err(format!(
                "{case_name} missing-grant witness must provide an empty grant set"
            ));
        }
    }
    let service = ControlServiceImpl::new_with_clock(
        storage.clone(),
        authority_domain_id.clone(),
        Arc::new(TestClock::new(Timestamp {
            seconds: 100,
            nanos: 0,
        })),
    )
    .await?;
    let login = service
        .verify_operator_password(Request::new(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId {
                value: actor_id.clone(),
            }),
            password: OPERATOR_PASSWORD.to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId {
                    value: if case_name == "session_case" {
                        string(
                            &vector.input,
                            "/session_case/operation/sender/endpoint_id/value",
                        )?
                        .to_owned()
                    } else {
                        string(&vector.input, "/resource_case/verified_issuer/endpoint_id")?
                            .to_owned()
                    },
                }),
                device_id: Some(DeviceId {
                    value: "conformance-host".to_owned(),
                }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .map_err(|error| error.to_string())?
        .into_inner();
    let session_id = login
        .operator_session_id
        .as_ref()
        .ok_or("operation login omitted operator session")?
        .value
        .clone();
    let principal = login.principal.ok_or("operation login omitted principal")?;
    let result = service
        .submit(authenticated_control(
            SubmitRequest {
                operation: Some(operation(vector, case_name, &authority_domain_id)?),
            },
            &actor_id,
            &session_id,
            &principal.principal_id,
            &principal.secret,
        )?)
        .await
        .map_err(|error| error.to_string())?
        .into_inner();
    let events = storage
        .read_after(&authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(|error| error.to_string())?;
    let operation_events = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .collect::<Vec<_>>();
    let expected_pointer = format!("/{case_name}");
    if with_grant {
        let expected_append_count = vector
            .expected_outcome
            .pointer(&format!("{expected_pointer}/durable_record/append_count"))
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize;
        let durable = operation_events
            .first()
            .ok_or_else(|| format!("{case_name} server acceptance did not append an Operation"))?;
        let accepted = AcceptedOperation::decode(durable.payload.payload.as_slice())
            .map_err(|error| error.to_string())?;
        let durable_operation = accepted
            .operation
            .as_ref()
            .ok_or("durable acceptance omitted Operation")?;
        let expected_result_command = vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/submission_result/command_id/value"
            ))
            .and_then(Value::as_str);
        let expected_durable_command = vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/durable_record/command_id/value"
            ))
            .and_then(Value::as_str);
        let expected_deduplicated = vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/submission_result/deduplicated"
            ))
            .and_then(Value::as_bool);
        if string(
            &vector.expected_outcome,
            &format!("{expected_pointer}/submission_result/outcome"),
        )? != "SUBMISSION_OUTCOME_ACCEPTED"
            || string(
                &vector.expected_outcome,
                &format!("{expected_pointer}/submission_result/operation_state"),
            )? != "OPERATION_STATE_ACCEPTED"
            || string(
                &vector.expected_outcome,
                &format!("{expected_pointer}/durable_record/operation_state"),
            )? != "OPERATION_STATE_ACCEPTED"
            || result.outcome != SubmissionOutcome::Accepted as i32
            || result.operation_state != OperationState::Accepted as i32
            || operation_events.len() != expected_append_count
            || result.accepted_lsn != durable.event_id.lsn
            || result.command_id != durable_operation.command_id
            || expected_result_command.is_some_and(|expected| {
                result.command_id.as_ref().map(|id| id.value.as_str()) != Some(expected)
            })
            || expected_durable_command.is_some_and(|expected| {
                durable_operation
                    .command_id
                    .as_ref()
                    .map(|id| id.value.as_str())
                    != Some(expected)
            })
            || expected_deduplicated.is_some_and(|expected| result.deduplicated != expected)
        {
            return Err(format!(
                "{case_name} durable-acceptance outcome disagrees with server execution"
            ));
        }
        if let Some(expected_grant) = vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/submission_result/decision_grant_id"
            ))
            .and_then(Value::as_str)
        {
            if result
                .decision_grant_id
                .as_ref()
                .map(|id| id.value.as_str())
                != Some(expected_grant)
            {
                return Err(format!(
                    "{case_name} server acceptance selected the wrong grant"
                ));
            }
        }
    } else if string(
        &vector.expected_outcome,
        &format!("{expected_pointer}/submission_result/outcome"),
    )? != "SUBMISSION_OUTCOME_REJECTED"
        || string(
            &vector.expected_outcome,
            &format!("{expected_pointer}/submission_result/failure_code"),
        )? != "FAILURE_CODE_AUTHORIZATION_DENIED"
        || vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/submission_result/command_id/value"
            ))
            .and_then(Value::as_str)
            .is_some_and(|expected| {
                result.command_id.as_ref().map(|id| id.value.as_str()) != Some(expected)
            })
        || vector
            .expected_outcome
            .pointer(&format!(
                "{expected_pointer}/submission_result/deduplicated"
            ))
            .and_then(Value::as_bool)
            .is_some_and(|expected| result.deduplicated != expected)
        || result.outcome != SubmissionOutcome::Rejected as i32
        || result.failure_code != FailureCode::AuthorizationDenied as i32
        || !operation_events.is_empty()
        || boolean(
            &vector.expected_outcome,
            &format!("{expected_pointer}/durable_acceptance_record_created"),
        )?
        || boolean(
            &vector.expected_outcome,
            &format!("{expected_pointer}/delivered_to_adapter"),
        )?
    {
        return Err(format!(
            "{case_name} missing-grant outcome disagrees with server execution"
        ));
    }
    Ok(())
}

async fn server_operation_scenario(
    vector: &ConformanceVector,
    with_grant: bool,
) -> Result<(), String> {
    server_operation_case(vector, "session_case", with_grant).await?;
    server_operation_case(vector, "resource_case", with_grant).await
}

async fn disconnect_degrades_snapshot(vector: &ConformanceVector) -> Result<(), String> {
    let domain = AuthorityDomainId { value: string(&vector.input, "/authority_domain_id")?.to_owned() };
    let adapter_id = AdapterId { value: string(&vector.input, "/adapter_id")?.to_owned() };
    let generation = vector.input.pointer("/adapter_generation").and_then(Value::as_u64).ok_or("missing adapter generation")?;
    let identity = tuple(&vector.input, "/resource_identity")?;
    let kind = identity.resource_kind.clone().ok_or("resource identity missing kind")?;
    if identity.adapter_id.as_ref() != Some(&adapter_id)
        || string(&vector.input, "/disconnect")? != "abnormal_delivery_stream_drop"
    {
        return Err("disconnect witness has inconsistent adapter identity or source".to_owned());
    }
    let projection = vector.input.pointer("/current_projection").ok_or("missing projection")?;
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new(
        storage.clone(), domain.clone(), AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
    ).await?;
    let attached = service.attach(Request::new(AttachRequest {
        registration: Some(registration(&domain, &adapter_id, &kind, generation)),
        attachment_evidence: EVIDENCE.as_bytes().to_vec(),
    })).await.map_err(|error| error.to_string())?;
    let token = attachment_token(&attached)?;
    let projection_payload = serde_json::to_vec(projection).map_err(|error| error.to_string())?;
    service.ingest_observation(authenticated(
        ObservationRequest {
            authority_domain_id: Some(domain.clone()),
            observation: Some(observation_request::Observation::ResourceReport(ResourceReport {
                adapter_id: Some(adapter_id.clone()),
                adapter_generation: Some(Generation { value: generation }),
                report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
                    views: vec![ResourceViewReport {
                        resource_kind: Some(kind.clone()),
                        completeness: AdapterSnapshotSupport::Partial as i32,
                        mutations: vec![ResourceReportMutation {
                            identity: Some(identity.clone()),
                            mutation: Some(resource_report_mutation::Mutation::Upsert(ResourceStateUpsert {
                                resource_payload: Some(PayloadEnvelope {
                                    payload: vec![1], content_type: PayloadContentType::Protobuf as i32,
                                    schema_ref: format!("{}.payload.v1", kind.value),
                                }),
                                projection_payload: Some(PayloadEnvelope {
                                    payload: projection_payload, content_type: PayloadContentType::Json as i32,
                                    schema_ref: format!("{}.projection.v1", kind.value),
                                }),
                            })),
                        }],
                    }],
                })),
                observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
            })),
        },
        &adapter_id.value,
        &token,
    )?).await.map_err(|error| error.to_string())?;

    let stream = service.receive_deliveries(authenticated(
        ReceiveRequest { adapter_id: Some(adapter_id.clone()), cursor: Some(Lsn { value: 0 }) },
        &adapter_id.value,
        &token,
    )?).await.map_err(|error| error.to_string())?.into_inner();
    drop(stream);

    let expected_domain_identity = patchbay_core::resource::ResourceIdentity::try_from_wire(&identity).map_err(|error| error.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let rebuilt = patchbay_core::resource::rebuild_from_log(&storage, &domain).await.expect("resource replay");
            if rebuilt.get(&expected_domain_identity).is_some_and(|record| record.freshness == patchbay_contracts::patchbay::ResourceFreshnessState::Stale) {
                break;
            }
            tokio::task::yield_now().await;
        }
    }).await.map_err(|_| "disconnect degradation timed out".to_owned())?;
    let state = ProjectionState::rebuild(&storage, &domain).await?;
    let snapshot = state.materialize_resource_snapshot(domain, Timestamp { seconds: 101, nanos: 0 }).await;
    let record = snapshot.resources.first().ok_or("degraded snapshot omitted resource")?;
    if string(&vector.expected_outcome, "/snapshot_record/freshness")? != "RESOURCE_FRESHNESS_STATE_STALE"
        || record.freshness != patchbay_contracts::patchbay::ResourceFreshnessState::Stale as i32
        || boolean(&vector.expected_outcome, "/snapshot_record/has_cached_payload")? != record.resource_payload.is_some()
        || boolean(&vector.expected_outcome, "/snapshot_record/tombstoned")? != record.tombstoned
    {
        return Err("adapter disconnect did not produce honest stale cached resource snapshot".to_owned());
    }
    Ok(())
}

fn lsn_values(value: &Value, pointer: &str) -> Result<Vec<u64>, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing LSN list {pointer}"))?
        .iter()
        .map(|entry| {
            entry
                .pointer("/lsn/value")
                .or_else(|| entry.pointer("/value"))
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("invalid LSN entry in {pointer}"))
        })
        .collect()
}

async fn session_snapshot_reconciliation(vector: &ConformanceVector) -> Result<(), String> {
    let cached_lsn = vector
        .input
        .pointer("/session_case/cached_snapshot/snapshot_lsn/value")
        .and_then(Value::as_u64)
        .ok_or("missing cached session snapshot LSN")?;
    let current_revision = vector
        .input
        .pointer("/session_case/current_authoritative_view/current_revision_lsn/value")
        .and_then(Value::as_u64)
        .ok_or("missing current session revision")?;
    let cached_core_generation = vector
        .input
        .pointer("/session_case/cached_snapshot/core_generation/value")
        .and_then(Value::as_u64)
        .ok_or("missing cached session core generation")?;
    let current_core_generation = vector
        .input
        .pointer("/session_case/current_authoritative_view/core_generation/value")
        .and_then(Value::as_u64)
        .ok_or("missing current session core generation")?;
    let expected_core_generation = vector
        .expected_outcome
        .pointer("/session_case/replacement_core_generation/value")
        .and_then(Value::as_u64)
        .ok_or("missing expected session core generation")?;
    if current_core_generation == 0
        || cached_core_generation != current_core_generation
        || expected_core_generation != current_core_generation
    {
        return Err("session vector core-generation anchors disagree".to_owned());
    }
    let replacement_lsn = vector
        .expected_outcome
        .pointer("/session_case/snapshot_decision/replacement_required_from_lsn/value")
        .and_then(Value::as_u64)
        .ok_or("missing expected session replacement LSN")?;
    let cursor = vector
        .input
        .pointer("/session_case/subscription/cursor/value")
        .and_then(Value::as_u64)
        .ok_or("missing session replay cursor")?;
    let available = lsn_values(&vector.input, "/session_case/available_observations")?;
    let replayed = lsn_values(
        &vector.expected_outcome,
        "/session_case/replayed_observations",
    )?;
    let excluded = lsn_values(&vector.expected_outcome, "/session_case/excluded_lsns")?;
    let expected_replayed = available
        .iter()
        .copied()
        .filter(|lsn| *lsn > cursor)
        .collect::<Vec<_>>();
    let expected_excluded = available
        .iter()
        .copied()
        .filter(|lsn| *lsn <= cursor)
        .collect::<Vec<_>>();
    if vector
        .expected_outcome
        .pointer("/session_case/snapshot_decision/accepted")
        .and_then(Value::as_bool)
        != Some(false)
        || string(
            &vector.expected_outcome,
            "/session_case/snapshot_decision/failure_code",
        )? != "FAILURE_CODE_STALE_EVENT"
        || replacement_lsn != current_revision
        || current_revision <= cached_lsn
        || replayed != expected_replayed
        || excluded != expected_excluded
    {
        return Err("session snapshot/replay expectation contradicts stale rejection".to_owned());
    }

    let authority_domain_id = AuthorityDomainId {
        value: string(
            &vector.input,
            "/session_case/cached_snapshot/authority_domain_id/value",
        )?
        .to_owned(),
    };
    if string(
        &vector.input,
        "/session_case/current_authoritative_view/authority_domain_id/value",
    )? != authority_domain_id.value
    {
        return Err("session cached/current authority domains differ".to_owned());
    }
    let actor_id = string(
        &vector.input,
        "/session_case/subscription/subscriber/actor_id/value",
    )?
    .to_owned();
    let target = vector
        .input
        .pointer("/session_case/cached_snapshot/view_revisions/0/target_scope")
        .ok_or("missing session view target")?;
    let adapter_id = AdapterId {
        value: string(target, "/adapter_id/value")?.to_owned(),
    };
    let deployment_scope = string(target, "/deployment_scope")?.to_owned();
    let runtime_session_id = RuntimeSessionId {
        value: string(target, "/runtime_session_id/value")?.to_owned(),
    };
    let generation = Generation {
        value: target
            .pointer("/session_generation/value")
            .and_then(Value::as_u64)
            .ok_or("missing snapshot session generation")?,
    };

    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    storage
        .load_or_create_core_generation(
            &authority_domain_id,
            Generation {
                value: current_core_generation,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    seed_operator(&storage, &authority_domain_id, &actor_id).await?;
    for index in 0..(current_revision.saturating_sub(2)) {
        let filler = Grant {
            grant_id: Some(GrantId {
                value: format!("snapshot-filler-{index}"),
            }),
            authority_domain_id: Some(authority_domain_id.clone()),
            subject_actor_id: Some(ActorId {
                value: actor_id.clone(),
            }),
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::AuthorityDomain as i32,
                ..TargetScope::default()
            }),
            allowed_operation_kinds: vec![OperationKind::Query as i32],
            provenance: Some(GrantProvenance {
                reason: "snapshot revision fixture".to_owned(),
                ..GrantProvenance::default()
            }),
            revocation_policy: GrantRevocationPolicy::Continue as i32,
            ..Grant::default()
        };
        storage
            .append(
                &authority_domain_id,
                authority_events::grant(authority_domain_id.clone(), filler),
            )
            .await
            .map_err(|error| error.to_string())?;
    }
    let registration = session_events::registered(
        authority_domain_id.clone(),
        SessionRegistered {
            adapter_id: Some(adapter_id.clone()),
            deployment_scope: deployment_scope.clone(),
            runtime_session_id: Some(runtime_session_id.clone()),
            session_generation: Some(generation),
            initial_state: Some(SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            }),
            project: "conformance".to_owned(),
            cwd: "/conformance".to_owned(),
            name: "snapshot-vector".to_owned(),
            model: "provider/model".to_owned(),
            spawn_origin: None,
            source_cursor: None,
        },
    );
    let registration_event = storage
        .append(&authority_domain_id, session_events::encode(&registration))
        .await
        .map_err(|error| error.to_string())?;
    if registration_event.lsn.as_ref().map(|lsn| lsn.value) != Some(current_revision) {
        return Err("session fixture did not commit at the vector's current revision".to_owned());
    }

    if vector
        .input
        .pointer("/session_case/cached_snapshot/sessions")
        .and_then(Value::as_array)
        .is_none_or(|sessions| !sessions.is_empty())
        || string(
            &vector.input,
            "/session_case/cached_snapshot/materialized_at",
        )? != "2026-07-06T00:02:00Z"
    {
        return Err("session cached-checkpoint fixture is not the registered empty view".to_owned());
    }
    let cached_checkpoint = SessionSnapshot {
        authority_domain_id: Some(authority_domain_id.clone()),
        snapshot_lsn: Some(Lsn { value: cached_lsn }),
        core_generation: Some(Generation {
            value: cached_core_generation,
        }),
        sessions: Vec::new(),
        view_revisions: vec![ViewRevision {
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::RuntimeSession as i32,
                adapter_id: Some(adapter_id.clone()),
                deployment_scope: deployment_scope.clone(),
                runtime_session_id: Some(runtime_session_id.clone()),
                session_generation: Some(generation),
                ..TargetScope::default()
            }),
            revision_lsn: Some(Lsn { value: cached_lsn }),
        }],
        materialized_at: Some(Timestamp {
            seconds: 1_783_296_120,
            nanos: 0,
        }),
        lockdown: None,
    };
    let cached_checkpoint_payload = cached_checkpoint.encode_to_vec();
    storage
        .write_snapshot(
            &authority_domain_id,
            Lsn { value: cached_lsn },
            encode_session_checkpoint(&cached_checkpoint),
        )
        .await
        .map_err(|error| error.to_string())?;
    let stored_checkpoint = storage
        .load_latest_snapshot(
            &authority_domain_id,
            Some(Lsn { value: cached_lsn }),
        )
        .await
        .map_err(|error| error.to_string())?
        .ok_or("session stale checkpoint was not stored")?;
    if decode_compatible_session_checkpoint(
        &stored_checkpoint,
        &authority_domain_id,
        &Generation {
            value: current_core_generation,
        },
    )
    .map_err(|error| format!("seeded stale checkpoint is incompatible: {error}"))?
        != cached_checkpoint
    {
        return Err("stored session checkpoint differs from the compatible stale witness".to_owned());
    }

    let service = ControlServiceImpl::new_with_clock(
        storage.clone(),
        authority_domain_id.clone(),
        Arc::new(TestClock::new(Timestamp {
            seconds: 100,
            nanos: 0,
        })),
    )
    .await?;
    let login = service
        .verify_operator_password(Request::new(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId {
                value: actor_id.clone(),
            }),
            password: OPERATOR_PASSWORD.to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId {
                    value: "session-snapshot".to_owned(),
                }),
                device_id: Some(DeviceId {
                    value: "conformance-host".to_owned(),
                }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .map_err(|error| error.to_string())?
        .into_inner();
    let session_id = login
        .operator_session_id
        .as_ref()
        .ok_or("session snapshot login omitted operator session")?
        .value
        .clone();
    let principal = login
        .principal
        .ok_or("session snapshot login omitted principal")?;
    let response = service
        .load_snapshot(authenticated_control(
            LoadSnapshotRequest {
                authority_domain_id: Some(authority_domain_id.clone()),
                at_or_before: Some(Lsn { value: cached_lsn }),
                view_kind: SnapshotViewKind::Session as i32,
            },
            &actor_id,
            &session_id,
            &principal.principal_id,
            &principal.secret,
        )?)
        .await
        .map_err(|error| error.to_string())?
        .into_inner();
    let snapshot = SessionSnapshot::decode(response.snapshot_payload.as_slice())
        .map_err(|error| format!("load_snapshot returned a non-session payload: {error}"))?;
    let current_lsn = snapshot
        .snapshot_lsn
        .as_ref()
        .ok_or("session RPC snapshot missing LSN")?
        .value;
    let session = snapshot.sessions.iter().find(|session| {
        session.adapter_id.as_ref() == Some(&adapter_id)
            && session.deployment_scope == deployment_scope
            && session.runtime_session_id.as_ref() == Some(&runtime_session_id)
            && session.session_generation == Some(generation)
    });
    if response.view_kind != SnapshotViewKind::Session as i32
        || !response.present
        || response
            .event_id
            .as_ref()
            .and_then(|event| event.lsn.as_ref())
            .map(|lsn| lsn.value)
            != Some(current_lsn)
        || current_lsn < current_revision
        || current_lsn <= cached_lsn
        || response.snapshot_payload == cached_checkpoint_payload
        || snapshot.authority_domain_id.as_ref() != Some(&authority_domain_id)
        || snapshot.core_generation
            != Some(Generation {
                value: expected_core_generation,
            })
        || session.is_none()
        || vector
            .expected_outcome
            .pointer("/session_case/stored_checkpoint/seeded_at_lsn/value")
            .and_then(Value::as_u64)
            != Some(cached_lsn)
        || vector
            .expected_outcome
            .pointer("/session_case/stored_checkpoint/compatible")
            .and_then(Value::as_bool)
            != Some(true)
        || vector
            .expected_outcome
            .pointer("/session_case/stored_checkpoint/returned")
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err(
            "load_snapshot RPC did not replace the stale session view with current authority"
                .to_owned(),
        );
    }
    Ok(())
}

async fn resource_snapshot_reconciliation(vector: &ConformanceVector) -> Result<(), String> {
    let request_kind = string(&vector.input, "/resource_case/request/view_kind")?;
    let expected_request_kind = string(&vector.expected_outcome, "/resource_case/requested_view_kind")?;
    let expected_return_kind = string(&vector.expected_outcome, "/resource_case/returned_view_kind")?;
    if request_kind != "SNAPSHOT_VIEW_KIND_RESOURCE"
        || request_kind != expected_request_kind
        || request_kind != expected_return_kind
    {
        return Err("resource snapshot request/response discriminator disagrees".to_owned());
    }
    let cached_lsn = vector.input.pointer("/resource_case/cached_snapshot/snapshot_lsn/value").and_then(Value::as_u64).ok_or("missing cached snapshot LSN")?;
    let cached_core_generation = vector.input.pointer("/resource_case/cached_snapshot/core_generation/value").and_then(Value::as_u64).ok_or("missing cached resource core generation")?;
    let expected_core_generation = vector.expected_outcome.pointer("/resource_case/replacement/core_generation/value").and_then(Value::as_u64).ok_or("missing expected resource core generation")?;
    if cached_core_generation == 0 || cached_core_generation != expected_core_generation {
        return Err("resource vector core-generation anchors disagree".to_owned());
    }
    let report = vector.input.pointer("/resource_case/current_report").ok_or("missing current resource report")?;
    let identity = tuple(report, "/identity")?;
    let authority_domain_id = AuthorityDomainId {
        value: string(&vector.input, "/resource_case/cached_snapshot/authority_domain_id/value")?.to_owned(),
    };
    let adapter_id = identity.adapter_id.clone().ok_or("identity missing adapter")?;
    let resource_kind = identity.resource_kind.clone().ok_or("identity missing kind")?;
    let generation = report.pointer("/source_adapter_generation").and_then(Value::as_u64).ok_or("missing source generation")?;
    if string(report, "/mode")? != "snapshot"
        || string(report, "/completeness")? != "ADAPTER_SNAPSHOT_SUPPORT_AUTHORITATIVE"
    {
        return Err("snapshot witness must be authoritative".to_owned());
    }
    let resource_payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        string(report, "/resource_payload")?,
    ).map_err(|error| error.to_string())?;
    let projection_payload = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        string(report, "/projection_payload")?,
    ).map_err(|error| error.to_string())?;
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    storage
        .load_or_create_core_generation(
            &authority_domain_id,
            Generation {
                value: cached_core_generation,
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    storage.append(
        &authority_domain_id,
        StoredEventPayload {
            kind: StoredEventKind::OperatorRecord as i32,
            payload: OperatorRecord {
                actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
                password_hash: "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g".to_owned(),
                created_at: Some(Timestamp { seconds: 1, nanos: 0 }),
                authority_domain_id: Some(authority_domain_id.clone()),
            }.encode_to_vec(),
        },
    ).await.map_err(|error| error.to_string())?;
    storage.append(
        &authority_domain_id,
        authority_events::grant(
            authority_domain_id.clone(),
            Grant {
                grant_id: Some(GrantId { value: "resource-snapshot-exact-query".to_owned() }),
                authority_domain_id: Some(authority_domain_id.clone()),
                subject_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::Resource as i32,
                    resource: Some(identity.clone()),
                    ..TargetScope::default()
                }),
                allowed_operation_kinds: vec![OperationKind::Query as i32],
                provenance: Some(GrantProvenance {
                    reason: "resource snapshot conformance fixture".to_owned(),
                    ..GrantProvenance::default()
                }),
                revocation_policy: GrantRevocationPolicy::Continue as i32,
                ..Grant::default()
            },
        ),
    ).await.map_err(|error| error.to_string())?;
    let service = ControlServiceImpl::new(storage.clone(), authority_domain_id.clone()).await?;
    let login = service.verify_operator_password(Request::new(VerifyOperatorPasswordRequest {
        operator_actor_id: Some(ActorId { value: OPERATOR_ACTOR.to_owned() }),
        password: OPERATOR_PASSWORD.to_owned(),
        principal: Some(PrincipalEnrollment {
            endpoint_id: Some(EndpointId { value: "conformance-snapshot".to_owned() }),
            device_id: Some(DeviceId { value: "conformance-host".to_owned() }),
            endpoint_generation: Some(Generation { value: 1 }),
        }),
    })).await.map_err(|error| error.to_string())?.into_inner();
    let session_id = login.operator_session_id.as_ref().ok_or("snapshot login omitted operator session")?.value.clone();
    let principal = login.principal.ok_or("snapshot login omitted principal")?;

    let mut resources = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut resources,
        ValidatedResourceReport {
            authority_domain_id: authority_domain_id.clone(),
            adapter_id,
            adapter_generation: Generation { value: generation },
            mode: ResourceReportMode::Snapshot,
            views: vec![patchbay_contracts::patchbay::ResourceViewReport {
                resource_kind: Some(resource_kind),
                completeness: AdapterSnapshotSupport::Authoritative as i32,
                mutations: vec![patchbay_contracts::patchbay::ResourceReportMutation {
                    identity: Some(identity.clone()),
                    mutation: Some(patchbay_contracts::patchbay::resource_report_mutation::Mutation::Upsert(
                        patchbay_contracts::patchbay::ResourceStateUpsert {
                            resource_payload: Some(PayloadEnvelope {
                                payload: resource_payload.clone(),
                                content_type: PayloadContentType::Protobuf as i32,
                                schema_ref: "provider_pool.payload.v1".to_owned(),
                            }),
                            projection_payload: Some(PayloadEnvelope {
                                payload: projection_payload.clone(),
                                content_type: PayloadContentType::Json as i32,
                                schema_ref: "provider_pool.projection.v1".to_owned(),
                            }),
                        },
                    )),
                }],
            }],
            observed_at: Timestamp { seconds: 100, nanos: 0 },
        },
    ).await.map_err(|error| error.to_string())?;
    let requested_view_kind = match request_kind {
        "SNAPSHOT_VIEW_KIND_RESOURCE" => SnapshotViewKind::Resource,
        _ => return Err(format!("unsupported snapshot request discriminator {request_kind}")),
    };
    let response = service.load_snapshot(authenticated_control(
        LoadSnapshotRequest {
            authority_domain_id: Some(authority_domain_id.clone()),
            at_or_before: Some(Lsn { value: cached_lsn }),
            view_kind: requested_view_kind as i32,
        },
        OPERATOR_ACTOR,
        &session_id,
        &principal.principal_id,
        &principal.secret,
    )?).await.map_err(|error| error.to_string())?.into_inner();
    let snapshot = ResourceSnapshot::decode(response.snapshot_payload.as_slice())
        .map_err(|error| format!("load_snapshot returned a non-resource payload: {error}"))?;
    let current_lsn = snapshot.snapshot_lsn.as_ref().ok_or("RPC snapshot missing LSN")?.value;
    let resource = snapshot.resources.first().ok_or("RPC snapshot missing resource")?;
    if response.view_kind != requested_view_kind as i32
        || expected_return_kind != "SNAPSHOT_VIEW_KIND_RESOURCE"
        || !response.present
        || response.event_id.as_ref().and_then(|event| event.lsn.as_ref()).map(|lsn| lsn.value) != Some(current_lsn)
        || cached_lsn >= current_lsn
        || vector.expected_outcome.pointer("/resource_case/snapshot_decision/accepted").and_then(Value::as_bool) != Some(false)
        || vector.expected_outcome.pointer("/resource_case/snapshot_decision/replacement_required_from_lsn/value").and_then(Value::as_u64).is_none_or(|lsn| lsn <= cached_lsn)
        || vector.expected_outcome.pointer("/resource_case/replacement/resource_count").and_then(Value::as_u64) != Some(snapshot.resources.len() as u64)
        || tuple(&vector.expected_outcome, "/resource_case/replacement/identity")? != identity
        || resource.identity.as_ref() != Some(&identity)
        || string(&vector.expected_outcome, "/resource_case/replacement/freshness")? != "RESOURCE_FRESHNESS_STATE_CURRENT"
        || resource.freshness != patchbay_contracts::patchbay::ResourceFreshnessState::Current as i32
        || resource.resource_payload.as_ref().map(|payload| payload.payload.as_slice()) != Some(resource_payload.as_slice())
        || resource.projection_payload.as_ref().map(|payload| payload.payload.as_slice()) != Some(projection_payload.as_slice())
        || vector.expected_outcome.pointer("/resource_case/replacement/record_revision_equals_snapshot_lsn").and_then(Value::as_bool) != Some(resource.revision_lsn.as_ref().is_some_and(|revision| revision.value == current_lsn))
        || string(&vector.expected_outcome, "/resource_case/replacement/authority_domain_id")? != authority_domain_id.value
        || snapshot.authority_domain_id.as_ref() != Some(&authority_domain_id)
        || snapshot.core_generation
            != Some(Generation {
                value: expected_core_generation,
            })
    {
        return Err("load_snapshot RPC did not return the vector's current resource replacement".to_owned());
    }
    Ok(())
}

async fn snapshot_reconciliation(vector: &ConformanceVector) -> Result<(), String> {
    session_snapshot_reconciliation(vector).await?;
    resource_snapshot_reconciliation(vector).await
}

async fn source_binding(vector: &ConformanceVector) -> Result<(), String> {
    let domain = AuthorityDomainId { value: string(&vector.input, "/authority_domain_id")?.to_owned() };
    let adapter_id = AdapterId { value: string(&vector.input, "/authenticated_adapter_id")?.to_owned() };
    let generation = vector.input.pointer("/attachment_generation").and_then(Value::as_u64).ok_or("missing attachment generation")?;
    if generation < 2 { return Err("source-binding witness needs a stale predecessor generation".to_owned()); }
    let exact = tuple(&vector.input, "/target_identity")?;
    let kind = exact.resource_kind.clone().ok_or("target identity missing kind")?;
    if exact.adapter_id.as_ref() != Some(&adapter_id) { return Err("target must belong to authenticated adapter".to_owned()); }

    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new(
        storage.clone(), domain.clone(), AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
    ).await.map_err(|error| error.to_string())?;
    let old = service.attach(Request::new(AttachRequest {
        registration: Some(registration(&domain, &adapter_id, &kind, generation - 1)),
        attachment_evidence: EVIDENCE.as_bytes().to_vec(),
    })).await.map_err(|error| error.to_string())?;
    let old_token = attachment_token(&old)?;
    let current = service.attach(Request::new(AttachRequest {
        registration: Some(registration(&domain, &adapter_id, &kind, generation)),
        attachment_evidence: EVIDENCE.as_bytes().to_vec(),
    })).await.map_err(|error| error.to_string())?;
    let current_token = attachment_token(&current)?;
    let before = storage.read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 }).await.map_err(|error| error.to_string())?;

    let missing = service.ingest_observation(Request::new(observation(vector, &domain, exact.clone())?)).await.expect_err("missing channel evidence must reject");
    if missing.code() != Code::Unauthenticated || string(&vector.expected_outcome, "/unauthenticated/status")? != "UNAUTHENTICATED" {
        return Err("missing attachment did not reject unauthenticated".to_owned());
    }
    let stale = service.ingest_observation(authenticated(observation(vector, &domain, exact.clone())?, &adapter_id.value, &old_token)?).await.expect_err("stale token must reject");
    if stale.code() != Code::Unauthenticated || string(&vector.expected_outcome, "/stale_token/status")? != "UNAUTHENTICATED" {
        return Err("stale attachment token did not reject unauthenticated".to_owned());
    }
    if vector.property_id == "TokenCommuneCurrentGenerationSourceAuthenticated" {
        let stale_generation = service.ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(ResourceReport {
                    adapter_id: Some(adapter_id.clone()),
                    adapter_generation: Some(Generation { value: generation - 1 }),
                    report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
                        views: vec![ResourceViewReport {
                            resource_kind: Some(kind.clone()),
                            completeness: AdapterSnapshotSupport::Partial as i32,
                            mutations: Vec::new(),
                        }],
                    })),
                    observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
                })),
            },
            &adapter_id.value,
            &current_token,
        )?).await.expect_err("stale report generation must reject");
        if stale_generation.code() != Code::FailedPrecondition
            || string(&vector.expected_outcome, "/stale_generation/status")? != "FAILED_PRECONDITION"
            || boolean(&vector.expected_outcome, "/stale_generation/resource_state_appended")?
            || boolean(&vector.expected_outcome, "/stale_generation_appended")?
        {
            return Err("stale report generation was not fenced before resource append".to_owned());
        }
    }
    let cross = service.ingest_observation(authenticated(observation(vector, &domain, tuple(&vector.input, "/cross_adapter_target")?)?, &adapter_id.value, &current_token)?).await.expect_err("cross-adapter target must reject");
    if cross.code() != Code::PermissionDenied || string(&vector.expected_outcome, "/cross_adapter_target/status")? != "PERMISSION_DENIED" {
        return Err("cross-adapter resource Observation was not fenced".to_owned());
    }
    service.ingest_observation(authenticated(observation(vector, &domain, exact)?, &adapter_id.value, &current_token)?)
        .await.map_err(|error| error.to_string())?;
    let after = storage.read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 }).await.map_err(|error| error.to_string())?;
    let appended = &after[before.len()..];
    if appended.len() != 1
        || appended[0].payload.kind != StoredEventKind::Observation as i32
        || !boolean(&vector.expected_outcome, "/authenticated_owner/observation_appended")?
        || string(&vector.expected_outcome, "/authenticated_owner/stored_event_kind")? != "STORED_EVENT_KIND_OBSERVATION"
        || boolean(&vector.expected_outcome, "/unauthenticated/observation_appended")?
        || boolean(&vector.expected_outcome, "/stale_token/observation_appended")?
        || boolean(&vector.expected_outcome, "/cross_adapter_target/observation_appended")?
        || boolean(&vector.expected_outcome, "/forged_claim/authority_changed")?
        || boolean(&vector.expected_outcome, "/forged_claim/resource_state_changed")?
        || boolean(&vector.expected_outcome, "/forged_claim/operation_created")?
        || after.iter().any(|event| matches!(StoredEventKind::try_from(event.payload.kind).ok(), Some(StoredEventKind::Grant | StoredEventKind::Operation | StoredEventKind::ResourceState)))
    {
        return Err("authenticated Observation source/isolation result disagrees with vector".to_owned());
    }
    Ok(())
}

#[derive(Debug)]
enum MutationExecutionError {
    Harness(String),
    Oracle(String),
}

impl MutationExecutionError {
    fn harness(error: impl ToString) -> Self {
        Self::Harness(error.to_string())
    }

    fn oracle(message: impl Into<String>) -> Self {
        Self::Oracle(message.into())
    }
}

struct SessionReportValues<'a> {
    session_generation: u64,
    adapter_generation: u64,
    revision: u64,
    model: &'a str,
}

fn session_report_request(
    domain: &AuthorityDomainId,
    adapter: &AdapterId,
    deployment_scope: &str,
    runtime_session_id: &RuntimeSessionId,
    values: SessionReportValues<'_>,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::SessionReport(
            SessionReport {
                adapter_id: Some(adapter.clone()),
                deployment_scope: deployment_scope.to_owned(),
                runtime_session_id: Some(runtime_session_id.clone()),
                session_generation: Some(Generation {
                    value: values.session_generation,
                }),
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
                project: "conformance".to_owned(),
                cwd: "/conformance".to_owned(),
                name: "source-ordering".to_owned(),
                model: values.model.to_owned(),
                spawn_origin: None,
                source_cursor: Some(SessionReportSourceCursor {
                    adapter_generation: Some(Generation {
                        value: values.adapter_generation,
                    }),
                    revision: values.revision,
                }),
            },
        )),
    }
}

async fn attach_session_adapter(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    domain: &AuthorityDomainId,
    adapter: &AdapterId,
    generation: u64,
) -> Result<String, MutationExecutionError> {
    let response = service
        .attach(Request::new(AttachRequest {
            registration: Some(session_registration(domain, adapter, generation)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .map_err(MutationExecutionError::harness)?;
    attachment_token(&response).map_err(MutationExecutionError::harness)
}

async fn materialized_session_snapshot(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> Result<SessionSnapshot, MutationExecutionError> {
    let state = ProjectionState::rebuild(storage, domain)
        .await
        .map_err(MutationExecutionError::harness)?;
    Ok(state
        .materialize_session_snapshot(
            domain.clone(),
            Timestamp {
                seconds: 200,
                nanos: 0,
            },
        )
        .await)
}

fn session_state_event_count(events: &[patchbay_core::storage::RecordedEvent]) -> usize {
    events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::SessionState as i32)
        .count()
}

struct ExpectedSessionSnapshot<'a> {
    session_generation: u64,
    adapter_generation: u64,
    revision: u64,
    model: &'a str,
}

fn assert_session_snapshot(
    snapshot: &SessionSnapshot,
    adapter: &AdapterId,
    deployment_scope: &str,
    runtime_session_id: &RuntimeSessionId,
    expected: ExpectedSessionSnapshot<'_>,
) -> Result<(), MutationExecutionError> {
    let session = snapshot
        .sessions
        .iter()
        .find(|session| {
            session.adapter_id.as_ref() == Some(adapter)
                && session.deployment_scope == deployment_scope
                && session.runtime_session_id.as_ref() == Some(runtime_session_id)
        })
        .ok_or_else(|| MutationExecutionError::oracle("session snapshot omitted target"))?;
    if session
        .session_generation
        .as_ref()
        .map(|generation| generation.value)
        != Some(expected.session_generation)
        || session.model != expected.model
        || session
            .last_source_cursor
            .as_ref()
            .and_then(|cursor| cursor.adapter_generation.as_ref())
            .map(|generation| generation.value)
            != Some(expected.adapter_generation)
        || session
            .last_source_cursor
            .as_ref()
            .map(|cursor| cursor.revision)
            != Some(expected.revision)
    {
        return Err(MutationExecutionError::oracle(format!(
            "session snapshot disagreed with expected {}/{}:{}: {session:?}",
            expected.model, expected.adapter_generation, expected.revision
        )));
    }
    Ok(())
}

#[cfg(feature = "conformance-fault-injection")]
async fn assert_session_hot_equals_replay(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> Result<(), MutationExecutionError> {
    let hot = service.conformance_session_registry().await;
    let replay = session::rebuild_from_log(storage, domain)
        .await
        .map_err(MutationExecutionError::harness)?;
    if hot != replay {
        return Err(MutationExecutionError::oracle(
            "authenticated service session projection differed from fresh replay",
        ));
    }
    Ok(())
}

#[cfg(not(feature = "conformance-fault-injection"))]
async fn assert_session_hot_equals_replay(
    _service: &AdapterControlServiceImpl<RusqliteStorage>,
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> Result<(), MutationExecutionError> {
    let first = session::rebuild_from_log(storage, domain)
        .await
        .map_err(MutationExecutionError::harness)?;
    let second = session::rebuild_from_log(storage, domain)
        .await
        .map_err(MutationExecutionError::harness)?;
    if first != second {
        return Err(MutationExecutionError::oracle(
            "fresh session replays disagreed",
        ));
    }
    Ok(())
}

async fn run_session_report_source_trace(
    vector: &ConformanceVector,
    storage: &RusqliteStorage,
    service: &AdapterControlServiceImpl<RusqliteStorage>,
) -> Result<(), MutationExecutionError> {
    let domain = AuthorityDomainId {
        value: string(&vector.input, "/authority_domain_id")
            .map_err(MutationExecutionError::harness)?
            .to_owned(),
    };
    let adapter = AdapterId {
        value: string(&vector.input, "/adapter_id")
            .map_err(MutationExecutionError::harness)?
            .to_owned(),
    };
    let deployment_scope =
        string(&vector.input, "/deployment_scope").map_err(MutationExecutionError::harness)?;
    let runtime_session_id = RuntimeSessionId {
        value: string(&vector.input, "/runtime_session_id")
            .map_err(MutationExecutionError::harness)?
            .to_owned(),
    };
    let runtime_generation = unsigned(&vector.input, "/runtime_session_generation")
        .map_err(MutationExecutionError::harness)?;
    let initial_adapter_generation = unsigned(&vector.input, "/initial_attachment_generation")
        .map_err(MutationExecutionError::harness)?;
    let token =
        attach_session_adapter(service, &domain, &adapter, initial_adapter_generation).await?;

    for index in 0..2 {
        let pointer = format!("/primary_reports/{index}");
        service
            .ingest_observation(
                authenticated(
                    session_report_request(
                        &domain,
                        &adapter,
                        deployment_scope,
                        &runtime_session_id,
                        SessionReportValues {
                            session_generation: runtime_generation,
                            adapter_generation: unsigned(
                                &vector.input,
                                &format!("{pointer}/adapter_generation"),
                            )
                            .map_err(MutationExecutionError::harness)?,
                            revision: unsigned(&vector.input, &format!("{pointer}/revision"))
                                .map_err(MutationExecutionError::harness)?,
                            model: string(&vector.input, &format!("{pointer}/model"))
                                .map_err(MutationExecutionError::harness)?,
                        },
                    ),
                    &adapter.value,
                    &token,
                )
                .map_err(MutationExecutionError::harness)?,
            )
            .await
            .map_err(|error| {
                MutationExecutionError::harness(format!(
                    "current source report {index} failed: {error}"
                ))
            })?;
    }

    let before_stale = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    let stale_pointer = "/primary_reports/2";
    let stale = service
        .ingest_observation(
            authenticated(
                session_report_request(
                    &domain,
                    &adapter,
                    deployment_scope,
                    &runtime_session_id,
                    SessionReportValues {
                        session_generation: runtime_generation,
                        adapter_generation: unsigned(
                            &vector.input,
                            &format!("{stale_pointer}/adapter_generation"),
                        )
                        .map_err(MutationExecutionError::harness)?,
                        revision: unsigned(&vector.input, &format!("{stale_pointer}/revision"))
                            .map_err(MutationExecutionError::harness)?,
                        model: string(&vector.input, &format!("{stale_pointer}/model"))
                            .map_err(MutationExecutionError::harness)?,
                    },
                ),
                &adapter.value,
                &token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await;
    match stale {
        Err(status) if status.code() == Code::FailedPrecondition => {}
        Err(status) => {
            return Err(MutationExecutionError::oracle(format!(
                "delayed report returned {:?}, expected FAILED_PRECONDITION",
                status.code()
            )));
        }
        Ok(_) => {
            return Err(MutationExecutionError::oracle(
                "delayed non-increasing report was accepted",
            ));
        }
    }
    if string(&vector.expected_outcome, "/primary/delayed_status")
        .map_err(MutationExecutionError::harness)?
        != "FAILED_PRECONDITION"
    {
        return Err(MutationExecutionError::harness(
            "vector expected stale status is not FAILED_PRECONDITION",
        ));
    }

    let after_stale = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    let expected_session_events = unsigned(
        &vector.expected_outcome,
        "/primary/session_state_event_count",
    )
    .map_err(MutationExecutionError::harness)? as usize;
    if session_state_event_count(&before_stale) != expected_session_events
        || session_state_event_count(&after_stale) != expected_session_events
    {
        return Err(MutationExecutionError::oracle(
            "delayed report changed the session-state event count",
        ));
    }
    let stale_audit = after_stale
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::AuditRecord as i32)
        .filter_map(|event| AuditRecord::decode(event.payload.payload.as_slice()).ok())
        .find(|audit| audit.reason_code == "session_report_source_cursor_stale")
        .ok_or_else(|| MutationExecutionError::oracle("stale source audit was not durable"))?;
    if stale_audit.kind != AuditEventKind::StaleEventIgnored as i32
        || stale_audit.failure_code != FailureCode::StaleEvent as i32
        || string(&vector.expected_outcome, "/primary/audit_kind")
            .map_err(MutationExecutionError::harness)?
            != "AUDIT_EVENT_KIND_STALE_EVENT_IGNORED"
        || string(&vector.expected_outcome, "/primary/audit_failure_code")
            .map_err(MutationExecutionError::harness)?
            != "FAILURE_CODE_STALE_EVENT"
        || string(&vector.expected_outcome, "/primary/audit_reason_code")
            .map_err(MutationExecutionError::harness)?
            != stale_audit.reason_code
    {
        return Err(MutationExecutionError::oracle(
            "stale source audit disagreed with the vector",
        ));
    }

    let primary_snapshot = materialized_session_snapshot(storage, &domain).await?;
    assert_session_snapshot(
        &primary_snapshot,
        &adapter,
        deployment_scope,
        &runtime_session_id,
        ExpectedSessionSnapshot {
            session_generation: runtime_generation,
            adapter_generation: unsigned(
                &vector.expected_outcome,
                "/primary/snapshot_adapter_generation",
            )
            .map_err(MutationExecutionError::harness)?,
            revision: unsigned(&vector.expected_outcome, "/primary/snapshot_revision")
                .map_err(MutationExecutionError::harness)?,
            model: string(&vector.expected_outcome, "/primary/snapshot_model")
                .map_err(MutationExecutionError::harness)?,
        },
    )?;
    if !boolean(&vector.expected_outcome, "/primary/hot_equals_replay")
        .map_err(MutationExecutionError::harness)?
    {
        return Err(MutationExecutionError::harness(
            "vector does not require hot/replay equality",
        ));
    }
    assert_session_hot_equals_replay(service, storage, &domain).await?;

    let replacement_generation = unsigned(
        &vector.input,
        "/adapter_generation_reset/attachment_generation",
    )
    .map_err(MutationExecutionError::harness)?;
    let replacement_token =
        attach_session_adapter(service, &domain, &adapter, replacement_generation).await?;
    service
        .ingest_observation(
            authenticated(
                session_report_request(
                    &domain,
                    &adapter,
                    deployment_scope,
                    &runtime_session_id,
                    SessionReportValues {
                        session_generation: runtime_generation,
                        adapter_generation: replacement_generation,
                        revision: unsigned(
                            &vector.input,
                            "/adapter_generation_reset/accepted_revision",
                        )
                        .map_err(MutationExecutionError::harness)?,
                        model: string(&vector.input, "/adapter_generation_reset/accepted_model")
                            .map_err(MutationExecutionError::harness)?,
                    },
                ),
                &adapter.value,
                &replacement_token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await
        .map_err(MutationExecutionError::harness)?;
    let before_old_producer = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    let old_producer = service
        .ingest_observation(
            authenticated(
                session_report_request(
                    &domain,
                    &adapter,
                    deployment_scope,
                    &runtime_session_id,
                    SessionReportValues {
                        session_generation: runtime_generation,
                        adapter_generation: unsigned(
                            &vector.input,
                            "/adapter_generation_reset/old_adapter_generation",
                        )
                        .map_err(MutationExecutionError::harness)?,
                        revision: unsigned(&vector.input, "/adapter_generation_reset/old_revision")
                            .map_err(MutationExecutionError::harness)?,
                        model: string(&vector.input, "/adapter_generation_reset/old_model")
                            .map_err(MutationExecutionError::harness)?,
                    },
                ),
                &adapter.value,
                &replacement_token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await;
    if !matches!(old_producer, Err(ref status) if status.code() == Code::FailedPrecondition)
        || string(
            &vector.expected_outcome,
            "/adapter_generation_reset/old_producer_status",
        )
        .map_err(MutationExecutionError::harness)?
            != "FAILED_PRECONDITION"
    {
        return Err(MutationExecutionError::oracle(
            "old adapter producer was not fenced",
        ));
    }
    let after_old_producer = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    if session_state_event_count(&before_old_producer)
        != session_state_event_count(&after_old_producer)
        || boolean(
            &vector.expected_outcome,
            "/adapter_generation_reset/old_producer_mutated",
        )
        .map_err(MutationExecutionError::harness)?
    {
        return Err(MutationExecutionError::oracle(
            "old adapter producer changed session state",
        ));
    }
    assert_session_snapshot(
        &materialized_session_snapshot(storage, &domain).await?,
        &adapter,
        deployment_scope,
        &runtime_session_id,
        ExpectedSessionSnapshot {
            session_generation: runtime_generation,
            adapter_generation: unsigned(
                &vector.expected_outcome,
                "/adapter_generation_reset/snapshot_adapter_generation",
            )
            .map_err(MutationExecutionError::harness)?,
            revision: unsigned(
                &vector.expected_outcome,
                "/adapter_generation_reset/snapshot_revision",
            )
            .map_err(MutationExecutionError::harness)?,
            model: string(
                &vector.expected_outcome,
                "/adapter_generation_reset/accepted_model",
            )
            .map_err(MutationExecutionError::harness)?,
        },
    )?;

    let new_runtime_generation = unsigned(
        &vector.input,
        "/runtime_generation_reset/session_generation",
    )
    .map_err(MutationExecutionError::harness)?;
    service
        .ingest_observation(
            authenticated(
                session_report_request(
                    &domain,
                    &adapter,
                    deployment_scope,
                    &runtime_session_id,
                    SessionReportValues {
                        session_generation: new_runtime_generation,
                        adapter_generation: replacement_generation,
                        revision: unsigned(
                            &vector.input,
                            "/runtime_generation_reset/accepted_revision",
                        )
                        .map_err(MutationExecutionError::harness)?,
                        model: string(&vector.input, "/runtime_generation_reset/accepted_model")
                            .map_err(MutationExecutionError::harness)?,
                    },
                ),
                &adapter.value,
                &replacement_token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await
        .map_err(MutationExecutionError::harness)?;
    let before_old_runtime = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    let old_runtime = service
        .ingest_observation(
            authenticated(
                session_report_request(
                    &domain,
                    &adapter,
                    deployment_scope,
                    &runtime_session_id,
                    SessionReportValues {
                        session_generation: unsigned(
                            &vector.input,
                            "/runtime_generation_reset/old_session_generation",
                        )
                        .map_err(MutationExecutionError::harness)?,
                        adapter_generation: replacement_generation,
                        revision: unsigned(&vector.input, "/runtime_generation_reset/old_revision")
                            .map_err(MutationExecutionError::harness)?,
                        model: string(&vector.input, "/runtime_generation_reset/old_model")
                            .map_err(MutationExecutionError::harness)?,
                    },
                ),
                &adapter.value,
                &replacement_token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await;
    if !matches!(old_runtime, Err(ref status) if status.code() == Code::FailedPrecondition)
        || string(
            &vector.expected_outcome,
            "/runtime_generation_reset/old_runtime_status",
        )
        .map_err(MutationExecutionError::harness)?
            != "FAILED_PRECONDITION"
    {
        return Err(MutationExecutionError::oracle(
            "old runtime generation was not fenced",
        ));
    }
    let after_old_runtime = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .map_err(MutationExecutionError::harness)?;
    if session_state_event_count(&before_old_runtime)
        != session_state_event_count(&after_old_runtime)
        || boolean(
            &vector.expected_outcome,
            "/runtime_generation_reset/old_runtime_mutated",
        )
        .map_err(MutationExecutionError::harness)?
    {
        return Err(MutationExecutionError::oracle(
            "old runtime generation changed session state",
        ));
    }
    assert_session_snapshot(
        &materialized_session_snapshot(storage, &domain).await?,
        &adapter,
        deployment_scope,
        &runtime_session_id,
        ExpectedSessionSnapshot {
            session_generation: unsigned(
                &vector.expected_outcome,
                "/runtime_generation_reset/snapshot_session_generation",
            )
            .map_err(MutationExecutionError::harness)?,
            adapter_generation: unsigned(
                &vector.expected_outcome,
                "/runtime_generation_reset/snapshot_adapter_generation",
            )
            .map_err(MutationExecutionError::harness)?,
            revision: unsigned(
                &vector.expected_outcome,
                "/runtime_generation_reset/snapshot_revision",
            )
            .map_err(MutationExecutionError::harness)?,
            model: string(
                &vector.expected_outcome,
                "/runtime_generation_reset/accepted_model",
            )
            .map_err(MutationExecutionError::harness)?,
        },
    )?;
    assert_session_hot_equals_replay(service, storage, &domain).await
}

async fn session_report_source_ordering(vector: &ConformanceVector) -> Result<(), String> {
    let domain = AuthorityDomainId {
        value: string(&vector.input, "/authority_domain_id")?.to_owned(),
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain,
        AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
    )
    .await?;
    run_session_report_source_trace(vector, &storage, &service)
        .await
        .map_err(|error| match error {
            MutationExecutionError::Harness(message) | MutationExecutionError::Oracle(message) => {
                message
            }
        })
}

fn resource_upsert(identity: ResourceIdentity, kind: &ResourceKind) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(identity),
        mutation: Some(resource_report_mutation::Mutation::Upsert(
            ResourceStateUpsert {
                resource_payload: Some(PayloadEnvelope {
                    payload: vec![1],
                    content_type: PayloadContentType::Protobuf as i32,
                    schema_ref: format!("{}.payload.v1", kind.value),
                }),
                projection_payload: Some(PayloadEnvelope {
                    payload: b"{}".to_vec(),
                    content_type: PayloadContentType::Json as i32,
                    schema_ref: format!("{}.projection.v1", kind.value),
                }),
            },
        )),
    }
}

fn resource_report_request(
    domain: &AuthorityDomainId,
    adapter: &AdapterId,
    kind: &ResourceKind,
    generation: u64,
    mutations: Vec<ResourceReportMutation>,
    observed_seconds: i64,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::ResourceReport(
            ResourceReport {
                adapter_id: Some(adapter.clone()),
                adapter_generation: Some(Generation { value: generation }),
                report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
                    views: vec![ResourceViewReport {
                        resource_kind: Some(kind.clone()),
                        completeness: AdapterSnapshotSupport::Partial as i32,
                        mutations,
                    }],
                })),
                observed_at: Some(Timestamp {
                    seconds: observed_seconds,
                    nanos: 0,
                }),
            },
        )),
    }
}

async fn materialized_resource_snapshot(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> Result<ResourceSnapshot, MutationExecutionError> {
    let state = ProjectionState::rebuild(storage, domain)
        .await
        .map_err(MutationExecutionError::harness)?;
    Ok(state
        .materialize_resource_snapshot(
            domain.clone(),
            Timestamp {
                seconds: 200,
                nanos: 0,
            },
        )
        .await)
}

#[cfg(feature = "conformance-fault-injection")]
fn assert_source_snapshot_unchanged(
    before: &ResourceSnapshot,
    after: &ResourceSnapshot,
) -> Result<(), MutationExecutionError> {
    if before.resources == after.resources && before.view_revisions == after.view_revisions {
        Ok(())
    } else {
        Err(MutationExecutionError::oracle(
            "rejected source evidence changed the materialized resource snapshot",
        ))
    }
}

fn snapshot_freshness(
    snapshot: &ResourceSnapshot,
    identity: &ResourceIdentity,
) -> Option<patchbay_contracts::patchbay::ResourceFreshnessState> {
    snapshot
        .resources
        .iter()
        .find(|resource| resource.identity.as_ref() == Some(identity))
        .and_then(|resource| {
            patchbay_contracts::patchbay::ResourceFreshnessState::try_from(resource.freshness).ok()
        })
}

async fn run_token_degradation_trace_with_service(
    vector: &ConformanceVector,
    storage: &RusqliteStorage,
    service: &AdapterControlServiceImpl<RusqliteStorage>,
) -> Result<(), MutationExecutionError> {
    let domain = AuthorityDomainId {
        value: "token-conformance".into(),
    };
    let adapter = AdapterId {
        value: "token-commune".into(),
    };
    let kind = ResourceKind {
        value: "token-commune.provider-pool".into(),
    };
    let steps = vector
        .input
        .pointer("/steps")
        .and_then(Value::as_array)
        .ok_or_else(|| MutationExecutionError::harness("degradation vector is missing steps"))?;
    let step_id = |step: usize, field: &str| -> Result<String, MutationExecutionError> {
        steps
            .get(step)
            .and_then(|value| value.get(field))
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                MutationExecutionError::harness(format!(
                    "degradation step {step} is missing {field}"
                ))
            })
    };
    let identity = |id: &str| ResourceIdentity {
        adapter_id: Some(adapter.clone()),
        resource_kind: Some(kind.clone()),
        resource_id: Some(ResourceId { value: id.into() }),
    };
    let a = identity(&step_id(0, "cached")?);
    let unknown = identity(&step_id(0, "no_payload")?);
    let b = identity(&step_id(3, "listed")?);
    let current = patchbay_contracts::patchbay::ResourceFreshnessState::Current;
    let stale = patchbay_contracts::patchbay::ResourceFreshnessState::Stale;
    let unknown_state = patchbay_contracts::patchbay::ResourceFreshnessState::Unknown;

    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(&domain, &adapter, &kind, 1)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .map_err(MutationExecutionError::harness)?;
    let token = attachment_token(&attached).map_err(MutationExecutionError::harness)?;
    let ingest = |request| {
        authenticated(request, &adapter.value, &token).map_err(MutationExecutionError::harness)
    };

    service
        .ingest_observation(ingest(resource_report_request(
            &domain,
            &adapter,
            &kind,
            1,
            vec![
                resource_upsert(a.clone(), &kind),
                ResourceReportMutation {
                    identity: Some(unknown.clone()),
                    mutation: Some(resource_report_mutation::Mutation::Unknown(
                        ResourceStateUnknown {},
                    )),
                },
            ],
            100,
        ))?)
        .await
        .map_err(MutationExecutionError::harness)?;
    let snapshot = materialized_resource_snapshot(storage, &domain).await?;
    if snapshot_freshness(&snapshot, &a) != Some(current)
        || snapshot_freshness(&snapshot, &unknown) != Some(unknown_state)
    {
        return Err(MutationExecutionError::oracle(
            "baseline ingress did not materialize current and unknown resources",
        ));
    }

    service
        .ingest_observation(ingest(resource_report_request(
            &domain,
            &adapter,
            &kind,
            1,
            vec![],
            101,
        ))?)
        .await
        .map_err(MutationExecutionError::harness)?;
    let snapshot = materialized_resource_snapshot(storage, &domain).await?;
    if snapshot_freshness(&snapshot, &a) != Some(stale)
        || snapshot_freshness(&snapshot, &unknown) != Some(unknown_state)
    {
        return Err(MutationExecutionError::oracle(
            "empty PARTIAL ingress did not materialize stale and unknown truth",
        ));
    }

    service
        .ingest_observation(ingest(resource_report_request(
            &domain,
            &adapter,
            &kind,
            1,
            vec![resource_upsert(a.clone(), &kind)],
            102,
        ))?)
        .await
        .map_err(MutationExecutionError::harness)?;
    let snapshot = materialized_resource_snapshot(storage, &domain).await?;
    if snapshot_freshness(&snapshot, &a) != Some(current) {
        return Err(MutationExecutionError::oracle(
            "pre-disconnect report did not restore current state",
        ));
    }

    let stream = service
        .receive_deliveries(
            authenticated(
                ReceiveRequest {
                    adapter_id: Some(adapter.clone()),
                    cursor: Some(Lsn { value: 0 }),
                },
                &adapter.value,
                &token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await
        .map_err(MutationExecutionError::harness)?
        .into_inner();
    drop(stream);
    let disconnect_snapshot = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let snapshot = materialized_resource_snapshot(storage, &domain).await?;
            if snapshot_freshness(&snapshot, &a) != Some(current) {
                return Ok::<_, MutationExecutionError>(snapshot);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| {
        MutationExecutionError::oracle(
            "disconnect degradation did not reach the materialized snapshot",
        )
    })??;
    if snapshot_freshness(&disconnect_snapshot, &a) != Some(stale) {
        return Err(MutationExecutionError::oracle(
            "abnormal stream loss did not materialize stale cached state",
        ));
    }

    let replacement = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(&domain, &adapter, &kind, 2)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .map_err(MutationExecutionError::harness)?;
    let replacement_token =
        attachment_token(&replacement).map_err(MutationExecutionError::harness)?;
    service
        .ingest_observation(
            authenticated(
                resource_report_request(
                    &domain,
                    &adapter,
                    &kind,
                    2,
                    vec![resource_upsert(b.clone(), &kind)],
                    103,
                ),
                &adapter.value,
                &replacement_token,
            )
            .map_err(MutationExecutionError::harness)?,
        )
        .await
        .map_err(MutationExecutionError::harness)?;
    let snapshot = materialized_resource_snapshot(storage, &domain).await?;
    if snapshot_freshness(&snapshot, &a) != Some(stale)
        || snapshot_freshness(&snapshot, &b) != Some(current)
        || snapshot_freshness(&snapshot, &unknown) != Some(unknown_state)
    {
        return Err(MutationExecutionError::oracle(
            "generation-2 PARTIAL reconnect violated the materialized snapshot oracle",
        ));
    }
    Ok(())
}

async fn run_token_degradation_trace(vector: &ConformanceVector) -> Result<(), String> {
    let domain = AuthorityDomainId {
        value: "token-conformance".into(),
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain,
        AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
    )
    .await?;
    run_token_degradation_trace_with_service(vector, &storage, &service)
        .await
        .map_err(|error| match error {
            MutationExecutionError::Harness(message) | MutationExecutionError::Oracle(message) => {
                message
            }
        })
}

#[cfg(feature = "conformance-fault-injection")]
async fn kill_session_source_ordering_mutation(
    vector: &ConformanceVector,
    mutation_id: &str,
) -> Result<(), String> {
    if mutation_id != "accept-nonincreasing-session-revision" {
        return Err(format!(
            "unhandled session source-order mutation {mutation_id}"
        ));
    }
    session_report_source_ordering(vector).await?;
    let domain = AuthorityDomainId {
        value: string(&vector.input, "/authority_domain_id")?.to_owned(),
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new_with_conformance_fault(
        storage.clone(),
        domain,
        AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
        AdapterServiceConformanceFault::AcceptNonIncreasingSessionRevision,
    )
    .await?;
    match run_session_report_source_trace(vector, &storage, &service).await {
        Err(MutationExecutionError::Oracle(_)) => Ok(()),
        Err(MutationExecutionError::Harness(error)) => Err(format!(
            "session source-order mutation {mutation_id} had a harness failure: {error}"
        )),
        Ok(()) => Err(format!(
            "session source-order mutation {mutation_id} survived the authenticated ingress oracle"
        )),
    }
}

#[cfg(not(feature = "conformance-fault-injection"))]
async fn kill_session_source_ordering_mutation(
    _vector: &ConformanceVector,
    _mutation_id: &str,
) -> Result<(), String> {
    Err("Rust mutation witnesses require conformance-fault-injection".into())
}

#[cfg(feature = "conformance-fault-injection")]
async fn kill_degradation_mutation(
    vector: &ConformanceVector,
    mutation_id: &str,
) -> Result<(), String> {
    run_token_degradation_trace(vector).await?;
    let fault = match mutation_id {
        "skip-empty-partial-report" => {
            AdapterServiceConformanceFault::IgnoreEmptyPartialResourceReport
        }
        "disconnect-remains-current" => {
            AdapterServiceConformanceFault::KeepResourcesCurrentOnDisconnect
        }
        _ => return Err(format!("unhandled degradation mutation {mutation_id}")),
    };
    let domain = AuthorityDomainId {
        value: "token-conformance".into(),
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new_with_conformance_fault(
        storage.clone(),
        domain,
        AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
        fault,
    )
    .await?;
    match run_token_degradation_trace_with_service(vector, &storage, &service).await {
        Err(MutationExecutionError::Oracle(_)) => Ok(()),
        Err(MutationExecutionError::Harness(error)) => Err(format!(
            "degradation mutation {mutation_id} had a harness failure: {error}"
        )),
        Ok(()) => Err(format!(
            "degradation mutation {mutation_id} survived the service-boundary snapshot oracle"
        )),
    }
}

#[cfg(not(feature = "conformance-fault-injection"))]
async fn kill_degradation_mutation(
    _vector: &ConformanceVector,
    _mutation_id: &str,
) -> Result<(), String> {
    Err("Rust mutation witnesses require conformance-fault-injection".into())
}

#[cfg(feature = "conformance-fault-injection")]
async fn kill_source_ingress_mutation(
    vector: &ConformanceVector,
    mutation_id: &str,
) -> Result<(), String> {
    source_binding(vector).await?;
    let domain = AuthorityDomainId {
        value: string(&vector.input, "/authority_domain_id")?.to_owned(),
    };
    let adapter = AdapterId {
        value: string(&vector.input, "/authenticated_adapter_id")?.to_owned(),
    };
    let generation = vector
        .input
        .pointer("/attachment_generation")
        .and_then(Value::as_u64)
        .ok_or("missing attachment generation")?;
    let exact = tuple(&vector.input, "/target_identity")?;
    let kind = exact
        .resource_kind
        .clone()
        .ok_or("target identity missing kind")?;
    let fault = match mutation_id {
        "ignore-generation-equality" => AdapterServiceConformanceFault::IgnoreResourceGeneration,
        "accept-prior-attachment-token" => {
            AdapterServiceConformanceFault::AcceptPriorAttachmentToken
        }
        "compare-local-id-only" => {
            AdapterServiceConformanceFault::NormalizeResourceOwnerToAuthenticatedAdapter
        }
        _ => return Err(format!("unhandled source-ingress mutation {mutation_id}")),
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let service = AdapterControlServiceImpl::new_with_conformance_fault(
        storage.clone(),
        domain.clone(),
        AdapterEvidenceVerifier::new(EVIDENCE).map_err(|error| error.to_string())?,
        fault,
    )
    .await?;
    let old = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(&domain, &adapter, &kind, generation - 1)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .map_err(|error| error.to_string())?;
    let old_token = attachment_token(&old)?;
    let current = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(&domain, &adapter, &kind, generation)),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .map_err(|error| error.to_string())?;
    let current_token = attachment_token(&current)?;
    let before = materialized_resource_snapshot(&storage, &domain)
        .await
        .map_err(|error| format!("source mutation snapshot setup failed: {error:?}"))?;
    let attempted_identity = if mutation_id == "compare-local-id-only" {
        tuple(&vector.input, "/cross_adapter_target")?
    } else {
        exact
    };
    let attempted_generation = if mutation_id == "ignore-generation-equality" {
        generation - 1
    } else {
        generation
    };
    let token = if mutation_id == "accept-prior-attachment-token" {
        &old_token
    } else {
        &current_token
    };
    service
        .ingest_observation(authenticated(
            resource_report_request(
                &domain,
                &adapter,
                &kind,
                attempted_generation,
                vec![resource_upsert(attempted_identity, &kind)],
                100,
            ),
            &adapter.value,
            token,
        )?)
        .await
        .map_err(|error| {
            format!("source mutation {mutation_id} failed before its snapshot oracle: {error}")
        })?;
    let after = materialized_resource_snapshot(&storage, &domain)
        .await
        .map_err(|error| format!("source mutation snapshot load failed: {error:?}"))?;
    match assert_source_snapshot_unchanged(&before, &after) {
        Err(MutationExecutionError::Oracle(_)) => Ok(()),
        Err(MutationExecutionError::Harness(error)) => Err(format!(
            "source-ingress mutation {mutation_id} had a harness failure: {error}"
        )),
        Ok(()) => Err(format!(
            "source-ingress mutation {mutation_id} survived the materialized snapshot oracle"
        )),
    }
}

#[cfg(not(feature = "conformance-fault-injection"))]
async fn kill_source_ingress_mutation(
    _vector: &ConformanceVector,
    _mutation_id: &str,
) -> Result<(), String> {
    Err("Rust mutation witnesses require conformance-fault-injection".into())
}

async fn execute_case(vector: &ConformanceVector, case: &str) -> Result<(), String> {
    if vector.property_id.is_empty() || !matches!(vector.promotion_status.as_str(), "draft" | "promoted") {
        return Err("conformance vector has invalid property or promotion metadata".to_owned());
    }
    match case {
        "operation_durable_acceptance" => server_operation_scenario(vector, true).await,
        "operation_missing_grant" => server_operation_scenario(vector, false).await,
        "resource_disconnect_degrades_snapshot" => disconnect_degrades_snapshot(vector).await,
        "token_commune_degradation_projection" => run_token_degradation_trace(vector).await,
        "snapshot_reconciliation" => snapshot_reconciliation(vector).await,
        "session_report_source_ordering" => session_report_source_ordering(vector).await,
        "resource_observation_source_binding" | "token_commune_current_generation_source_binding" => source_binding(vector).await,
        _ => Err(format!("unhandled {RUNNER} conformance case {}:{case}", vector.vector_id)),
    }
}

#[tokio::test]
async fn conformance_vector_runner() {
    let vectors = vectors();
    let requested = if env::var("PATCHBAY_CONFORMANCE_REQUESTS").is_ok() {
        requests()
    } else {
        vectors.values().flat_map(|vector| vector.implementation_checks.iter()
            .filter(|check| check.runner == RUNNER)
            .map(|check| RequestedCheck { vector_id: vector.vector_id.clone(), case: check.case.clone() }))
            .collect()
    };
    for request in requested {
        let vector = vectors.get(&request.vector_id).unwrap_or_else(|| panic!("unknown vector id {}", request.vector_id));
        assert!(vector.implementation_checks.iter().any(|check| check.runner == RUNNER && check.case == request.case), "unregistered requested check {}:{}", request.vector_id, request.case);
        execute_case(vector, &request.case).await.unwrap_or_else(|error| panic!("{error}"));
        println!("PATCHBAY_CONFORMANCE_EXECUTED={}:{}", request.vector_id, request.case);
    }
    for request in mutation_requests() {
        let vector = vectors.get(&request.vector_id).unwrap_or_else(|| panic!("unknown mutation vector id {}", request.vector_id));
        assert!(vector.mutation_witnesses.iter().any(|witness| witness.runner == RUNNER && witness.mutation_id == request.mutation_id),
            "unregistered requested mutation {}:{}", request.vector_id, request.mutation_id);
        let result = if vector.property_id == "TokenCommuneDegradationHonesty" {
            kill_degradation_mutation(vector, &request.mutation_id).await
        } else if vector.property_id == "SessionReportSourceOrdering" {
            kill_session_source_ordering_mutation(vector, &request.mutation_id).await
        } else {
            kill_source_ingress_mutation(vector, &request.mutation_id).await
        };
        result.unwrap_or_else(|error| panic!("{error}"));
        println!("PATCHBAY_CONFORMANCE_MUTATION_KILLED={}:{}", request.vector_id, request.mutation_id);
    }
}
