use std::sync::Arc;

use patchbay_contracts::patchbay::{
    spawn_claim_disposition_changed, spawn_claim_event, spawn_request, AbandonSpawnTargetRequest,
    AcceptedOperation, ActorEndpointRef, ActorId, AdapterId, AuditEventKind, AuditRecord,
    AuthorityDomainId, CommandId, DeviceId, EndpointId, FreshSpawn, Generation, Grant, GrantId,
    GrantProvenance, GrantRevocationPolicy, IdempotencyKey, LogicalTargetCreated, LogicalTargetId,
    Lsn, Operation, OperationKind, OperatorRecord, PayloadContentType, PayloadEnvelope,
    PrincipalEnrollment, SpawnClaimAccepted, SpawnClaimDisposition, SpawnClaimEvent,
    SpawnGenerationClaim, SpawnRequest, SpawnTargetSpec, StoredEventKind, StoredEventPayload,
    TargetScope, TargetScopeKind, VerifyOperatorPasswordRequest,
};
use patchbay_core::{
    session::events as session_events,
    storage::{AuditRecordDraft, RusqliteStorage, Storage, TargetKey},
    time::TestClock,
};
use patchbay_core_server::{
    issuer::{
        OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER,
    },
    rpc::control_service_server::ControlService,
    service::ControlServiceImpl,
};
use prost::Message;
use prost_types::Timestamp;
use tonic::{Code, Request};

const OPERATOR: &str = "operator";
const PASSWORD_HASH: &str = "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g";

#[derive(Clone)]
struct Auth {
    session_id: String,
    principal_id: String,
    principal_secret: String,
}

#[derive(Clone, Copy)]
enum GrantPosture {
    Live,
    WrongKind,
    WrongTarget,
    Revoked,
    Expired,
}

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn command() -> CommandId {
    CommandId {
        value: "spawn-a".to_owned(),
    }
}

fn logical() -> LogicalTargetId {
    LogicalTargetId {
        value: "logical-a".to_owned(),
    }
}

fn adapter_scope(adapter: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: adapter.to_owned(),
        }),
        ..TargetScope::default()
    }
}

fn operation() -> Operation {
    Operation {
        command_id: Some(command()),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: OPERATOR.to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "web".to_owned(),
            }),
            device_id: Some(DeviceId {
                value: "device".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 1 }),
        }),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(adapter_scope("pi")),
        idempotency_key: "spawn-a-key".to_owned(),
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
    }
}

async fn fixture(
    posture: GrantPosture,
) -> (ControlServiceImpl<RusqliteStorage>, RusqliteStorage, Auth) {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let operator = OperatorRecord {
        actor_id: Some(ActorId {
            value: OPERATOR.to_owned(),
        }),
        password_hash: PASSWORD_HASH.to_owned(),
        created_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        authority_domain_id: Some(domain()),
    };
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::OperatorRecord as i32,
                payload: operator.encode_to_vec(),
            },
        )
        .await
        .unwrap();

    let grant = Grant {
        grant_id: Some(GrantId {
            value: "abandon-grant".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId {
            value: OPERATOR.to_owned(),
        }),
        subject_endpoint_id: Some(EndpointId {
            value: "web".to_owned(),
        }),
        target_scope: Some(match posture {
            GrantPosture::WrongTarget => adapter_scope("other"),
            _ => adapter_scope("pi"),
        }),
        allowed_operation_kinds: vec![match posture {
            GrantPosture::WrongKind => OperationKind::Query as i32,
            _ => OperationKind::SessionManagement as i32,
        }],
        created_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        expires_at: matches!(posture, GrantPosture::Expired).then_some(Timestamp {
            seconds: 99,
            nanos: 0,
        }),
        revocation_generation: matches!(posture, GrantPosture::Revoked)
            .then_some(Generation { value: 1 }),
        revoked_at: matches!(posture, GrantPosture::Revoked).then_some(Timestamp {
            seconds: 50,
            nanos: 0,
        }),
        provenance: Some(GrantProvenance {
            reason: "abandonment fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: grant.encode_to_vec(),
            },
        )
        .await
        .unwrap();
    storage
        .append(
            &domain(),
            session_events::encode(&session_events::logical_target_created(
                domain(),
                LogicalTargetCreated {
                    logical_target_id: Some(logical()),
                    adapter_id: Some(AdapterId {
                        value: "pi".to_owned(),
                    }),
                    deployment_scope: "machine-a".to_owned(),
                },
            )),
        )
        .await
        .unwrap();

    let operation = operation();
    let accepted = SpawnClaimAccepted {
        accepted_operation: Some(AcceptedOperation {
            operation: Some(operation.clone()),
            authorizing_grant_id: Some(GrantId {
                value: "spawn-grant".to_owned(),
            }),
        }),
        claim: Some(SpawnGenerationClaim {
            authority_domain_id: Some(domain()),
            claim_operation_id: Some(command()),
            logical_target_id: Some(logical()),
            expected_prior: None,
            claimed_generation: Some(Generation { value: 1 }),
        }),
        ..SpawnClaimAccepted::default()
    };
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 2,
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
    audit.command_id = Some(command());
    audit.grant_id = Some(GrantId {
        value: "spawn-grant".to_owned(),
    });
    audit.target_scope = operation.target_scope.clone();
    audit.reason_code = "operation_spawn".to_owned();
    storage
        .append_spawn_claim_accepted(
            &domain(),
            &IdempotencyKey {
                value: operation.idempotency_key.clone(),
            },
            &TargetKey::new("adapter:pi".to_owned()).unwrap(),
            accepted,
            audit,
            operation.encode_to_vec(),
        )
        .await
        .unwrap();

    let service = ControlServiceImpl::new_with_clock(
        storage.clone(),
        domain(),
        Arc::new(TestClock::new(Timestamp {
            seconds: 100,
            nanos: 7,
        })),
    )
    .await
    .unwrap();
    let login = service
        .verify_operator_password(Request::new(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(ActorId {
                value: OPERATOR.to_owned(),
            }),
            password: "correct-password".to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(EndpointId {
                    value: "web".to_owned(),
                }),
                device_id: Some(DeviceId {
                    value: "device".to_owned(),
                }),
                endpoint_generation: Some(Generation { value: 1 }),
            }),
        }))
        .await
        .unwrap()
        .into_inner();
    let principal = login.principal.unwrap();
    (
        service,
        storage,
        Auth {
            session_id: login.operator_session_id.unwrap().value,
            principal_id: principal.principal_id,
            principal_secret: principal.secret,
        },
    )
}

