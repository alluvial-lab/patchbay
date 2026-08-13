use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, EventId, ExternalRuntimeRef, Generation,
    LogicalTargetCandidateReserved, LogicalTargetCreated, LogicalTargetId, Lsn,
    RuntimeGenerationRef, RuntimeSessionId,
};
use patchbay_core::{
    session::{
        events, rebuild_from_log, ExternalRuntimeOwnership, LogicalTargetError,
        LogicalTargetRegistry, SessionError, SessionRegistry,
    },
    storage::{RecordedEvent, RusqliteStorage, Storage},
};

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn target(value: &str) -> LogicalTargetId {
    LogicalTargetId {
        value: value.to_owned(),
    }
}

fn adapter(value: &str) -> AdapterId {
    AdapterId {
        value: value.to_owned(),
    }
}

fn external(adapter_id: &str, scope: &str, runtime: &str, generation: u64) -> ExternalRuntimeRef {
    ExternalRuntimeRef {
        adapter_id: Some(adapter(adapter_id)),
        deployment_scope: scope.to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: runtime.to_owned(),
        }),
        generation: Some(Generation { value: generation }),
    }
}

fn current(
    logical_target_id: &LogicalTargetId,
    external: &ExternalRuntimeRef,
) -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(logical_target_id.clone()),
        external_runtime_ref: Some(external.clone()),
    }
}

fn registry() -> LogicalTargetRegistry {
    LogicalTargetRegistry::new(domain("authority-main")).unwrap()
}

