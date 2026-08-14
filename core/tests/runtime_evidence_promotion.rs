use patchbay_contracts::patchbay::{
    quarantined_runtime_evidence, runtime_generation_disposition, spawn_claim_event,
    typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId, AdapterCapability, AdapterId,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AuditEventKind,
    AuthorityDomainId, CommandId, CommandTransition, ContinuationAuthorityProvenance,
    DescendantGrant, DescendantGrantProvenance, Elicitation, ElicitationId, ElicitationState,
    EndpointId, EventId, ExternalRuntimeRef, FailureCode, Generation, Grant, GrantId,
    GrantProvenance, GrantRevocationPolicy, IdempotencyKey, LogicalTargetCreated, LogicalTargetId,
    LogicalTargetInitialCurrentAssigned, Lsn, Observation, ObservationKind, Operation,
    OperationKind, OperationState, PayloadContentType, PayloadEnvelope, QuarantinedRuntimeEvidence,
    Revocation, RuntimeDeliveryAcknowledgementEvidence, RuntimeElicitationMutationEvidence,
    RuntimeEvidenceClassificationContext, RuntimeEvidenceQuarantineReason,
    RuntimeEvidenceSourceAttachment, RuntimeGenerationDisposition, RuntimeGenerationRef,
    RuntimeGenerationUnknown, RuntimeSessionId, RuntimeTranscriptStatusEvidence,
    SessionActivityState, SessionConnectivityState, SessionRegistered, SessionReport,
    SessionReportSourceCursor, SessionState, SpawnClaimAccepted, SpawnClaimDisposition,
    SpawnClaimEvent, SpawnGenerationClaim, SpawnPendingReplacementFence,
    SpawnPromotionAuthorityEvidence, SpawnPromotionCommitted, SpawnPromotionLifecycleEvidence,
    SpawnPromotionResultEvidence, SpawnPromotionStagedEvidence, SpawnSuccessorEvidenceStaged,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    acceptance::{CommandIndex, ElicitationSlotLayer},
    adapter::AdapterRegistry,
    authority::{AuthorityRegistry, DESCENDANT_GRANT_ALLOWED_KINDS},
    diagnostics::DiagnosticsProjection,
    resource::ResourceRegistry,
    session::{
        classify_session_report, encode_quarantined_runtime_evidence, encode_spawn_claim_event,
        encode_staged_successor, fold_spawn_promotion_ordered, SessionRegistry, SpawnClaimQuery,
        SpawnClaimRegistry,
    },
    storage::{
        AuditRecordDraft, AuditedStorage, RecordedEvent, RusqliteStorage, Storage, StorageError,
        TargetKey,
    },
    target::TargetRegistry,
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

fn quarantined(mut candidate: Observation) -> QuarantinedRuntimeEvidence {
    candidate.authority_domain_id = Some(domain());
    candidate.target_scope = Some(TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: external(1).adapter_id,
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: external(1).runtime_session_id,
        session_generation: Some(Generation { value: 1 }),
        ..TargetScope::default()
    });
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
            classified_target: Some(RuntimeGenerationRef {
                logical_target_id: None,
                external_runtime: Some(external(1)),
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
        accepted_claim_event_id: Some(event_id(5)),
        accepted_claim: Some(accepted_claim()),
        lifecycle: vec![
            SpawnPromotionLifecycleEvidence {
                event_id: Some(event_id(6)),
                transition: Some(transition(
                    OperationState::Accepted,
                    OperationState::Delivered,
                )),
            },
            SpawnPromotionLifecycleEvidence {
                event_id: Some(event_id(7)),
                transition: Some(transition(
                    OperationState::Delivered,
                    OperationState::Running,
                )),
            },
        ],
        successful_result: Some(SpawnPromotionResultEvidence {
            event_id: Some(event_id(8)),
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
            event_id: Some(event_id(9)),
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
                    value: "desc:authority-main:spawn-a".to_owned(),
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
                allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
                    .iter()
                    .map(|kind| *kind as i32)
                    .collect(),
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

fn attachment_event(lsn: u64) -> RecordedEvent {
    let adapter_id = AdapterId {
        value: "pi".to_owned(),
    };
    let endpoint_id = EndpointId {
        value: "pi-endpoint".to_owned(),
    };
    let registration = AdapterRegistration {
        adapter_id: Some(adapter_id.clone()),
        endpoint_id: Some(endpoint_id.clone()),
        authority_domain_id: Some(domain()),
        adapter_generation: Some(Generation { value: 3 }),
        capability: Some(AdapterCapability {
            session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
            target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    };
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: Observation {
                authority_domain_id: Some(domain()),
                sender: Some(ActorEndpointRef {
                    actor_id: Some(ActorId {
                        value: "pi".to_owned(),
                    }),
                    endpoint_id: Some(endpoint_id),
                    ..ActorEndpointRef::default()
                }),
                kind: ObservationKind::Event as i32,
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::Adapter as i32,
                    adapter_id: Some(adapter_id),
                    ..TargetScope::default()
                }),
                payload: Some(PayloadEnvelope {
                    payload: registration.encode_to_vec(),
                    content_type: PayloadContentType::Protobuf as i32,
                    schema_ref: "patchbay.AdapterRegistration".to_owned(),
                }),
                ..Observation::default()
            }
            .encode_to_vec(),
        },
    )
}

fn parent_grant() -> Grant {
    Grant {
        grant_id: Some(GrantId {
            value: "spawn-grant".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId {
            value: "operator".to_owned(),
        }),
        subject_endpoint_id: Some(EndpointId {
            value: "web".to_owned(),
        }),
        target_scope: accepted_operation()
            .operation
            .and_then(|operation| operation.target_scope),
        allowed_operation_kinds: vec![OperationKind::Spawn as i32],
        created_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        provenance: Some(GrantProvenance {
            reason: "spawn fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn successful_result() -> Observation {
    Observation {
        authority_domain_id: Some(domain()),
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
    }
}

fn valid_prefix() -> Vec<RecordedEvent> {
    vec![
        attachment_event(1),
        recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: parent_grant().encode_to_vec(),
            },
        ),
        recorded(
            3,
            patchbay_core::session::events::encode(
                &patchbay_core::session::events::logical_target_created(
                    domain(),
                    LogicalTargetCreated {
                        logical_target_id: Some(LogicalTargetId {
                            value: "logical-a".to_owned(),
                        }),
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        deployment_scope: "machine-a".to_owned(),
                    },
                ),
            ),
        ),
        recorded(
            4,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation().encode_to_vec(),
            },
        ),
        recorded(
            5,
            encode_spawn_claim_event(&SpawnClaimEvent {
                authority_domain_id: Some(domain()),
                mutation: Some(spawn_claim_event::Mutation::Accepted(accepted_claim())),
            }),
        ),
        recorded(
            6,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
        ),
        recorded(
            7,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Delivered, OperationState::Running)
                    .encode_to_vec(),
            },
        ),
        recorded(
            8,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: successful_result().encode_to_vec(),
            },
        ),
        recorded(9, encode_staged_successor(&staged())),
    ]
}

fn stamped_promotion(lsn: u64) -> SpawnPromotionCommitted {
    let mut promotion = unstamped_promotion();
    promotion.promotion_event_id = Some(event_id(lsn));
    promotion.completion_audit_event_id = Some(event_id(lsn + 1));
    promotion
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .audit_id = Some(event_id(lsn + 1));
    promotion
}

fn continuation_fixture() -> (Vec<RecordedEvent>, SpawnPromotionCommitted) {
    let prior = RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        external_runtime: Some(external(1)),
    };
    let continuation = ContinuationAuthorityProvenance {
        exact_prior: Some(prior.clone()),
        replacement_grant_id: Some(GrantId {
            value: "replacement-grant".to_owned(),
        }),
        replacement_authority_kind: OperationKind::SessionManagement as i32,
    };
    let continuation_claim = SpawnGenerationClaim {
        expected_prior: Some(prior.clone()),
        claimed_generation: Some(Generation { value: 2 }),
        ..claim()
    };
    let continuation_accepted = SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation()),
        claim: Some(continuation_claim.clone()),
        compound_authority: Some(continuation.clone()),
        pending_replacement: Some(SpawnPendingReplacementFence {
            exact_prior: Some(prior.clone()),
            failure_code: FailureCode::Superseded as i32,
            reason_code: "replacement_pending".to_owned(),
        }),
        prior_work_effects: vec![],
    };
    let mut candidate_report = report("spawn-a");
    candidate_report.session_generation = Some(Generation { value: 2 });
    let candidate = RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        external_runtime: Some(external(2)),
    };
    let continuation_staged = SpawnSuccessorEvidenceStaged {
        authority_domain_id: Some(domain()),
        exact_claim: Some(continuation_claim.clone()),
        report: Some(candidate_report.clone()),
        classified_target: Some(candidate.clone()),
        disposition: Some(RuntimeGenerationDisposition {
            disposition: Some(
                runtime_generation_disposition::Disposition::ClaimedSuccessor(
                    patchbay_contracts::patchbay::RuntimeGenerationClaimedSuccessor {
                        claim_operation_id: Some(command()),
                        expected_prior: Some(prior.clone()),
                        claimed_generation: Some(Generation { value: 2 }),
                    },
                ),
            ),
        }),
        source_attachment: Some(source_attachment()),
        external_runtime_reservation: Some(external(2)),
    };
    let replacement_grant = Grant {
        grant_id: Some(GrantId {
            value: "replacement-grant".to_owned(),
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
        allowed_operation_kinds: vec![OperationKind::SessionManagement as i32],
        created_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        provenance: Some(GrantProvenance {
            reason: "replacement fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    let prefix = vec![
        attachment_event(1),
        recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: parent_grant().encode_to_vec(),
            },
        ),
        recorded(
            3,
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: replacement_grant.encode_to_vec(),
            },
        ),
        recorded(
            4,
            patchbay_core::session::events::encode(
                &patchbay_core::session::events::logical_target_created(
                    domain(),
                    LogicalTargetCreated {
                        logical_target_id: Some(LogicalTargetId {
                            value: "logical-a".to_owned(),
                        }),
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        deployment_scope: "machine-a".to_owned(),
                    },
                ),
            ),
        ),
        recorded(
            5,
            patchbay_core::session::events::encode(
                &patchbay_core::session::events::logical_target_initial_current_assigned(
                    domain(),
                    LogicalTargetInitialCurrentAssigned {
                        logical_target_id: Some(LogicalTargetId {
                            value: "logical-a".to_owned(),
                        }),
                        external_runtime_ref: Some(external(1)),
                    },
                ),
            ),
        ),
        recorded(
            6,
            patchbay_core::session::events::encode(&patchbay_core::session::events::registered(
                domain(),
                SessionRegistered {
                    adapter_id: Some(AdapterId {
                        value: "pi".to_owned(),
                    }),
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: Some(RuntimeSessionId {
                        value: "runtime-a".to_owned(),
                    }),
                    session_generation: Some(Generation { value: 1 }),
                    initial_state: Some(SessionState {
                        connectivity: SessionConnectivityState::Offline as i32,
                        activity: SessionActivityState::Unknown as i32,
                    }),
                    source_cursor: Some(SessionReportSourceCursor {
                        adapter_generation: Some(Generation { value: 3 }),
                        revision: 1,
                    }),
                    ..SessionRegistered::default()
                },
            )),
        ),
        recorded(
            7,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation().encode_to_vec(),
            },
        ),
        recorded(
            8,
            encode_spawn_claim_event(&SpawnClaimEvent {
                authority_domain_id: Some(domain()),
                mutation: Some(spawn_claim_event::Mutation::Accepted(
                    continuation_accepted.clone(),
                )),
            }),
        ),
        recorded(
            9,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
        ),
        recorded(
            10,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Delivered, OperationState::Running)
                    .encode_to_vec(),
            },
        ),
        recorded(
            11,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: successful_result().encode_to_vec(),
            },
        ),
        recorded(
            12,
            StoredEventPayload {
                kind: StoredEventKind::SpawnSuccessorEvidenceStaged as i32,
                payload: continuation_staged.encode_to_vec(),
            },
        ),
    ];
    let mut promotion = unstamped_promotion();
    promotion.accepted_claim_event_id = Some(event_id(8));
    promotion.accepted_claim = Some(continuation_accepted);
    promotion.lifecycle[0].event_id = Some(event_id(9));
    promotion.lifecycle[1].event_id = Some(event_id(10));
    promotion.successful_result.as_mut().unwrap().event_id = Some(event_id(11));
    promotion.staged_successor = Some(SpawnPromotionStagedEvidence {
        event_id: Some(event_id(12)),
        staged: Some(continuation_staged),
    });
    promotion.promoted_runtime = Some(candidate);
    promotion.external_runtime_reservation = Some(external(2));
    let authority = promotion.authority.as_mut().unwrap();
    authority.continuation_authority = Some(continuation.clone());
    let descendant = authority.descendant_grant.as_mut().unwrap();
    descendant.target_scope.as_mut().unwrap().session_generation = Some(Generation { value: 2 });
    descendant
        .provenance
        .as_mut()
        .unwrap()
        .continuation_authority = Some(continuation);
    promotion.promotion_event_id = Some(event_id(13));
    promotion.completion_audit_event_id = Some(event_id(14));
    descendant.audit_id = Some(event_id(14));
    (prefix, promotion)
}

