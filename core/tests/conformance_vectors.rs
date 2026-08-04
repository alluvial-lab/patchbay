use std::{collections::BTreeMap, env, fs, path::PathBuf};

use patchbay_contracts::patchbay::{
    resource_state_mutation, ActorEndpointRef, ActorId, AdapterId, AdapterSnapshotSupport,
    AuthorityDomainId, CommandId, DeviceId, EndpointId, Generation, Grant, GrantId,
    GrantProvenance, GrantRevocationPolicy, Observation, ObservationKind, Operation,
    OperationKind, OperationState, PayloadContentType, PayloadEnvelope, ResourceId,
    ResourceKind, ResourceStateEvent, ResourceStateMutation, ResourceStateUpsert,
    ResourceViewStateUpdate, StoredEventKind, SubmissionOutcome, TimeWindow,
};
use patchbay_core::{
    acceptance::{
        ingest_observation, submit_with_clock, target_key_for, ActiveElicitation, CommandIndex,
        CommandSnapshot, CommandStateLookup, ElicitationContractLookup,
    },
    authority::{ingest_grant, target_scope_matches, AuthorityRegistry, IssuerContext},
    resource::{events, rebuild_from_log, ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    storage::{event_id, RecordedEvent, RusqliteStorage, Storage},
    target::TargetRegistry,
    time::TestClock,
};
use prost::Message;
use prost_types::Timestamp;
use serde::Deserialize;
use serde_json::Value;

const RUNNER: &str = "rust-core";

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
struct ImplementationCheck {
    runner: String,
    case: String,
}

#[derive(Debug, Deserialize)]
struct RequestedCheck {
    vector_id: String,
    case: String,
}

fn vectors() -> BTreeMap<String, ConformanceVector> {
    let vector_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/vectors");
    let mut files = fs::read_dir(vector_dir)
        .expect("conformance vector directory must be readable")
        .map(|entry| entry.expect("vector directory entry must be readable").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .collect::<Vec<_>>();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let vector: ConformanceVector = serde_json::from_slice(
                &fs::read(&path).expect("conformance vector must be readable"),
            )
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            (vector.vector_id.clone(), vector)
        })
        .collect()
}

fn requests() -> Vec<RequestedCheck> {
    env::var("PATCHBAY_CONFORMANCE_REQUESTS")
        .ok()
        .map(|raw| serde_json::from_str(&raw).expect("requested checks must be valid JSON"))
        .unwrap_or_default()
}

fn string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field {pointer}"))
}

fn boolean(value: &Value, pointer: &str) -> Result<bool, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing boolean field {pointer}"))
}

fn tuple(value: &Value, pointer: &str) -> Result<ResourceIdentity, String> {
    let values = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing identity tuple {pointer}"))?;
    if values.len() != 3 {
        return Err(format!("identity tuple {pointer} must have three fields"));
    }
    ResourceIdentity::new(
        AdapterId { value: values[0].as_str().ok_or_else(|| format!("{pointer}[0] must be a string"))?.to_owned() },
        ResourceKind { value: values[1].as_str().ok_or_else(|| format!("{pointer}[1] must be a string"))?.to_owned() },
        ResourceId { value: values[2].as_str().ok_or_else(|| format!("{pointer}[2] must be a string"))?.to_owned() },
    )
    .map_err(|error| error.to_string())
}

fn wire_identity(value: &Value, pointer: &str) -> Result<ResourceIdentity, String> {
    ResourceIdentity::new(
        AdapterId { value: string(value, &format!("{pointer}/adapter_id/value"))?.to_owned() },
        ResourceKind { value: string(value, &format!("{pointer}/resource_kind/value"))?.to_owned() },
        ResourceId { value: string(value, &format!("{pointer}/resource_id/value"))?.to_owned() },
    )
    .map_err(|error| error.to_string())
}

fn domain(value: &Value, pointer: &str) -> Result<AuthorityDomainId, String> {
    Ok(AuthorityDomainId { value: string(value, pointer)?.to_owned() })
}

fn resource_registry(authority_domain_id: &AuthorityDomainId, identities: &[ResourceIdentity]) -> ResourceRegistry {
    let mut registry = ResourceRegistry::new();
    for (index, identity) in identities.iter().enumerate() {
        let state = ResourceStateEvent {
            authority_domain_id: Some(authority_domain_id.clone()),
            source_adapter_id: Some(identity.adapter_id().clone()),
            source_adapter_generation: Some(Generation { value: 1 }),
            views: vec![ResourceViewStateUpdate {
                resource_kind: Some(identity.resource_kind().clone()),
                completeness: AdapterSnapshotSupport::Partial as i32,
            }],
            mutations: vec![ResourceStateMutation {
                identity: Some(identity.to_scope().resource.expect("resource scope")),
                from_revision_lsn: None,
                mutation: Some(resource_state_mutation::Mutation::Upsert(ResourceStateUpsert {
                    resource_payload: Some(envelope("resource.schema", vec![1])),
                    projection_payload: Some(envelope("projection.schema", vec![2])),
                })),
            }],
            observed_at: Some(Timestamp { seconds: 1, nanos: 0 }),
        };
        registry
            .observe(&RecordedEvent {
                event_id: event_id(authority_domain_id.clone(), index as u64 + 1),
                payload: events::encode(&state),
            })
            .expect("valid resource fixture");
    }
    registry
}

