use std::sync::{Arc, Mutex};

use patchbay_contracts::patchbay::{
    no_external_effect_proof, session_state_event, spawn_claim_disposition_changed,
    spawn_claim_event, AcceptedOperation, ActorEndpointRef, ActorId, AdapterCapability, AdapterId,
    AdapterRefusalBeforeDeliveryProof, AdapterRegistration, AdapterSnapshotSupport,
    AdapterTargetCategory, AuditEventKind, AuthorityDomainId, CommandId, CommandTransition,
    ContinuationAuthorityProvenance, EndpointId, EventId, ExternalEffectDisposition,
    ExternalRuntimeRef, FailureCode, Generation, GrantId, IdempotencyKey, LogicalTargetId, Lsn,
    NoExternalEffectProof, Observation, ObservationKind, Operation, OperationKind, OperationState,
    PayloadContentType, PayloadEnvelope, RuntimeGenerationRef, RuntimeSessionId,
    SessionConnectivityChanged, SessionConnectivityState, SessionStateEvent,
    SpawnClaimAbandonmentEvidence, SpawnClaimAccepted, SpawnClaimAmbiguityEvidence,
    SpawnClaimCheckpoint, SpawnClaimCheckpointRecord, SpawnClaimDisposition,
    SpawnClaimDispositionChanged, SpawnClaimEvent, SpawnClaimNoEffectRelease,
    SpawnClaimPromotionEvidence, SpawnEvidenceAttachment, SpawnExecutionEvidence,
    SpawnExecutionEvidenceProducer, SpawnExecutionPhase, SpawnGenerationClaim,
    SpawnPendingReplacementFence, SpawnPriorWorkDisposition, SpawnPriorWorkEffect, StoredEventKind,
    StoredEventPayload, SupervisorPreLaunchFailureProof, TargetScope, TargetScopeKind,
};
use patchbay_core::session::{
    allowed_spawn_claim_transition, encode_spawn_claim_event, encode_spawn_execution_evidence,
    rebuild_spawn_claims_from_log, SpawnClaimError, SpawnClaimQuery, SpawnClaimRegistry,
    SpawnClaimability, SpawnDeliveryFence, REPLACEMENT_PENDING_REASON,
};
use patchbay_core::storage::{
    AuditRecordDraft, RecordedEvent, RusqliteStorage, Storage, TargetKey,
};
use proptest::prelude::*;
use prost::Message;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn command(value: &str) -> CommandId {
    CommandId {
        value: value.to_owned(),
    }
}

fn target() -> LogicalTargetId {
    LogicalTargetId {
        value: "logical-a".to_owned(),
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

fn runtime(generation: u64) -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(target()),
        external_runtime: Some(external(generation)),
    }
}

fn event_id(lsn: u64) -> EventId {
    EventId {
        authority_domain_id: Some(domain()),
        lsn: Some(Lsn { value: lsn }),
    }
}

fn recorded(lsn: u64, payload: StoredEventPayload) -> RecordedEvent {
    RecordedEvent {
        event_id: event_id(lsn),
        payload,
    }
}

fn sibling(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: Vec::new(),
        },
    )
}

fn terminal_sibling(lsn: u64, state: OperationState) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::CommandTransition as i32,
            payload: CommandTransition {
                command_id: Some(command("spawn-a")),
                from_state: OperationState::Running as i32,
                to_state: state as i32,
                ..CommandTransition::default()
            }
            .encode_to_vec(),
        },
    )
}

fn pre_delivery_terminal_decision(
    lsn: u64,
    to_state: OperationState,
    failure: FailureCode,
) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::CommandTransition as i32,
            payload: CommandTransition {
                command_id: Some(command("spawn-a")),
                from_state: OperationState::Accepted as i32,
                to_state: to_state as i32,
                failure_code: failure as i32,
                ..CommandTransition::default()
            }
            .encode_to_vec(),
        },
    )
}

fn delivered_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::CommandTransition as i32,
            payload: CommandTransition {
                command_id: Some(command("spawn-a")),
                from_state: OperationState::Accepted as i32,
                to_state: OperationState::Delivered as i32,
                failure_code: FailureCode::Unspecified as i32,
                ..CommandTransition::default()
            }
            .encode_to_vec(),
        },
    )
}

fn claim(command_id: &str, expected_prior: Option<u64>) -> SpawnGenerationClaim {
    SpawnGenerationClaim {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(command(command_id)),
        logical_target_id: Some(target()),
        expected_prior: expected_prior.map(runtime),
        claimed_generation: Some(Generation {
            value: expected_prior.map_or(1, |prior| prior + 1),
        }),
    }
}

