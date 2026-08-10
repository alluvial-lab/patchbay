use std::{collections::BTreeMap, env, fs, path::PathBuf};

use patchbay_contracts::patchbay::{
    resource_report_mutation, resource_state_mutation, AcceptedOperation, ActorEndpointRef,
    ActorId, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, CommandId, DeviceId, EndpointId,
    FailureCode, Generation, Grant, GrantId, GrantProvenance, GrantRevocationPolicy, Lsn,
    Observation,
    ObservationKind, Operation, OperationKind, OperationState, PayloadContentType, PayloadEnvelope,
    ResourceId, ResourceFreshnessState, ResourceKind, ResourceReportMutation, ResourceStateEvent,
    ResourceStateMutation, ResourceStateTombstone, ResourceStateUnknown, ResourceStateUpsert,
    ResourceViewReport,
    ResourceViewStateUpdate, RuntimeSessionId, SessionActivityState, SessionConnectivityState,
    SessionRegistered, SessionState, StoredEventKind, SubmissionOutcome, TargetScope,
    TargetScopeKind, TimeWindow,
};
use patchbay_core::{
    acceptance::{
        ingest_observation, submit_with_clock, target_key_for, ActiveElicitation, CommandIndex,
        CommandSnapshot, CommandStateLookup, ElicitationContractLookup,
    },
    authority::{ingest_grant, target_scope_matches, AuthorityRegistry, IssuerContext},
    resource::{
        events as resource_events, ingest_resource_report, rebuild_from_log, ResourceError,
        ResourceIdentity, ResourceRegistry, ResourceReportMode, ValidatedResourceReport,
    },
    session::{events as session_events, SessionRegistry},
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
                payload: resource_events::encode(&state),
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

fn case_pointer(case_name: &str, suffix: &str) -> String {
    format!("/{case_name}{suffix}")
}

fn issuer(
    vector: &ConformanceVector,
    case_name: &str,
    authority_domain_id: AuthorityDomainId,
) -> Result<Issuer, String> {
    let (actor_pointer, endpoint_pointer) = if case_name == "session_case" {
        (
            "/operation/sender/actor_id/value",
            "/operation/sender/endpoint_id/value",
        )
    } else {
        ("/verified_issuer/actor_id", "/verified_issuer/endpoint_id")
    };
    Ok(Issuer {
        actor: ActorId {
            value: string(&vector.input, &case_pointer(case_name, actor_pointer))?.to_owned(),
        },
        endpoint: EndpointId {
            value: string(&vector.input, &case_pointer(case_name, endpoint_pointer))?.to_owned(),
        },
        device: DeviceId {
            value: "conformance-device".to_owned(),
        },
        domain: authority_domain_id,
    })
}

fn operation_target(vector: &ConformanceVector, case_name: &str) -> Result<TargetScope, String> {
    let target_pointer = case_pointer(case_name, "/operation/target_scope");
    match string(&vector.input, &format!("{target_pointer}/kind"))? {
        "TARGET_SCOPE_KIND_RUNTIME_SESSION" => Ok(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(AdapterId {
                value: string(&vector.input, &format!("{target_pointer}/adapter_id/value"))?
                    .to_owned(),
            }),
            deployment_scope: string(&vector.input, &format!("{target_pointer}/deployment_scope"))?
                .to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: string(
                    &vector.input,
                    &format!("{target_pointer}/runtime_session_id/value"),
                )?
                .to_owned(),
            }),
            session_generation: Some(Generation {
                value: vector
                    .input
                    .pointer(&format!("{target_pointer}/session_generation/value"))
                    .and_then(Value::as_u64)
                    .ok_or("missing session generation")?,
            }),
            ..TargetScope::default()
        }),
        "TARGET_SCOPE_KIND_RESOURCE" => {
            Ok(wire_identity(&vector.input, &format!("{target_pointer}/resource"))?.to_scope())
        }
        kind => Err(format!("unsupported conformance target kind {kind}")),
    }
}