#[test]
fn boundary_validation_rejects_invalid_identity_before_mutation() {
    let mut registry = registry();
    let original = registry.clone();
    assert_eq!(
        registry.create(target(""), adapter("pi"), "machine-a".to_owned()),
        Err(LogicalTargetError::EmptyLogicalTargetId)
    );
    assert_eq!(registry, original);

    registry
        .create(target("target-a"), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    let created = registry.clone();
    for invalid in [
        external("pi", "machine-a", "runtime-a", 0),
        external("", "machine-a", "runtime-a", 1),
        external("pi", "machine-a", "", 1),
        external("pi", "machine a", "runtime-a", 1),
    ] {
        assert!(registry
            .reserve_candidate(&target("target-a"), invalid)
            .is_err());
        assert_eq!(
            registry, created,
            "failed validation mutated identity state"
        );
    }
}

#[test]
fn slot_transitions_are_exact_and_tombstones_retain_ownership() {
    let logical = target("target-a");
    let generation_one = external("pi", "machine-a", "runtime-a", 1);
    let generation_two = external("pi", "machine-a", "runtime-b", 2);
    let mut registry = registry();
    registry
        .create(logical.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    registry
        .assign_initial_current(&logical, generation_one.clone())
        .unwrap();
    registry
        .reserve_candidate(&logical, generation_two.clone())
        .unwrap();

    let before_mismatch = registry.clone();
    let wrong_current = current(&logical, &external("pi", "machine-a", "other", 1));
    assert_eq!(
        registry.commit_reserved_candidate(&logical, Some(&wrong_current), &generation_two, 7),
        Err(LogicalTargetError::RuntimeRefMismatch)
    );
    assert_eq!(registry, before_mismatch);

    registry
        .commit_reserved_candidate(
            &logical,
            Some(&current(&logical, &generation_one)),
            &generation_two,
            7,
        )
        .unwrap();
    let record = registry.get(&logical).unwrap();
    assert_eq!(record.current, Some(current(&logical, &generation_two)));
    assert!(record.reserved_candidate.is_none());
    assert_eq!(record.tombstones.len(), 1);
    assert_eq!(registry.owner_of(&generation_one), Some(&logical));
    assert_eq!(registry.owner_of(&generation_two), Some(&logical));

    let other = target("target-b");
    registry
        .create(other.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    assert_eq!(
        registry.reserve_candidate(&other, generation_one.clone()),
        Err(LogicalTargetError::DuplicateNativeReference {
            owner: logical,
            attempted_owner: other,
        })
    );
}

#[test]
fn release_removes_only_the_candidate_reservation() {
    let logical = target("target-a");
    let candidate = external("pi", "machine-a", "runtime-a", 1);
    let mut registry = registry();
    registry
        .create(logical.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    registry
        .reserve_candidate(&logical, candidate.clone())
        .unwrap();
    registry.release_candidate(&logical, &candidate).unwrap();
    assert_eq!(registry.owner_of(&candidate), None);
    assert!(registry.get(&logical).unwrap().reserved_candidate.is_none());
}

#[test]
fn cross_adapter_scope_and_runtime_ref_mismatches_are_non_mutating() {
    let logical = target("target-a");
    let mut registry = registry();
    registry
        .create(logical.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    let original = registry.clone();
    assert_eq!(
        registry.reserve_candidate(&logical, external("other", "machine-a", "runtime-a", 1)),
        Err(LogicalTargetError::TargetScopeMutation)
    );
    assert_eq!(registry, original);
    assert_eq!(
        registry.reserve_candidate(&logical, external("pi", "machine-b", "runtime-a", 1)),
        Err(LogicalTargetError::TargetScopeMutation)
    );
    assert_eq!(registry, original);
}

#[test]
fn cross_domain_log_event_rejects_before_logical_projection_mutation() {
    let authority = domain("authority-main");
    let mut registry = SessionRegistry::new(authority.clone()).unwrap();
    let original = registry.clone();
    let payload = events::encode(&events::logical_target_created(
        authority.clone(),
        LogicalTargetCreated {
            logical_target_id: Some(target("target-a")),
            adapter_id: Some(adapter("pi")),
            deployment_scope: "machine-a".to_owned(),
        },
    ));
    let error = registry
        .observe(&RecordedEvent {
            event_id: EventId {
                authority_domain_id: Some(domain("authority-other")),
                lsn: Some(Lsn { value: 1 }),
            },
            payload,
        })
        .unwrap_err();
    assert!(matches!(error, SessionError::AuthorityDomainMismatch { .. }));
    assert_eq!(registry, original);
}

#[tokio::test]
async fn hot_fold_restart_replay_and_duplicate_rejection_are_identical() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let authority = domain("authority-main");
    let first = target("target-a");
    let second = target("target-b");
    let exact = external("pi", "machine-a", "runtime-a", 1);
    let source = [
        events::logical_target_created(
            authority.clone(),
            LogicalTargetCreated {
                logical_target_id: Some(first.clone()),
                adapter_id: Some(adapter("pi")),
                deployment_scope: "machine-a".to_owned(),
            },
        ),
        events::logical_target_created(
            authority.clone(),
            LogicalTargetCreated {
                logical_target_id: Some(second.clone()),
                adapter_id: Some(adapter("pi")),
                deployment_scope: "machine-a".to_owned(),
            },
        ),
        events::logical_target_candidate_reserved(
            authority.clone(),
            LogicalTargetCandidateReserved {
                logical_target_id: Some(first.clone()),
                external_runtime_ref: Some(exact.clone()),
            },
        ),
    ];

    let mut hot = SessionRegistry::new(authority.clone()).unwrap();
    for source_event in &source {
        let payload = events::encode(source_event);
        let event_id = storage.append(&authority, payload.clone()).await.unwrap();
        hot.observe(&RecordedEvent { event_id, payload }).unwrap();
    }
    let replayed = rebuild_from_log(&storage, &authority).await.unwrap();
    assert_eq!(hot, replayed);
    assert_eq!(replayed.logical_targets().owner_of(&exact), Some(&first));

    let before = hot.clone();
    let duplicate = events::logical_target_candidate_reserved(
        authority.clone(),
        LogicalTargetCandidateReserved {
            logical_target_id: Some(second.clone()),
            external_runtime_ref: Some(exact.clone()),
        },
    );
    let duplicate_payload = events::encode(&duplicate);
    let duplicate_event_id = storage
        .append(&authority, duplicate_payload.clone())
        .await
        .unwrap();
    let duplicate_record = RecordedEvent {
        event_id: duplicate_event_id,
        payload: duplicate_payload,
    };
    assert!(matches!(
        hot.observe(&duplicate_record),
        Err(SessionError::LogicalTarget(
            LogicalTargetError::DuplicateNativeReference { .. }
        ))
    ));
    assert_eq!(hot, before);
    assert!(matches!(
        rebuild_from_log(&storage, &authority).await,
        Err(SessionError::LogicalTarget(
            LogicalTargetError::DuplicateNativeReference { .. }
        ))
    ));
}

#[test]
fn checkpoint_rebuild_restores_all_slots_and_reverse_exclusivity() {
    let authority = domain("authority-main");
    let logical = target("target-a");
    let other = target("target-b");
    let generation_one = external("pi", "machine-a", "runtime-a", 1);
    let generation_two = external("pi", "machine-a", "runtime-b", 2);
    let generation_three = external("pi", "machine-a", "runtime-c", 3);
    let mut original = LogicalTargetRegistry::new(authority.clone()).unwrap();
    original
        .create(logical.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    original
        .assign_initial_current(&logical, generation_one.clone())
        .unwrap();
    original
        .reserve_candidate(&logical, generation_two.clone())
        .unwrap();
    original
        .commit_reserved_candidate(
            &logical,
            Some(&current(&logical, &generation_one)),
            &generation_two,
            5,
        )
        .unwrap();
    original
        .reserve_candidate(&logical, generation_three.clone())
        .unwrap();

    let mut recovered =
        LogicalTargetRegistry::from_checkpoint(authority, 7, original.checkpoint_records()).unwrap();
    assert_eq!(recovered, original);
    recovered
        .create(other.clone(), adapter("pi"), "machine-a".to_owned())
        .unwrap();
    for external in [generation_one, generation_two, generation_three] {
        assert!(matches!(
            recovered.reserve_candidate(&other, external),
            Err(LogicalTargetError::DuplicateNativeReference { .. })
        ));
    }
}
