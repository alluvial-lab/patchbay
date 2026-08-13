use std::sync::{Arc, Mutex};

use patchbay_contracts::patchbay::{
    no_external_effect_proof, spawn_claim_disposition_changed, spawn_claim_event,
    AcceptedOperation, AdapterId, AdapterRefusalBeforeDeliveryProof, AuthorityDomainId, CommandId,
    CommandTransition, ContinuationAuthorityProvenance, EventId, ExternalRuntimeRef, FailureCode,
    Generation, GrantId, LogicalTargetId, Lsn, NoExternalEffectProof, Operation, OperationKind,
    OperationState, RuntimeGenerationRef, RuntimeSessionId, SpawnClaimAbandonmentEvidence,
    SpawnClaimAccepted, SpawnClaimAmbiguityEvidence, SpawnClaimCheckpoint,
    SpawnClaimCheckpointRecord, SpawnClaimDisposition, SpawnClaimDispositionChanged,
    SpawnClaimEvent, SpawnClaimNoEffectRelease, SpawnClaimPromotionEvidence, SpawnGenerationClaim,
    SpawnPendingReplacementFence, SpawnPriorWorkDisposition, SpawnPriorWorkEffect, StoredEventKind,
    StoredEventPayload, SupervisorPreLaunchFailureProof,
};
use patchbay_core::session::{
    allowed_spawn_claim_transition, encode_spawn_claim_event, rebuild_spawn_claims_from_log,
    SpawnClaimError, SpawnClaimQuery, SpawnClaimRegistry, SpawnClaimability, SpawnDeliveryFence,
    REPLACEMENT_PENDING_REASON,
};
use patchbay_core::storage::{RecordedEvent, RusqliteStorage, Storage};
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
    proof_lsn: u64,
    prior_liveness_lsn: Option<u64>,
) -> spawn_claim_disposition_changed::Evidence {
    spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(SpawnClaimNoEffectRelease {
        proof: Some(NoExternalEffectProof {
            proof: Some(no_external_effect_proof::Proof::CorePreDeliveryTerminal(
                patchbay_contracts::patchbay::CorePreDeliveryTerminalProof {
                    decision_event_id: Some(event_id(proof_lsn)),
                },
            )),
        }),
        exact_prior_liveness: prior_liveness_lsn.map(|_| runtime(7)),
        prior_liveness_event_id: prior_liveness_lsn.map(event_id),
    })
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
fn lsn_only_mutant_arbitrary_event_cannot_poison_or_clear_fence() {
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
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(2),
        ))
        .is_err());
    assert_eq!(registry, before);
    assert!(matches!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
    assert!(matches!(
        registry.classify_claim(&claim("spawn-b", Some(7))),
        SpawnClaimability::Conflict(_)
    ));
}

#[test]
fn lsn_only_mutant_arbitrary_promotion_evidence_cannot_clear_fence() {
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
    assert!(matches!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
}

#[test]
fn prior_n_liveness_lsn_only_mutant_cannot_release_or_clear_fence() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    registry.observe(&sibling(3)).unwrap();
    let before = registry.clone();

    assert!(registry
        .observe(&disposition_event(
            4,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(2, Some(3)),
        ))
        .is_err());
    assert_eq!(registry, before);
    assert!(matches!(
        registry.delivery_fence(&runtime(7)),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
}

#[test]
fn wrong_event_kind_lsn_only_mutant_cannot_release_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, fresh_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    let before = registry.clone();

    assert!(registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(2, None),
        ))
        .is_err());
    assert_eq!(registry, before);
    assert!(matches!(
        registry.classify_claim(&claim("spawn-b", None)),
        SpawnClaimability::Conflict(_)
    ));
}

