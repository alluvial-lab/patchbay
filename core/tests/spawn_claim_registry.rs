use std::sync::{Arc, Mutex};

use patchbay_contracts::patchbay::{
    no_external_effect_proof, session_state_event, spawn_claim_disposition_changed,
    spawn_claim_event, spawn_request, AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterCapability, AdapterId, AdapterRefusalBeforeDeliveryProof, AdapterRegistration,
    AdapterSnapshotSupport, AdapterTargetCategory, AuditEventKind, AuthorityDomainId, CommandId,
    CommandTransition, ContinuationAuthorityProvenance, DeviceId, EndpointId, EventId,
    ExternalEffectDisposition, ExternalRuntimeRef, FailureCode, FreshSpawn, Generation, GrantId,
    IdempotencyKey, LogicalTargetId, Lsn, NoExternalEffectProof, Observation, ObservationKind,
    Operation, OperationKind, OperationState, PayloadContentType, PayloadEnvelope,
    RuntimeGenerationRef, RuntimeSessionId, SessionActivityState, SessionConnectivityChanged,
    SessionConnectivityState, SessionRegistered, SessionState, SessionStateEvent,
    SpawnClaimAbandonmentEvidence, SpawnClaimAccepted, SpawnClaimAmbiguityEvidence,
    SpawnClaimCheckpoint, SpawnClaimCheckpointRecord, SpawnClaimDisposition,
    SpawnClaimDispositionChanged, SpawnClaimEvent, SpawnClaimNoEffectRelease,
    SpawnClaimPromotionEvidence, SpawnContinuation, SpawnEvidenceAttachment,
    SpawnExecutionEvidence, SpawnExecutionEvidenceProducer, SpawnExecutionPhase,
    SpawnGenerationClaim, SpawnPendingReplacementFence, SpawnPriorWorkDisposition,
    SpawnPriorWorkEffect, SpawnRequest, SpawnTargetSpec, StoredEventKind, StoredEventPayload,
    SupervisorPreLaunchFailureProof, TargetScope, TargetScopeKind,
};
use patchbay_core::session::{
    allowed_spawn_claim_transition, encode_spawn_claim_event, encode_spawn_execution_evidence,
    rebuild_spawn_claims_from_log, SpawnClaimError, SpawnClaimQuery, SpawnClaimRegistry,
    SpawnClaimability, SpawnDeliveryFence, REPLACEMENT_PENDING_REASON,
};
use patchbay_core::storage::{
    AuditRecordDraft, RecordedEvent, RusqliteStorage, Storage, StorageError, TargetKey,
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
    logical("logical-a")
}

fn logical(value: &str) -> LogicalTargetId {
    LogicalTargetId {
        value: value.to_owned(),
    }
}