fn aggregate_from_prefix(
    prefix: &[RecordedEvent],
) -> (
    AuthorityRegistry,
    TargetRegistry,
    SpawnClaimRegistry,
    CommandIndex,
) {
    let mut authority = AuthorityRegistry::new();
    let mut targets = TargetRegistry::with_adapters(
        SessionRegistry::new(domain()).unwrap(),
        ResourceRegistry::new(),
        AdapterRegistry::new(),
    );
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    let mut commands = CommandIndex::new();
    for event in prefix {
        authority.observe(event).unwrap();
        targets.observe_event(event).unwrap();
        claims.observe(event).unwrap();
        commands.apply(event).unwrap();
    }
    (authority, targets, claims, commands)
}

#[test]
fn classifier_admits_only_the_exact_active_claimed_successor() {
    let prefix = valid_prefix();
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    for event in prefix.iter().take(5) {
        claims.observe(event).unwrap();
    }
    let mut adapters = AdapterRegistry::new();
    adapters.observe(&prefix[0]).unwrap();
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

    let exact = classify_session_report(
        &domain(),
        &report("spawn-a"),
        &source_attachment(),
        &adapters,
        &claims,
        &sessions,
    );
    assert!(matches!(
        exact.disposition,
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
    ));
    let mut omitted_origin = report("spawn-a");
    omitted_origin.spawn_origin = None;
    for bypass in [report("spawn-b"), omitted_origin] {
        let rejected = classify_session_report(
            &domain(),
            &bypass,
            &source_attachment(),
            &adapters,
            &claims,
            &sessions,
        );
        assert!(matches!(
            rejected.disposition,
            Some(runtime_generation_disposition::Disposition::IdentityMismatch(_))
        ));
    }
    assert_eq!(
        claims.claim_for_operation(&command()).unwrap().disposition,
        SpawnClaimDisposition::Active
    );
}