fn accepted_operation(command_id: &str) -> AcceptedOperation {
    AcceptedOperation {
        operation: Some(Operation {
            command_id: Some(command(command_id)),
            authority_domain_id: Some(domain()),
            kind: OperationKind::Spawn as i32,
            target_scope: Some(TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(AdapterId {
                    value: "pi".to_owned(),
                }),
                ..TargetScope::default()
            }),
            idempotency_key: format!("{command_id}-key"),
            ..Operation::default()
        }),
        authorizing_grant_id: Some(GrantId {
            value: "spawn-grant".to_owned(),
        }),
    }
}

fn continuation_authority(prior: u64) -> ContinuationAuthorityProvenance {
    ContinuationAuthorityProvenance {
        exact_prior: Some(runtime(prior)),
        replacement_grant_id: Some(GrantId {
            value: "replace-grant".to_owned(),
        }),
        replacement_authority_kind: OperationKind::SessionManagement as i32,
    }
}

fn continuation_accepted(command_id: &str) -> SpawnClaimAccepted {
    SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation(command_id)),
        claim: Some(claim(command_id, Some(7))),
        compound_authority: Some(continuation_authority(7)),
        pending_replacement: Some(SpawnPendingReplacementFence {
            exact_prior: Some(runtime(7)),
            failure_code: FailureCode::Superseded as i32,
            reason_code: REPLACEMENT_PENDING_REASON.to_owned(),
        }),
        prior_work_effects: vec![
            SpawnPriorWorkEffect {
                command_id: Some(command("accepted-n-work")),
                prior_state: OperationState::Accepted as i32,
                disposition: SpawnPriorWorkDisposition::SupersededBeforeOffer as i32,
                failure_code: FailureCode::Superseded as i32,
                reason_code: REPLACEMENT_PENDING_REASON.to_owned(),
            },
            SpawnPriorWorkEffect {
                command_id: Some(command("running-n-work")),
                prior_state: OperationState::Running as i32,
                disposition: SpawnPriorWorkDisposition::QuiesceOutcomeReconciliation as i32,
                failure_code: FailureCode::Unspecified as i32,
                reason_code: REPLACEMENT_PENDING_REASON.to_owned(),
            },
        ],
    }
}

fn fresh_accepted(command_id: &str) -> SpawnClaimAccepted {
    SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation(command_id)),
        claim: Some(claim(command_id, None)),
        compound_authority: None,
        pending_replacement: None,
        prior_work_effects: Vec::new(),
    }
}

fn accepted_event(lsn: u64, accepted: SpawnClaimAccepted) -> RecordedEvent {
    claim_event(lsn, spawn_claim_event::Mutation::Accepted(accepted))
}

async fn persist_claim(storage: &RusqliteStorage, accepted: SpawnClaimAccepted) {
    let accepted_operation = accepted.accepted_operation.as_ref().unwrap();
    let operation = accepted_operation.operation.as_ref().unwrap();
    let mut audit = AuditRecordDraft::new(
        prost_types::Timestamp {
            seconds: 1,
            nanos: 0,
        },
        AuditEventKind::CommandSubmissionAccepted,
    );
    audit.command_id = operation.command_id.clone();
    audit.grant_id = accepted_operation.authorizing_grant_id.clone();
    audit.target_scope = operation.target_scope.clone();
    audit.reason_code = "operation_spawn".to_owned();
    let key = IdempotencyKey {
        value: format!("{}-key", operation.command_id.as_ref().unwrap().value),
    };
    let logical_payload = operation.encode_to_vec();
    storage
        .append_spawn_claim_accepted(
            &domain(),
            &key,
            &TargetKey::new("spawn-target".to_owned()).unwrap(),
            accepted,
            audit,
            logical_payload,
        )
        .await
        .unwrap();
}

fn disposition_event(
    lsn: u64,
    command_id: &str,
    from: SpawnClaimDisposition,
    to: SpawnClaimDisposition,
    evidence: spawn_claim_disposition_changed::Evidence,
) -> RecordedEvent {
    claim_event(
        lsn,
        spawn_claim_event::Mutation::DispositionChanged(SpawnClaimDispositionChanged {
            claim_operation_id: Some(command(command_id)),
            from_disposition: from as i32,
            to_disposition: to as i32,
            evidence: Some(evidence),
        }),
    )
}

fn claim_event(lsn: u64, mutation: spawn_claim_event::Mutation) -> RecordedEvent {
    let event = SpawnClaimEvent {
        authority_domain_id: Some(domain()),
        mutation: Some(mutation),
    };
    recorded(lsn, encode_spawn_claim_event(&event))
}

