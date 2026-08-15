use std::sync::Arc;

use patchbay_contracts::patchbay::{
    spawn_claim_event, AcceptedOperation, ActorEndpointRef, ActorId, AdapterId, AuditEventKind,
    AuthorityDomainId, CommandId, CommandTransition, ContinuationAuthorityProvenance, DeviceId,
    EndpointId, ExternalRuntimeRef, FailureCode, Generation, GrantId, IdempotencyKey,
    LogicalTargetId, Lsn, Operation, OperationKind, OperationState, RuntimeGenerationRef,
    RuntimeSessionId, SpawnClaimAccepted, SpawnClaimEvent, SpawnGenerationClaim,
    SpawnPendingReplacementFence, SpawnPriorWorkDisposition, StoredEventKind, StoredEventPayload,
    TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::CommandIndex,
    session::{
        encode_spawn_claim_event, rebuild_spawn_claims_from_log, SpawnClaimQuery,
        SpawnDeliveryFence, REPLACEMENT_PENDING_REASON,
    },
    storage::{
        AuditRecordDraft, AuditedStorage, DedupOutcome, RusqliteStorage, SpawnClaimDedupOutcome,
        Storage, StorageError, TargetKey,
    },
};
use prost::Message;
use prost_types::Timestamp;
use tokio::sync::Barrier;

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

fn adapter() -> AdapterId {
    AdapterId {
        value: "pi".to_owned(),
    }
}

fn sender() -> ActorEndpointRef {
    ActorEndpointRef {
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
    }
}

fn prior() -> RuntimeGenerationRef {
    RuntimeGenerationRef {
        logical_target_id: Some(LogicalTargetId {
            value: "logical-a".to_owned(),
        }),
        external_runtime: Some(ExternalRuntimeRef {
            adapter_id: Some(adapter()),
            deployment_scope: "machine-a".to_owned(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "runtime-a".to_owned(),
            }),
            generation: Some(Generation { value: 7 }),
        }),
    }
}

fn spawn_operation(command_id: &str, idempotency_key: &str) -> Operation {
    Operation {
        command_id: Some(command(command_id)),
        authority_domain_id: Some(domain()),
        sender: Some(sender()),
        kind: OperationKind::Spawn as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(adapter()),
            ..TargetScope::default()
        }),
        idempotency_key: idempotency_key.to_owned(),
        ..Operation::default()
    }
}

fn prior_operation(command_id: &str, idempotency_key: &str) -> Operation {
    let external = prior().external_runtime.unwrap();
    Operation {
        command_id: Some(command(command_id)),
        authority_domain_id: Some(domain()),
        sender: Some(sender()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: external.adapter_id,
            deployment_scope: external.deployment_scope,
            runtime_session_id: external.runtime_session_id,
            session_generation: external.generation,
            ..TargetScope::default()
        }),
        idempotency_key: idempotency_key.to_owned(),
        ..Operation::default()
    }
}

fn accepted_operation(operation: Operation, grant_id: &str) -> AcceptedOperation {
    AcceptedOperation {
        operation: Some(operation),
        authorizing_grant_id: Some(GrantId {
            value: grant_id.to_owned(),
        }),
    }
}

fn accepted_claim(command_id: &str, key: &str, replacement_grant_id: &str) -> SpawnClaimAccepted {
    let exact_prior = prior();
    SpawnClaimAccepted {
        accepted_operation: Some(accepted_operation(
            spawn_operation(command_id, key),
            "spawn-grant",
        )),
        claim: Some(SpawnGenerationClaim {
            authority_domain_id: Some(domain()),
            claim_operation_id: Some(command(command_id)),
            logical_target_id: exact_prior.logical_target_id.clone(),
            expected_prior: Some(exact_prior.clone()),
            claimed_generation: Some(Generation { value: 8 }),
        }),
        compound_authority: Some(ContinuationAuthorityProvenance {
            exact_prior: Some(exact_prior.clone()),
            replacement_grant_id: Some(GrantId {
                value: replacement_grant_id.to_owned(),
            }),
            replacement_authority_kind: OperationKind::SessionManagement as i32,
        }),
        pending_replacement: Some(SpawnPendingReplacementFence {
            exact_prior: Some(exact_prior),
            failure_code: FailureCode::Superseded as i32,
            reason_code: REPLACEMENT_PENDING_REASON.to_owned(),
        }),
        // The writer must replace caller input with its in-transaction derivation.
        prior_work_effects: Vec::new(),
    }
}

fn claim_audit(accepted: &SpawnClaimAccepted) -> AuditRecordDraft {
    let accepted_operation = accepted.accepted_operation.as_ref().unwrap();
    let operation = accepted_operation.operation.as_ref().unwrap();
    let mut audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
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
    audit.grant_id = accepted_operation.authorizing_grant_id.clone();
    audit.target_scope = operation.target_scope.clone();
    audit.reason_code = "operation_spawn".to_owned();
    audit
}