#[test]
fn classifier_kills_each_attachment_claim_prior_deployment_and_generation_mutation() {
    let prefix = valid_prefix();
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    for event in prefix.iter().take(5) {
        claims.observe(event).unwrap();
    }
    let mut adapters = AdapterRegistry::new();
    adapters.observe(&prefix[0]).unwrap();
    let base_sessions = {
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
    };
    let is_claimed = |report: &SessionReport,
                      source: &RuntimeEvidenceSourceAttachment,
                      sessions: &SessionRegistry| {
        matches!(
            classify_session_report(&domain(), report, source, &adapters, &claims, sessions)
                .disposition,
            Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
        )
    };
    assert!(is_claimed(
        &report("spawn-a"),
        &source_attachment(),
        &base_sessions
    ));

    let mut wrong_attachment = source_attachment();
    wrong_attachment.attachment_event_id = Some(event_id(2));
    assert!(!is_claimed(
        &report("spawn-a"),
        &wrong_attachment,
        &base_sessions
    ));
    let mut wrong_adapter_generation = source_attachment();
    wrong_adapter_generation.adapter_generation = Some(Generation { value: 2 });
    assert!(!is_claimed(
        &report("spawn-a"),
        &wrong_adapter_generation,
        &base_sessions
    ));
    assert!(!is_claimed(
        &report("spawn-other"),
        &source_attachment(),
        &base_sessions
    ));
    let mut wrong_deployment = report("spawn-a");
    wrong_deployment.deployment_scope = "machine-b".to_owned();
    assert!(!is_claimed(
        &wrong_deployment,
        &source_attachment(),
        &base_sessions
    ));
    let mut wrong_generation = report("spawn-a");
    wrong_generation.session_generation = Some(Generation { value: 2 });
    assert!(!is_claimed(
        &wrong_generation,
        &source_attachment(),
        &base_sessions
    ));
    let mut wrong_prior = base_sessions.clone();
    wrong_prior
        .logical_targets_mut()
        .assign_initial_current(
            &LogicalTargetId {
                value: "logical-a".to_owned(),
            },
            external(1),
        )
        .unwrap();
    assert!(!is_claimed(
        &report("spawn-a"),
        &source_attachment(),
        &wrong_prior
    ));

    let mut ordinary_current_report = report("spawn-a");
    ordinary_current_report.spawn_origin = None;
    let current = classify_session_report(
        &domain(),
        &ordinary_current_report,
        &source_attachment(),
        &adapters,
        &claims,
        &wrong_prior,
    );
    assert!(matches!(
        current.disposition,
        Some(runtime_generation_disposition::Disposition::Current(_))
    ));
    let unauthenticated_current = classify_session_report(
        &domain(),
        &ordinary_current_report,
        &wrong_attachment,
        &adapters,
        &claims,
        &wrong_prior,
    );
    assert!(!matches!(
        unauthenticated_current.disposition,
        Some(runtime_generation_disposition::Disposition::Current(_))
    ));
}

