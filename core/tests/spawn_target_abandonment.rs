use patchbay_contracts::patchbay::{
    session_state_event, spawn_claim_event, spawn_request, AcceptedOperation, ActorEndpointRef,
    ActorId, AdapterId, AuditEventKind, AuthorityDomainId, CommandId,
    ContinuationAuthorityProvenance, DeviceId, EndpointId, ExternalRuntimeRef, FailureCode,
    Generation, GrantId, IdempotencyKey, LogicalTargetCandidateReserved, LogicalTargetCreated,
    LogicalTargetId, LogicalTargetInitialCurrentAssigned, Lsn, Operation, OperationKind,
    PayloadContentType, PayloadEnvelope, RuntimeGenerationRef, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionRegistered, SessionState,
    SessionStateEvent, SpawnClaimAbandonmentEvidence, SpawnClaimAccepted, SpawnClaimDisposition,
    SpawnClaimEvent, SpawnContinuation, SpawnGenerationClaim, SpawnPendingReplacementFence,
    SpawnRequest, SpawnTargetSpec, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::CommandIndex,
    diagnostics::DiagnosticsProjection,
    session::{
        encode_spawn_claim_event, rebuild_from_log, rebuild_spawn_claims_from_log,
        ExternalRuntimeOwnership, LogicalTargetError, SpawnClaimQuery, SpawnDeliveryFence,
        REPLACEMENT_PENDING_REASON,
    },
    storage::{AuditRecordDraft, RusqliteStorage, Storage, StorageError, TargetKey},
};
use prost::Message;
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn command() -> CommandId {
    CommandId {
        value: "spawn-replacement".to_owned(),
    }
}

fn logical() -> LogicalTargetId {
    LogicalTargetId {
        value: "logical-a".to_owned(),
    }
}

fn external(runtime: &str, generation: u64) -> ExternalRuntimeRef {
    ExternalRuntimeRef {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: runtime.to_owned(),
        }),
        generation: Some(Generation { value: generation }),
    }
}

fn runtime(runtime: &str, generation: u64) -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(logical()),
        external_runtime: Some(external(runtime, generation)),
    }
}

fn prior_scope() -> TargetScope {
    let prior = external("runtime-a", 7);
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: prior.adapter_id,
        deployment_scope: prior.deployment_scope,
        runtime_session_id: prior.runtime_session_id,
        session_generation: prior.generation,
        ..TargetScope::default()
    }
}

