use std::{collections::BTreeMap, env, fs, path::PathBuf};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, ActorEndpointRef, ActorId,
    AdapterCapability, AdapterId,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest,
    AuthorityDomainId, DeviceId, EndpointId, Generation, LoadSnapshotRequest, Lsn, Observation,
    ObservationKind, ObservationRequest, OperatorRecord, PayloadContentType, PayloadEnvelope,
    PrincipalEnrollment, ReceiveRequest, ResourceCapability, ResourceId, ResourceIdentity,
    ResourceKind, ResourceProjectionContract, ResourceReport, ResourceReportMutation,
    ResourceSnapshot, ResourceSnapshotReport, ResourceStateUpsert, ResourceViewReport,
    SchemaDescriptor, SnapshotViewKind, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind, VerifyOperatorPasswordRequest,
};
use patchbay_core::{
    resource::{ingest_resource_report, ResourceRegistry, ResourceReportMode, ValidatedResourceReport},
    storage::{RusqliteStorage, Storage},
};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    issuer::{OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER},
    rpc::{adapter_control_service_server::AdapterControlService, control_service_server::ControlService},
    service::ControlServiceImpl,
    state::ProjectionState,
};
use prost::Message;
use prost_types::Timestamp;
use serde::Deserialize;
use serde_json::Value;
use tonic::{Code, Request, Response};

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
    input: Value,
    expected_outcome: Value,
}

#[derive(Debug, Deserialize)]
struct ImplementationCheck { runner: String, case: String }
#[derive(Debug, Deserialize)]
struct RequestedCheck { vector_id: String, case: String }

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

fn string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    value.pointer(pointer).and_then(Value::as_str).ok_or_else(|| format!("missing string field {pointer}"))
}

fn boolean(value: &Value, pointer: &str) -> Result<bool, String> {
    value.pointer(pointer).and_then(Value::as_bool).ok_or_else(|| format!("missing boolean field {pointer}"))
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

fn authenticated_control<T>(message: T, session_id: &str, principal_id: &str, principal_secret: &str) -> Result<Request<T>, String> {
    let mut request = Request::new(message);
    for (header, value) in [
        (OPERATOR_SESSION_HEADER, session_id),
        (OPERATOR_ID_HEADER, OPERATOR_ACTOR),
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

async fn snapshot_reconciliation(vector: &ConformanceVector) -> Result<(), String> {
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
    {
        return Err("load_snapshot RPC did not return the vector's current resource replacement".to_owned());
    }
    Ok(())
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

async fn execute_case(vector: &ConformanceVector, case: &str) -> Result<(), String> {
    if vector.property_id.is_empty() || !matches!(vector.promotion_status.as_str(), "draft" | "promoted") {
        return Err("conformance vector has invalid property or promotion metadata".to_owned());
    }
    match case {
        "resource_disconnect_degrades_snapshot" => disconnect_degrades_snapshot(vector).await,
        "resource_snapshot_reconciliation" => snapshot_reconciliation(vector).await,
        "resource_observation_source_binding" => source_binding(vector).await,
        _ => Err(format!("unhandled {RUNNER} conformance case {}:{case}", vector.vector_id)),
    }
}

#[tokio::test]
async fn conformance_vector_runner() {
    let vectors = vectors();
    for request in requests() {
        let vector = vectors.get(&request.vector_id).unwrap_or_else(|| panic!("unknown vector id {}", request.vector_id));
        assert!(vector.implementation_checks.iter().any(|check| check.runner == RUNNER && check.case == request.case), "unregistered requested check {}:{}", request.vector_id, request.case);
        execute_case(vector, &request.case).await.unwrap_or_else(|error| panic!("{error}"));
        println!("PATCHBAY_CONFORMANCE_EXECUTED={}:{}", request.vector_id, request.case);
    }
}