fn ambiguity(lsn: u64) -> spawn_claim_disposition_changed::Evidence {
    spawn_claim_disposition_changed::Evidence::AmbiguousExternalEffect(
        SpawnClaimAmbiguityEvidence {
            evidence_event_id: Some(event_id(lsn)),
        },
    )
}

fn promotion(lsn: u64, generation: u64) -> spawn_claim_disposition_changed::Evidence {
    spawn_claim_disposition_changed::Evidence::Promotion(SpawnClaimPromotionEvidence {
        promotion_event_id: Some(event_id(lsn)),
        promoted_runtime: Some(runtime(generation)),
    })
}

fn abandonment(lsn: u64) -> spawn_claim_disposition_changed::Evidence {
    spawn_claim_disposition_changed::Evidence::TargetAbandonment(SpawnClaimAbandonmentEvidence {
        abandonment_event_id: Some(event_id(lsn)),
    })
}

fn core_no_effect(
    evidence_lsn: u64,
    prior_liveness_lsn: Option<u64>,
) -> spawn_claim_disposition_changed::Evidence {
    spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(SpawnClaimNoEffectRelease {
        evidence_event_id: Some(event_id(evidence_lsn)),
        exact_prior_liveness: prior_liveness_lsn.map(|_| runtime(7)),
        prior_liveness_event_id: prior_liveness_lsn.map(event_id),
    })
}

fn attachment_event(lsn: u64, adapter: &str, generation: u64) -> RecordedEvent {
    let adapter_id = AdapterId {
        value: adapter.to_owned(),
    };
    let endpoint_id = EndpointId {
        value: format!("{adapter}-endpoint"),
    };
    let registration = AdapterRegistration {
        adapter_id: Some(adapter_id.clone()),
        endpoint_id: Some(endpoint_id.clone()),
        authority_domain_id: Some(domain()),
        adapter_generation: Some(Generation { value: generation }),
        capability: Some(AdapterCapability {
            session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
            target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    };
    let observation = Observation {
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: adapter.to_owned(),
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
    };
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: observation.encode_to_vec(),
        },
    )
}

fn prior_live_event(lsn: u64) -> RecordedEvent {
    prior_live_event_for(lsn, 7)
}

fn prior_live_event_for(lsn: u64, generation: u64) -> RecordedEvent {
    recorded(
        lsn,
        StoredEventPayload {
            kind: StoredEventKind::SessionState as i32,
            payload: SessionStateEvent {
                authority_domain_id: Some(domain()),
                mutation: Some(session_state_event::Mutation::ConnectivityChanged(
                    SessionConnectivityChanged {
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        deployment_scope: "machine-a".to_owned(),
                        runtime_session_id: Some(RuntimeSessionId {
                            value: "runtime-a".to_owned(),
                        }),
                        session_generation: Some(Generation { value: generation }),
                        from: SessionConnectivityState::Stale as i32,
                        to: SessionConnectivityState::Live as i32,
                    },
                )),
            }
            .encode_to_vec(),
        },
    )
}

fn source(adapter: &str, generation: u64, attachment_lsn: u64) -> SpawnEvidenceAttachment {
    SpawnEvidenceAttachment {
        adapter_id: Some(AdapterId {
            value: adapter.to_owned(),
        }),
        adapter_generation: Some(Generation { value: generation }),
        attachment_event_id: Some(event_id(attachment_lsn)),
    }
}

#[allow(clippy::too_many_arguments)]
fn execution_evidence_event(
    lsn: u64,
    exact_claim: SpawnGenerationClaim,
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
    producer: SpawnExecutionEvidenceProducer,
    failure: FailureCode,
    proof: Option<NoExternalEffectProof>,
    external_runtime: Option<RuntimeGenerationRef>,
) -> RecordedEvent {
    execution_evidence_event_from(
        lsn,
        exact_claim,
        phase,
        disposition,
        producer,
        failure,
        proof,
        external_runtime,
        source("pi", 3, 1),
    )
}

#[allow(clippy::too_many_arguments)]
fn execution_evidence_event_from(
    lsn: u64,
    exact_claim: SpawnGenerationClaim,
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
    producer: SpawnExecutionEvidenceProducer,
    failure: FailureCode,
    proof: Option<NoExternalEffectProof>,
    external_runtime: Option<RuntimeGenerationRef>,
    source_attachment: SpawnEvidenceAttachment,
) -> RecordedEvent {
    recorded(
        lsn,
        encode_spawn_execution_evidence(&SpawnExecutionEvidence {
            authority_domain_id: Some(domain()),
            exact_claim: Some(exact_claim),
            phase: phase as i32,
            external_effect_disposition: disposition as i32,
            producer: producer as i32,
            source_attachment: Some(source_attachment),
            failure_code: failure as i32,
            no_external_effect_proof: proof,
            external_runtime,
        }),
    )
}