#[test]
fn every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds() {
    let runtime_scope = TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: external(1).adapter_id,
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: external(1).runtime_session_id,
        session_generation: Some(Generation { value: 1 }),
        ..TargetScope::default()
    };
    let nested_observation = Observation {
        authority_domain_id: Some(domain()),
        kind: ObservationKind::Result as i32,
        correlations: vec![TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(command())),
        }],
        target_scope: Some(runtime_scope.clone()),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let runtime_target = RuntimeGenerationRef {
        logical_target_id: None,
        external_runtime: Some(external(1)),
    };
    let mut nested_report = report("spawn-a");
    nested_report.spawn_origin = None;
    nested_report.source_cursor.as_mut().unwrap().revision = 2;
    let nested_elicitation = Elicitation {
        elicitation_id: Some(ElicitationId {
            value: "pending-runtime-elicitation".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        target_context: Some(runtime_scope.clone()),
        state: ElicitationState::Pending as i32,
        ..Elicitation::default()
    };
    let candidates = vec![
        (
            "observation",
            quarantined_runtime_evidence::Candidate::Observation(nested_observation.clone()),
        ),
        (
            "session-report",
            quarantined_runtime_evidence::Candidate::SessionReport(nested_report),
        ),
        (
            "delivery-acknowledgement",
            quarantined_runtime_evidence::Candidate::DeliveryAcknowledgement(
                RuntimeDeliveryAcknowledgementEvidence {
                    command_id: Some(command()),
                    target: Some(runtime_target.clone()),
                    observed_at: Some(Timestamp {
                        seconds: 10,
                        nanos: 0,
                    }),
                },
            ),
        ),
        (
            "transcript-status",
            quarantined_runtime_evidence::Candidate::TranscriptStatus(
                RuntimeTranscriptStatusEvidence {
                    observation: Some(nested_observation.clone()),
                },
            ),
        ),
        (
            "elicitation-mutation",
            quarantined_runtime_evidence::Candidate::ElicitationMutation(
                RuntimeElicitationMutationEvidence {
                    elicitation: Some(nested_elicitation.clone()),
                    from_state: ElicitationState::Pending as i32,
                    to_state: ElicitationState::Stale as i32,
                },
            ),
        ),
    ];

    for (family, candidate) in candidates {
        let mut envelope = quarantined(nested_observation.clone());
        envelope.candidate = Some(candidate);
        let event = recorded(3, encode_quarantined_runtime_evidence(&envelope).unwrap());

        // Every admitted family receives independently useful pre-state. A
        // recursive-dispatch mutant therefore has a visible effect instead of
        // being allowed to redispatch into an empty projection.
        let mut nested_accepted = accepted_operation();
        nested_accepted.operation.as_mut().unwrap().target_scope = Some(runtime_scope.clone());
        let accepted_event = recorded(
            1,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: nested_accepted.encode_to_vec(),
            },
        );
        let delivered_event = recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: transition(OperationState::Accepted, OperationState::Delivered)
                    .encode_to_vec(),
            },
        );
        let filler_event = recorded(
            2,
            StoredEventPayload {
                kind: StoredEventKind::AuditRecord as i32,
                payload: vec![],
            },
        );

        let mut authority = AuthorityRegistry::new();
        authority
            .observe(&recorded(
                1,
                StoredEventPayload {
                    kind: StoredEventKind::Grant as i32,
                    payload: parent_grant().encode_to_vec(),
                },
            ))
            .unwrap();
        let mut sessions = SessionRegistry::new(domain()).unwrap();
        sessions
            .observe(&recorded(
                1,
                patchbay_core::session::events::encode(
                    &patchbay_core::session::events::registered(
                        domain(),
                        SessionRegistered {
                            adapter_id: external(1).adapter_id,
                            deployment_scope: "machine-a".to_owned(),
                            runtime_session_id: external(1).runtime_session_id,
                            session_generation: Some(Generation { value: 1 }),
                            initial_state: Some(SessionState {
                                connectivity: SessionConnectivityState::Offline as i32,
                                activity: SessionActivityState::Unknown as i32,
                            }),
                            source_cursor: Some(SessionReportSourceCursor {
                                adapter_generation: Some(Generation { value: 3 }),
                                revision: 1,
                            }),
                            ..SessionRegistered::default()
                        },
                    ),
                ),
            ))
            .unwrap();
        let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
        claims.observe(&accepted_event).unwrap();
        claims.observe(&filler_event).unwrap();
        let mut commands = CommandIndex::new();
        commands.apply(&accepted_event).unwrap();
        if matches!(family, "observation" | "transcript-status") {
            commands.apply(&delivered_event).unwrap();
        }
        let mut elicitations = ElicitationSlotLayer::new();
        elicitations
            .observe(&recorded(
                1,
                StoredEventPayload {
                    kind: StoredEventKind::Elicitation as i32,
                    payload: nested_elicitation.encode_to_vec(),
                },
            ))
            .unwrap();
        let mut diagnostics = DiagnosticsProjection::new(domain()).unwrap();
        diagnostics.observe(&accepted_event).unwrap();
        if matches!(family, "observation" | "transcript-status") {
            diagnostics.observe(&delivered_event).unwrap();
        }
        let mut adapters = AdapterRegistry::new();
        adapters.observe(&attachment_event(1)).unwrap();
        let before = (
            authority.clone(),
            sessions.clone(),
            commands.clone(),
            elicitations.clone(),
            diagnostics.clone(),
            adapters.clone(),
        );
        for _ in 0..2 {
            authority.observe(&event).unwrap();
            sessions.observe(&event).unwrap();
            claims.observe(&event).unwrap();
            commands.apply(&event).unwrap();
            elicitations.observe(&event).unwrap();
            diagnostics.observe(&event).unwrap();
            adapters.observe(&event).unwrap();
        }
        assert_eq!(
            (
                authority,
                sessions,
                commands,
                elicitations,
                diagnostics,
                adapters
            ),
            before,
            "nested {family} must remain inert across hot fold and replay"
        );
        assert!(claims.claim_for_operation(&command()).is_none());
    }
}