fn adapter(value: &str) -> AdapterId {
    AdapterId {
        value: value.to_owned(),
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

fn accepted_operation(command_id: &str, expected_prior: Option<u64>) -> AcceptedOperation {
    let intent = expected_prior.map_or_else(
        || spawn_request::Intent::Fresh(FreshSpawn {}),
        |generation| {
            spawn_request::Intent::Continuation(SpawnContinuation {
                prior: Some(runtime(generation)),
            })
        },
    );
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
            payload: Some(PayloadEnvelope {
                payload: SpawnRequest {
                    intent: Some(intent),
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
        accepted_operation: Some(accepted_operation(command_id, Some(7))),
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
        accepted_operation: Some(accepted_operation(command_id, None)),
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
        logical_target_id: Some(target()),
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
        reason_code: "operator_recovery".to_owned(),
        abandoned_at: Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        }),
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

fn prior_registration_event(lsn: u64) -> RecordedEvent {
    recorded(
        lsn,
        patchbay_core::session::events::encode(&patchbay_core::session::events::registered(
            domain(),
            SessionRegistered {
                adapter_id: Some(adapter("pi")),
                deployment_scope: "machine-a".to_owned(),
                runtime_session_id: Some(RuntimeSessionId {
                    value: "runtime-a".to_owned(),
                }),
                session_generation: Some(Generation { value: 7 }),
                initial_state: Some(SessionState {
                    connectivity: SessionConnectivityState::Stale as i32,
                    activity: SessionActivityState::Unknown as i32,
                }),
                ..SessionRegistered::default()
            },
        )),
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
fn execution_evidence(
    exact_claim: SpawnGenerationClaim,
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
    producer: SpawnExecutionEvidenceProducer,
    failure: FailureCode,
    proof: Option<NoExternalEffectProof>,
    external_runtime: Option<RuntimeGenerationRef>,
) -> SpawnExecutionEvidence {
    SpawnExecutionEvidence {
        authority_domain_id: Some(domain()),
        exact_claim: Some(exact_claim),
        phase: phase as i32,
        external_effect_disposition: disposition as i32,
        producer: producer as i32,
        source_attachment: Some(source("pi", 3, 1)),
        failure_code: failure as i32,
        no_external_effect_proof: proof,
        external_runtime,
    }
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
            abandonment(3),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReconciledClaimConsequence {
    Active,
    Poisoned,
    Released,
}

#[derive(Clone, Copy, Debug)]
struct ReconciliationMatrixCase {
    label: &'static str,
    phase: SpawnExecutionPhase,
    effect: ExternalEffectDisposition,
    failure: FailureCode,
    continuation: bool,
    expected: ReconciledClaimConsequence,
}

#[tokio::test]
async fn storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row() {
    let cases = [
        ReconciliationMatrixCase {
            label: "accepted_not_offered/proved_none",
            phase: SpawnExecutionPhase::AcceptedNotOffered,
            effect: ExternalEffectDisposition::ProvedNone,
            failure: FailureCode::Cancelled,
            continuation: false,
            expected: ReconciledClaimConsequence::Released,
        },
        ReconciliationMatrixCase {
            label: "offered/proved_none",
            phase: SpawnExecutionPhase::Offered,
            effect: ExternalEffectDisposition::ProvedNone,
            failure: FailureCode::DeliveryRejected,
            continuation: false,
            expected: ReconciledClaimConsequence::Released,
        },
        ReconciliationMatrixCase {
            label: "offered/may_exist",
            phase: SpawnExecutionPhase::Offered,
            effect: ExternalEffectDisposition::MayExist,
            failure: FailureCode::ExecutionOutcomeUnknown,
            continuation: false,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "quiescing_prior/proved_none",
            phase: SpawnExecutionPhase::QuiescingPrior,
            effect: ExternalEffectDisposition::ProvedNone,
            failure: FailureCode::ExecutionFailed,
            continuation: true,
            expected: ReconciledClaimConsequence::Active,
        },
        ReconciliationMatrixCase {
            label: "quiescing_prior/may_exist",
            phase: SpawnExecutionPhase::QuiescingPrior,
            effect: ExternalEffectDisposition::MayExist,
            failure: FailureCode::ExecutionOutcomeUnknown,
            continuation: true,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "prior_terminated/proved_none",
            phase: SpawnExecutionPhase::PriorTerminated,
            effect: ExternalEffectDisposition::ProvedNone,
            failure: FailureCode::ExecutionFailed,
            continuation: true,
            expected: ReconciledClaimConsequence::Active,
        },
        ReconciliationMatrixCase {
            label: "prior_terminated/may_exist",
            phase: SpawnExecutionPhase::PriorTerminated,
            effect: ExternalEffectDisposition::MayExist,
            failure: FailureCode::ExecutionOutcomeUnknown,
            continuation: true,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "launch_attempted/may_exist",
            phase: SpawnExecutionPhase::LaunchAttempted,
            effect: ExternalEffectDisposition::MayExist,
            failure: FailureCode::ExecutionOutcomeUnknown,
            continuation: false,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "launch_attempted/identified/unspecified",
            phase: SpawnExecutionPhase::LaunchAttempted,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::Unspecified,
            continuation: true,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "external_identity_known/identified/progress",
            phase: SpawnExecutionPhase::ExternalIdentityKnown,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::Unspecified,
            continuation: false,
            expected: ReconciledClaimConsequence::Active,
        },
        ReconciliationMatrixCase {
            label: "external_identity_known/identified/failure",
            phase: SpawnExecutionPhase::ExternalIdentityKnown,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::DeliveryRejected,
            continuation: false,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "handshake_reconciling/identified/progress",
            phase: SpawnExecutionPhase::HandshakeReconciling,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::Unspecified,
            continuation: false,
            expected: ReconciledClaimConsequence::Active,
        },
        ReconciliationMatrixCase {
            label: "handshake_reconciling/identified/failure",
            phase: SpawnExecutionPhase::HandshakeReconciling,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::DeliveryRejected,
            continuation: false,
            expected: ReconciledClaimConsequence::Poisoned,
        },
        ReconciliationMatrixCase {
            label: "success_evidence_reported/identified/progress",
            phase: SpawnExecutionPhase::SuccessEvidenceReported,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::Unspecified,
            continuation: false,
            expected: ReconciledClaimConsequence::Active,
        },
        ReconciliationMatrixCase {
            label: "success_evidence_reported/identified/failure",
            phase: SpawnExecutionPhase::SuccessEvidenceReported,
            effect: ExternalEffectDisposition::Identified,
            failure: FailureCode::DeliveryRejected,
            continuation: false,
            expected: ReconciledClaimConsequence::Poisoned,
        },
    ];

    for case in cases {
        let storage = RusqliteStorage::open_in_memory().expect("storage opens");
        storage
            .append(&domain(), attachment_event(1, "pi", 3).payload)
            .await
            .expect("attachment appends");
        let accepted = if case.continuation {
            continuation_accepted("spawn-a")
        } else {
            fresh_accepted("spawn-a")
        };
        let exact_claim = accepted.claim.clone().expect("claim fixture");
        persist_claim(&storage, accepted).await;

        let (producer, proof) = match case.effect {
            ExternalEffectDisposition::ProvedNone
                if case.phase == SpawnExecutionPhase::AcceptedNotOffered =>
            {
                let decision = storage
                    .append(
                        &domain(),
                        pre_delivery_terminal_decision(
                            0,
                            OperationState::Cancelled,
                            FailureCode::Cancelled,
                        )
                        .payload,
                    )
                    .await
                    .expect("pre-delivery terminal decision appends");
                let decision_lsn = decision.lsn.expect("decision LSN").value;
                (
                    SpawnExecutionEvidenceProducer::Core,
                    Some(core_proof(decision_lsn)),
                )
            }
            ExternalEffectDisposition::ProvedNone if case.phase == SpawnExecutionPhase::Offered => {
                (
                    SpawnExecutionEvidenceProducer::CurrentAdapter,
                    Some(refusal_proof("pi", 3)),
                )
            }
            ExternalEffectDisposition::ProvedNone => (
                SpawnExecutionEvidenceProducer::CurrentAdapter,
                Some(supervisor_proof("pi", 3)),
            ),
            ExternalEffectDisposition::MayExist | ExternalEffectDisposition::Identified => {
                (SpawnExecutionEvidenceProducer::CurrentAdapter, None)
            }
            ExternalEffectDisposition::Unspecified => unreachable!("matrix excludes unspecified"),
        };
        let identified_runtime = (case.effect == ExternalEffectDisposition::Identified)
            .then(|| runtime(if case.continuation { 8 } else { 1 }));
        let evidence = execution_evidence(
            exact_claim,
            case.phase,
            case.effect,
            producer,
            case.failure,
            proof,
            identified_runtime.clone(),
        );
        let committed = storage
            .append_spawn_execution_evidence_reconciled(&domain(), evidence)
            .await
            .unwrap_or_else(|error| panic!("{} did not commit: {error}", case.label));
        assert_eq!(
            committed.disposition_event_id.is_some(),
            case.expected != ReconciledClaimConsequence::Active,
            "{} disposition append",
            case.label
        );

        let events = storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .expect("matrix prefix reads");
        assert_eq!(
            events
                .iter()
                .filter(|event| {
                    event.payload.kind == StoredEventKind::SpawnExecutionEvidence as i32
                })
                .count(),
            1,
            "{} must commit exactly one evidence record",
            case.label
        );
        let claims = rebuild_spawn_claims_from_log(&storage, &domain())
            .await
            .unwrap_or_else(|error| panic!("{} did not replay: {error}", case.label));
        let record = claims
            .claim_for_operation(&command("spawn-a"))
            .expect("matrix claim replays");
        let expected_disposition = match case.expected {
            ReconciledClaimConsequence::Active => SpawnClaimDisposition::Active,
            ReconciledClaimConsequence::Poisoned => {
                SpawnClaimDisposition::PoisonedPendingReconciliation
            }
            ReconciledClaimConsequence::Released => SpawnClaimDisposition::ReleasedNoExternalEffect,
        };
        assert_eq!(record.disposition, expected_disposition, "{}", case.label);
        if let Some(identified_runtime) = identified_runtime.as_ref() {
            assert_eq!(
                claims.identified_runtime_for_operation(&command("spawn-a")),
                Some(identified_runtime),
                "{} identified runtime ownership",
                case.label
            );
        }
        if case.label == "launch_attempted/identified/unspecified" {
            assert!(matches!(
                claims.delivery_fence(&runtime(7)),
                SpawnDeliveryFence::ReplacementPending { .. }
            ));
            assert!(matches!(
                claims.classify_claim(&claim("spawn-b", Some(7))),
                SpawnClaimability::Conflict(_)
            ));
        }
        let commands = patchbay_core::acceptance::rebuild_from_log(&storage, &domain())
            .await
            .unwrap_or_else(|error| panic!("{} command replay failed: {error}", case.label));
        assert!(
            commands.delivery_is_suppressed(&command("spawn-a")),
            "{} original attempt must remain suppressed",
            case.label
        );
    }
}

#[tokio::test]
async fn reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(&domain(), attachment_event(1, "pi", 3).payload)
        .await
        .unwrap();
    persist_claim(&storage, fresh_accepted("spawn-a")).await;
    let evidence = execution_evidence(
        claim("spawn-a", None),
        SpawnExecutionPhase::LaunchAttempted,
        ExternalEffectDisposition::MayExist,
        SpawnExecutionEvidenceProducer::CurrentAdapter,
        FailureCode::ExecutionOutcomeUnknown,
        None,
        None,
    );

    assert!(matches!(
        storage
            .append(&domain(), encode_spawn_execution_evidence(&evidence))
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    let first = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence.clone())
        .await
        .unwrap();
    assert!(!first.deduplicated);
    assert!(first.disposition_event_id.is_some());
    let event_count = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let retry = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence)
        .await
        .unwrap();
    assert!(retry.deduplicated);
    assert_eq!(retry.evidence_event_id, first.evidence_event_id);
    assert!(retry.disposition_event_id.is_none());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        event_count
    );

    let claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        claims
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::PoisonedPendingReconciliation
    );
    assert!(matches!(
        claims.classify_claim(&claim("spawn-b", None)),
        SpawnClaimability::Conflict(_)
    ));
    let commands = patchbay_core::acceptance::rebuild_from_log(&storage, &domain())
        .await
        .unwrap();
    assert!(commands.delivery_is_suppressed(&command("spawn-a")));
}

#[tokio::test]
async fn proved_none_releases_once_but_never_redelivers_the_original_attempt() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(&domain(), attachment_event(1, "pi", 3).payload)
        .await
        .unwrap();
    persist_claim(&storage, fresh_accepted("spawn-a")).await;
    let evidence = execution_evidence(
        claim("spawn-a", None),
        SpawnExecutionPhase::Offered,
        ExternalEffectDisposition::ProvedNone,
        SpawnExecutionEvidenceProducer::CurrentAdapter,
        FailureCode::DeliveryRejected,
        Some(refusal_proof("pi", 3)),
        None,
    );

    let first = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence.clone())
        .await
        .unwrap();
    assert!(first.disposition_event_id.is_some());
    let count = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let retry = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence)
        .await
        .unwrap();
    assert!(retry.deduplicated);
    assert!(retry.disposition_event_id.is_none());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        count
    );

    let claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        claims
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::ReleasedNoExternalEffect
    );
    assert!(matches!(
        claims.classify_claim(&claim("spawn-b", None)),
        SpawnClaimability::Available
    ));
    let commands = patchbay_core::acceptance::rebuild_from_log(&storage, &domain())
        .await
        .unwrap();
    assert!(commands.delivery_is_suppressed(&command("spawn-a")));
}