fn operation(
    vector: &ConformanceVector,
    case_name: &str,
    authority_domain_id: AuthorityDomainId,
) -> Result<Operation, String> {
    let operation_pointer = case_pointer(case_name, "/operation");
    let kind = match string(&vector.input, &format!("{operation_pointer}/kind"))? {
        "OPERATION_KIND_INSTRUCT" => OperationKind::Instruct,
        "OPERATION_KIND_QUERY" => OperationKind::Query,
        kind => return Err(format!("unsupported conformance OperationKind {kind}")),
    };
    for suffix in [
        "/submitted_at",
        "/validity_window/starts_at",
        "/validity_window/expires_at",
    ] {
        let _ = string(&vector.input, &format!("{operation_pointer}{suffix}"))?;
    }
    let sender_actor = vector
        .input
        .pointer(&format!("{operation_pointer}/sender/actor_id/value"))
        .and_then(Value::as_str)
        .unwrap_or("payload-claim");
    Ok(Operation {
        command_id: Some(CommandId {
            value: string(
                &vector.input,
                &format!("{operation_pointer}/command_id/value"),
            )?
            .to_owned(),
        }),
        authority_domain_id: Some(authority_domain_id),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: sender_actor.to_owned(),
            }),
            ..ActorEndpointRef::default()
        }),
        kind: kind as i32,
        target_scope: Some(operation_target(vector, case_name)?),
        idempotency_key: string(
            &vector.input,
            &format!("{operation_pointer}/idempotency_key"),
        )?
        .to_owned(),
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

fn matching_grant(
    vector: &ConformanceVector,
    case_name: &str,
    authority_domain_id: AuthorityDomainId,
    target_scope: &TargetScope,
) -> Result<Grant, String> {
    let (grant_pointer, actor_pointer, endpoint_pointer, kind) = if case_name == "session_case" {
        (
            "/session_case/preconditions/matching_grant",
            "/session_case/operation/sender/actor_id/value",
            Some("/session_case/operation/sender/endpoint_id/value"),
            OperationKind::Instruct,
        )
    } else {
        (
            "/resource_case/matching_grant",
            "/resource_case/verified_issuer/actor_id",
            None,
            OperationKind::Query,
        )
    };
    let grant_id_pointer = if case_name == "session_case" {
        "/grant_id/value"
    } else {
        "/grant_id"
    };
    if case_name == "resource_case" {
        if string(
            &vector.input,
            "/resource_case/matching_grant/operation_kind",
        )? != "OPERATION_KIND_QUERY"
            || tuple(&vector.input, "/resource_case/matching_grant/target")?.to_scope()
                != *target_scope
        {
            return Err(
                "resource matching grant differs from the requested target/kind".to_owned(),
            );
        }
    } else {
        let grant_target = "/session_case/preconditions/matching_grant/target_scope";
        if string(
            &vector.input,
            "/session_case/preconditions/matching_grant/allowed_operation_kinds/0",
        )? != "OPERATION_KIND_INSTRUCT"
            || string(&vector.input, &format!("{grant_target}/kind"))?
                != "TARGET_SCOPE_KIND_RUNTIME_SESSION"
            || string(&vector.input, &format!("{grant_target}/adapter_id/value"))?
                != target_scope
                    .adapter_id
                    .as_ref()
                    .map_or("", |id| id.value.as_str())
            || string(&vector.input, &format!("{grant_target}/deployment_scope"))?
                != target_scope.deployment_scope
            || string(
                &vector.input,
                &format!("{grant_target}/runtime_session_id/value"),
            )? != target_scope
                .runtime_session_id
                .as_ref()
                .map_or("", |id| id.value.as_str())
            || vector
                .input
                .pointer(&format!("{grant_target}/session_generation/value"))
                .and_then(Value::as_u64)
                != target_scope
                    .session_generation
                    .map(|generation| generation.value)
        {
            return Err("session matching grant differs from the requested target/kind".to_owned());
        }
    }
    Ok(Grant {
        grant_id: Some(GrantId {
            value: string(&vector.input, &format!("{grant_pointer}{grant_id_pointer}"))?.to_owned(),
        }),
        authority_domain_id: Some(authority_domain_id),
        subject_actor_id: Some(ActorId {
            value: string(&vector.input, actor_pointer)?.to_owned(),
        }),
        subject_endpoint_id: endpoint_pointer.map(|pointer| EndpointId {
            value: string(&vector.input, pointer)
                .expect("session endpoint was validated")
                .to_owned(),
        }),
        target_scope: Some(target_scope.clone()),
        allowed_operation_kinds: vec![kind as i32],
        provenance: Some(GrantProvenance {
            reason: "conformance vector".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    })
}

fn session_registry(
    authority_domain_id: &AuthorityDomainId,
    target: &TargetScope,
) -> Result<SessionRegistry, String> {
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
        },
    );
    let mut sessions = SessionRegistry::new();
    sessions
        .observe(&RecordedEvent {
            event_id: event_id(authority_domain_id.clone(), 1),
            payload: session_events::encode(&registration),
        })
        .map_err(|error| error.to_string())?;
    Ok(sessions)
}