fn core_proof(terminal_decision_lsn: u64) -> NoExternalEffectProof {
    NoExternalEffectProof {
        proof: Some(no_external_effect_proof::Proof::CorePreDeliveryTerminal(
            patchbay_contracts::patchbay::CorePreDeliveryTerminalProof {
                terminal_decision_event_id: Some(event_id(terminal_decision_lsn)),
            },
        )),
    }
}

fn refusal_proof(adapter: &str, generation: u64) -> NoExternalEffectProof {
    NoExternalEffectProof {
        proof: Some(
            no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(
                AdapterRefusalBeforeDeliveryProof {
                    adapter_id: Some(AdapterId {
                        value: adapter.to_owned(),
                    }),
                    adapter_generation: Some(Generation { value: generation }),
                },
            ),
        ),
    }
}

fn supervisor_proof(adapter: &str, generation: u64) -> NoExternalEffectProof {
    NoExternalEffectProof {
        proof: Some(
            no_external_effect_proof::Proof::ExactSupervisorPreLaunchFailure(
                SupervisorPreLaunchFailureProof {
                    adapter_id: Some(AdapterId {
                        value: adapter.to_owned(),
                    }),
                    adapter_generation: Some(Generation { value: generation }),
                },
            ),
        ),
    }
}

#[test]
fn generated_claim_and_effect_contracts_round_trip() {
    let accepted = continuation_accepted("spawn-a");
    let decoded = SpawnClaimAccepted::decode(accepted.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, accepted);
    assert_eq!(decoded.prior_work_effects.len(), 2);
}

#[test]
fn continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();

    let record = registry.claim_for_operation(&command("spawn-a")).unwrap();
    assert_eq!(record.disposition, SpawnClaimDisposition::Active);
    assert_eq!(record.pending_replacement, Some(runtime(7)));
    assert_eq!(registry.prior_work_effects(&command("spawn-a")).len(), 2);
    assert_eq!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending {
            claim_operation_id: command("spawn-a"),
            failure_code: FailureCode::Superseded,
            reason_code: REPLACEMENT_PENDING_REASON,
        }
    );
    assert_eq!(
        registry.delivery_fence(&runtime(8)),
        SpawnDeliveryFence::Open
    );
}

#[test]
fn exact_retry_projects_original_while_changed_or_competing_claim_conflicts() {
    let original = claim("spawn-a", Some(7));
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();

    assert!(matches!(
        registry.classify_claim(&original),
        SpawnClaimability::ExactRetry(record) if record.claim == original
    ));
    let mut changed = original.clone();
    changed.claimed_generation = Some(Generation { value: 9 });
    assert!(matches!(
        registry.classify_claim(&changed),
        SpawnClaimability::Conflict(_)
    ));
    assert!(matches!(
        registry.classify_claim(&claim("spawn-b", Some(7))),
        SpawnClaimability::Conflict(_)
    ));
}

#[test]
fn disposition_table_matches_every_legal_cell() {
    let states = [
        SpawnClaimDisposition::Unspecified,
        SpawnClaimDisposition::Active,
        SpawnClaimDisposition::ReleasedNoExternalEffect,
        SpawnClaimDisposition::PoisonedPendingReconciliation,
        SpawnClaimDisposition::Promoted,
        SpawnClaimDisposition::TargetAbandoned,
    ];
    let legal = [
        (
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
        ),
        (
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
        ),
        (
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::Promoted,
        ),
        (
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::TargetAbandoned,
        ),
        (
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
        ),
        (
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            SpawnClaimDisposition::Promoted,
        ),
        (
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            SpawnClaimDisposition::TargetAbandoned,
        ),
    ];
    for from in states {
        for to in states {
            assert_eq!(
                allowed_spawn_claim_transition(from, to),
                legal.contains(&(from, to)),
                "unexpected transition {from:?} -> {to:?}"
            );
        }
    }
}

