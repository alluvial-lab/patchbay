use patchbay_contracts::patchbay::{
    spawn_request, ActorEndpointRef, AdapterId, AuthorityDomainId, CommandId,
    ContinuationAuthorityProvenance, DescendantGrantProvenance, ExternalRuntimeRef, FreshSpawn,
    Generation, GrantId, LogicalTargetId, Operation, OperationKind, PayloadContentType,
    PayloadEnvelope, RuntimeGenerationRef, RuntimeSessionId, SpawnContinuation, SpawnRequest,
    SpawnTargetSpec, TargetScope, TargetScopeKind, TimeWindow,
};
use patchbay_core::acceptance::{
    validate_operation_boundary, validate_spawn_authority_carriage,
    validate_spawn_operation_payload, validate_spawn_request, SpawnValidationError,
    SPAWN_REQUEST_SCHEMA,
};
use prost::Message;
use prost_types::Timestamp;

fn prior(generation: u64) -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-1".to_owned(),
        }),
        external_runtime: Some(ExternalRuntimeRef {
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            deployment_scope: "local".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-1".to_owned(),
            }),
            generation: Some(Generation { value: generation }),
        }),
    }
}

fn target_spec() -> SpawnTargetSpec {
    SpawnTargetSpec {
        shape: "session".to_owned(),
        adapter_payload: Some(PayloadEnvelope {
            payload: vec![1, 2, 3],
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: "pi.SpawnTarget".to_owned(),
        }),
        deployment_authority_ref: "workspace-key-1".to_owned(),
    }
}

fn fresh_request() -> SpawnRequest {
    SpawnRequest {
        intent: Some(spawn_request::Intent::Fresh(FreshSpawn {})),
        target_spec: Some(target_spec()),
    }
}

fn continuation_request(generation: u64) -> SpawnRequest {
    SpawnRequest {
        intent: Some(spawn_request::Intent::Continuation(SpawnContinuation {
            prior: Some(prior(generation)),
        })),
        target_spec: Some(target_spec()),
    }
}

fn grant(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn provenance(generation: u64) -> ContinuationAuthorityProvenance {
    ContinuationAuthorityProvenance {
        exact_prior: Some(prior(generation)),
        replacement_grant_id: Some(grant("replace-grant")),
        replacement_authority_kind: OperationKind::SessionManagement as i32,
    }
}

fn operation(request: SpawnRequest) -> Operation {
    Operation {
        command_id: Some(patchbay_contracts::patchbay::CommandId {
            value: "spawn-command".to_owned(),
        }),
        authority_domain_id: Some(AuthorityDomainId {
            value: "authority-main".to_owned(),
        }),
        sender: Some(ActorEndpointRef::default()),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            ..TargetScope::default()
        }),
        idempotency_key: "spawn-key".to_owned(),
        payload: Some(PayloadEnvelope {
            payload: request.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: SPAWN_REQUEST_SCHEMA.to_owned(),
        }),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp {
                seconds: 10,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 30,
                nanos: 0,
            }),
        }),
        submitted_at: Some(Timestamp {
            seconds: 20,
            nanos: 0,
        }),
        ..Operation::default()
    }
}

#[test]
fn generated_spawn_intents_round_trip_as_disjoint_variants() {
    let fresh = SpawnRequest::decode(fresh_request().encode_to_vec().as_slice()).unwrap();
    let continuation =
        SpawnRequest::decode(continuation_request(7).encode_to_vec().as_slice()).unwrap();

    assert!(matches!(
        fresh.intent,
        Some(spawn_request::Intent::Fresh(_))
    ));
    assert!(matches!(
        continuation.intent,
        Some(spawn_request::Intent::Continuation(_))
    ));
}

#[test]
fn descendant_provenance_round_trip_preserves_both_grant_links() {
    let wire = DescendantGrantProvenance {
        spawn_operation_id: Some(CommandId {
            value: "spawn-command".to_owned(),
        }),
        spawning_grant_id: Some(grant("spawn-grant")),
        continuation_authority: Some(provenance(7)),
    };

    let decoded = DescendantGrantProvenance::decode(wire.encode_to_vec().as_slice()).unwrap();
    assert_eq!(
        decoded.spawning_grant_id.as_ref().unwrap().value,
        "spawn-grant"
    );
    let continuation = decoded.continuation_authority.as_ref().unwrap();
    assert_eq!(
        continuation.replacement_grant_id.as_ref().unwrap().value,
        "replace-grant"
    );
    assert_eq!(continuation.exact_prior, Some(prior(7)));
}

