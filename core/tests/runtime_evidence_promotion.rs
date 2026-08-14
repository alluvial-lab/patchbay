use patchbay_contracts::patchbay::{
    quarantined_runtime_evidence, runtime_generation_disposition, spawn_claim_event,
    typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId, AdapterId, AuditEventKind,
    AuthorityDomainId, CommandId, CommandTransition, DescendantGrant, DescendantGrantProvenance,
    EndpointId, EventId, ExternalRuntimeRef, FailureCode, Generation, GrantId,
    GrantRevocationPolicy, LogicalTargetId, Lsn, Observation, ObservationKind, Operation,
    OperationKind, OperationState, QuarantinedRuntimeEvidence,
    RuntimeEvidenceClassificationContext, RuntimeEvidenceQuarantineReason,
    RuntimeEvidenceSourceAttachment, RuntimeGenerationDisposition, RuntimeGenerationUnknown,
    RuntimeSessionId, SessionActivityState, SessionConnectivityState, SessionReport,
    SessionReportSourceCursor, SpawnClaimAccepted, SpawnClaimDisposition, SpawnClaimEvent,
    SpawnGenerationClaim, SpawnPromotionAuthorityEvidence, SpawnPromotionCommitted,
    SpawnPromotionLifecycleEvidence, SpawnPromotionResultEvidence, SpawnPromotionStagedEvidence,
    SpawnSuccessorEvidenceStaged, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    acceptance::CommandIndex,
    session::{
        classify_session_report, encode_quarantined_runtime_evidence, encode_spawn_claim_event,
        encode_staged_successor, LogicalTargetRegistry, SessionRegistry, SpawnClaimQuery,
        SpawnClaimRegistry,
    },
    storage::{AuditRecordDraft, RecordedEvent, RusqliteStorage, Storage, StorageError},
};
use prost::Message;
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn event_id(lsn: u64) -> EventId {
    EventId {
        authority_domain_id: Some(domain()),
        lsn: Some(Lsn { value: lsn }),
    }
}

fn command() -> CommandId {
    CommandId {
        value: "spawn-a".to_owned(),
    }
}

fn external(generation: u64) -> ExternalRuntimeRef {
    ExternalRuntimeRef {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "runtime-a".to_owned(),
        }),
        generation: Some(Generation { value: generation }),
    }
}

fn claim() -> SpawnGenerationClaim {
    SpawnGenerationClaim {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(command()),
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        expected_prior: None,
        claimed_generation: Some(Generation { value: 1 }),
    }
}

fn accepted_operation() -> AcceptedOperation {
    AcceptedOperation {
        operation: Some(Operation {
            command_id: Some(command()),
            authority_domain_id: Some(domain()),
            sender: Some(ActorEndpointRef {
                actor_id: Some(ActorId {
                    value: "operator".to_owned(),
                }),
                endpoint_id: Some(EndpointId {
                    value: "web".to_owned(),
                }),
                ..ActorEndpointRef::default()
            }),
            kind: OperationKind::Spawn as i32,
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(AdapterId {
                    value: "pi".to_owned(),
                }),
                ..TargetScope::default()
            }),
            idempotency_key: "spawn-key".to_owned(),
            ..Operation::default()
        }),
        authorizing_grant_id: Some(GrantId {
            value: "spawn-grant".to_owned(),
        }),
    }
}

fn accepted_claim() -> SpawnClaimAccepted {
    SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation()),
        claim: Some(claim()),
        ..SpawnClaimAccepted::default()
    }
}

fn report(operation: &str) -> SessionReport {
    SessionReport {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "runtime-a".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        connectivity: SessionConnectivityState::Live as i32,
        activity: SessionActivityState::Idle as i32,
        spawn_origin: Some(TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(CommandId {
                value: operation.to_owned(),
            })),
        }),
        source_cursor: Some(SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 3 }),
            revision: 1,
        }),
        ..SessionReport::default()
    }
}

fn source_attachment() -> RuntimeEvidenceSourceAttachment {
    RuntimeEvidenceSourceAttachment {
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        adapter_generation: Some(Generation { value: 3 }),
        attachment_event_id: Some(event_id(1)),
    }
}