fn accepted_claim() -> SpawnClaimAccepted {
    let prior = runtime("runtime-a", 7);
    let operation = Operation {
        command_id: Some(command()),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "operator".to_owned(),
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
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            ..TargetScope::default()
        }),
        idempotency_key: "spawn-replacement-key".to_owned(),
        payload: Some(PayloadEnvelope {
            payload: SpawnRequest {
                intent: Some(spawn_request::Intent::Continuation(SpawnContinuation {
                    prior: Some(prior.clone()),
                })),
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
    SpawnClaimAccepted {
        accepted_operation: Some(AcceptedOperation {
            operation: Some(operation),
            authorizing_grant_id: Some(GrantId {
                value: "spawn-grant".to_owned(),
            }),
        }),
        claim: Some(SpawnGenerationClaim {
            authority_domain_id: Some(domain()),
            claim_operation_id: Some(command()),
            logical_target_id: Some(logical()),
            expected_prior: Some(prior.clone()),
            claimed_generation: Some(Generation { value: 8 }),
        }),
        compound_authority: Some(ContinuationAuthorityProvenance {
            exact_prior: Some(prior.clone()),
            replacement_grant_id: Some(GrantId {
                value: "replacement-grant".to_owned(),
            }),
            replacement_authority_kind: OperationKind::SessionManagement as i32,
        }),
        pending_replacement: Some(SpawnPendingReplacementFence {
            exact_prior: Some(prior),
            failure_code: FailureCode::Superseded as i32,
            reason_code: REPLACEMENT_PENDING_REASON.to_owned(),
        }),
        prior_work_effects: Vec::new(),
    }
}

async fn seed_active_continuation(storage: &RusqliteStorage) {
    let registration = patchbay_core::session::events::registered(
        domain(),
        SessionRegistered {
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            session_generation: Some(Generation { value: 7 }),
            initial_state: Some(SessionState {
                connectivity: SessionConnectivityState::Stale as i32,
                activity: SessionActivityState::Unknown as i32,
            }),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "prior".to_owned(),
            ..SessionRegistered::default()
        },
    );
    storage
        .append(
            &domain(),
            patchbay_core::session::events::encode(&registration),
        )
        .await
        .unwrap();
    for mutation in [
        session_state_event::Mutation::LogicalTargetCreated(LogicalTargetCreated {
            logical_target_id: Some(logical()),
            adapter_id: Some(AdapterId {
                value: "pi".to_owned(),
            }),
            deployment_scope: "machine-a".to_owned(),
        }),
        session_state_event::Mutation::LogicalTargetInitialCurrentAssigned(
            LogicalTargetInitialCurrentAssigned {
                logical_target_id: Some(logical()),
                external_runtime_ref: Some(external("runtime-a", 7)),
            },
        ),
    ] {
        storage
            .append(
                &domain(),
                patchbay_core::session::events::encode(&SessionStateEvent {
                    authority_domain_id: Some(domain()),
                    mutation: Some(mutation),
                }),
            )
            .await
            .unwrap();
    }

    let accepted = accepted_claim();
    let operation = accepted
        .accepted_operation
        .as_ref()
        .and_then(|accepted| accepted.operation.as_ref())
        .unwrap()
        .clone();
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

    storage
        .append(
            &domain(),
            patchbay_core::session::events::encode(&SessionStateEvent {
                authority_domain_id: Some(domain()),
                mutation: Some(
                    session_state_event::Mutation::LogicalTargetCandidateReserved(
                        LogicalTargetCandidateReserved {
                            logical_target_id: Some(logical()),
                            external_runtime_ref: Some(external("runtime-b", 8)),
                        },
                    ),
                ),
            }),
        )
        .await
        .unwrap();
}

fn abandonment_evidence(target: LogicalTargetId, reason: &str) -> SpawnClaimAbandonmentEvidence {
    SpawnClaimAbandonmentEvidence {
        abandonment_event_id: None,
        logical_target_id: Some(target),
        authorizing_grant_id: Some(GrantId {
            value: "abandon-grant".to_owned(),
        }),
        abandonment_authority_kind: OperationKind::SessionManagement as i32,
        abandoned_by: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "operator".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "web".to_owned(),
            }),
            device_id: Some(DeviceId {
                value: "device".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 1 }),
        }),
        reason_code: reason.to_owned(),
        abandoned_at: Some(Timestamp {
            seconds: 1_700_000_100,
            nanos: 0,
        }),
    }
}

fn abandonment_audit(reason: &str) -> AuditRecordDraft {
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 1_700_000_100,
            nanos: 0,
        },
        AuditEventKind::SpawnTargetAbandoned,
    );
    audit.actor_id = Some(ActorId {
        value: "operator".to_owned(),
    });
    audit.endpoint_id = Some(EndpointId {
        value: "web".to_owned(),
    });
    audit.device_id = Some(DeviceId {
        value: "device".to_owned(),
    });
    audit.command_id = Some(command());
    audit.grant_id = Some(GrantId {
        value: "abandon-grant".to_owned(),
    });
    audit.target_scope = Some(prior_scope());
    audit.reason_code = reason.to_owned();
    audit
}