#[test]
fn spawn_payload_contract_rejects_envelope_mutations() {
    let valid = operation(fresh_request());
    assert!(validate_spawn_operation_payload(&valid).is_ok());

    let mut missing = valid.clone();
    missing.payload = None;
    assert_eq!(
        validate_spawn_operation_payload(&missing).unwrap_err(),
        SpawnValidationError::MissingPayload
    );

    let mut wrong_content_type = valid.clone();
    wrong_content_type.payload.as_mut().unwrap().content_type = PayloadContentType::Json as i32;
    assert_eq!(
        validate_spawn_operation_payload(&wrong_content_type).unwrap_err(),
        SpawnValidationError::WrongPayloadContract
    );

    let mut wrong_schema = valid.clone();
    wrong_schema.payload.as_mut().unwrap().schema_ref = "patchbay.Other".to_owned();
    assert_eq!(
        validate_spawn_operation_payload(&wrong_schema).unwrap_err(),
        SpawnValidationError::WrongPayloadContract
    );

    let mut mixed = valid.clone();
    mixed
        .payload
        .as_mut()
        .unwrap()
        .payload
        .extend(continuation_request(7).encode_to_vec());
    assert_eq!(
        validate_spawn_operation_payload(&mixed).unwrap_err(),
        SpawnValidationError::MixedIntent
    );

    let mut malformed = valid;
    malformed.payload.as_mut().unwrap().payload = vec![0xff];
    assert!(matches!(
        validate_spawn_operation_payload(&malformed),
        Err(SpawnValidationError::MalformedPayload(_))
    ));
}

#[test]
fn request_shape_mutations_reject_independently() {
    let mut no_intent = fresh_request();
    no_intent.intent = None;
    assert_eq!(
        validate_spawn_request(&no_intent),
        Err(SpawnValidationError::MissingIntent)
    );

    let mut no_target = fresh_request();
    no_target.target_spec = None;
    assert_eq!(
        validate_spawn_request(&no_target),
        Err(SpawnValidationError::MissingTargetSpec)
    );

    for shape in [String::new(), "x".repeat(129), "not printable".to_owned()] {
        let mut request = fresh_request();
        request.target_spec.as_mut().unwrap().shape = shape;
        assert_eq!(
            validate_spawn_request(&request),
            Err(SpawnValidationError::MalformedTargetShape)
        );
    }

    let mut bad_authority_ref = fresh_request();
    bad_authority_ref
        .target_spec
        .as_mut()
        .unwrap()
        .deployment_authority_ref = "bad ref".to_owned();
    assert_eq!(
        validate_spawn_request(&bad_authority_ref),
        Err(SpawnValidationError::MalformedDeploymentAuthorityRef)
    );

    let mut unknown_content = fresh_request();
    unknown_content
        .target_spec
        .as_mut()
        .unwrap()
        .adapter_payload
        .as_mut()
        .unwrap()
        .content_type = 999;
    assert_eq!(
        validate_spawn_request(&unknown_content),
        Err(SpawnValidationError::InvalidAdapterPayloadContentType)
    );

    let mut unspecified_content = fresh_request();
    unspecified_content
        .target_spec
        .as_mut()
        .unwrap()
        .adapter_payload
        .as_mut()
        .unwrap()
        .content_type = PayloadContentType::Unspecified as i32;
    assert_eq!(
        validate_spawn_request(&unspecified_content),
        Err(SpawnValidationError::InvalidAdapterPayloadContentType)
    );

    let mut empty_schema = fresh_request();
    empty_schema
        .target_spec
        .as_mut()
        .unwrap()
        .adapter_payload
        .as_mut()
        .unwrap()
        .schema_ref
        .clear();
    assert_eq!(
        validate_spawn_request(&empty_schema),
        Err(SpawnValidationError::InvalidAdapterPayloadSchema)
    );

    let mut oversized_payload = fresh_request();
    oversized_payload
        .target_spec
        .as_mut()
        .unwrap()
        .adapter_payload
        .as_mut()
        .unwrap()
        .payload = vec![0; 1024 * 1024 + 1];
    assert_eq!(
        validate_spawn_request(&oversized_payload),
        Err(SpawnValidationError::AdapterPayloadTooLarge)
    );
}