#[test]
fn execution_phase_disposition_table_is_closed_and_complete() {
    let phases = [
        SpawnExecutionPhase::Unspecified,
        SpawnExecutionPhase::AcceptedNotOffered,
        SpawnExecutionPhase::Offered,
        SpawnExecutionPhase::QuiescingPrior,
        SpawnExecutionPhase::PriorTerminated,
        SpawnExecutionPhase::LaunchAttempted,
        SpawnExecutionPhase::ExternalIdentityKnown,
        SpawnExecutionPhase::HandshakeReconciling,
        SpawnExecutionPhase::SuccessEvidenceReported,
    ];
    let dispositions = [
        ExternalEffectDisposition::Unspecified,
        ExternalEffectDisposition::ProvedNone,
        ExternalEffectDisposition::MayExist,
        ExternalEffectDisposition::Identified,
    ];
    let allowed = [
        (
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::ProvedNone,
        ),
        (
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
        ),
        (
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::MayExist,
        ),
        (
            SpawnExecutionPhase::QuiescingPrior,
            ExternalEffectDisposition::ProvedNone,
        ),
        (
            SpawnExecutionPhase::QuiescingPrior,
            ExternalEffectDisposition::MayExist,
        ),
        (
            SpawnExecutionPhase::PriorTerminated,
            ExternalEffectDisposition::ProvedNone,
        ),
        (
            SpawnExecutionPhase::PriorTerminated,
            ExternalEffectDisposition::MayExist,
        ),
        (
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::MayExist,
        ),
        (
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::Identified,
        ),
        (
            SpawnExecutionPhase::ExternalIdentityKnown,
            ExternalEffectDisposition::Identified,
        ),
        (
            SpawnExecutionPhase::HandshakeReconciling,
            ExternalEffectDisposition::Identified,
        ),
        (
            SpawnExecutionPhase::SuccessEvidenceReported,
            ExternalEffectDisposition::Identified,
        ),
    ];
    for phase in phases {
        for disposition in dispositions {
            assert_eq!(
                patchbay_core::session::allowed_external_effect_disposition(phase, disposition),
                allowed.contains(&(phase, disposition)),
                "unexpected phase/disposition cell {phase:?}/{disposition:?}"
            );
        }
    }
}

#[test]
fn terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant() {
    for terminal in [
        OperationState::Failed,
        OperationState::Cancelled,
        OperationState::Expired,
    ] {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry
            .observe(&accepted_event(1, continuation_accepted("spawn-a")))
            .unwrap();
        registry.observe(&terminal_sibling(2, terminal)).unwrap();
        let record = registry.claim_for_operation(&command("spawn-a")).unwrap();
        assert_eq!(record.disposition, SpawnClaimDisposition::Active);
        assert_eq!(record.pending_replacement, Some(runtime(7)));
        assert!(matches!(
            registry.classify_claim(&claim("spawn-b", Some(7))),
            SpawnClaimability::Conflict(_)
        ));
    }
}

#[test]
fn typed_external_effect_evidence_poison_transition_fires_and_retains_fence() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::MayExist,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionOutcomeUnknown,
            None,
            None,
        ))
        .unwrap();
    registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(3),
        ))
        .unwrap();

    assert_eq!(
        registry
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::PoisonedPendingReconciliation
    );
    assert!(matches!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
}

#[test]
fn delivered_cancellation_expiry_and_unknown_outcome_poison_never_release() {
    for failure in [
        FailureCode::Cancelled,
        FailureCode::Expired,
        FailureCode::ExecutionOutcomeUnknown,
    ] {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry.observe(&attachment_event(1, "pi", 3)).unwrap();
        registry
            .observe(&accepted_event(2, continuation_accepted("spawn-a")))
            .unwrap();
        registry
            .observe(&execution_evidence_event(
                3,
                claim("spawn-a", Some(7)),
                SpawnExecutionPhase::Offered,
                ExternalEffectDisposition::MayExist,
                SpawnExecutionEvidenceProducer::CurrentAdapter,
                failure,
                None,
                None,
            ))
            .unwrap();

        let before = registry.clone();
        assert!(registry
            .observe(&disposition_event(
                4,
                "spawn-a",
                SpawnClaimDisposition::Active,
                SpawnClaimDisposition::ReleasedNoExternalEffect,
                core_no_effect(3, Some(3)),
            ))
            .is_err());
        assert_eq!(registry, before);
        registry
            .observe(&disposition_event(
                4,
                "spawn-a",
                SpawnClaimDisposition::Active,
                SpawnClaimDisposition::PoisonedPendingReconciliation,
                ambiguity(3),
            ))
            .unwrap();
        assert_eq!(
            registry
                .claim_for_operation(&command("spawn-a"))
                .unwrap()
                .disposition,
            SpawnClaimDisposition::PoisonedPendingReconciliation
        );
    }
}