#[test]
fn promotion_installs_the_staged_runtime_but_staging_alone_does_not() {
    let prefix = valid_prefix();
    let mut sessions = SessionRegistry::new(domain()).unwrap();
    for event in &prefix {
        sessions.observe(event).unwrap();
    }
    assert!(sessions.sessions().next().is_none());
    assert!(sessions
        .logical_targets()
        .get(&LogicalTargetId {
            value: "logical-a".to_owned()
        })
        .unwrap()
        .current
        .is_none());

    let promotion = stamped_promotion(10);
    sessions
        .observe(&recorded(
            10,
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
    let prefix = valid_prefix();
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    for event in &prefix {
        claims.observe(event).unwrap();
    }
    let promotion = stamped_promotion(10);
    claims
        .observe(&recorded(
            10,
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

#[test]
fn aggregate_promotion_fold_requires_and_publishes_all_four_views_in_order() {
    let prefix = valid_prefix();
    let mut authority = AuthorityRegistry::new();
    let mut targets = TargetRegistry::with_adapters(
        SessionRegistry::new(domain()).unwrap(),
        ResourceRegistry::new(),
        AdapterRegistry::new(),
    );
    let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
    let mut commands = CommandIndex::new();
    for event in &prefix {
        authority.observe(event).unwrap();
        targets.observe_event(event).unwrap();
        claims.observe(event).unwrap();
        commands.apply(event).unwrap();
    }
    let promotion = recorded(
        10,
        StoredEventPayload {
            kind: StoredEventKind::SpawnPromotionCommitted as i32,
            payload: stamped_promotion(10).encode_to_vec(),
        },
    );

    // Mutation (a)/(d): omitting the authority prefix cannot publish N+1,
    // consume the claim, or complete the command.
    let mut missing_authority = AuthorityRegistry::new();
    let targets_before = targets.clone();
    let claims_before = claims.clone();
    let commands_before = commands.clone();
    assert!(fold_spawn_promotion_ordered(
        &mut missing_authority,
        &mut targets,
        &mut claims,
        &mut commands,
        &promotion,
    )
    .is_err());
    assert_eq!(targets, targets_before);
    assert_eq!(claims, claims_before);
    assert_eq!(commands, commands_before);

    fold_spawn_promotion_ordered(
        &mut authority,
        &mut targets,
        &mut claims,
        &mut commands,
        &promotion,
    )
    .unwrap();
    assert!(authority
        .get_grant(&GrantId {
            value: "desc:authority-main:spawn-a".to_owned()
        })
        .is_some());
    assert_eq!(targets.sessions().sessions().count(), 1);
    assert_eq!(
        claims.claim_for_operation(&command()).unwrap().disposition,
        SpawnClaimDisposition::Promoted
    );
    assert_eq!(
        commands.get_command(&command()).unwrap().state,
        OperationState::Completed
    );
}

#[test]
fn promotion_rejects_one_dimension_at_a_time_authority_laundering() {
    let base = stamped_promotion(10);
    let mut mutations = Vec::new();

    let mut wrong_actor = base.clone();
    wrong_actor
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .subject_actor_id = Some(ActorId {
        value: "other-actor".to_owned(),
    });
    mutations.push(wrong_actor);

    let mut wrong_endpoint = base.clone();
    wrong_endpoint
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .subject_endpoint_id = Some(EndpointId {
        value: "other-endpoint".to_owned(),
    });
    mutations.push(wrong_endpoint);

    let mut wrong_target = base.clone();
    wrong_target
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .target_scope
        .as_mut()
        .unwrap()
        .runtime_session_id = Some(RuntimeSessionId {
        value: "other-runtime".to_owned(),
    });
    mutations.push(wrong_target);

    let mut wrong_kinds = base.clone();
    wrong_kinds
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .allowed_operation_kinds
        .pop();
    mutations.push(wrong_kinds);

    let mut wrong_id = base.clone();
    wrong_id
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .grant_id = Some(GrantId {
        value: "descendant".to_owned(),
    });
    mutations.push(wrong_id);

    let mut wrong_time = base.clone();
    wrong_time
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .created_at = Some(Timestamp {
        seconds: 11,
        nanos: 0,
    });
    mutations.push(wrong_time);

    let mut wrong_operation = base.clone();
    wrong_operation
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .provenance
        .as_mut()
        .unwrap()
        .spawn_operation_id = Some(CommandId {
        value: "other-operation".to_owned(),
    });
    mutations.push(wrong_operation);

    let mut wrong_parent = base;
    wrong_parent
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .provenance
        .as_mut()
        .unwrap()
        .spawning_grant_id = Some(GrantId {
        value: "other-parent".to_owned(),
    });
    mutations.push(wrong_parent);

    for mutation in mutations {
        let mut authority = AuthorityRegistry::new();
        let mut targets = TargetRegistry::with_adapters(
            SessionRegistry::new(domain()).unwrap(),
            ResourceRegistry::new(),
            AdapterRegistry::new(),
        );
        let mut claims = SpawnClaimRegistry::new(domain()).unwrap();
        let mut commands = CommandIndex::new();
        for event in valid_prefix() {
            authority.observe(&event).unwrap();
            targets.observe_event(&event).unwrap();
            claims.observe(&event).unwrap();
            commands.apply(&event).unwrap();
        }
        let before = (
            authority.clone(),
            targets.clone(),
            claims.clone(),
            commands.clone(),
        );
        let event = recorded(
            10,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: mutation.encode_to_vec(),
            },
        );
        assert!(fold_spawn_promotion_ordered(
            &mut authority,
            &mut targets,
            &mut claims,
            &mut commands,
            &event,
        )
        .is_err());
        assert_eq!((authority, targets, claims, commands), before);
    }
}

#[test]
fn promotion_revalidates_parent_grant_kind_scope_and_liveness() {
    let mut variants = Vec::new();
    let mut wrong_kind = parent_grant();
    wrong_kind.allowed_operation_kinds = vec![OperationKind::Instruct as i32];
    variants.push(wrong_kind);
    let mut wrong_scope = parent_grant();
    wrong_scope.target_scope.as_mut().unwrap().adapter_id = Some(AdapterId {
        value: "other-adapter".to_owned(),
    });
    variants.push(wrong_scope);
    let mut expired = parent_grant();
    expired.expires_at = Some(Timestamp {
        seconds: 5,
        nanos: 0,
    });
    variants.push(expired);

    for parent in variants {
        let mut prefix = valid_prefix();
        prefix[1].payload.payload = parent.encode_to_vec();
        let (mut authority, mut targets, mut claims, mut commands) = aggregate_from_prefix(&prefix);
        assert!(fold_spawn_promotion_ordered(
            &mut authority,
            &mut targets,
            &mut claims,
            &mut commands,
            &recorded(
                10,
                StoredEventPayload {
                    kind: StoredEventKind::SpawnPromotionCommitted as i32,
                    payload: stamped_promotion(10).encode_to_vec(),
                },
            ),
        )
        .is_err());
    }
}

#[test]
fn continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion() {
    let (prefix, promotion) = continuation_fixture();
    let (mut authority, mut targets, mut claims, mut commands) = aggregate_from_prefix(&prefix);
    fold_spawn_promotion_ordered(
        &mut authority,
        &mut targets,
        &mut claims,
        &mut commands,
        &recorded(
            13,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: promotion.encode_to_vec(),
            },
        ),
    )
    .unwrap();
    let sessions = targets.sessions();
    assert!(sessions
        .get_tombstone(
            &AdapterId {
                value: "pi".to_owned(),
            },
            "machine-a",
            &RuntimeSessionId {
                value: "runtime-a".to_owned(),
            },
            &Generation { value: 1 },
        )
        .is_some());
    assert_eq!(
        sessions
            .sessions()
            .next()
            .unwrap()
            .identity
            .session_generation
            .value,
        2
    );

    let (mut expired_prefix, expired_promotion) = continuation_fixture();
    let mut expired_replacement =
        Grant::decode(expired_prefix[2].payload.payload.as_slice()).unwrap();
    expired_replacement.expires_at = Some(Timestamp {
        seconds: 5,
        nanos: 0,
    });
    expired_prefix[2].payload.payload = expired_replacement.encode_to_vec();
    let (mut authority, mut targets, mut claims, mut commands) =
        aggregate_from_prefix(&expired_prefix);
    assert!(fold_spawn_promotion_ordered(
        &mut authority,
        &mut targets,
        &mut claims,
        &mut commands,
        &recorded(
            13,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: expired_promotion.encode_to_vec(),
            },
        ),
    )
    .is_err());
    assert!(targets
        .sessions()
        .sessions()
        .next()
        .is_some_and(|session| { session.identity.session_generation.value == 1 }));

    let (mut revoked_prefix, mut revoked_promotion) = continuation_fixture();
    revoked_prefix.push(recorded(
        13,
        StoredEventPayload {
            kind: StoredEventKind::Revocation as i32,
            payload: Revocation {
                authority_domain_id: Some(domain()),
                grant_id: Some(GrantId {
                    value: "replacement-grant".to_owned(),
                }),
                revoked_at: Some(Timestamp {
                    seconds: 9,
                    nanos: 0,
                }),
                revocation_generation: Some(Generation { value: 1 }),
                accepted_operation_policy: GrantRevocationPolicy::Continue as i32,
                reason: "promotion-time revocation".to_owned(),
                ..Revocation::default()
            }
            .encode_to_vec(),
        },
    ));
    revoked_promotion.promotion_event_id = Some(event_id(14));
    revoked_promotion.completion_audit_event_id = Some(event_id(15));
    revoked_promotion
        .authority
        .as_mut()
        .unwrap()
        .descendant_grant
        .as_mut()
        .unwrap()
        .audit_id = Some(event_id(15));
    let (mut authority, mut targets, mut claims, mut commands) =
        aggregate_from_prefix(&revoked_prefix);
    assert!(fold_spawn_promotion_ordered(
        &mut authority,
        &mut targets,
        &mut claims,
        &mut commands,
        &recorded(
            14,
            StoredEventPayload {
                kind: StoredEventKind::SpawnPromotionCommitted as i32,
                payload: revoked_promotion.encode_to_vec(),
            },
        ),
    )
    .is_err());
}

#[tokio::test]
async fn quarantine_requires_and_supports_one_atomic_outer_audit_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(&domain(), attachment_event(1).payload)
        .await
        .unwrap();
    let envelope = quarantined(Observation {
        kind: ObservationKind::Status as i32,
        ..Observation::default()
    });
    let source = encode_quarantined_runtime_evidence(&envelope).unwrap();
    assert!(matches!(
        storage.append(&domain(), source.clone()).await,
        Err(StorageError::UnsupportedOperation)
    ));
    let mut generic_audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::StaleEventIgnored,
    );
    generic_audit.failure_code = Some(FailureCode::StaleEvent);
    generic_audit.reason_code = "runtime_evidence_unknown_target".to_owned();
    generic_audit.target_scope =
        envelope
            .candidate
            .as_ref()
            .and_then(|candidate| match candidate {
                quarantined_runtime_evidence::Candidate::Observation(observation) => {
                    observation.target_scope.clone()
                }
                _ => None,
            });
    assert!(matches!(
        storage
            .append_audited(&domain(), source, generic_audit.clone())
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    let committed = storage
        .append_quarantined_runtime_evidence_audited(&domain(), envelope, generic_audit)
        .await
        .unwrap();
    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].event_id, committed.source_event_id);
    assert_eq!(
        events[1].payload.kind,
        StoredEventKind::QuarantinedRuntimeEvidence as i32
    );
    assert_eq!(events[2].event_id, committed.audit_event_id);
    assert_eq!(events[2].payload.kind, StoredEventKind::AuditRecord as i32);
}