#[test]
fn exact_prior_mutations_reject_before_acceptance() {
    let mut missing_prior = continuation_request(7);
    let Some(spawn_request::Intent::Continuation(continuation)) = missing_prior.intent.as_mut()
    else {
        unreachable!()
    };
    continuation.prior = None;
    assert_eq!(
        validate_spawn_request(&missing_prior),
        Err(SpawnValidationError::MissingExactPrior)
    );

    let mut mutations = Vec::new();

    let mut missing_logical = prior(7);
    missing_logical.logical_target_id = None;
    mutations.push((missing_logical, SpawnValidationError::EmptyLogicalTargetId));

    let mut missing_external = prior(7);
    missing_external.external_runtime = None;
    mutations.push((
        missing_external,
        SpawnValidationError::MissingExternalRuntime,
    ));

    let mut missing_adapter = prior(7);
    missing_adapter
        .external_runtime
        .as_mut()
        .unwrap()
        .adapter_id = None;
    mutations.push((missing_adapter, SpawnValidationError::EmptyAdapterId));

    let mut empty_scope = prior(7);
    empty_scope
        .external_runtime
        .as_mut()
        .unwrap()
        .deployment_scope
        .clear();
    mutations.push((empty_scope, SpawnValidationError::MalformedDeploymentScope));

    let mut missing_runtime = prior(7);
    missing_runtime
        .external_runtime
        .as_mut()
        .unwrap()
        .runtime_session_id = None;
    mutations.push((missing_runtime, SpawnValidationError::EmptyRuntimeSessionId));

    let mut missing_generation = prior(7);
    missing_generation
        .external_runtime
        .as_mut()
        .unwrap()
        .generation = None;
    mutations.push((
        missing_generation,
        SpawnValidationError::NonPositiveGeneration,
    ));
    mutations.push((prior(0), SpawnValidationError::NonPositiveGeneration));
    mutations.push((prior(u64::MAX), SpawnValidationError::GenerationOverflow));

    for (mutated_prior, expected) in mutations {
        let mut request = continuation_request(7);
        let Some(spawn_request::Intent::Continuation(continuation)) = request.intent.as_mut()
        else {
            unreachable!()
        };
        continuation.prior = Some(mutated_prior);
        assert_eq!(validate_spawn_request(&request), Err(expected));
    }
}

#[test]
fn compound_authority_requires_both_distinct_grants_and_exact_prior() {
    let request = continuation_request(7);
    let spawn_grant = grant("spawn-grant");
    let valid = provenance(7);
    assert!(validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&valid)).is_ok());

    // A broad adapter-scoped spawn Grant by itself is never continuation authority.
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), None),
        Err(SpawnValidationError::MissingReplacementAuthority)
    );
    assert_eq!(
        validate_spawn_authority_carriage(&request, None, Some(&valid)),
        Err(SpawnValidationError::MissingSpawningGrant)
    );

    let mut missing_prior = valid.clone();
    missing_prior.exact_prior = None;
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&missing_prior)),
        Err(SpawnValidationError::MissingExactPrior)
    );

    let mut wrong_prior = valid.clone();
    wrong_prior.exact_prior = Some(prior(6));
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&wrong_prior)),
        Err(SpawnValidationError::ReplacementPriorMismatch)
    );

    let mut invalid_prior = valid.clone();
    invalid_prior.exact_prior = Some(prior(0));
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&invalid_prior)),
        Err(SpawnValidationError::NonPositiveGeneration)
    );

    let mut missing_replacement_grant = valid.clone();
    missing_replacement_grant.replacement_grant_id = None;
    assert_eq!(
        validate_spawn_authority_carriage(
            &request,
            Some(&spawn_grant),
            Some(&missing_replacement_grant)
        ),
        Err(SpawnValidationError::MissingReplacementGrant)
    );

    let mut reused_grant = valid.clone();
    reused_grant.replacement_grant_id = Some(spawn_grant.clone());
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&reused_grant)),
        Err(SpawnValidationError::ReusedSpawningGrant)
    );

    for kind in [OperationKind::Unspecified, OperationKind::Spawn] {
        let mut wrong_kind = valid.clone();
        wrong_kind.replacement_authority_kind = kind as i32;
        assert_eq!(
            validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&wrong_kind)),
            Err(SpawnValidationError::WrongReplacementAuthorityKind)
        );
    }
}

#[test]
fn fresh_authority_carries_only_the_spawning_grant() {
    let request = fresh_request();
    let spawn_grant = grant("spawn-grant");
    assert!(validate_spawn_authority_carriage(&request, Some(&spawn_grant), None).is_ok());
    assert_eq!(
        validate_spawn_authority_carriage(&request, Some(&spawn_grant), Some(&provenance(7))),
        Err(SpawnValidationError::UnexpectedContinuationAuthority)
    );
}

#[test]
fn acceptance_boundary_validates_spawn_structure_before_stateful_work() {
    let now = Timestamp {
        seconds: 20,
        nanos: 0,
    };
    assert!(validate_operation_boundary(&operation(fresh_request()), &now).is_ok());
    assert!(validate_operation_boundary(&operation(continuation_request(7)), &now).is_ok());

    let mut zero_generation = operation(continuation_request(0));
    let rejected = validate_operation_boundary(&zero_generation, &now).unwrap_err();
    assert_eq!(
        patchbay_contracts::patchbay::FailureCode::try_from(rejected.failure_code).unwrap(),
        patchbay_contracts::patchbay::FailureCode::ValidationFailed
    );

    zero_generation.payload.as_mut().unwrap().payload = fresh_request().encode_to_vec();
    assert!(validate_operation_boundary(&zero_generation, &now).is_ok());
}