#[tokio::test]
async fn continuation_no_effect_waits_for_post_proof_exact_prior_liveness() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(&domain(), attachment_event(1, "pi", 3).payload)
        .await
        .unwrap();
    storage
        .append(&domain(), prior_registration_event(2).payload)
        .await
        .unwrap();
    persist_claim(&storage, continuation_accepted("spawn-a")).await;
    let evidence = execution_evidence(
        claim("spawn-a", Some(7)),
        SpawnExecutionPhase::Offered,
        ExternalEffectDisposition::ProvedNone,
        SpawnExecutionEvidenceProducer::CurrentAdapter,
        FailureCode::DeliveryRejected,
        Some(refusal_proof("pi", 3)),
        None,
    );
    let before_invalid = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let mut invalid = evidence.clone();
    invalid.source_attachment = Some(source("pi", 2, 1));
    assert!(storage
        .append_spawn_execution_evidence_reconciled(&domain(), invalid)
        .await
        .is_err());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        before_invalid
    );

    let first = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence.clone())
        .await
        .unwrap();
    assert!(first.disposition_event_id.is_none());
    let pending = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        pending
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::Active
    );

    storage
        .append(&domain(), prior_live_event(99).payload)
        .await
        .unwrap();
    let released = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence.clone())
        .await
        .unwrap();
    assert!(released.deduplicated);
    assert!(released.disposition_event_id.is_some());
    let count = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let retry = storage
        .append_spawn_execution_evidence_reconciled(&domain(), evidence)
        .await
        .unwrap();
    assert!(retry.deduplicated);
    assert!(retry.disposition_event_id.is_none());
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()
            .len(),
        count
    );
    let claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        claims
            .claim_for_operation(&command("spawn-a"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::ReleasedNoExternalEffect
    );
    assert!(matches!(
        claims.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::Open
    ));
}