fn envelope(schema_ref: &str, payload: Vec<u8>) -> PayloadEnvelope {
    PayloadEnvelope {
        payload,
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: schema_ref.to_owned(),
    }
}

#[derive(Clone)]
struct Issuer {
    actor: ActorId,
    endpoint: EndpointId,
    device: DeviceId,
    domain: AuthorityDomainId,
}

impl IssuerContext for Issuer {
    fn verified_actor(&self) -> Option<&ActorId> { Some(&self.actor) }
    fn verified_endpoint(&self) -> Option<&EndpointId> { Some(&self.endpoint) }
    fn verified_device(&self) -> Option<&DeviceId> { Some(&self.device) }
    fn endpoint_generation(&self) -> Option<Generation> { Some(Generation { value: 1 }) }
    fn authority_domain_id(&self) -> &AuthorityDomainId { &self.domain }
}

struct NoContracts;
impl ElicitationContractLookup for NoContracts {
    async fn active_contract(
        &self,
        _elicitation_id: &patchbay_contracts::patchbay::ElicitationId,
    ) -> Option<ActiveElicitation> {
        None
    }
}

impl CommandStateLookup for NoContracts {
    async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> { None }
}

fn issuer(vector: &ConformanceVector, authority_domain_id: AuthorityDomainId) -> Result<Issuer, String> {
    Ok(Issuer {
        actor: ActorId { value: string(&vector.input, "/resource_case/verified_issuer/actor_id")?.to_owned() },
        endpoint: EndpointId { value: string(&vector.input, "/resource_case/verified_issuer/endpoint_id")?.to_owned() },
        device: DeviceId { value: "conformance-device".to_owned() },
        domain: authority_domain_id,
    })
}

fn operation(vector: &ConformanceVector, authority_domain_id: AuthorityDomainId, identity: &ResourceIdentity) -> Result<Operation, String> {
    if string(&vector.input, "/resource_case/operation/kind")? != "OPERATION_KIND_QUERY" {
        return Err("resource conformance operation must be QUERY".to_owned());
    }
    for pointer in [
        "/resource_case/operation/submitted_at",
        "/resource_case/operation/validity_window/starts_at",
        "/resource_case/operation/validity_window/expires_at",
    ] {
        let _ = string(&vector.input, pointer)?;
    }
    Ok(Operation {
        command_id: Some(CommandId { value: string(&vector.input, "/resource_case/operation/command_id/value")?.to_owned() }),
        authority_domain_id: Some(authority_domain_id),
        sender: Some(ActorEndpointRef { actor_id: Some(ActorId { value: string(&vector.input, "/resource_case/operation/sender/actor_id/value").unwrap_or("payload-claim").to_owned() }), ..ActorEndpointRef::default() }),
        kind: OperationKind::Query as i32,
        target_scope: Some(identity.to_scope()),
        idempotency_key: string(&vector.input, "/resource_case/operation/idempotency_key")?.to_owned(),
        payload: Some(PayloadEnvelope::default()),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp { seconds: 99, nanos: 0 }),
            expires_at: Some(Timestamp { seconds: 101, nanos: 0 }),
        }),
        submitted_at: Some(Timestamp { seconds: 100, nanos: 0 }),
        ..Operation::default()
    })
}