async fn append_claim(
    storage: &RusqliteStorage,
    accepted: SpawnClaimAccepted,
) -> Result<SpawnClaimDedupOutcome, StorageError> {
    let operation = accepted
        .accepted_operation
        .as_ref()
        .unwrap()
        .operation
        .as_ref()
        .unwrap();
    let key = IdempotencyKey {
        value: operation.idempotency_key.clone(),
    };
    let logical_payload = operation.encode_to_vec();
    let audit = claim_audit(&accepted);
    storage
        .append_spawn_claim_accepted(
            &domain(),
            &key,
            &TargetKey::new("pi-spawn".to_owned()).unwrap(),
            accepted,
            audit,
            logical_payload,
        )
        .await
}

async fn append_prior_operation(
    storage: &RusqliteStorage,
    operation: &Operation,
) -> Result<DedupOutcome, StorageError> {
    storage
        .append_dedup_with_payload(
            &domain(),
            &IdempotencyKey {
                value: operation.idempotency_key.clone(),
            },
            &TargetKey::new("runtime-n".to_owned()).unwrap(),
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation(operation.clone(), "session-grant").encode_to_vec(),
            },
            operation.encode_to_vec(),
        )
        .await
}

async fn replay_commands(storage: &RusqliteStorage) -> CommandIndex {
    patchbay_core::acceptance::rebuild_from_log(storage, &domain())
        .await
        .unwrap()
}

#[tokio::test]
async fn distinct_continuations_race_to_exactly_one_durable_owner() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let left_storage = storage.clone();
    let left_barrier = Arc::clone(&barrier);
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        append_claim(
            &left_storage,
            accepted_claim("spawn-left", "left-key", "replace-a"),
        )
        .await
    });
    let right_storage = storage.clone();
    let right_barrier = Arc::clone(&barrier);
    let right = tokio::spawn(async move {
        right_barrier.wait().await;
        append_claim(
            &right_storage,
            accepted_claim("spawn-right", "right-key", "replace-b"),
        )
        .await
    });
    barrier.wait().await;
    let outcomes = [left.await.unwrap(), right.await.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(SpawnClaimDedupOutcome::Appended(_))))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(StorageError::SpawnClaimConflict { .. })))
            .count(),
        1
    );

    let events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::SpawnClaim as i32)
            .count(),
        1
    );
    let replayed = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(replayed.records().count(), 1);
}

#[tokio::test]
async fn exact_retry_returns_original_claim_and_changed_payload_is_inert() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let original = accepted_claim("spawn-a", "spawn-key", "replacement-original");
    let first = append_claim(&storage, original.clone()).await.unwrap();
    let SpawnClaimDedupOutcome::Appended(first) = first else {
        panic!("first claim must append");
    };

    // A later resolver result may not substitute provenance into an exact
    // retry. Storage returns the original durable acceptance bytes.
    let retry_candidate = accepted_claim("spawn-a", "spawn-key", "replacement-newer");
    let retry = append_claim(&storage, retry_candidate).await.unwrap();
    let SpawnClaimDedupOutcome::Duplicate(retry) = retry else {
        panic!("exact retry must deduplicate");
    };
    assert_eq!(retry.source_event_id, first.source_event_id);
    assert_eq!(retry.accepted, first.accepted);
    assert_eq!(
        retry
            .accepted
            .compound_authority
            .as_ref()
            .and_then(|authority| authority.replacement_grant_id.as_ref())
            .map(|grant| grant.value.as_str()),
        Some("replacement-original")
    );

    let before_conflict = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap();
    let changed = accepted_claim("spawn-a", "spawn-key", "replacement-original");
    let operation = changed
        .accepted_operation
        .as_ref()
        .unwrap()
        .operation
        .as_ref()
        .unwrap();
    let conflict = storage
        .append_spawn_claim_accepted(
            &domain(),
            &IdempotencyKey {
                value: "spawn-key".to_owned(),
            },
            &TargetKey::new("pi-spawn".to_owned()).unwrap(),
            changed.clone(),
            claim_audit(&changed),
            [operation.encode_to_vec(), vec![0xff]].concat(),
        )
        .await;
    assert!(matches!(conflict, Err(StorageError::IdempotencyConflict)));
    assert_eq!(
        storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap(),
        before_conflict
    );
}