fn staged() -> SpawnSuccessorEvidenceStaged {
    let report = report("spawn-a");
    let target = patchbay_contracts::patchbay::RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        external_runtime: Some(external(1)),
    };
    SpawnSuccessorEvidenceStaged {
        authority_domain_id: Some(domain()),
        exact_claim: Some(claim()),
        report: Some(report),
        classified_target: Some(target),
        disposition: Some(RuntimeGenerationDisposition {
            disposition: Some(
                runtime_generation_disposition::Disposition::ClaimedSuccessor(
                    patchbay_contracts::patchbay::RuntimeGenerationClaimedSuccessor {
                        claim_operation_id: Some(command()),
                        expected_prior: None,
                        claimed_generation: Some(Generation { value: 1 }),
                    },
                ),
            ),
        }),
        source_attachment: Some(source_attachment()),
        external_runtime_reservation: Some(external(1)),
    }
}

fn quarantined(candidate: Observation) -> QuarantinedRuntimeEvidence {
    QuarantinedRuntimeEvidence {
        authority_domain_id: Some(domain()),
        candidate: Some(quarantined_runtime_evidence::Candidate::Observation(
            candidate,
        )),
        classification: Some(RuntimeEvidenceClassificationContext {
            disposition: Some(RuntimeGenerationDisposition {
                disposition: Some(runtime_generation_disposition::Disposition::Unknown(
                    RuntimeGenerationUnknown {},
                )),
            }),
            ..RuntimeEvidenceClassificationContext::default()
        }),
        reason: RuntimeEvidenceQuarantineReason::UnknownTarget as i32,
        source_attachment: Some(source_attachment()),
    }
}

fn transition(from: OperationState, to: OperationState) -> CommandTransition {
    CommandTransition {
        command_id: Some(command()),
        from_state: from as i32,
        to_state: to as i32,
        failure_code: FailureCode::Unspecified as i32,
        ..CommandTransition::default()
    }
}

fn unstamped_promotion() -> SpawnPromotionCommitted {
    let promoted = patchbay_contracts::patchbay::RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        external_runtime: Some(external(1)),
    };
    SpawnPromotionCommitted {
        authority_domain_id: Some(domain()),
        promotion_event_id: None,
        completion_audit_event_id: None,
        accepted_claim_event_id: Some(event_id(1)),
        accepted_claim: Some(accepted_claim()),
        lifecycle: vec![
            SpawnPromotionLifecycleEvidence {
                event_id: Some(event_id(2)),
                transition: Some(transition(
                    OperationState::Accepted,
                    OperationState::Delivered,
                )),
            },
            SpawnPromotionLifecycleEvidence {
                event_id: Some(event_id(3)),
                transition: Some(transition(
                    OperationState::Delivered,
                    OperationState::Running,
                )),
            },
        ],
        successful_result: Some(SpawnPromotionResultEvidence {
            event_id: Some(event_id(4)),
            command_id: Some(command()),
            target_scope: accepted_operation()
                .operation
                .and_then(|operation| operation.target_scope),
            failure_code: FailureCode::Unspecified as i32,
            observed_at: Some(Timestamp {
                seconds: 10,
                nanos: 0,
            }),
        }),
        staged_successor: Some(SpawnPromotionStagedEvidence {
            event_id: Some(event_id(5)),
            staged: Some(staged()),
        }),
        promoted_runtime: Some(promoted.clone()),
        external_runtime_reservation: Some(external(1)),
        authority: Some(SpawnPromotionAuthorityEvidence {
            spawning_grant_id: Some(GrantId {
                value: "spawn-grant".to_owned(),
            }),
            continuation_authority: None,
            descendant_grant: Some(DescendantGrant {
                grant_id: Some(GrantId {
                    value: "descendant".to_owned(),
                }),
                authority_domain_id: Some(domain()),
                subject_actor_id: Some(ActorId {
                    value: "operator".to_owned(),
                }),
                subject_endpoint_id: Some(EndpointId {
                    value: "web".to_owned(),
                }),
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::RuntimeSession as i32,
                    adapter_id: external(1).adapter_id,
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: external(1).runtime_session_id,
                    session_generation: Some(Generation { value: 1 }),
                    ..TargetScope::default()
                }),
                allowed_operation_kinds: vec![OperationKind::Instruct as i32],
                provenance: Some(DescendantGrantProvenance {
                    spawn_operation_id: Some(command()),
                    spawning_grant_id: Some(GrantId {
                        value: "spawn-grant".to_owned(),
                    }),
                    continuation_authority: None,
                }),
                created_at: Some(Timestamp {
                    seconds: 10,
                    nanos: 0,
                }),
                revocation_policy: GrantRevocationPolicy::Continue as i32,
                audit_id: None,
                ..DescendantGrant::default()
            }),
        }),
        committed_at: Some(Timestamp {
            seconds: 10,
            nanos: 0,
        }),
    }
}