fn authenticated(auth: &Auth, target: LogicalTargetId) -> Request<AbandonSpawnTargetRequest> {
    let mut request = Request::new(AbandonSpawnTargetRequest {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(command()),
        logical_target_id: Some(target),
        reason_code: "operator_recovery".to_owned(),
    });
    request
        .metadata_mut()
        .insert(OPERATOR_SESSION_HEADER, auth.session_id.parse().unwrap());
    request
        .metadata_mut()
        .insert(OPERATOR_ID_HEADER, OPERATOR.parse().unwrap());
    request
        .metadata_mut()
        .insert(PRINCIPAL_ID_HEADER, auth.principal_id.parse().unwrap());
    request.metadata_mut().insert(
        PRINCIPAL_SECRET_HEADER,
        auth.principal_secret.parse().unwrap(),
    );
    request
}

#[tokio::test]
async fn authenticated_grant_checked_abandonment_is_audited_and_idempotent() {
    let (service, storage, auth) = fixture(GrantPosture::Live).await;
    let result = service
        .abandon_spawn_target(authenticated(&auth, logical()))
        .await
        .unwrap()
        .into_inner();
    assert!(result.changed);
    assert!(!result.already_abandoned);
    assert_eq!(
        result.disposition,
        SpawnClaimDisposition::TargetAbandoned as i32
    );
    assert_eq!(result.authorizing_grant_id.unwrap().value, "abandon-grant");

    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    let source_id = result.abandonment_event_id.clone().unwrap();
    let source = events
        .iter()
        .find(|event| event.event_id == source_id)
        .unwrap();
    let event = SpawnClaimEvent::decode(source.payload.payload.as_slice()).unwrap();
    let spawn_claim_event::Mutation::DispositionChanged(change) = event.mutation.unwrap() else {
        panic!("expected abandonment disposition");
    };
    let spawn_claim_disposition_changed::Evidence::TargetAbandonment(evidence) =
        change.evidence.unwrap()
    else {
        panic!("expected typed abandonment evidence");
    };
    let decision_time = evidence.abandoned_at.unwrap();
    assert_eq!(decision_time.seconds, 100);
    assert_eq!(decision_time.nanos, 7);
    assert_eq!(
        evidence.abandoned_by.unwrap().actor_id.unwrap().value,
        OPERATOR
    );
    let audit = events
        .iter()
        .find(|event| event.event_id == result.audit_event_id.clone().unwrap())
        .map(|event| AuditRecord::decode(event.payload.payload.as_slice()).unwrap())
        .unwrap();
    assert_eq!(
        AuditEventKind::try_from(audit.kind).ok(),
        Some(AuditEventKind::SpawnTargetAbandoned)
    );
    assert_eq!(audit.source_event_id, Some(source_id));
    assert_eq!(audit.occurred_at, Some(decision_time));

    let retry = service
        .abandon_spawn_target(authenticated(&auth, logical()))
        .await
        .unwrap()
        .into_inner();
    assert!(!retry.changed);
    assert!(retry.already_abandoned);
    assert_eq!(retry.abandonment_event_id, result.abandonment_event_id);
    assert_eq!(retry.audit_event_id, result.audit_event_id);
}

#[tokio::test]
async fn unauthenticated_wrong_kind_target_revoked_and_expired_grants_reject() {
    let (service, storage, auth) = fixture(GrantPosture::Live).await;
    let error = service
        .abandon_spawn_target(Request::new(AbandonSpawnTargetRequest {
            authority_domain_id: Some(domain()),
            claim_operation_id: Some(command()),
            logical_target_id: Some(logical()),
            reason_code: "operator_recovery".to_owned(),
        }))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::Unauthenticated);
    let before = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let wrong_target = LogicalTargetId {
        value: "logical-other".to_owned(),
    };
    let error = service
        .abandon_spawn_target(authenticated(&auth, wrong_target))
        .await
        .unwrap_err();
    assert_eq!(error.code(), Code::FailedPrecondition);
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        before
    );

    for posture in [
        GrantPosture::WrongKind,
        GrantPosture::WrongTarget,
        GrantPosture::Revoked,
        GrantPosture::Expired,
    ] {
        let (service, storage, auth) = fixture(posture).await;
        let before = storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len();
        let error = service
            .abandon_spawn_target(authenticated(&auth, logical()))
            .await
            .unwrap_err();
        assert_eq!(error.code(), Code::PermissionDenied);
        let events = storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap();
        assert_eq!(
            events.len(),
            before + 1,
            "denial must append only its audit"
        );
        assert_eq!(
            events.last().unwrap().payload.kind,
            StoredEventKind::AuditRecord as i32
        );
    }
}