#[test]
fn identified_external_runtime_is_bounded_to_original_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    let mut wrong_runtime = runtime(8);
    wrong_runtime.logical_target_id = Some(LogicalTargetId {
        value: "another-target".to_owned(),
    });
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::ExternalIdentityKnown,
            ExternalEffectDisposition::Identified,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionFailed,
            None,
            Some(wrong_runtime),
        ))
        .unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(3),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn legacy_two_event_claim_promotion_is_rejected() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::Promoted,
            promotion(2, 8),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn continuation_release_requires_typed_exact_prior_liveness() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&pre_delivery_terminal_decision(
            3,
            OperationState::Cancelled,
            FailureCode::Cancelled,
        ))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            4,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::Cancelled,
            Some(core_proof(3)),
            None,
        ))
        .unwrap();
    registry.observe(&sibling(5)).unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            6,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(4, Some(5)),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn continuation_release_fires_after_exact_post_proof_prior_liveness() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&pre_delivery_terminal_decision(
            3,
            OperationState::Cancelled,
            FailureCode::Cancelled,
        ))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            4,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::Cancelled,
            Some(core_proof(3)),
            None,
        ))
        .unwrap();
    registry.observe(&prior_live_event(5)).unwrap();
    registry
        .observe(&disposition_event(
            6,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(4, Some(5)),
        ))
        .unwrap();
    assert_eq!(
        registry
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::ReleasedNoExternalEffect
    );
    assert_eq!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::Open
    );
}

#[test]
fn wrong_event_kind_cannot_release_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(3)).unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn another_claims_typed_evidence_cannot_poison_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-other", Some(7)),
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::MayExist,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionOutcomeUnknown,
            None,
            None,
        ))
        .unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(3),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn pre_acceptance_typed_evidence_cannot_poison_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&execution_evidence_event(
            2,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::MayExist,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionOutcomeUnknown,
            None,
            None,
        ))
        .unwrap();
    registry
        .observe(&accepted_event(3, continuation_accepted("spawn-a")))
        .unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(2),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn stale_attachment_evidence_cannot_release_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", None),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("pi", 3)),
            None,
        ))
        .unwrap();
    registry.observe(&attachment_event(4, "pi", 4)).unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            5,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn wrong_adapter_proof_cannot_release_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", None),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("other", 3)),
            None,
        ))
        .unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn silence_without_delivered_ack_is_not_no_effect_proof() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&terminal_sibling(3, OperationState::Failed))
        .unwrap();
    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn phase_disposition_and_proof_mismatches_are_non_mutating() {
    let cases = [
        (
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::Cancelled,
            Some(core_proof(2)),
        ),
        (
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::MayExist,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionOutcomeUnknown,
            None,
        ),
    ];
    for (phase, disposition, producer, failure, proof) in cases {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry.observe(&attachment_event(1, "pi", 3)).unwrap();
        registry
            .observe(&accepted_event(2, fresh_accepted("spawn-a")))
            .unwrap();
        registry
            .observe(&execution_evidence_event(
                3,
                claim("spawn-a", None),
                phase,
                disposition,
                producer,
                failure,
                proof,
                None,
            ))
            .unwrap();
        let before = registry.clone();
        assert!(registry
            .observe(&disposition_event(
                4,
                "spawn-a",
                SpawnClaimDisposition::Active,
                if disposition == ExternalEffectDisposition::ProvedNone {
                    SpawnClaimDisposition::ReleasedNoExternalEffect
                } else {
                    SpawnClaimDisposition::PoisonedPendingReconciliation
                },
                if disposition == ExternalEffectDisposition::ProvedNone {
                    core_no_effect(3, None)
                } else {
                    ambiguity(3)
                },
            ))
            .is_err());
        assert_eq!(registry, before);
    }
}

#[test]
fn all_three_closed_no_effect_proofs_can_release_only_with_typed_evidence() {
    let cases = [
        (
            SpawnExecutionPhase::AcceptedNotOffered,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::Cancelled,
            core_proof(3),
            true,
        ),
        (
            SpawnExecutionPhase::Offered,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            refusal_proof("pi", 3),
            false,
        ),
        (
            SpawnExecutionPhase::Offered,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionFailed,
            supervisor_proof("pi", 3),
            false,
        ),
    ];
    for (phase, producer, failure, proof, needs_core_decision) in cases {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry.observe(&attachment_event(1, "pi", 3)).unwrap();
        registry
            .observe(&accepted_event(2, fresh_accepted("spawn-a")))
            .unwrap();
        let evidence_lsn = if needs_core_decision {
            registry
                .observe(&pre_delivery_terminal_decision(
                    3,
                    OperationState::Cancelled,
                    FailureCode::Cancelled,
                ))
                .unwrap();
            4
        } else {
            3
        };
        registry
            .observe(&execution_evidence_event(
                evidence_lsn,
                claim("spawn-a", None),
                phase,
                ExternalEffectDisposition::ProvedNone,
                producer,
                failure,
                Some(proof),
                None,
            ))
            .unwrap();
        registry
            .observe(&disposition_event(
                evidence_lsn + 1,
                "spawn-a",
                SpawnClaimDisposition::Active,
                SpawnClaimDisposition::ReleasedNoExternalEffect,
                core_no_effect(evidence_lsn, None),
            ))
            .unwrap();
        assert_eq!(
            registry
                .claim_for_operation(&command("spawn-a"))
                .unwrap()
                .disposition,
            SpawnClaimDisposition::ReleasedNoExternalEffect
        );
        assert!(matches!(
            registry.classify_claim(&claim("spawn-b", None)),
            SpawnClaimability::Available
        ));
    }
}