#[tokio::test]
async fn quarantine_rejects_non_durable_attachment_and_semantic_mismatch_without_writes() {
    for mutate in [0_u8, 1_u8] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        storage
            .append(&domain(), attachment_event(1).payload)
            .await
            .unwrap();
        let mut envelope = quarantined(Observation {
            kind: ObservationKind::Status as i32,
            ..Observation::default()
        });
        if mutate == 0 {
            envelope
                .source_attachment
                .as_mut()
                .unwrap()
                .attachment_event_id = Some(event_id(99));
        } else {
            envelope.reason = RuntimeEvidenceQuarantineReason::Tombstoned as i32;
        }
        let reason = RuntimeEvidenceQuarantineReason::try_from(envelope.reason).unwrap();
        let mut audit = AuditRecordDraft::new(
            Timestamp {
                seconds: 10,
                nanos: 0,
            },
            AuditEventKind::StaleEventIgnored,
        );
        audit.failure_code = Some(FailureCode::StaleEvent);
        audit.reason_code = patchbay_core::session::quarantine_reason_code(reason).to_owned();
        audit.target_scope = envelope
            .candidate
            .as_ref()
            .and_then(|candidate| match candidate {
                quarantined_runtime_evidence::Candidate::Observation(observation) => {
                    observation.target_scope.clone()
                }
                _ => None,
            });
        assert!(storage
            .append_quarantined_runtime_evidence_audited(&domain(), envelope, audit)
            .await
            .is_err());
        assert_eq!(
            storage
                .read_after(&domain(), Lsn { value: 0 })
                .await
                .unwrap()
                .len(),
            1
        );
    }
}

