use patchbay_contracts::patchbay::{
    no_external_effect_proof, observation_request, spawn_request, AcceptedOperation,
    ActorEndpointRef, ActorId, AdapterCapability, AdapterId, AdapterRefusalBeforeDeliveryProof,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest,
    AuditEventKind, AuthorityDomainId, CommandId, DeviceId, EndpointId, ExternalEffectDisposition,
    FailureCode, FreshSpawn, Generation, GrantId, IdempotencyKey, LogicalTargetId, Lsn,
    NoExternalEffectProof, ObservationRequest, Operation, OperationKind, PayloadContentType,
    PayloadEnvelope, SpawnClaimAccepted, SpawnExecutionEvidence, SpawnExecutionEvidenceProducer,
    SpawnExecutionPhase, SpawnGenerationClaim, SpawnRequest, SpawnTargetSpec, StoredEventKind,
    TargetScope, TargetScopeKind,
};
use patchbay_core::storage::{AuditRecordDraft, RusqliteStorage, Storage, TargetKey};
use patchbay_core_server::{
    adapter_service::{
        AdapterControlServiceImpl, AdapterEvidenceVerifier, ADAPTER_ATTACHMENT_TOKEN_HEADER,
        ADAPTER_EVIDENCE_HEADER, ADAPTER_ID_HEADER,
    },
    rpc::adapter_control_service_server::AdapterControlService,
};
use prost::Message;
use prost_types::Timestamp;
use tonic::Request;

const EVIDENCE: &str = "spawn-evidence-secret";

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".into(),
    }
}

fn adapter_id() -> AdapterId {
    AdapterId { value: "pi".into() }
}

fn authenticated<T>(message: T, token: &str) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert(ADAPTER_ID_HEADER, "pi".parse().expect("metadata"));
    request
        .metadata_mut()
        .insert(ADAPTER_EVIDENCE_HEADER, EVIDENCE.parse().expect("metadata"));
    request.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        token.parse().expect("metadata"),
    );
    request
}

#[tokio::test]
async fn authenticated_evidence_is_canonicalized_and_wrong_claim_is_not_appended() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let verifier = AdapterEvidenceVerifier::new([("pi", EVIDENCE)]).expect("verifier");
    let service = AdapterControlServiceImpl::new(storage.clone(), domain(), verifier)
        .await
        .expect("service");
    let attach = service
        .attach(Request::new(AttachRequest {
            registration: Some(AdapterRegistration {
                adapter_id: Some(adapter_id()),
                endpoint_id: Some(patchbay_contracts::patchbay::EndpointId {
                    value: "pi-endpoint".into(),
                }),
                authority_domain_id: Some(domain()),
                adapter_generation: Some(Generation { value: 1 }),
                capability: Some(AdapterCapability {
                    session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                    target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                    ..AdapterCapability::default()
                }),
                ..AdapterRegistration::default()
            }),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach");
    let token = attach
        .metadata()
        .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .expect("token")
        .to_str()
        .expect("ASCII")
        .to_owned();

    let claim = SpawnGenerationClaim {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(CommandId {
            value: "spawn-evidence".into(),
        }),
        logical_target_id: Some(LogicalTargetId {
            value: "logical-evidence".into(),
        }),
        expected_prior: None,
        claimed_generation: Some(Generation { value: 1 }),
    };
    let operation = Operation {
        command_id: claim.claim_operation_id.clone(),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "operator".into(),
            }),
            endpoint_id: Some(EndpointId {
                value: "web".into(),
            }),
            device_id: Some(DeviceId {
                value: "device".into(),
            }),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(adapter_id()),
            ..TargetScope::default()
        }),
        idempotency_key: "spawn-evidence-key".into(),
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
            schema_ref: patchbay_core::acceptance::SPAWN_REQUEST_SCHEMA.to_owned(),
        }),
        ..Operation::default()
    };
    let accepted = SpawnClaimAccepted {
        accepted_operation: Some(AcceptedOperation {
            operation: Some(operation.clone()),
            authorizing_grant_id: Some(GrantId {
                value: "spawn-grant".into(),
            }),
        }),
        claim: Some(claim.clone()),
        ..SpawnClaimAccepted::default()
    };
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        },
        AuditEventKind::CommandSubmissionAccepted,
    );
    audit.actor_id = operation
        .sender
        .as_ref()
        .and_then(|sender| sender.actor_id.clone());
    audit.endpoint_id = operation
        .sender
        .as_ref()
        .and_then(|sender| sender.endpoint_id.clone());
    audit.device_id = operation
        .sender
        .as_ref()
        .and_then(|sender| sender.device_id.clone());
    audit.command_id = operation.command_id.clone();
    audit.grant_id = Some(GrantId {
        value: "spawn-grant".into(),
    });
    audit.target_scope = operation.target_scope.clone();
    audit.reason_code = "operation_spawn".into();
    storage
        .append_spawn_claim_accepted(
            &domain(),
            &IdempotencyKey {
                value: operation.idempotency_key.clone(),
            },
            &TargetKey::new("adapter:pi".into()).unwrap(),
            accepted,
            audit,
            operation.encode_to_vec(),
        )
        .await
        .expect("claim append");

    let evidence = SpawnExecutionEvidence {
        authority_domain_id: Some(domain()),
        exact_claim: Some(claim.clone()),
        phase: SpawnExecutionPhase::Offered as i32,
        external_effect_disposition: ExternalEffectDisposition::ProvedNone as i32,
        // Both fields are untrusted input and must be canonicalized.
        producer: SpawnExecutionEvidenceProducer::Core as i32,
        source_attachment: None,
        failure_code: FailureCode::DeliveryRejected as i32,
        no_external_effect_proof: Some(NoExternalEffectProof {
            proof: Some(
                no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(
                    AdapterRefusalBeforeDeliveryProof {
                        adapter_id: Some(adapter_id()),
                        adapter_generation: Some(Generation { value: 1 }),
                    },
                ),
            ),
        }),
        external_runtime: None,
    };
    let receipt = service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::SpawnExecutionEvidence(
                    evidence.clone(),
                )),
            },
            &token,
        ))
        .await
        .expect("evidence accepted")
        .into_inner()
        .event_id
        .expect("event id");

    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("events");
    let stored = events
        .iter()
        .find(|event| event.event_id == receipt)
        .expect("stored evidence");
    assert_eq!(
        StoredEventKind::try_from(stored.payload.kind).ok(),
        Some(StoredEventKind::SpawnExecutionEvidence)
    );
    let canonical =
        SpawnExecutionEvidence::decode(stored.payload.payload.as_slice()).expect("typed evidence");
    assert_eq!(
        canonical.producer,
        SpawnExecutionEvidenceProducer::CurrentAdapter as i32
    );
    assert_eq!(
        canonical
            .source_attachment
            .as_ref()
            .and_then(|source| source.adapter_id.as_ref()),
        Some(&adapter_id())
    );
    assert_eq!(
        canonical
            .source_attachment
            .as_ref()
            .and_then(|source| source.adapter_generation)
            .map(|generation| generation.value),
        Some(1)
    );

    let before = events.len();
    let mut wrong = evidence;
    wrong
        .exact_claim
        .as_mut()
        .expect("claim")
        .claim_operation_id = Some(CommandId {
        value: "another-claim".into(),
    });
    assert!(service
        .ingest_observation(authenticated(
            ObservationRequest {
                authority_domain_id: Some(domain()),
                observation: Some(observation_request::Observation::SpawnExecutionEvidence(
                    wrong
                ),),
            },
            &token,
        ))
        .await
        .is_err());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .expect("events after rejection")
            .len(),
        before
    );
}