fn recorded(lsn: u64, payload: StoredEventPayload) -> RecordedEvent {
    RecordedEvent {
        event_id: event_id(lsn),
        payload,
    }
}

#[test]
fn classifier_admits_only_the_exact_active_claimed_successor() {
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    let accepted = SpawnClaimEvent {
        authority_domain_id: Some(domain()),
        mutation: Some(spawn_claim_event::Mutation::Accepted(accepted_claim())),
    };
    claims
        .observe(&recorded(1, encode_spawn_claim_event(&accepted)))
        .unwrap();
    let mut targets = LogicalTargetRegistry::new(domain()).unwrap();
    targets
        .create(
            LogicalTargetId {
                value: "logical-a".to_owned(),
            },
            AdapterId {
                value: "pi".to_owned(),
            },
            "machine-a".to_owned(),
        )
        .unwrap();

    let exact = classify_session_report(
        &domain(),
        &report("spawn-a"),
        &source_attachment(),
        &claims,
        &targets,
    );
    assert!(matches!(
        exact.disposition,
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
    ));
    let wrong_operation = classify_session_report(
        &domain(),
        &report("spawn-b"),
        &source_attachment(),
        &claims,
        &targets,
    );
    assert!(!matches!(
        wrong_operation.disposition,
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
    ));
    assert_eq!(
        claims.claim_for_operation(&command()).unwrap().disposition,
        SpawnClaimDisposition::Active
    );
}

#[test]
fn quarantine_is_outer_only_and_cannot_terminalize_a_nested_result() {
    let mut commands = CommandIndex::new();
    commands
        .apply(&recorded(
            1,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation().encode_to_vec(),
            },
        ))
        .unwrap();
    commands
        .apply(&recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
        ))
        .unwrap();
    let nested = Observation {
        kind: ObservationKind::Result as i32,
        correlations: vec![TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(command())),
        }],
        target_scope: accepted_operation()
            .operation
            .and_then(|operation| operation.target_scope),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let quarantined = quarantined(nested);
    commands
        .apply(&recorded(
            3,
            encode_quarantined_runtime_evidence(&quarantined).unwrap(),
        ))
        .unwrap();
    assert_eq!(
        commands.get_command(&command()).unwrap().state,
        OperationState::Delivered
    );
    assert!(!commands.has_deferred_spawn_success(&command()));
}

#[test]
fn promotion_installs_the_staged_runtime_but_staging_alone_does_not() {
    let mut sessions = SessionRegistry::new(domain()).unwrap();
    sessions
        .logical_targets_mut()
        .create(
            LogicalTargetId {
                value: "logical-a".to_owned(),
            },
            AdapterId {
                value: "pi".to_owned(),
            },
            "machine-a".to_owned(),
        )
        .unwrap();
    sessions
        .observe(&recorded(5, encode_staged_successor(&staged())))
        .unwrap();
    assert!(sessions.sessions().next().is_none());
    assert!(sessions
        .logical_targets()
        .get(&LogicalTargetId {
            value: "logical-a".to_owned()
        })
        .unwrap()
        .current
        .is_none());

    let mut promotion = unstamped_promotion();
    promotion.promotion_event_id = Some(event_id(6));
    promotion.completion_audit_event_id = Some(event_id(7));
    promotion
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .audit_id = Some(event_id(7));
    sessions
        .observe(&recorded(
            6,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: promotion.encode_to_vec(),
            },
        ))
        .unwrap();
    let current = sessions.sessions().next().unwrap();
    assert_eq!(current.identity.session_generation.value, 1);
    assert_eq!(
        current.state.connectivity,
        SessionConnectivityState::Live as i32
    );
}

