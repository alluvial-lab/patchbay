use patchbay_contracts::patchbay::{
    AuthorityDomainId, ExternalEffectDisposition, ExternalRuntimeRef, Generation,
    LogicalTargetCreated, LogicalTargetId, LogicalTargetInitialCurrentAssigned,
    RuntimeGenerationRef, RuntimeSessionId, SessionActivityState, SessionConnectivityState,
    SessionRegistered, SessionState, SpawnExecutionPhase, SpawnGenerationClaim, StoredEventKind,
};
use patchbay_core::{
    session::{
        events, phase_outcome, validate_continuation_prior_quiesced, CandidateOutcome,
        ClaimFenceOutcome, ContinuationContextStatus, PriorRuntimeOutcome, SessionRegistry,
        SpawnOrchestrationError,
    },
    storage::RecordedEvent,
};

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".into(),
    }
}

fn external(generation: u64) -> ExternalRuntimeRef {
    ExternalRuntimeRef {
        adapter_id: Some(patchbay_contracts::patchbay::AdapterId { value: "pi".into() }),
        deployment_scope: "machine-a".into(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "runtime-a".into(),
        }),
        generation: Some(Generation { value: generation }),
    }
}

fn claim() -> SpawnGenerationClaim {
    let logical = LogicalTargetId {
        value: "logical-a".into(),
    };
    SpawnGenerationClaim {
        authority_domain_id: Some(domain()),
        claim_operation_id: Some(patchbay_contracts::patchbay::CommandId {
            value: "spawn-a".into(),
        }),
        logical_target_id: Some(logical.clone()),
        expected_prior: Some(RuntimeGenerationRef {
            logical_target_id: Some(logical),
            external_runtime: Some(external(1)),
        }),
        claimed_generation: Some(Generation { value: 2 }),
    }
}

fn registry(
    connectivity: SessionConnectivityState,
    activity: SessionActivityState,
) -> SessionRegistry {
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let payloads = [
        events::encode(&events::logical_target_created(
            domain(),
            LogicalTargetCreated {
                logical_target_id: claim().logical_target_id,
                adapter_id: external(1).adapter_id,
                deployment_scope: "machine-a".into(),
            },
        )),
        events::encode(&events::logical_target_initial_current_assigned(
            domain(),
            LogicalTargetInitialCurrentAssigned {
                logical_target_id: claim().logical_target_id,
                external_runtime_ref: Some(external(1)),
            },
        )),
        events::encode(&events::registered(
            domain(),
            SessionRegistered {
                adapter_id: external(1).adapter_id,
                deployment_scope: "machine-a".into(),
                runtime_session_id: external(1).runtime_session_id,
                session_generation: Some(Generation { value: 1 }),
                initial_state: Some(SessionState {
                    connectivity: connectivity as i32,
                    activity: activity as i32,
                }),
                ..SessionRegistered::default()
            },
        )),
    ];
    for (index, payload) in payloads.into_iter().enumerate() {
        assert_ne!(payload.kind, StoredEventKind::Unspecified as i32);
        registry
            .observe(&RecordedEvent {
                event_id: patchbay_core::storage::event_id(domain(), index as u64 + 1),
                payload,
            })
            .unwrap();
    }
    registry
}

#[test]
fn continuation_context_vocabulary_is_closed_and_process_state_honest() {
    for value in ["resumed", "new_context", "unknown"] {
        assert_eq!(
            ContinuationContextStatus::try_from(value).unwrap().as_str(),
            value
        );
    }
    assert!(matches!(
        ContinuationContextStatus::try_from("restored_process"),
        Err(SpawnOrchestrationError::UnknownContinuationContextStatus(_))
    ));
}

#[test]
fn prior_must_be_exact_current_unavailable_and_unknown_before_staging_or_promotion() {
    for connectivity in [
        SessionConnectivityState::Offline,
        SessionConnectivityState::Stale,
        SessionConnectivityState::Failed,
    ] {
        validate_continuation_prior_quiesced(
            &registry(connectivity, SessionActivityState::Unknown),
            &claim(),
        )
        .unwrap();
    }
    assert_eq!(
        validate_continuation_prior_quiesced(
            &registry(
                SessionConnectivityState::Live,
                SessionActivityState::Unknown
            ),
            &claim(),
        ),
        Err(SpawnOrchestrationError::PriorConnectivityNotUnavailable),
        "availability-preserving mutant must die before N+1 staging"
    );
    assert_eq!(
        validate_continuation_prior_quiesced(
            &registry(
                SessionConnectivityState::Offline,
                SessionActivityState::Idle
            ),
            &claim(),
        ),
        Err(SpawnOrchestrationError::PriorActivityNotQuiesced),
        "known-activity mutant must die before N+1 staging"
    );
    let mut wrong = claim();
    wrong
        .expected_prior
        .as_mut()
        .unwrap()
        .external_runtime
        .as_mut()
        .unwrap()
        .generation = Some(Generation { value: 9 });
    assert_eq!(
        validate_continuation_prior_quiesced(
            &registry(
                SessionConnectivityState::Offline,
                SessionActivityState::Unknown
            ),
            &wrong,
        ),
        Err(SpawnOrchestrationError::PriorNotCurrent)
    );
}

#[test]
fn representative_phase_cells_kill_release_poison_and_early_publication_mutants() {
    let offered_ambiguous = phase_outcome(
        SpawnExecutionPhase::Offered,
        ExternalEffectDisposition::MayExist,
        patchbay_contracts::patchbay::FailureCode::ExecutionOutcomeUnknown,
        true,
    )
    .unwrap();
    assert_eq!(offered_ambiguous.claim, ClaimFenceOutcome::Poisoned);
    assert_eq!(
        offered_ambiguous.prior,
        PriorRuntimeOutcome::FencedUnknownActivity
    );
    assert_eq!(
        offered_ambiguous.candidate,
        CandidateOutcome::UnpublishableUnknown
    );

    let success = phase_outcome(
        SpawnExecutionPhase::SuccessEvidenceReported,
        ExternalEffectDisposition::Identified,
        patchbay_contracts::patchbay::FailureCode::Unspecified,
        true,
    )
    .unwrap();
    assert_eq!(success.claim, ClaimFenceOutcome::Active);
    assert_eq!(success.candidate, CandidateOutcome::StagedReady);
    assert_ne!(success.candidate, CandidateOutcome::CurrentAfterPromotion);

    let no_effect = phase_outcome(
        SpawnExecutionPhase::AcceptedNotOffered,
        ExternalEffectDisposition::ProvedNone,
        patchbay_contracts::patchbay::FailureCode::DeliveryRejected,
        false,
    )
    .unwrap();
    assert_eq!(no_effect.claim, ClaimFenceOutcome::ReleaseEligible);
    assert!(phase_outcome(
        SpawnExecutionPhase::AcceptedNotOffered,
        ExternalEffectDisposition::MayExist,
        patchbay_contracts::patchbay::FailureCode::Unspecified,
        false,
    )
    .is_err());
}