fn grant(vector: &ConformanceVector, authority_domain_id: AuthorityDomainId, identity: &ResourceIdentity) -> Result<Grant, String> {
    if string(&vector.input, "/resource_case/matching_grant/operation_kind")? != "OPERATION_KIND_QUERY" {
        return Err("resource conformance grant must permit QUERY".to_owned());
    }
    let grant_identity = tuple(&vector.input, "/resource_case/matching_grant/target")?;
    if &grant_identity != identity {
        return Err("matching grant target differs from registered resource".to_owned());
    }
    Ok(Grant {
        grant_id: Some(GrantId { value: string(&vector.input, "/resource_case/matching_grant/grant_id")?.to_owned() }),
        authority_domain_id: Some(authority_domain_id),
        subject_actor_id: Some(ActorId { value: string(&vector.input, "/resource_case/verified_issuer/actor_id")?.to_owned() }),
        target_scope: Some(identity.to_scope()),
        allowed_operation_kinds: vec![OperationKind::Query as i32],
        provenance: Some(GrantProvenance { reason: "conformance vector".to_owned(), ..GrantProvenance::default() }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    })
}

async fn resource_operation(vector: &ConformanceVector, with_grant: bool) -> Result<(), String> {
    let authority_domain_id = domain(&vector.input, "/resource_case/operation/authority_domain_id/value")?;
    let identity = wire_identity(&vector.input, "/resource_case/operation/target_scope/resource")?;
    if tuple(&vector.input, "/resource_case/registered_resource")? != identity {
        return Err("registered resource differs from Operation target".to_owned());
    }
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut authority = AuthorityRegistry::new();
    if with_grant {
        ingest_grant(&storage, &mut authority, &authority_domain_id, grant(vector, authority_domain_id.clone(), &identity)?)
            .await
            .map_err(|error| error.to_string())?;
    } else if vector.input.pointer("/resource_case/available_grants").and_then(Value::as_array).is_none_or(|grants| !grants.is_empty()) {
        return Err("missing-grant vector must provide an empty grant set".to_owned());
    }
    let targets = TargetRegistry::new(
        SessionRegistry::new(),
        resource_registry(&authority_domain_id, std::slice::from_ref(&identity)),
    );
    let result = submit_with_clock(
        &storage,
        &authority,
        &targets,
        &CommandIndex::new(),
        &NoContracts,
        &issuer(vector, authority_domain_id.clone())?,
        operation(vector, authority_domain_id.clone(), &identity)?,
        &TestClock::new(Timestamp { seconds: 100, nanos: 0 }),
    )
    .await
    .map_err(|error| error.to_string())?;
    let events = storage
        .read_after(&authority_domain_id, patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .map_err(|error| error.to_string())?;
    let operation_count = events.iter().filter(|event| event.payload.kind == StoredEventKind::Operation as i32).count();
    if with_grant {
        if string(&vector.expected_outcome, "/resource_case/submission_result/outcome")? != "SUBMISSION_OUTCOME_ACCEPTED"
            || string(&vector.expected_outcome, "/resource_case/submission_result/operation_state")? != "OPERATION_STATE_ACCEPTED"
            || result.outcome != SubmissionOutcome::Accepted as i32
            || result.operation_state != OperationState::Accepted as i32
            || operation_count != vector.expected_outcome.pointer("/resource_case/durable_record/append_count").and_then(Value::as_u64).unwrap_or(0) as usize
            || !boolean(&vector.expected_outcome, "/resource_case/durable_before_delivery")?
            || boolean(&vector.expected_outcome, "/resource_case/delivered_before_append")?
        {
            return Err("resource durable-acceptance outcome disagrees with product execution".to_owned());
        }
        if result.decision_grant_id.as_ref().map(|id| id.value.as_str()) != Some(string(&vector.expected_outcome, "/resource_case/submission_result/decision_grant_id")?) {
            return Err("acceptance selected the wrong grant".to_owned());
        }
    } else if string(&vector.expected_outcome, "/resource_case/submission_result/outcome")? != "SUBMISSION_OUTCOME_REJECTED"
        || string(&vector.expected_outcome, "/resource_case/submission_result/failure_code")? != "FAILURE_CODE_AUTHORIZATION_DENIED"
        || result.outcome != SubmissionOutcome::Rejected as i32
        || operation_count != 0
        || boolean(&vector.expected_outcome, "/resource_case/durable_acceptance_record_created")?
        || boolean(&vector.expected_outcome, "/resource_case/delivered_to_adapter")?
    {
        return Err("resource missing-grant outcome disagrees with product execution".to_owned());
    }
    Ok(())
}

fn collision(vector: &ConformanceVector) -> Result<(), String> {
    let grant = tuple(&vector.input, "/grant_identity")?;
    let cases = [
        ("exact", true, "/exact_tuple_authorized"),
        ("changed_adapter", false, "/changed_adapter_authorized"),
        ("changed_kind", false, "/changed_kind_authorized"),
        ("changed_resource_id", false, "/changed_resource_id_authorized"),
    ];
    let mut keys = std::collections::HashSet::new();
    for (name, oracle, expected_pointer) in cases {
        let requested = tuple(&vector.input, &format!("/requests/{name}"))?;
        let actual = target_scope_matches(&grant.to_scope(), &requested.to_scope());
        if actual != oracle || actual != boolean(&vector.expected_outcome, expected_pointer)? {
            return Err(format!("identity collision case {name} disagrees with exact-tuple oracle"));
        }
        let operation = Operation { target_scope: Some(requested.to_scope()), ..Operation::default() };
        keys.insert(target_key_for(&operation).map_err(|error| error.to_string())?);
    }
    if boolean(&vector.expected_outcome, "/target_keys_all_distinct")? != (keys.len() == 4) {
        return Err("resource target keys do not preserve all tuple dimensions".to_owned());
    }
    Ok(())
}

async fn injection(vector: &ConformanceVector) -> Result<(), String> {
    let authority_domain_id = AuthorityDomainId { value: "auth-main".to_owned() };
    let claimed = tuple(&vector.input, "/opaque_payload/encoded_claim/resource_identity")?;
    let forged_domain = AuthorityDomainId { value: string(&vector.input, "/opaque_payload/encoded_claim/authority_domain_id")?.to_owned() };
    let forged_state = ResourceStateEvent {
        authority_domain_id: Some(forged_domain),
        source_adapter_id: Some(AdapterId { value: string(&vector.input, "/opaque_payload/encoded_claim/source_adapter_id")?.to_owned() }),
        source_adapter_generation: Some(Generation { value: vector.input.pointer("/opaque_payload/encoded_claim/source_adapter_generation").and_then(Value::as_u64).ok_or("missing forged generation")? }),
        views: vec![ResourceViewStateUpdate { resource_kind: Some(claimed.resource_kind().clone()), completeness: AdapterSnapshotSupport::Authoritative as i32 }],
        mutations: vec![ResourceStateMutation {
            identity: Some(claimed.to_scope().resource.expect("resource identity")),
            from_revision_lsn: None,
            mutation: Some(resource_state_mutation::Mutation::Upsert(ResourceStateUpsert {
                resource_payload: Some(envelope("resource.schema", vec![1])),
                projection_payload: Some(envelope("projection.schema", vec![2])),
            })),
        }],
        observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
    };
    let target = tuple(&vector.input, "/observation_target")?;
    let observation = Observation {
        authority_domain_id: Some(authority_domain_id.clone()),
        kind: ObservationKind::Event as i32,
        target_scope: Some(target.to_scope()),
        payload: Some(envelope(string(&vector.input, "/opaque_payload/schema_ref")?, forged_state.encode_to_vec())),
        ..Observation::default()
    };
    if string(&vector.input, "/opaque_payload/content_type")? != "PAYLOAD_CONTENT_TYPE_PROTOBUF" {
        return Err("injection witness must carry protobuf bytes".to_owned());
    }
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    ingest_observation(&storage, &NoContracts, observation)
        .await
        .map_err(|error| error.to_string())?;
    let events = storage.read_after(&authority_domain_id, patchbay_contracts::patchbay::Lsn { value: 0 }).await.map_err(|error| error.to_string())?;
    let registry = rebuild_from_log(&storage, &authority_domain_id).await.map_err(|error| error.to_string())?;
    if events.len() != 1
        || events[0].payload.kind != StoredEventKind::Observation as i32
        || string(&vector.expected_outcome, "/stored_event_kind")? != "STORED_EVENT_KIND_OBSERVATION"
        || registry.contains(&claimed)
        || boolean(&vector.expected_outcome, "/resource_registry_changed")?
        || boolean(&vector.expected_outcome, "/resource_resolved")?
        || boolean(&vector.expected_outcome, "/adapter_assigned_lsn_accepted")?
        || vector.input.pointer("/opaque_payload/encoded_claim/adapter_assigned_lsn").and_then(Value::as_u64).is_none()
    {
        return Err("opaque Observation payload crossed into core resource state".to_owned());
    }
    Ok(())
}

async fn execute_case(vector: &ConformanceVector, case: &str) -> Result<(), String> {
    if vector.property_id.is_empty() || !matches!(vector.promotion_status.as_str(), "draft" | "promoted") {
        return Err("conformance vector has invalid property or promotion metadata".to_owned());
    }
    match case {
        "resource_operation_durable_acceptance" => resource_operation(vector, true).await,
        "resource_operation_missing_grant" => resource_operation(vector, false).await,
        "resource_identity_collision_fenced" => collision(vector),
        "opaque_observation_cannot_fold_resource_state" => injection(vector).await,
        _ => Err(format!("unhandled {RUNNER} conformance case {}:{case}", vector.vector_id)),
    }
}

#[tokio::test]
async fn conformance_vector_runner() {
    let vectors = vectors();
    for request in requests() {
        let vector = vectors.get(&request.vector_id).unwrap_or_else(|| panic!("unknown vector id {}", request.vector_id));
        assert!(
            vector.implementation_checks.iter().any(|check| check.runner == RUNNER && check.case == request.case),
            "unregistered requested check {}:{}", request.vector_id, request.case,
        );
        execute_case(vector, &request.case).await.unwrap_or_else(|error| panic!("{error}"));
        println!("PATCHBAY_CONFORMANCE_EXECUTED={}:{}", request.vector_id, request.case);
    }
}