#[test]
fn promotion_event_directly_consumes_the_claim() {
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    let accepted = SpawnClaimEvent {
        authority_domain_id: Some(domain()),
        mutation: Some(spawn_claim_event::Mutation::Accepted(accepted_claim())),
    };
    claims
        .observe(&recorded(1, encode_spawn_claim_event(&accepted)))
        .unwrap();
    claims
        .observe(&recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
        ))
        .unwrap();
    claims
        .observe(&recorded(
            3,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Delivered, OperationState::Running)
                    .encode_to_vec(),
            },
        ))
        .unwrap();
    let result = Observation {
        kind: ObservationKind::Result as i32,
        correlations: vec![TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(command())),
        }],
        target_scope: accepted_operation()
            .operation
            .and_then(|operation| operation.target_scope),
        failure_code: FailureCode::Unspecified as i32,
        observed_at: Some(Timestamp {
            seconds: 10,
            nanos: 0,
        }),
        ..Observation::default()
    };
    claims
        .observe(&recorded(
            4,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: result.encode_to_vec(),
            },
        ))
        .unwrap();
    claims
        .observe(&recorded(5, encode_staged_successor(&staged())))
        .unwrap();
    let mut promotion = unstamped_promotion();
    promotion.promotion_event_id = Some(event_id(6));
    promotion.completion_audit_event_id = Some(event_id(7));
    promotion
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .audit_id = Some(event_id(7));
    claims
        .observe(&recorded(
            6,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: promotion.encode_to_vec(),
            },
        ))
        .unwrap();
    let record = claims.claim_for_operation(&command()).unwrap();
    assert_eq!(record.disposition, SpawnClaimDisposition::Promoted);
    assert!(record.pending_replacement.is_none());
}

#[tokio::test]
async fn quarantine_requires_and_supports_one_atomic_outer_audit_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let envelope = quarantined(Observation {
        kind: ObservationKind::Status as i32,
        ..Observation::default()
    });
    let source = encode_quarantined_runtime_evidence(&envelope).unwrap();
    assert!(matches!(
        storage.append(&domain(), source.clone()).await,
        Err(StorageError::UnsupportedOperation)
    ));
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::StaleEventIgnored,
    );
    audit.reason_code = "stale_event".to_owned();
    let committed = storage
        .append_audited(&domain(), source, audit)
        .await
        .unwrap();
    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id, committed.source_event_id);
    assert_eq!(
        events[0].payload.kind,
        StoredEventKind::QuarantinedRuntimeEvidence as i32
    );
    assert_eq!(events[1].event_id, committed.audit_event_id);
    assert_eq!(events[1].payload.kind, StoredEventKind::AuditRecord as i32);
}

#[tokio::test]
async fn storage_stamps_and_commits_complete_promotion_plus_audit_atomically() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    for _ in 0..5 {
        storage
            .append(
                &domain(),
                StoredEventPayload {
                    kind: StoredEventKind::Observation as i32,
                    payload: Vec::new(),
                },
            )
            .await
            .unwrap();
    }
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::CommandCompleted,
    );
    audit.command_id = Some(command());
    audit.reason_code = "spawn_completion".to_owned();
    let committed = storage
        .append_spawn_promotion_audited(&domain(), unstamped_promotion(), audit)
        .await
        .unwrap();
    assert_eq!(committed.source_event_id, event_id(6));
    assert_eq!(committed.audit_event_id, event_id(7));
    assert_eq!(committed.promotion.promotion_event_id, Some(event_id(6)));
    assert_eq!(
        committed.promotion.completion_audit_event_id,
        Some(event_id(7))
    );
    assert_eq!(
        committed
            .promotion
            .authority
            .as_ref()
            .unwrap()
            .descendant_grant
            .as_ref()
            .unwrap()
            .audit_id,
        Some(event_id(7))
    );
    let events = storage
        .read_after(&domain(), Lsn { value: 5 })
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].payload.kind,
        StoredEventKind::SpawnPromotionCommitted as i32
    );
    assert_eq!(events[1].payload.kind, StoredEventKind::AuditRecord as i32);

    let generic = storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: committed.promotion.encode_to_vec(),
            },
        )
        .await;
    assert!(matches!(generic, Err(StorageError::UnsupportedOperation)));
}