#[tokio::test]
async fn quarantine_rejects_each_forged_classification_context_field() {
    let base = quarantined(Observation {
        kind: ObservationKind::Status as i32,
        ..Observation::default()
    });
    let mut mutations = Vec::new();

    let mut fake_owner = base.clone();
    fake_owner
        .classification
        .as_mut()
        .unwrap()
        .classified_target
        .as_mut()
        .unwrap()
        .logical_target_id = Some(LogicalTargetId {
        value: "invented-owner".to_owned(),
    });
    mutations.push(("fake logical owner", fake_owner));

    let mut wrong_current = base.clone();
    wrong_current.classification.as_mut().unwrap().current = Some(RuntimeGenerationRef {
        logical_target_id: None,
        external_runtime: Some(external(1)),
    });
    mutations.push(("wrong current", wrong_current));

    let mut fake_claim = base.clone();
    fake_claim.classification.as_mut().unwrap().active_claim = Some(claim());
    mutations.push(("fake active claim", fake_claim));

    let mut mismatched_claim = base.clone();
    let mut wrong_claim = claim();
    wrong_claim.claimed_generation = Some(Generation { value: 2 });
    mismatched_claim
        .classification
        .as_mut()
        .unwrap()
        .active_claim = Some(wrong_claim);
    mutations.push(("mismatched active claim", mismatched_claim));

    let mut wrong_tombstone = base;
    wrong_tombstone.classification.as_mut().unwrap().tombstone =
        Some(patchbay_contracts::patchbay::LogicalTargetTombstone {
            external_runtime_ref: Some(external(1)),
            superseded_at_lsn: Some(Lsn { value: 99 }),
        });
    mutations.push(("wrong tombstone", wrong_tombstone));

    for (name, envelope) in mutations {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        storage
            .append(&domain(), attachment_event(1).payload)
            .await
            .unwrap();
        let mut audit = AuditRecordDraft::new(
            Timestamp {
                seconds: 10,
                nanos: 0,
            },
            AuditEventKind::StaleEventIgnored,
        );
        audit.failure_code = Some(FailureCode::StaleEvent);
        audit.reason_code = "runtime_evidence_unknown_target".to_owned();
        audit.target_scope = envelope
            .candidate
            .as_ref()
            .and_then(|candidate| match candidate {
                quarantined_runtime_evidence::Candidate::Observation(observation) => {
                    observation.target_scope.clone()
                }
                _ => None,
            });
        assert!(
            storage
                .append_quarantined_runtime_evidence_audited(&domain(), envelope, audit)
                .await
                .is_err(),
            "{name} must be rejected against the rebuilt durable context"
        );
        assert_eq!(
            storage
                .read_after(&domain(), Lsn { value: 0 })
                .await
                .unwrap()
                .len(),
            1,
            "{name} rejection must be atomic"
        );
    }
}