#[tokio::test]
async fn claim_acceptance_derives_complete_prior_effects_and_replay_suppresses_delivery() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let accepted_n = prior_operation("accepted-n", "accepted-n-key");
    let delivered_n = prior_operation("delivered-n", "delivered-n-key");
    append_prior_operation(&storage, &accepted_n).await.unwrap();
    append_prior_operation(&storage, &delivered_n)
        .await
        .unwrap();
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: delivered_n.command_id.clone(),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Delivered as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
        )
        .await
        .unwrap();

    let committed = append_claim(
        &storage,
        accepted_claim("spawn-a", "spawn-key", "replacement-grant"),
    )
    .await
    .unwrap();
    let SpawnClaimDedupOutcome::Appended(committed) = committed else {
        panic!("claim appends");
    };
    assert_eq!(committed.accepted.prior_work_effects.len(), 2);
    assert_eq!(
        committed.accepted.prior_work_effects[0].command_id,
        accepted_n.command_id
    );
    assert_eq!(
        SpawnPriorWorkDisposition::try_from(committed.accepted.prior_work_effects[0].disposition)
            .unwrap(),
        SpawnPriorWorkDisposition::SupersededBeforeOffer
    );
    assert_eq!(
        committed.accepted.prior_work_effects[1].command_id,
        delivered_n.command_id
    );
    assert_eq!(
        SpawnPriorWorkDisposition::try_from(committed.accepted.prior_work_effects[1].disposition)
            .unwrap(),
        SpawnPriorWorkDisposition::QuiesceOutcomeReconciliation
    );

    let replayed = replay_commands(&storage).await;
    let accepted_record = replayed
        .get_command(accepted_n.command_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(accepted_record.state, OperationState::Superseded);
    assert_eq!(accepted_record.failure_code, Some(FailureCode::Superseded));
    let delivered_record = replayed
        .get_command(delivered_n.command_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(delivered_record.state, OperationState::Delivered);
    assert!(replayed.delivery_is_suppressed(&delivered_record.command_id));

    let restarted_claims = rebuild_spawn_claims_from_log(&storage, &domain())
        .await
        .unwrap();
    assert!(matches!(
        restarted_claims.delivery_fence(&prior()),
        SpawnDeliveryFence::ReplacementPending { .. }
    ));
}

#[tokio::test]
async fn acceptance_fence_barrier_has_one_before_or_after_winner_and_exact_retry_survives() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let n_operation = prior_operation("n-instruct", "n-key");
    let barrier = Arc::new(Barrier::new(3));

    let operation_storage = storage.clone();
    let operation_barrier = Arc::clone(&barrier);
    let operation_for_task = n_operation.clone();
    let operation_task = tokio::spawn(async move {
        operation_barrier.wait().await;
        append_prior_operation(&operation_storage, &operation_for_task).await
    });
    let claim_storage = storage.clone();
    let claim_barrier = Arc::clone(&barrier);
    let claim_task = tokio::spawn(async move {
        claim_barrier.wait().await;
        append_claim(
            &claim_storage,
            accepted_claim("spawn-a", "spawn-key", "replacement-grant"),
        )
        .await
    });
    barrier.wait().await;
    let operation_outcome = operation_task.await.unwrap();
    let claim_outcome = claim_task.await.unwrap().unwrap();
    assert!(matches!(claim_outcome, SpawnClaimDedupOutcome::Appended(_)));

    let replayed = replay_commands(&storage).await;
    match operation_outcome {
        Ok(DedupOutcome::Appended(_)) => {
            let record = replayed
                .get_command(n_operation.command_id.as_ref().unwrap())
                .expect("before-fence operation is explicitly resolved");
            assert_eq!(record.state, OperationState::Superseded);
        }
        Err(StorageError::ReplacementPending { .. }) => {
            assert!(replayed
                .get_command(n_operation.command_id.as_ref().unwrap())
                .is_none());
        }
        other => panic!("unexpected barrier outcome: {other:?}"),
    }

    let rejected_after = prior_operation("after-fence", "after-fence-key");
    assert!(matches!(
        append_prior_operation(&storage, &rejected_after).await,
        Err(StorageError::ReplacementPending { .. })
    ));

    // Exact pre-fence records remain reconcilable after activation. Ensure the
    // branch exists deterministically even if this run's barrier chose after.
    let retry_storage = RusqliteStorage::open_in_memory().unwrap();
    append_prior_operation(&retry_storage, &n_operation)
        .await
        .unwrap();
    append_claim(
        &retry_storage,
        accepted_claim("spawn-b", "spawn-b-key", "replacement-grant"),
    )
    .await
    .unwrap();
    assert!(matches!(
        append_prior_operation(&retry_storage, &n_operation).await,
        Ok(DedupOutcome::Duplicate(_))
    ));
}

#[tokio::test]
async fn generic_routes_cannot_commit_spawn_claim_acceptance() {
    let raw = RusqliteStorage::open_in_memory().unwrap();
    let accepted = accepted_claim("spawn-a", "spawn-key", "replacement-grant");
    let source = encode_spawn_claim_event(&SpawnClaimEvent {
        authority_domain_id: Some(domain()),
        mutation: Some(spawn_claim_event::Mutation::Accepted(accepted.clone())),
    });
    assert!(matches!(
        raw.append(&domain(), source.clone()).await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(matches!(
        raw.append_dedup(
            &domain(),
            &IdempotencyKey {
                value: "generic-key".to_owned()
            },
            &TargetKey::new("generic-target".to_owned()).unwrap(),
            source.clone(),
        )
        .await,
        Err(StorageError::UnsupportedOperation)
    ));

    let audited = AuditedStorage::new(raw.clone());
    assert!(matches!(
        audited
            .append_audited(&domain(), source, claim_audit(&accepted))
            .await,
        Err(StorageError::UnsupportedOperation)
    ));
    assert!(raw
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .is_empty());
}