#[test]
fn other_claim_correlation_lsn_only_mutant_cannot_promote_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, continuation_accepted("spawn-a")))
        .unwrap();
    let mut other = fresh_accepted("spawn-other");
    other.claim.as_mut().unwrap().logical_target_id = Some(LogicalTargetId {
        value: "logical-other".to_owned(),
    });
    registry.observe(&accepted_event(2, other)).unwrap();
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
fn pre_acceptance_lsn_only_mutant_cannot_poison_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry.observe(&sibling(1)).unwrap();
    registry
        .observe(&accepted_event(2, continuation_accepted("spawn-a")))
        .unwrap();
    let before = registry.clone();

    assert!(registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::PoisonedPendingReconciliation,
            ambiguity(1),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn stale_or_wrong_adapter_lsn_only_mutant_cannot_release_claim() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, fresh_accepted("spawn-a")))
        .unwrap();
    registry.observe(&sibling(2)).unwrap();
    let before = registry.clone();
    let evidence = spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(
        SpawnClaimNoEffectRelease {
            proof: Some(NoExternalEffectProof {
                proof: Some(
                    no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(
                        AdapterRefusalBeforeDeliveryProof {
                            evidence_event_id: Some(event_id(2)),
                            adapter_id: Some(AdapterId {
                                value: "wrong-or-stale-adapter".to_owned(),
                            }),
                            adapter_generation: Some(Generation { value: 99 }),
                        },
                    ),
                ),
            }),
            exact_prior_liveness: None,
            prior_liveness_event_id: None,
        },
    );

    assert!(registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            evidence,
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn silence_without_delivered_ack_lsn_only_mutant_is_not_no_effect_proof() {
    let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
    registry
        .observe(&accepted_event(1, fresh_accepted("spawn-a")))
        .unwrap();
    registry
        .observe(&terminal_sibling(2, OperationState::Failed))
        .unwrap();
    let before = registry.clone();

    assert!(registry
        .observe(&disposition_event(
            3,
            "spawn-a",
            SpawnClaimDisposition::Active,
            SpawnClaimDisposition::ReleasedNoExternalEffect,
            core_no_effect(2, None),
        ))
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn every_placeholder_no_effect_variant_is_guarded_until_leaf_five() {
    let proofs = [
        NoExternalEffectProof {
            proof: Some(no_external_effect_proof::Proof::CorePreDeliveryTerminal(
                patchbay_contracts::patchbay::CorePreDeliveryTerminalProof {
                    decision_event_id: Some(event_id(2)),
                },
            )),
        },
        NoExternalEffectProof {
            proof: Some(
                no_external_effect_proof::Proof::AuthenticatedAdapterRefusalBeforeDelivery(
                    AdapterRefusalBeforeDeliveryProof {
                        evidence_event_id: Some(event_id(2)),
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        adapter_generation: Some(Generation { value: 3 }),
                    },
                ),
            ),
        },
        NoExternalEffectProof {
            proof: Some(
                no_external_effect_proof::Proof::ExactSupervisorPreLaunchFailure(
                    SupervisorPreLaunchFailureProof {
                        evidence_event_id: Some(event_id(2)),
                        adapter_id: Some(AdapterId {
                            value: "pi".to_owned(),
                        }),
                        adapter_generation: Some(Generation { value: 3 }),
                    },
                ),
            ),
        },
    ];
    for proof in proofs {
        let mut registry = SpawnClaimRegistry::new(domain()).unwrap();
        registry
            .observe(&accepted_event(1, fresh_accepted("spawn-a")))
            .unwrap();
        registry.observe(&sibling(2)).unwrap();
        let before = registry.clone();
        assert!(registry
            .observe(&disposition_event(
                3,
                "spawn-a",
                SpawnClaimDisposition::Active,
                SpawnClaimDisposition::ReleasedNoExternalEffect,
                spawn_claim_disposition_changed::Evidence::NoExternalEffectRelease(
                    SpawnClaimNoEffectRelease {
                        proof: Some(proof),
                        exact_prior_liveness: None,
                        prior_liveness_event_id: None,
                    },
                ),
            ))
            .is_err());
        assert_eq!(registry, before);
    }
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
    storage
        .append(
            &domain(),
            accepted_event(1, continuation_accepted("spawn-a")).payload,
        )
        .await
        .unwrap();

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
    let payloads = [
        accepted_event(1, continuation_accepted("spawn-a")).payload,
        sibling(2).payload,
    ];
    for payload in payloads {
        storage.append(&domain(), payload).await.unwrap();
    }
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