#[test]
fn execution_outcome_unknown_is_never_a_core_no_effect_proof() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&pre_delivery_terminal_decision(
            3,
            OperationState::Failed,
            FailureCode::ExecutionOutcomeUnknown,
        ))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            4,
            claim("spawn-a", None),
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::ExecutionOutcomeUnknown,
            Some(core_proof(3)),
            None,
        ))
        .unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            5,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(4, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn obsolete_refusal_proof_cannot_release_after_later_ambiguity_poison() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", None),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("pi", 3)),
            None,
        ))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            4,
            claim("spawn-a", None),
            SpawnExecutionPhase::LaunchAttempted,
            ExternalEffectDisposition::MayExist,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionOutcomeUnknown,
            None,
            None,
        ))
        .unwrap();
    registry
        .observe(&disposition_event(
            5,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(4),
        ))
        .unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            6,
            "spawn-a",
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
    assert_eq!(
        registry
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::PoisonedPendingReconciliation
    );
}

#[test]
fn obsolete_core_proof_cannot_release_after_later_delivery() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&pre_delivery_terminal_decision(
            3,
            OperationState::Cancelled,
            FailureCode::Cancelled,
        ))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            4,
            claim("spawn-a", None),
            SpawnExecutionPhase::AcceptedNotOffered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::Core,
            FailureCode::Cancelled,
            Some(core_proof(3)),
            None,
        ))
        .unwrap();
    registry.observe(&delivered_event(5)).unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            6,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(4, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn missing_evidence_event_id_is_silence_not_proof() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", None),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("pi", 3)),
            None,
        ))
        .unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(
                SpawnClaimNoEffectRelease {
                    evidence_event_id: None,
                    exact_prior_liveness: None,
                    prior_liveness_event_id: None,
                },
            ),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn evidence_source_adapter_must_match_claim_adapter() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry.observe(&attachment_event(2, "other", 3)).unwrap();
    registry
        .observe(&accepted_event(3, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event_from(
            4,
            claim("spawn-a", None),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("other", 3)),
            None,
            source("other", 3, 2),
        ))
        .unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            5,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(4, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn fresh_claim_cannot_use_prior_runtime_phase_evidence() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", None),
            SpawnExecutionPhase::QuiescingPrior,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::ExecutionFailed,
            Some(supervisor_proof("pi", 3)),
            None,
        ))
        .unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn live_evidence_must_match_exact_prior_n_identity() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&attachment_event(1, "pi", 3)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&execution_evidence_event(
            3,
            claim("spawn-a", Some(7)),
            SpawnExecutionPhase::Offered,
            ExternalEffectDisposition::ProvedNone,
            SpawnExecutionEvidenceProducer::CurrentAdapter,
            FailureCode::DeliveryRejected,
            Some(refusal_proof("pi", 3)),
            None,
        ))
        .unwrap();
    registry.observe(&prior_live_event_for(4, 6)).unwrap();

    let before = registry.clone();
    assert!(registry
        .observe(&disposition_event(
            5,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(3, Some(4)),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn target_abandonment_clears_fence_but_permanently_consumes_generation() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::TargetAbandoned,
            abandonment(2),
        ))
        .unwrap();
    assert_eq!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::Open
    );
    assert!(matches!(
        registry.classify_claim(&claim("spawn-b", Some(7))),
        SpawnClaimability::Conflict(_)
    ));
}

