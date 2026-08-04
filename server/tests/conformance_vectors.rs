use std::{collections::BTreeMap, env, fs, path::PathBuf};

use patchbay_contracts::patchbay::{
    observation_request, ActorEndpointRef, ActorId, AdapterCapability, AdapterId,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest,
    AuthorityDomainId, EndpointId, Generation, Observation, ObservationKind, ObservationRequest,
    PayloadContentType, PayloadEnvelope, ResourceCapability, ResourceId, ResourceIdentity,
    ResourceKind, ResourceProjectionContract, SchemaDescriptor, StoredEventKind, TargetScope,
    TargetScopeKind,
};
use patchbay_core::storage::{RusqliteStorage, Storage};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    rpc::adapter_control_service_server::AdapterControlService,
};
use serde::Deserialize;
use serde_json::Value;
use tonic::{Code, Request, Response};

const RUNNER: &str = "rust-server";
const EVIDENCE: &str = "conformance-adapter-evidence";

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