async fn operation_case(
    vector: &ConformanceVector,
    case_name: &str,
    with_grant: bool,
) -> Result<(), String> {
    let authority_domain_id = domain(
        &vector.input,
        &case_pointer(case_name, "/operation/authority_domain_id/value"),
    )?;
    let target_scope = operation_target(vector, case_name)?;
    let (sessions, resources) = if case_name == "session_case" {
        (
            session_registry(&authority_domain_id, &target_scope)?,
            ResourceRegistry::new(),
        )
    } else {
        let identity =
            ResourceIdentity::try_from_scope(&target_scope).map_err(|error| error.to_string())?;
        if tuple(&vector.input, "/resource_case/registered_resource")? != identity {
            return Err("registered resource differs from Operation target".to_owned());
        }
        (
            SessionRegistry::new(),
            resource_registry(&authority_domain_id, std::slice::from_ref(&identity)),
        )
    };
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut authority = AuthorityRegistry::new();
    if with_grant {
        ingest_grant(
            &storage,
            &mut authority,
            &authority_domain_id,
            matching_grant(
                vector,
                case_name,
                authority_domain_id.clone(),
                &target_scope,
            )?,
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
    let result = submit_with_clock(
        &storage,
        &authority,
        &TargetRegistry::new(sessions, resources),
        &CommandIndex::new(),
        &NoContracts,
        &issuer(vector, case_name, authority_domain_id.clone())?,
        operation(vector, case_name, authority_domain_id.clone())?,
        &TestClock::new(Timestamp {
            seconds: 100,
            nanos: 0,
        }),
    )
    .await
    .map_err(|error| error.to_string())?;
    let events = storage
        .read_after(
            &authority_domain_id,
            patchbay_contracts::patchbay::Lsn { value: 0 },
        )
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
            .ok_or_else(|| format!("{case_name} acceptance did not append an Operation"))?;
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
                "{case_name} durable-acceptance outcome disagrees with core execution"
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
                return Err(format!("{case_name} acceptance selected the wrong grant"));
            }
        }
        if case_name == "resource_case"
            && (!boolean(
                &vector.expected_outcome,
                "/resource_case/durable_before_delivery",
            )? || boolean(
                &vector.expected_outcome,
                "/resource_case/delivered_before_append",
            )?)
        {
            return Err("resource delivery ordering expectation is contradictory".to_owned());
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
            "{case_name} missing-grant outcome disagrees with core execution"
        ));
    }
    Ok(())
}

async fn operation_scenario(vector: &ConformanceVector, with_grant: bool) -> Result<(), String> {
    operation_case(vector, "session_case", with_grant).await?;
    operation_case(vector, "resource_case", with_grant).await
}