#[tokio::test]
async fn malformed_quarantine_wire_is_rejected_on_every_generic_storage_route() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let malformed = StoredEventPayload {
        kind: StoredEventKind::QuarantinedRuntimeEvidence as i32,
        payload: vec![0xff],
    };
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::StaleEventIgnored,
    );
    audit.failure_code = Some(FailureCode::StaleEvent);
    audit.reason_code = "runtime_evidence_unknown_target".to_owned();
    assert!(matches!(
        storage.append(&domain(), malformed.clone()).await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_audited(&domain(), malformed.clone(), audit.clone())
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_batch_audited(&domain(), vec![malformed.clone()], audit.clone())
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_decision_audited_many(&domain(), malformed.clone(), vec![audit.clone()])
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_dedup(
                &domain(),
                &IdempotencyKey {
                    value: "malformed-quarantine".to_owned(),
                },
                &TargetKey::new("runtime".to_owned()).unwrap(),
                malformed,
            )
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn promotion_rejects_result_before_delivered_or_later_running_evidence() {
    for result_position in ["before-delivery", "before-running"] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let mut prefix = valid_prefix();
        prefix.truncate(5);
        let ordered_tail = if result_position == "before-delivery" {
            vec![
                recorded(
                    6,
                    StoredEventPayload {
                        kind: StoredEventKind::Observation as i32,
                        payload: successful_result().encode_to_vec(),
                    },
                ),
                recorded(
                    7,
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: transition(OperationState::Accepted, OperationState::Delivered)
                            .encode_to_vec(),
                    },
                ),
                recorded(
                    8,
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: transition(OperationState::Delivered, OperationState::Running)
                            .encode_to_vec(),
                    },
                ),
            ]
        } else {
            vec![
                recorded(
                    6,
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: transition(OperationState::Accepted, OperationState::Delivered)
                            .encode_to_vec(),
                    },
                ),
                recorded(
                    7,
                    StoredEventPayload {
                        kind: StoredEventKind::Observation as i32,
                        payload: successful_result().encode_to_vec(),
                    },
                ),
                recorded(
                    8,
                    StoredEventPayload {
                        kind: StoredEventKind::CommandTransition as i32,
                        payload: transition(OperationState::Delivered, OperationState::Running)
                            .encode_to_vec(),
                    },
                ),
            ]
        };
        prefix.extend(ordered_tail);
        prefix.push(recorded(9, encode_staged_successor(&staged())));
        for event in &prefix {
            storage
                .append(&domain(), event.payload.clone())
                .await
                .unwrap();
        }

        let mut promotion = unstamped_promotion();
        promotion.lifecycle[0].event_id = Some(event_id(if result_position == "before-delivery" {
            7
        } else {
            6
        }));
        promotion.lifecycle[1].event_id = Some(event_id(8));
        promotion.successful_result.as_mut().unwrap().event_id =
            Some(event_id(if result_position == "before-delivery" {
                6
            } else {
                7
            }));
        let mut audit = AuditRecordDraft::new(
            Timestamp {
                seconds: 10,
                nanos: 0,
            },
            AuditEventKind::CommandCompleted,
        );
        audit.command_id = Some(command());
        audit.reason_code = "spawn_completion".to_owned();
        assert!(
            storage
                .append_spawn_promotion_audited(&domain(), promotion, audit)
                .await
                .is_err(),
            "{result_position} must not become authority-bearing promotion evidence"
        );
        assert_eq!(
            storage
                .read_after(&domain(), Lsn { value: 0 })
                .await
                .unwrap()
                .len(),
            9,
            "rejected promotion must not append source or audit"
        );
    }
}

#[tokio::test]
async fn promotion_audit_failure_rolls_back_source_and_grant_identity_reservation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("promotion-rollback.sqlite");
    let path_string = path.to_string_lossy().into_owned();
    let storage = RusqliteStorage::open(&path_string).unwrap();
    for event in valid_prefix() {
        storage.append(&domain(), event.payload).await.unwrap();
    }
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(&format!(
            "CREATE TRIGGER fail_promotion_audit BEFORE INSERT ON events \
             WHEN NEW.kind = {} BEGIN SELECT RAISE(ABORT, 'injected crash after promotion source'); END;",
            StoredEventKind::AuditRecord as i32
        ))
        .unwrap();
    drop(connection);
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::CommandCompleted,
    );
    audit.command_id = Some(command());
    audit.reason_code = "spawn_completion".to_owned();
    assert!(storage
        .append_spawn_promotion_audited(&domain(), unstamped_promotion(), audit.clone())
        .await
        .is_err());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        9,
        "source event and grant identity reservation roll back with the audit failure"
    );

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER fail_promotion_audit;")
        .unwrap();
    drop(connection);
    let committed = storage
        .append_spawn_promotion_audited(&domain(), unstamped_promotion(), audit)
        .await
        .expect("rolled-back grant identity can be reserved by the complete retry");
    assert_eq!(committed.source_event_id, event_id(10));
    assert_eq!(committed.audit_event_id, event_id(11));
}

#[tokio::test]
async fn storage_stamps_and_commits_complete_promotion_plus_audit_atomically() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    for event in valid_prefix() {
        let appended = storage.append(&domain(), event.payload).await.unwrap();
        assert_eq!(appended, event.event_id);
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
    assert_eq!(committed.source_event_id, event_id(10));
    assert_eq!(committed.audit_event_id, event_id(11));
    assert_eq!(committed.promotion.promotion_event_id, Some(event_id(10)));
    assert_eq!(
        committed.promotion.completion_audit_event_id,
        Some(event_id(11))
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
        Some(event_id(11))
    );
    let events = storage
        .read_after(&domain(), Lsn { value: 9 })
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].payload.kind,
        StoredEventKind::SpawnPromotionCommitted as i32
    );
    assert_eq!(events[1].payload.kind, StoredEventKind::AuditRecord as i32);

    let generic_payload = StoredEventPayload {
        kind: StoredEventKind::SpawnPromotionCommitted as i32,
        payload: committed.promotion.encode_to_vec(),
    };
    let mut generic_audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::CommandCompleted,
    );
    generic_audit.command_id = Some(command());
    generic_audit.reason_code = "spawn_completion".to_owned();
    assert!(matches!(
        storage.append(&domain(), generic_payload.clone()).await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_audited(&domain(), generic_payload.clone(), generic_audit.clone())
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        AuditedStorage::new(storage.clone())
            .append_audited(&domain(), generic_payload.clone(), generic_audit.clone())
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_batch_audited(
                &domain(),
                vec![generic_payload.clone()],
                generic_audit.clone(),
            )
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_decision_audited_many(
                &domain(),
                generic_payload.clone(),
                vec![generic_audit.clone()],
            )
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        storage
            .append_dedup(
                &domain(),
                &IdempotencyKey {
                    value: "promotion-bypass".to_owned(),
                },
                &TargetKey::new("promotion-target".to_owned()).unwrap(),
                generic_payload,
            )
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        11
    );
}