#[tokio::test]
async fn atomic_abandonment_clears_fence_retires_target_and_is_restart_idempotent() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    seed_active_continuation(&storage).await;

    let appended = storage
        .append_spawn_target_abandonment_audited(
            &domain(),
            command(),
            logical(),
            abandonment_evidence(logical(), "operator_recovery"),
            abandonment_audit("operator_recovery"),
        )
        .await
        .unwrap();
    assert!(!appended.deduplicated);
    assert_eq!(
        appended.audit_event_id.lsn.unwrap().value,
        appended.source_event_id.lsn.unwrap().value + 1
    );
    assert_eq!(
        appended.change.to_disposition,
        SpawnClaimDisposition::TargetAbandoned as i32
    );

    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    let mut commands = CommandIndex::new();
    let mut diagnostics = DiagnosticsProjection::new(domain()).unwrap();
    for event in &events {
        commands.apply(event).unwrap();
        diagnostics.observe(event).unwrap();
    }
    assert!(commands.delivery_is_suppressed(&command()));
    let inspection = diagnostics
        .result_for_query(&command())
        .unwrap()
        .inspection
        .unwrap();
    assert!(inspection.history.iter().any(|entry| {
        entry.event_id.as_ref() == Some(&appended.source_event_id)
            && entry.failure_code == FailureCode::ExecutionOutcomeUnknown as i32
    }));

    let claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    let claim = claims.claim_for_operation(&command()).unwrap();
    assert_eq!(claim.disposition, SpawnClaimDisposition::TargetAbandoned);
    assert!(claim.pending_replacement.is_none());
    assert_eq!(
        claims.delivery_fence(&runtime("runtime-a", 7)),
        SpawnDeliveryFence::Open
    );

    let sessions = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert!(sessions.sessions().next().is_none());
    let target = sessions.logical_targets().get(&logical()).unwrap();
    assert_eq!(
        target.retired_at_lsn,
        appended.source_event_id.lsn.as_ref().map(|lsn| lsn.value)
    );
    assert!(target.current.is_none());
    assert!(target.reserved_candidate.is_none());
    assert_eq!(target.tombstones.len(), 2);
    assert_eq!(
        sessions
            .logical_targets()
            .owner_of(&external("runtime-b", 8)),
        Some(&logical())
    );

    let retry = storage
        .append_spawn_target_abandonment_audited(
            &domain(),
            command(),
            logical(),
            abandonment_evidence(logical(), "operator_recovery"),
            abandonment_audit("operator_recovery"),
        )
        .await
        .unwrap();
    assert!(retry.deduplicated);
    assert_eq!(retry.source_event_id, appended.source_event_id);
    assert_eq!(retry.audit_event_id, appended.audit_event_id);

    let restarted_claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    let restarted_sessions = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert_eq!(restarted_claims, claims);
    assert_eq!(restarted_sessions, sessions);
    let mut retired_targets = restarted_sessions.logical_targets().clone();
    assert_eq!(
        retired_targets.reserve_candidate(&logical(), external("runtime-c", 9)),
        Err(LogicalTargetError::RetiredTarget)
    );
}

#[tokio::test]
async fn wrong_target_terminal_retry_and_generic_routes_reject_without_mutation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    seed_active_continuation(&storage).await;
    let before = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let wrong_target = LogicalTargetId {
        value: "logical-other".to_owned(),
    };
    assert!(matches!(
        storage
            .append_spawn_target_abandonment_audited(
                &domain(),
                command(),
                wrong_target.clone(),
                abandonment_evidence(wrong_target, "operator_recovery"),
                abandonment_audit("operator_recovery"),
            )
            .await,
        Err(StorageError::SpawnTargetAbandonmentConflict { .. })
    ));
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        before
    );

    let mut wrong_scope_audit = abandonment_audit("operator_recovery");
    wrong_scope_audit.target_scope = Some(TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        ..TargetScope::default()
    });
    assert!(matches!(
        storage
            .append_spawn_target_abandonment_audited(
                &domain(),
                command(),
                logical(),
                abandonment_evidence(logical(), "operator_recovery"),
                wrong_scope_audit,
            )
            .await,
        Err(StorageError::InvalidAuditRecord(_))
    ));
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        before
    );

    let committed = storage
        .append_spawn_target_abandonment_audited(
            &domain(),
            command(),
            logical(),
            abandonment_evidence(logical(), "operator_recovery"),
            abandonment_audit("operator_recovery"),
        )
        .await
        .unwrap();
    assert!(matches!(
        storage
            .append_spawn_target_abandonment_audited(
                &domain(),
                command(),
                logical(),
                abandonment_evidence(logical(), "different_recovery"),
                abandonment_audit("different_recovery"),
            )
            .await,
        Err(StorageError::SpawnTargetAbandonmentConflict { .. })
    ));

    let generic = encode_spawn_claim_event(&SpawnClaimEvent {
        authority_domain_id: Some(domain()),
        mutation: Some(spawn_claim_event::Mutation::DispositionChanged(
            committed.change,
        )),
    });
    assert!(matches!(
        storage.append(&domain(), generic).await,
        Err(StorageError::UnsupportedOperation)
    ));
}