#[tokio::test]
async fn hostile_checkpoint_cannot_manufacture_released_disposition_or_availability() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    persist_claim(&storage, continuation_accepted("spawn-a")).await;

    let hostile = SpawnClaimCheckpoint {
        authority_domain_id: Some(domain()),
        snapshot_lsn: Some(Lsn { value: 99 }),
        records: vec![SpawnClaimCheckpointRecord {
            claim: Some(claim("spawn-a", Some(7))),
            accepted_lsn: Some(Lsn { value: 98 }),
            compound_authority: None,
            disposition: SpawnClaimDisposition::ReleasedNoExternalEffect as i32,
            pending_replacement: None,
            prior_work_effects: Vec::new(),
        }],
    };
    let decoded_hostile = SpawnClaimCheckpoint::decode(hostile.encode_to_vec().as_slice()).unwrap();
    assert_eq!(
        decoded_hostile.records[0].disposition,
        SpawnClaimDisposition::ReleasedNoExternalEffect as i32
    );

    // Raw checkpoint bytes have no registry manufacturing API. Authoritative
    // recovery rebuilds the claim and disposition from the durable log.
    let recovered = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        recovered
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::Active
    );
    assert!(matches!(
        recovered.classify_claim(&claim("spawn-b", Some(7))),
        SpawnClaimability::Conflict(_)
    ));
    assert!(matches!(
        recovered.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
}

#[tokio::test]
async fn cold_log_replay_matches_hot_active_claim_projection() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    persist_claim(&storage, continuation_accepted("spawn-a")).await;
    storage.append(&domain(), sibling(2).payload).await.unwrap();
    let replayed = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    let record = replayed.claim_for_operation(&command("spawn-a")).unwrap();
    assert_eq!(record.disposition, SpawnClaimDisposition::Active);
    assert_eq!(record.pending_replacement, Some(runtime(7)));
    assert!(matches!(
        replayed.classify_claim(&claim("spawn-b", Some(7))),
        SpawnClaimability::Conflict(_)
    ));
}

#[test]
fn competing_concurrent_claims_have_at_most_one_owner() {
    let registry = Arc::new(Mutex::new(SpawnClaimRegistry::new(domain()).unwrap()));
    let mut joins = Vec::new();
    for command_id in ["spawn-a", "spawn-b"] {
        let registry = Arc::clone(&registry);
        joins.push(std::thread::spawn(move || {
            registry
                .lock()
                .unwrap()
                .observe(&accepted_event(1, continuation_accepted(command_id)))
                .is_ok()
        }));
    }
    let successes = joins
        .into_iter()
        .map(|join| join.join().unwrap())
        .filter(|success| *success)
        .count();
    let registry = registry.lock().unwrap();
    assert_eq!(successes, 1);
    assert_eq!(registry.records().count(), 1);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn distinct_commands_never_share_one_active_generation_kills_reclaim_mutant(
        first in "spawn-[a-z]{1,8}",
        second in "spawn-[a-z]{1,8}",
        reverse in any::<bool>(),
    ) {
        prop_assume!(first != second);
        let (winner, competitor) = if reverse { (&second, &first) } else { (&first, &second) };
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry
            .observe(&accepted_event(1, continuation_accepted(winner)))
            .unwrap();
        let before = registry.clone();
        let error = registry
            .observe(&accepted_event(2, continuation_accepted(competitor)))
            .unwrap_err();
        prop_assert!(matches!(error, SpawnClaimError::GenerationAlreadyClaimed(_)));
        prop_assert_eq!(&registry, &before);
        prop_assert!(matches!(
            registry.classify_claim(&claim(competitor, Some(7))),
            SpawnClaimability::Conflict(_)
        ));

    }

    #[test]
    fn every_terminal_command_state_is_claim_inert(terminal in prop_oneof![
        Just(OperationState::Completed),
        Just(OperationState::Rejected),
        Just(OperationState::Failed),
        Just(OperationState::Expired),
        Just(OperationState::Cancelled),
        Just(OperationState::Superseded),
    ]) {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry
            .observe(&accepted_event(1, continuation_accepted("spawn-a")))
            .unwrap();
        registry.observe(&terminal_sibling(2, terminal)).unwrap();
        let record = registry.claim_for_operation(&command("spawn-a")).unwrap();
        prop_assert_eq!(record.disposition, SpawnClaimDisposition::Active);
        prop_assert_eq!(&record.pending_replacement, &Some(runtime(7)));
        prop_assert!(matches!(
            registry.classify_claim(&claim("spawn-b", Some(7))),
            SpawnClaimability::Conflict(_)
        ));
    }
}

#[test]
fn malformed_or_illegal_transition_is_non_mutating() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, fresh_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    let before = registry.clone();
    let error = registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::Active,
            ambiguity(2),
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        SpawnClaimError::IllegalDispositionTransition { .. }
    ));
    assert_eq!(registry, before);
}