#[tokio::test]
async fn identified_runtime_is_reserved_to_its_original_claim_at_ingress() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(&domain(), attachment_event(1, "pi", 3).payload)
        .await
        .unwrap();
    persist_claim(&storage, fresh_accepted("spawn-a")).await;
    let external = ExternalRuntimeRef {
        adapter_id: Some(adapter("pi")),
        deployment_scope: "machine-a".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "native-shared".to_owned(),
        }),
        generation: Some(Generation { value: 1 }),
    };
    let first_runtime = RuntimeGenerationRef {
        logical_target_id: Some(logical("logical-a")),
        external_runtime: Some(external.clone()),
    };
    let first = execution_evidence(
        claim("spawn-a", None),
        SpawnExecutionPhase::ExternalIdentityKnown,
        ExternalEffectDisposition::Identified,
        SpawnExecutionEvidenceProducer::CurrentAdapter,
        FailureCode::ExecutionFailed,
        None,
        Some(first_runtime.clone()),
    );
    storage
        .append_spawn_execution_evidence_reconciled(&domain(), first)
        .await
        .unwrap();

    let mut accepted_b = fresh_accepted("spawn-b");
    accepted_b.claim.as_mut().unwrap().logical_target_id = Some(logical("logical-b"));
    persist_claim(&storage, accepted_b).await;
    let mut claim_b = claim("spawn-b", None);
    claim_b.logical_target_id = Some(logical("logical-b"));
    let second_runtime = RuntimeGenerationRef {
        logical_target_id: Some(logical("logical-b")),
        external_runtime: Some(external),
    };
    let second = execution_evidence(
        claim_b,
        SpawnExecutionPhase::ExternalIdentityKnown,
        ExternalEffectDisposition::Identified,
        SpawnExecutionEvidenceProducer::CurrentAdapter,
        FailureCode::ExecutionFailed,
        None,
        Some(second_runtime),
    );
    assert!(matches!(
        storage
            .append_spawn_execution_evidence_reconciled(&domain(), second)
            .await,
        Err(StorageError::DuplicateNativeReference { .. })
    ));

    let claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(
        claims
            .claim_for_external_runtime(&first_runtime)
            .unwrap()
            .claim
            .claim_operation_id
            .as_ref(),
        Some(&command("spawn-a"))
    );
    assert_eq!(
        claims
            .claim_for_operation(&command("spawn-b"))
            .unwrap()
            .disposition,
        SpawnClaimDisposition::Active
    );
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