fn report_mutation(identity: &ResourceIdentity, cached: bool) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(identity.to_scope().resource.expect("resource identity")),
        mutation: Some(if cached {
            resource_report_mutation::Mutation::Upsert(ResourceStateUpsert {
                resource_payload: Some(envelope("resource.schema", vec![1])),
                projection_payload: Some(envelope("projection.schema", vec![2])),
            })
        } else {
            resource_report_mutation::Mutation::Unknown(ResourceStateUnknown {})
        }),
    }
}

fn validated_report(
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
    resource_kind: &ResourceKind,
    generation: u64,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    ValidatedResourceReport {
        authority_domain_id: authority_domain_id.clone(),
        adapter_id: adapter_id.clone(),
        adapter_generation: Generation { value: generation },
        mode,
        views: vec![ResourceViewReport {
            resource_kind: Some(resource_kind.clone()),
            completeness: tier as i32,
            mutations,
        }],
        observed_at: Timestamp { seconds: 100, nanos: 0 },
    }
}

async fn assert_resource_revisions_match_committed_lsns(
    storage: &RusqliteStorage,
    authority_domain_id: &AuthorityDomainId,
    registry: &ResourceRegistry,
) -> Result<(), String> {
    let mut expected_records = std::collections::HashMap::new();
    let mut expected_views = std::collections::HashMap::new();
    for event in storage
        .read_after(authority_domain_id, patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .map_err(|error| error.to_string())?
    {
        if event.payload.kind != StoredEventKind::ResourceState as i32 {
            continue;
        }
        let committed_lsn = event
            .event_id
            .lsn
            .as_ref()
            .ok_or("committed resource event missing LSN")?
            .value;
        let state = ResourceStateEvent::decode(event.payload.payload.as_slice())
            .map_err(|error| format!("committed resource event did not decode: {error}"))?;
        let adapter_id = state.source_adapter_id.ok_or("committed resource event missing adapter")?;
        for view in state.views {
            let resource_kind = view.resource_kind.ok_or("committed resource view missing kind")?;
            expected_views.insert((adapter_id.value.clone(), resource_kind.value), committed_lsn);
        }
        for mutation in state.mutations {
            let identity = ResourceIdentity::try_from_wire(
                mutation.identity.as_ref().ok_or("committed resource mutation missing identity")?,
            )
            .map_err(|error| error.to_string())?;
            expected_records.insert(identity, committed_lsn);
        }
    }

    if registry.resources().count() != expected_records.len()
        || registry.views().count() != expected_views.len()
        || registry.resources().any(|record| {
            expected_records.get(&record.identity).copied() != Some(record.revision_lsn)
        })
        || registry.views().any(|view| {
            expected_views
                .get(&(view.key.adapter_id.value.clone(), view.key.resource_kind.value.clone()))
                .copied()
                != Some(view.revision_lsn)
        })
    {
        return Err("production resource record/view revisions do not equal their committed event LSNs".to_owned());
    }
    Ok(())
}

async fn completeness(vector: &ConformanceVector) -> Result<(), String> {
    let authority_domain_id = AuthorityDomainId { value: string(&vector.input, "/authority_domain_id")?.to_owned() };
    let adapter_id = AdapterId { value: string(&vector.input, "/adapter_id")?.to_owned() };
    let resource_kind = ResourceKind { value: string(&vector.input, "/resource_kind")?.to_owned() };
    let generation = vector.input.pointer("/adapter_generation").and_then(Value::as_u64).ok_or("missing adapter generation")?;
    let cached = tuple(&vector.input, "/baseline/cached_identity")?;
    let unknown = tuple(&vector.input, "/baseline/unknown_identity")?;
    let cases = vector.input.pointer("/cases").and_then(Value::as_array).ok_or("missing completeness cases")?;

    for case in cases {
        let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
        let mut registry = ResourceRegistry::new();
        ingest_resource_report(
            &storage,
            &mut registry,
            validated_report(
                &authority_domain_id,
                &adapter_id,
                &resource_kind,
                generation,
                ResourceReportMode::Delta,
                AdapterSnapshotSupport::Partial,
                vec![report_mutation(&cached, true), report_mutation(&unknown, false)],
            ),
        )
        .await
        .map_err(|error| error.to_string())?;
        assert_resource_revisions_match_committed_lsns(&storage, &authority_domain_id, &registry).await?;
        let before_cached = registry.get(&cached).cloned().ok_or("cached baseline missing")?;
        let before_unknown = registry.get(&unknown).cloned().ok_or("unknown baseline missing")?;
        let mode_name = string(case, "/mode")?;
        let mode = match mode_name {
            "snapshot" => ResourceReportMode::Snapshot,
            "delta" => ResourceReportMode::Delta,
            _ => return Err(format!("unknown report mode {mode_name}")),
        };
        let tier_name = string(case, "/tier")?;
        let tier = match tier_name {
            "authoritative" => AdapterSnapshotSupport::Authoritative,
            "partial" => AdapterSnapshotSupport::Partial,
            "none" => AdapterSnapshotSupport::None,
            _ => return Err(format!("unknown snapshot tier {tier_name}")),
        };
        if case.pointer("/listed").and_then(Value::as_array).is_none_or(|listed| !listed.is_empty()) {
            return Err("deterministic omission cases must list no identities".to_owned());
        }
        ingest_resource_report(
            &storage,
            &mut registry,
            validated_report(
                &authority_domain_id,
                &adapter_id,
                &resource_kind,
                generation,
                mode,
                tier,
                Vec::new(),
            ),
        )
        .await
        .map_err(|error| error.to_string())?;
        assert_resource_revisions_match_committed_lsns(&storage, &authority_domain_id, &registry).await?;
        let cached_record = registry.get(&cached).ok_or("cached record disappeared")?;
        let unknown_record = registry.get(&unknown).ok_or("unknown record disappeared")?;
        match string(case, "/name")? {
            "authoritative-omission" => {
                if string(&vector.expected_outcome, "/authoritative_omission")? != "tombstoned"
                    || !cached_record.tombstoned() || !unknown_record.tombstoned()
                {
                    return Err("authoritative omission did not tombstone baseline identities".to_owned());
                }
            }
            "partial-omission" => {
                if string(&vector.expected_outcome, "/partial_cached_omission")? != "stale"
                    || cached_record.freshness != ResourceFreshnessState::Stale
                    || unknown_record.freshness != ResourceFreshnessState::Unknown
                    || cached_record.tombstoned()
                    || unknown_record.tombstoned()
                {
                    return Err("partial omission was dishonest".to_owned());
                }
            }
            "none-omission" => {
                if string(&vector.expected_outcome, "/none_cached_omission")? != "stale"
                    || cached_record.freshness != ResourceFreshnessState::Stale
                    || unknown_record.freshness != ResourceFreshnessState::Unknown
                    || cached_record.tombstoned()
                    || unknown_record.tombstoned()
                {
                    return Err("none-tier omission was dishonest".to_owned());
                }
            }
            "delta-omission" => {
                if string(&vector.expected_outcome, "/delta_omission")? != "unchanged"
                    || cached_record != &before_cached
                    || unknown_record != &before_unknown
                {
                    return Err("delta omission mutated resource records".to_owned());
                }
            }
            name => return Err(format!("unknown completeness case {name}")),
        }
        if string(&vector.expected_outcome, "/no_payload_omission")? != "unknown" && !unknown_record.tombstoned() {
            return Err("no-payload omission expectation is not unknown".to_owned());
        }
        let replayed = rebuild_from_log(&storage, &authority_domain_id).await.map_err(|error| error.to_string())?;
        if boolean(&vector.expected_outcome, "/hot_equals_replay")? && replayed != registry {
            return Err("hot resource registry diverges from durable replay".to_owned());
        }
        let events = storage.read_after(&authority_domain_id, patchbay_contracts::patchbay::Lsn { value: 0 }).await.map_err(|error| error.to_string())?;
        if events.iter().filter(|event| event.payload.kind == StoredEventKind::ResourceState as i32).count() != 2
            || vector.expected_outcome.pointer("/accepted_report_append_count").and_then(Value::as_u64) != Some(1)
            || !boolean(&vector.expected_outcome, "/record_and_view_revisions_equal_committed_lsn")?
        {
            return Err("accepted report did not produce exactly one durable resource event".to_owned());
        }
    }
    Ok(())
}

async fn resource_replay_prefix_idempotent(
    vector: &ConformanceVector,
) -> Result<(), String> {
    let authority_domain_id = AuthorityDomainId {
        value: string(&vector.input, "/authority_domain_id")?.to_owned(),
    };
    let adapter_id = AdapterId {
        value: string(&vector.input, "/adapter_id")?.to_owned(),
    };
    let resource_kind = ResourceKind {
        value: string(&vector.input, "/resource_kind")?.to_owned(),
    };
    let old = tuple(&vector.input, "/initial/identity")?;
    let replacement = tuple(&vector.input, "/replacement/replacement_identity")?;
    if old != tuple(&vector.input, "/replacement/retired_identity")?
        || old.adapter_id() != &adapter_id
        || old.resource_kind() != &resource_kind
        || replacement.adapter_id() != &adapter_id
        || replacement.resource_kind() != &resource_kind
        || vector.input.pointer("/initial/lsn").and_then(Value::as_u64) != Some(1)
        || vector
            .input
            .pointer("/initial/adapter_generation")
            .and_then(Value::as_u64)
            != Some(1)
        || vector
            .input
            .pointer("/replacement/lsn")
            .and_then(Value::as_u64)
            != Some(2)
        || vector
            .input
            .pointer("/replacement/adapter_generation")
            .and_then(Value::as_u64)
            != Some(2)
    {
        return Err("resource replay-prefix vector has inconsistent fixture identity/order".into());
    }

    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut registry = ResourceRegistry::new();
    let initial = ingest_resource_report(
        &storage,
        &mut registry,
        validated_report(
            &authority_domain_id,
            &adapter_id,
            &resource_kind,
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![report_mutation(&old, true)],
        ),
    )
    .await
    .map_err(|error| error.to_string())?;
    let replacement_result = ingest_resource_report(
        &storage,
        &mut registry,
        validated_report(
            &authority_domain_id,
            &adapter_id,
            &resource_kind,
            2,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![
                ResourceReportMutation {
                    identity: Some(old.to_scope().resource.expect("resource identity")),
                    mutation: Some(resource_report_mutation::Mutation::Tombstone(
                        ResourceStateTombstone {
                            replaced_by: Some(
                                replacement
                                    .to_scope()
                                    .resource
                                    .expect("replacement identity"),
                            ),
                        },
                    )),
                },
                report_mutation(&replacement, true),
            ],
        ),
    )
    .await
    .map_err(|error| error.to_string())?;
    if initial.event_id.lsn.as_ref().map(|lsn| lsn.value) != Some(1)
        || replacement_result
            .event_id
            .lsn
            .as_ref()
            .map(|lsn| lsn.value)
            != Some(2)
        || replacement_result.touched_resources != 2
        || registry.contains(&old)
        || !registry.contains(&replacement)
        || !registry.get(&old).is_some_and(|record| {
            record.tombstoned()
                && record.revision_lsn == 2
                && record.replaced_by.as_ref() == Some(&replacement)
        })
        || registry.get(&replacement).map(|record| record.revision_lsn) != Some(2)
    {
        return Err("resource replacement did not commit atomically at LSN 2".into());
    }

    let events = storage
        .read_after(&authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(|error| error.to_string())?;
    let after_replacement = registry.clone();
    registry
        .observe(&events[0])
        .map_err(|error| format!("covered lower-generation event rejected: {error}"))?;
    if vector
        .input
        .pointer("/covered_refeed/lsn")
        .and_then(Value::as_u64)
        != Some(1)
        || vector
            .input
            .pointer("/covered_refeed/adapter_generation")
            .and_then(Value::as_u64)
            != Some(1)
        || registry != after_replacement
        || string(&vector.expected_outcome, "/covered_refeed_result")? != "success_no_change"
        || vector
            .expected_outcome
            .pointer("/covered_refeed_applied_through_lsn")
            .and_then(Value::as_u64)
            != Some(2)
    {
        return Err("covered lower-generation resource event was not inert".into());
    }

    let mut lower_generation_next = events[0].clone();
    lower_generation_next.event_id = event_id(authority_domain_id.clone(), 3);
    if vector
        .input
        .pointer("/lower_generation_next_candidate/lsn")
        .and_then(Value::as_u64)
        != Some(3)
        || vector
            .input
            .pointer("/lower_generation_next_candidate/adapter_generation")
            .and_then(Value::as_u64)
            != Some(1)
        || !matches!(
            registry.observe(&lower_generation_next),
            Err(ResourceError::CorruptLog(_))
        )
        || registry != after_replacement
        || string(&vector.expected_outcome, "/lower_generation_next_result")? != "corrupt_log"
        || vector
            .expected_outcome
            .pointer("/rejected_candidate_applied_through_lsn")
            .and_then(Value::as_u64)
            != Some(2)
    {
        return Err("lower-generation next event was not atomic corruption".into());
    }

    let old_before_sibling = registry.get(&old).cloned();
    let replacement_before_sibling = registry.get(&replacement).cloned();
    let views_before_sibling = registry.views().cloned().collect::<Vec<_>>();
    let sibling_payload = patchbay_contracts::patchbay::StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: Vec::new(),
    };
    let sibling_event_id = storage
        .append(&authority_domain_id, sibling_payload.clone())
        .await
        .map_err(|error| error.to_string())?;
    let sibling = RecordedEvent {
        event_id: sibling_event_id.clone(),
        payload: sibling_payload,
    };
    registry
        .observe(&sibling)
        .map_err(|error| format!("valid sibling prefix probe rejected: {error}"))?;
    if sibling_event_id.lsn.as_ref().map(|lsn| lsn.value) != Some(3)
        || vector
            .input
            .pointer("/sibling_prefix_probe/lsn")
            .and_then(Value::as_u64)
            != Some(3)
        || string(
            &vector.input,
            "/sibling_prefix_probe/stored_event_kind",
        )? != "STORED_EVENT_KIND_OBSERVATION"
        || registry == after_replacement
        || registry.get(&old).cloned() != old_before_sibling
        || registry.get(&replacement).cloned() != replacement_before_sibling
        || registry.views().cloned().collect::<Vec<_>>() != views_before_sibling
        || !boolean(
            &vector.expected_outcome,
            "/sibling_probe_advanced_prefix",
        )?
        || !boolean(
            &vector.expected_outcome,
            "/sibling_probe_resource_state_unchanged",
        )?
        || vector
            .expected_outcome
            .pointer("/final_applied_through_lsn")
            .and_then(Value::as_u64)
            != Some(3)
    {
        return Err("sibling prefix probe did not advance only the applied cursor".into());
    }
    let after_sibling = registry.clone();
    registry
        .observe(&sibling)
        .map_err(|error| format!("covered sibling prefix probe rejected: {error}"))?;
    if registry != after_sibling {
        return Err("covered sibling prefix probe was not idempotent".into());
    }

    let mutation_names = vector
        .input
        .pointer("/retired_mutation_candidates")
        .and_then(Value::as_array)
        .ok_or("missing retired mutation candidates")?;
    let expected_names = vector
        .expected_outcome
        .pointer("/retired_mutations_rejected")
        .and_then(Value::as_array)
        .ok_or("missing retired mutation expectations")?;
    if mutation_names != expected_names {
        return Err("retired mutation inputs and expectations differ".into());
    }
    for mutation in mutation_names {
        let mutation = match mutation.as_str().ok_or("retired mutation must be a string")? {
            "upsert" => report_mutation(&old, true),
            "unknown" => report_mutation(&old, false),
            "tombstone" => ResourceReportMutation {
                identity: Some(old.to_scope().resource.expect("resource identity")),
                mutation: Some(resource_report_mutation::Mutation::Tombstone(
                    ResourceStateTombstone { replaced_by: None },
                )),
            },
            name => return Err(format!("unknown retired mutation candidate {name}")),
        };
        let before_events = storage
            .read_after(&authority_domain_id, Lsn { value: 0 })
            .await
            .map_err(|error| error.to_string())?;
        if ingest_resource_report(
            &storage,
            &mut registry,
            validated_report(
                &authority_domain_id,
                &adapter_id,
                &resource_kind,
                2,
                ResourceReportMode::Delta,
                AdapterSnapshotSupport::Partial,
                vec![mutation],
            ),
        )
        .await
        .is_ok()
            || registry != after_sibling
            || storage
                .read_after(&authority_domain_id, Lsn { value: 0 })
                .await
                .map_err(|error| error.to_string())?
                != before_events
        {
            return Err("retired resource mutation changed projection or durable prefix".into());
        }
    }

    let final_events = storage
        .read_after(&authority_domain_id, Lsn { value: 0 })
        .await
        .map_err(|error| error.to_string())?;
    let replay_a = rebuild_from_log(&storage, &authority_domain_id)
        .await
        .map_err(|error| error.to_string())?;
    let replay_b = rebuild_from_log(&storage, &authority_domain_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut covered_replay = replay_a.clone();
    for event in &final_events {
        covered_replay
            .observe(event)
            .map_err(|error| error.to_string())?;
    }
    if final_events.len() != 3
        || registry != replay_a
        || replay_a != replay_b
        || covered_replay != replay_a
        || !boolean(&vector.expected_outcome, "/initial_applied")?
        || !boolean(
            &vector.expected_outcome,
            "/replacement_applied_atomically",
        )?
        || !boolean(&vector.expected_outcome, "/retired_identity_tombstoned")?
        || !boolean(&vector.expected_outcome, "/replacement_identity_active")?
        || !boolean(
            &vector.expected_outcome,
            "/projection_unchanged_after_each_rejection",
        )?
        || !boolean(&vector.expected_outcome, "/hot_equals_fresh_replay")?
        || !boolean(&vector.expected_outcome, "/fresh_replays_equal")?
        || !boolean(
            &vector.expected_outcome,
            "/covered_prefix_replay_is_idempotent",
        )?
        || vector
            .expected_outcome
            .pointer("/durable_event_count")
            .and_then(Value::as_u64)
            != Some(3)
    {
        return Err("resource replay-prefix convergence expectation failed".into());
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
        "operation_durable_acceptance" => operation_scenario(vector, true).await,
        "operation_missing_grant" => operation_scenario(vector, false).await,
        "resource_snapshot_completeness_truth_table" => completeness(vector).await,
        "resource_replay_prefix_idempotent" => resource_replay_prefix_idempotent(vector).await,
        "resource_identity_collision_fenced" => collision(vector),
        "opaque_observation_cannot_fold_resource_state" => injection(vector).await,
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
        assert!(
            vector.implementation_checks.iter().any(|check| check.runner == RUNNER && check.case == request.case),
            "unregistered requested check {}:{}", request.vector_id, request.case,
        );
        execute_case(vector, &request.case).await.unwrap_or_else(|error| panic!("{error}"));
        println!("PATCHBAY_CONFORMANCE_EXECUTED={}:{}", request.vector_id, request.case);
    }
}
