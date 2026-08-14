use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use patchbay_contracts::patchbay::{
    session_state_event, typed_correlation, AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterId, AuditEventKind, AuthorityDomainId, CommandId, CommandTransition, DescendantGrant,
    DescendantGrantProvenance, DeviceId, EndpointId, EventId, FailureCode, Generation, Grant,
    GrantId, GrantProvenance, GrantRevocationPolicy, IdempotencyKey, Lsn, Observation,
    ObservationKind, Operation, OperationKind, OperationState, Revocation, RuntimeSessionId,
    SessionRegistered, SessionStateEvent, StoredEventKind, StoredEventPayload, TargetScope,
    TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    authority::{
        ingest_descendant_grant, ingest_grant, ingest_revocation, rebuild_from_log, AuthorityError,
        AuthorityRegistry, DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::{
        AuditPageSpec, AuditRecordDraft, DedupOutcome, GrantAppendOutcome, GrantIdentityKey,
        RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot, TargetKey,
    },
};
use prost::Message;
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn actor() -> ActorId {
    ActorId {
        value: "operator".to_owned(),
    }
}

fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn fleet_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    }
}

fn session_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        runtime_session_id: Some(RuntimeSessionId {
            value: "session-1".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    }
}

fn grant(id: &str) -> Grant {
    Grant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(actor()),
        subject_endpoint_class: "web".to_owned(),
        target_scope: Some(fleet_scope()),
        allowed_operation_kinds: vec![OperationKind::Spawn as i32],
        provenance: Some(GrantProvenance {
            reason: "test fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn descendant_grant(id: &str, spawning_grant_id: &str) -> DescendantGrant {
    DescendantGrant {
        grant_id: Some(grant_id(id)),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(actor()),
        subject_endpoint_class: "web".to_owned(),
        target_scope: Some(session_scope()),
        allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        provenance: Some(DescendantGrantProvenance {
            spawning_grant_id: Some(grant_id(spawning_grant_id)),
            ..DescendantGrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..DescendantGrant::default()
    }
}

async fn valid_descendant_candidate(
    storage: &RusqliteStorage,
    parent_id: &str,
) -> Result<(GrantId, DescendantGrant), AuthorityError> {
    let command_id = CommandId {
        value: "spawn-1".to_owned(),
    };
    let correlation = TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
    };
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: AcceptedOperation {
                    operation: Some(Operation {
                        command_id: Some(command_id.clone()),
                        authority_domain_id: Some(domain()),
                        sender: Some(ActorEndpointRef {
                            actor_id: Some(actor()),
                            ..ActorEndpointRef::default()
                        }),
                        kind: OperationKind::Spawn as i32,
                        target_scope: Some(fleet_scope()),
                        idempotency_key: "spawn-1-key".to_owned(),
                        ..Operation::default()
                    }),
                    authorizing_grant_id: Some(grant_id(parent_id)),
                }
                .encode_to_vec(),
            },
        )
        .await?;
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(command_id.clone()),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Delivered as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
        )
        .await?;
    let result = Observation {
        authority_domain_id: Some(domain()),
        kind: ObservationKind::Result as i32,
        correlations: vec![correlation.clone()],
        target_scope: Some(fleet_scope()),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let mut result_audit = AuditRecordDraft::new(
        Timestamp {
            seconds: 10,
            nanos: 0,
        },
        AuditEventKind::CommandRunning,
    );
    result_audit.command_id = Some(command_id.clone());
    result_audit.target_scope = result.target_scope.clone();
    result_audit.reason_code = "spawn_completion_deferred".to_owned();
    let source_event_id = storage
        .append_spawn_result_deferred_audited(&domain(), result, result_audit)
        .await?
        .source_event_id;
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::SessionState as i32,
                payload: SessionStateEvent {
                    authority_domain_id: Some(domain()),
                    mutation: Some(session_state_event::Mutation::Registered(
                        SessionRegistered {
                            adapter_id: session_scope().adapter_id,
                            deployment_scope: "machine-a".to_owned(),
                            runtime_session_id: session_scope().runtime_session_id,
                            session_generation: Some(Generation { value: 1 }),
                            spawn_origin: Some(correlation),
                            ..SessionRegistered::default()
                        },
                    )),
                }
                .encode_to_vec(),
            },
        )
        .await?;
    let occurred_at = Timestamp {
        seconds: 10,
        nanos: 0,
    };
    let mut audit = AuditRecordDraft::new(occurred_at, AuditEventKind::CommandCompleted);
    audit.actor_id = Some(actor());
    audit.command_id = Some(command_id.clone());
    audit.grant_id = Some(grant_id(parent_id));
    audit.target_scope = Some(session_scope());
    audit.reason_code = "spawn_completion".to_owned();
    audit.source_event_id = Some(source_event_id);
    let audit_id = storage.append_audit(&domain(), audit).await?;
    let id = grant_id("desc:authority-main:spawn-1");
    let mut descendant = descendant_grant(&id.value, parent_id);
    descendant.subject_endpoint_class.clear();
    descendant.provenance = Some(DescendantGrantProvenance {
        spawn_operation_id: Some(command_id),
        spawning_grant_id: Some(grant_id(parent_id)),
        continuation_authority: None,
    });
    descendant.created_at = Some(occurred_at);
    descendant.audit_id = Some(audit_id);
    Ok((id, descendant))
}

async fn ingest_valid_descendant(
    storage: &RusqliteStorage,
    registry: &mut AuthorityRegistry,
    parent_id: &str,
) -> Result<(patchbay_contracts::patchbay::EventId, GrantId), AuthorityError> {
    let (id, descendant) = valid_descendant_candidate(storage, parent_id).await?;
    let event_id = ingest_descendant_grant(storage, registry, &domain(), descendant).await?;
    Ok((event_id, id))
}

fn revocation(id: &str) -> Revocation {
    Revocation {
        authority_domain_id: Some(domain()),
        grant_id: Some(grant_id(id)),
        revocation_generation: Some(Generation { value: 1 }),
        accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
        reason: "test revocation".to_owned(),
        ..Revocation::default()
    }
}

async fn events(storage: &RusqliteStorage) -> Vec<RecordedEvent> {
    storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("the in-memory authority log remains readable")
}

async fn append_semantically_invalid_grant(storage: &RusqliteStorage) {
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Grant as i32,
                payload: Grant::default().encode_to_vec(),
            },
        )
        .await
        .expect("the storage boundary admits a known-kind corruption fixture");
}

#[derive(Clone)]
struct LoseFirstGrantAppendAcknowledgement {
    inner: RusqliteStorage,
    lose_next_append: Arc<AtomicBool>,
}

impl LoseFirstGrantAppendAcknowledgement {
    fn new(inner: RusqliteStorage) -> Self {
        Self {
            inner,
            lose_next_append: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl Storage for LoseFirstGrantAppendAcknowledgement {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn append_grant_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        identity: &GrantIdentityKey,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<GrantAppendOutcome, StorageError> {
        let outcome = self
            .inner
            .append_grant_audited(authority_domain_id, identity, source, audit)
            .await?;
        if matches!(outcome, GrantAppendOutcome::Appended(_))
            && self.lose_next_append.swap(false, Ordering::SeqCst)
        {
            return Err(StorageError::WriteFailed {
                message: "synthetic lost grant append acknowledgement".to_owned(),
                retryable: true,
            });
        }
        Ok(outcome)
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }
}

#[tokio::test]
async fn ingest_grant_writes_event_and_warms_registry() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();

    let event_id = ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .expect("the valid grant must be ingested");

    assert_eq!(event_id.authority_domain_id, Some(domain()));
    assert_eq!(event_id.lsn, Some(Lsn { value: 1 }));
    let record = registry
        .get_grant(&grant_id("parent"))
        .expect("ingestion must warm the grant projection");
    assert!(record.is_live());
    assert_eq!(record.allowed_operation_kinds, [OperationKind::Spawn]);

    let committed = events(&storage).await;
    assert_eq!(committed.len(), 2);
    assert_eq!(
        StoredEventKind::try_from(committed[0].payload.kind).unwrap(),
        StoredEventKind::Grant
    );
    assert_eq!(
        StoredEventKind::try_from(committed[1].payload.kind).unwrap(),
        StoredEventKind::AuditRecord
    );
}

#[tokio::test]
async fn grant_warming_tail_is_atomic_on_late_semantic_failure() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let candidate = grant("atomic-warm");
    let mut original = AuthorityRegistry::new();
    ingest_grant(&storage, &mut original, &domain(), candidate.clone())
        .await
        .expect("the canonical source must be durable before corruption is injected");
    append_semantically_invalid_grant(&storage).await;
    let durable_before = events(&storage).await;

    let mut fresh = AuthorityRegistry::new();
    let projection_before = fresh.clone();
    ingest_grant(&storage, &mut fresh, &domain(), candidate)
        .await
        .expect_err("a later semantically invalid record must reject the complete warm");

    assert_eq!(
        fresh, projection_before,
        "removing isolated staging would leak the valid leading grant"
    );
    assert_eq!(events(&storage).await, durable_before);
}

#[tokio::test]
async fn descendant_preflight_warm_is_atomic_on_late_semantic_failure() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut fixture_projection = AuthorityRegistry::new();
    ingest_grant(
        &storage,
        &mut fixture_projection,
        &domain(),
        grant("atomic-parent"),
    )
    .await
    .unwrap();
    let (_, candidate) = valid_descendant_candidate(&storage, "atomic-parent")
        .await
        .unwrap();
    append_semantically_invalid_grant(&storage).await;
    let durable_before = events(&storage).await;

    let mut fresh = AuthorityRegistry::new();
    let projection_before = fresh.clone();
    ingest_descendant_grant(&storage, &mut fresh, &domain(), candidate)
        .await
        .expect_err("a corrupt later record must reject before descendant append");

    assert_eq!(
        fresh, projection_before,
        "removing isolated preflight staging would leak the valid parent grant"
    );
    assert_eq!(events(&storage).await, durable_before);
}

#[tokio::test]
async fn normal_grant_retry_returns_original_id_without_source_or_audit_duplication() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let candidate = grant("retry");
    let mut warm = AuthorityRegistry::new();
    let first = ingest_grant(&storage, &mut warm, &domain(), candidate.clone())
        .await
        .unwrap();
    let prefix = events(&storage).await;
    let warm_before = warm.clone();
    assert_eq!(
        ingest_grant(&storage, &mut warm, &domain(), candidate.clone(),)
            .await
            .unwrap(),
        first
    );
    assert_eq!(warm, warm_before);
    assert_eq!(events(&storage).await, prefix);

    let mut fresh = AuthorityRegistry::new();
    assert_eq!(
        ingest_grant(&storage, &mut fresh, &domain(), candidate)
            .await
            .unwrap(),
        first
    );
    assert_eq!(events(&storage).await, prefix);
    assert_eq!(fresh, rebuild_from_log(&storage, &domain()).await.unwrap());

    let mut audit_filter = AuditPageSpec {
        kinds: vec![AuditEventKind::GrantCreated, AuditEventKind::GrantChanged],
        actor_id: None,
        endpoint_id: None,
        command_id: None,
        grant_id: Some(grant_id("retry")),
        target: None,
        failure_codes: vec![],
        reason_codes: vec![],
        occurred_from: None,
        occurred_before: None,
        before_lsn: None,
        limit: 10,
    };
    let audits = storage
        .query_audit(&domain(), audit_filter.clone())
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
    assert_eq!(audits.records[0].kind, AuditEventKind::GrantCreated as i32);
    audit_filter.kinds = vec![AuditEventKind::GrantChanged];
    assert!(storage
        .query_audit(&domain(), audit_filter)
        .await
        .unwrap()
        .records
        .is_empty());
}

#[tokio::test]
async fn normal_grant_retry_after_revocation_catches_a_fresh_projection_up_to_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let candidate = grant("retry-after-revocation");
    let mut warm = AuthorityRegistry::new();
    let earliest_id = ingest_grant(&storage, &mut warm, &domain(), candidate.clone())
        .await
        .unwrap();
    ingest_revocation(
        &storage,
        &mut warm,
        &domain(),
        revocation("retry-after-revocation"),
    )
    .await
    .unwrap();
    let prefix = events(&storage).await;

    let mut fresh = AuthorityRegistry::new();
    let retry_id = ingest_grant(&storage, &mut fresh, &domain(), candidate)
        .await
        .unwrap();
    assert_eq!(retry_id, earliest_id);
    assert_eq!(events(&storage).await, prefix);
    assert_eq!(fresh, rebuild_from_log(&storage, &domain()).await.unwrap());
    assert!(fresh
        .get_grant(&grant_id("retry-after-revocation"))
        .expect("the retried grant remains projected")
        .is_revoked());

    let audits = storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantCreated],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(grant_id("retry-after-revocation")),
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["grant_created".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
    assert_eq!(audits.records[0].source_event_id, Some(earliest_id));
}

#[tokio::test]
async fn committed_but_unacknowledged_normal_and_descendant_grants_retry_to_original_ids() {
    let normal_inner = RusqliteStorage::open_in_memory().unwrap();
    let normal_storage = LoseFirstGrantAppendAcknowledgement::new(normal_inner.clone());
    let normal = grant("ambiguous-normal");
    let mut normal_projection = AuthorityRegistry::new();
    assert!(matches!(
        ingest_grant(
            &normal_storage,
            &mut normal_projection,
            &domain(),
            normal.clone(),
        )
        .await,
        Err(AuthorityError::Storage(StorageError::WriteFailed {
            retryable: true,
            ..
        }))
    ));
    let normal_prefix = events(&normal_inner).await;
    let normal_source_id = normal_prefix
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::Grant as i32)
        .unwrap()
        .event_id
        .clone();
    let mut normal_fresh = AuthorityRegistry::new();
    assert_eq!(
        ingest_grant(&normal_storage, &mut normal_fresh, &domain(), normal,)
            .await
            .unwrap(),
        normal_source_id
    );
    assert_eq!(events(&normal_inner).await, normal_prefix);

    let descendant_inner = RusqliteStorage::open_in_memory().unwrap();
    let mut descendant_projection = AuthorityRegistry::new();
    ingest_grant(
        &descendant_inner,
        &mut descendant_projection,
        &domain(),
        grant("parent"),
    )
    .await
    .unwrap();
    let (descendant_id, descendant) = valid_descendant_candidate(&descendant_inner, "parent")
        .await
        .unwrap();
    let descendant_storage = LoseFirstGrantAppendAcknowledgement::new(descendant_inner.clone());
    assert!(matches!(
        ingest_descendant_grant(
            &descendant_storage,
            &mut descendant_projection,
            &domain(),
            descendant.clone(),
        )
        .await,
        Err(AuthorityError::Storage(StorageError::WriteFailed {
            retryable: true,
            ..
        }))
    ));
    let descendant_prefix = events(&descendant_inner).await;
    let descendant_source_id = descendant_prefix
        .iter()
        .find(|event| event.payload.kind == StoredEventKind::DescendantGrant as i32)
        .unwrap()
        .event_id
        .clone();
    let mut descendant_fresh = AuthorityRegistry::new();
    assert_eq!(
        ingest_descendant_grant(
            &descendant_storage,
            &mut descendant_fresh,
            &domain(),
            descendant,
        )
        .await
        .unwrap(),
        descendant_source_id
    );
    assert_eq!(events(&descendant_inner).await, descendant_prefix);

    for (storage, grant_id, reason) in [
        (&normal_inner, grant_id("ambiguous-normal"), "grant_created"),
        (&descendant_inner, descendant_id, "descendant_grant_created"),
    ] {
        let audits = storage
            .query_audit(
                &domain(),
                AuditPageSpec {
                    kinds: vec![AuditEventKind::GrantCreated],
                    actor_id: None,
                    endpoint_id: None,
                    command_id: None,
                    grant_id: Some(grant_id),
                    target: None,
                    failure_codes: vec![],
                    reason_codes: vec![reason.to_owned()],
                    occurred_from: None,
                    occurred_before: None,
                    before_lsn: None,
                    limit: 10,
                },
            )
            .await
            .unwrap();
        assert_eq!(audits.records.len(), 1);
    }
}

#[tokio::test]
async fn independent_projections_race_through_storage_identity_not_a_process_gate() {
    let exact_storage = RusqliteStorage::open_in_memory().unwrap();
    let exact_barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut exact_tasks = Vec::new();
    for _ in 0..2 {
        let storage = exact_storage.clone();
        let barrier = exact_barrier.clone();
        exact_tasks.push(tokio::spawn(async move {
            let mut projection = AuthorityRegistry::new();
            barrier.wait().await;
            ingest_grant(&storage, &mut projection, &domain(), grant("raced-exact")).await
        }));
    }
    exact_barrier.wait().await;
    let exact_ids = [
        exact_tasks.remove(0).await.unwrap().unwrap(),
        exact_tasks.remove(0).await.unwrap().unwrap(),
    ];
    assert_eq!(exact_ids[0], exact_ids[1]);
    assert_eq!(events(&exact_storage).await.len(), 2);

    let conflict_storage = RusqliteStorage::open_in_memory().unwrap();
    let conflict_barrier = Arc::new(tokio::sync::Barrier::new(3));
    let mut conflict_tasks = Vec::new();
    for kind in [OperationKind::Spawn, OperationKind::Query] {
        let storage = conflict_storage.clone();
        let barrier = conflict_barrier.clone();
        conflict_tasks.push(tokio::spawn(async move {
            let mut projection = AuthorityRegistry::new();
            let mut candidate = grant("raced-conflict");
            candidate.allowed_operation_kinds = vec![kind as i32];
            barrier.wait().await;
            ingest_grant(&storage, &mut projection, &domain(), candidate).await
        }));
    }
    conflict_barrier.wait().await;
    let conflict_results = [
        conflict_tasks.remove(0).await.unwrap(),
        conflict_tasks.remove(0).await.unwrap(),
    ];
    assert_eq!(
        conflict_results
            .iter()
            .filter(|result| result.is_ok())
            .count(),
        1
    );
    assert_eq!(
        conflict_results
            .iter()
            .filter(|result| matches!(result, Err(AuthorityError::CorruptLog(_))))
            .count(),
        1
    );
    assert_eq!(events(&conflict_storage).await.len(), 2);
    rebuild_from_log(&conflict_storage, &domain())
        .await
        .expect("the transactional winner leaves replayable authority history");
    let audits = conflict_storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantCreated],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(grant_id("raced-conflict")),
                target: None,
                failure_codes: vec![],
                reason_codes: vec![],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
}

#[tokio::test]
async fn normal_grant_changed_content_conflicts_before_log_or_projection_mutation() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    let original = grant("immutable");
    ingest_grant(&storage, &mut registry, &domain(), original.clone())
        .await
        .unwrap();
    let prefix = events(&storage).await;
    let projection = registry.clone();
    let mut changed = original;
    changed.allowed_operation_kinds = vec![OperationKind::Query as i32];

    let error = ingest_grant(&storage, &mut registry, &domain(), changed)
        .await
        .expect_err("same identity with changed canonical content must conflict");
    assert!(matches!(
        error,
        AuthorityError::CorruptLog(message)
            if message.contains("immutable") && message.contains("source LSN 1")
    ));
    assert_eq!(events(&storage).await, prefix);
    assert_eq!(registry, projection);
}

#[tokio::test]
async fn descendant_with_wrong_allowed_kinds_fails_before_write() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    let mut invalid = descendant_grant("descendant", "parent");
    invalid.allowed_operation_kinds.pop();

    let error = ingest_descendant_grant(&storage, &mut registry, &domain(), invalid)
        .await
        .expect_err("a partial descendant kind set must fail fast");

    assert!(
        matches!(error, AuthorityError::InvalidGrant(message) if message.contains("exactly the canonical"))
    );
    assert!(events(&storage).await.is_empty());
    assert!(registry.get_grant(&grant_id("descendant")).is_none());
}

#[tokio::test]
async fn descendant_with_canonical_kind_set_succeeds() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .unwrap();

    let (event_id, descendant_id) = ingest_valid_descendant(&storage, &mut registry, "parent")
        .await
        .expect("the canonical descendant grant must be ingested");

    assert_eq!(event_id.lsn, Some(Lsn { value: 9 }));
    let record = registry
        .get_grant(&descendant_id)
        .expect("ingestion must warm the descendant projection");
    assert!(record.is_descendant);
    assert_eq!(
        record.allowed_operation_kinds,
        DESCENDANT_GRANT_ALLOWED_KINDS
    );
}

#[tokio::test]
async fn descendant_retry_reuses_original_source_and_rebuilds_a_fresh_projection() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .unwrap();
    let (descendant_id, candidate) = valid_descendant_candidate(&storage, "parent")
        .await
        .unwrap();
    let first = ingest_descendant_grant(&storage, &mut registry, &domain(), candidate.clone())
        .await
        .unwrap();
    let prefix = events(&storage).await;

    let mut fresh = AuthorityRegistry::new();
    let retry = ingest_descendant_grant(&storage, &mut fresh, &domain(), candidate)
        .await
        .unwrap();
    assert_eq!(retry, first);
    assert_eq!(events(&storage).await, prefix);
    assert_eq!(fresh, rebuild_from_log(&storage, &domain()).await.unwrap());
    assert!(fresh.get_grant(&descendant_id).unwrap().is_descendant);

    let audits = storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantCreated],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(descendant_id),
                target: None,
                failure_codes: vec![],
                reason_codes: vec!["descendant_grant_created".to_owned()],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(audits.records.len(), 1);
    assert_eq!(audits.records[0].source_event_id, Some(first));
}

#[tokio::test]
async fn normal_and_descendant_grants_share_one_immutable_identity_namespace() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .unwrap();
    let (descendant_id, candidate) = valid_descendant_candidate(&storage, "parent")
        .await
        .unwrap();
    let mut normal = grant(&descendant_id.value);
    normal.provenance.as_mut().unwrap().reason = "cross-kind collision".to_owned();
    ingest_grant(&storage, &mut registry, &domain(), normal)
        .await
        .unwrap();
    let prefix = events(&storage).await;
    let first_identity = registry.get_grant(&descendant_id).unwrap().clone();

    let error = ingest_descendant_grant(&storage, &mut registry, &domain(), candidate)
        .await
        .expect_err("normal and descendant kinds cannot partition grant identity");
    assert!(matches!(
        error,
        AuthorityError::CorruptLog(message)
            if message.contains(&descendant_id.value) && message.contains("source LSN 9")
    ));
    assert_eq!(events(&storage).await, prefix);
    assert_eq!(registry.get_grant(&descendant_id), Some(&first_identity));
    assert_eq!(
        registry,
        rebuild_from_log(&storage, &domain()).await.unwrap()
    );
}

#[tokio::test]
async fn self_consistent_source_audit_and_grant_without_accepted_context_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("forged-descendant-prefix.sqlite3");
    let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
    let mut registry = AuthorityRegistry::new();
    let command_id = CommandId {
        value: "spawn-1".to_owned(),
    };
    let source = StoredEventPayload {
        kind: StoredEventKind::Observation as i32,
        payload: Observation {
            authority_domain_id: Some(domain()),
            kind: ObservationKind::Result as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
            }],
            target_scope: Some(fleet_scope()),
            failure_code: FailureCode::Unspecified as i32,
            ..Observation::default()
        }
        .encode_to_vec(),
    };
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO events (authority_domain_id, kind, payload) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                domain().value,
                StoredEventKind::Observation as i32,
                source.encode_to_vec()
            ],
        )
        .unwrap();
    let source_event_id = EventId {
        authority_domain_id: Some(domain()),
        lsn: Some(Lsn {
            value: connection.last_insert_rowid() as u64,
        }),
    };
    drop(connection);
    let occurred_at = Timestamp {
        seconds: 10,
        nanos: 0,
    };
    let mut audit = AuditRecordDraft::new(occurred_at, AuditEventKind::CommandCompleted);
    audit.actor_id = Some(actor());
    audit.command_id = Some(command_id.clone());
    audit.grant_id = Some(grant_id("forged-parent"));
    audit.target_scope = Some(session_scope());
    audit.reason_code = "spawn_completion".to_owned();
    audit.source_event_id = Some(source_event_id);
    let audit_id = storage.append_audit(&domain(), audit).await.unwrap();
    let mut descendant = descendant_grant("desc:authority-main:spawn-1", "forged-parent");
    descendant.subject_endpoint_class.clear();
    descendant.provenance = Some(DescendantGrantProvenance {
        spawn_operation_id: Some(command_id),
        spawning_grant_id: Some(grant_id("forged-parent")),
        continuation_authority: None,
    });
    descendant.created_at = Some(occurred_at);
    descendant.audit_id = Some(audit_id);

    let error = ingest_descendant_grant(&storage, &mut registry, &domain(), descendant)
        .await
        .expect_err("a self-consistent forged chain cannot create authority");
    assert!(matches!(
        error,
        AuthorityError::CorruptLog(_) | AuthorityError::InvalidGrant(_)
    ));
    assert_eq!(events(&storage).await.len(), 2);
}

#[tokio::test]
async fn revoking_parent_does_not_cascade_to_descendant() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .unwrap();
    let (_, descendant_id) = ingest_valid_descendant(&storage, &mut registry, "parent")
        .await
        .unwrap();

    ingest_revocation(&storage, &mut registry, &domain(), revocation("parent"))
        .await
        .expect("the existing parent grant must be revocable");

    assert!(registry
        .get_grant(&grant_id("parent"))
        .expect("revocation retains the parent record")
        .is_revoked());
    assert!(registry
        .get_grant(&descendant_id)
        .expect("non-cascade retains the descendant record")
        .is_live());
    assert_eq!(events(&storage).await.len(), 12);
}

#[tokio::test]
async fn revocation_audits_preserve_verified_actor_and_endpoint_attribution() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("attributed"))
        .await
        .unwrap();
    let mut revocation = revocation("attributed");
    revocation.revoked_by = Some(ActorEndpointRef {
        actor_id: Some(actor()),
        endpoint_id: Some(EndpointId {
            value: "cli-endpoint".to_owned(),
        }),
        device_id: Some(DeviceId {
            value: "cli-device".to_owned(),
        }),
        ..ActorEndpointRef::default()
    });

    ingest_revocation(&storage, &mut registry, &domain(), revocation)
        .await
        .expect("attributed revocation must append");

    let page = storage
        .query_audit(
            &domain(),
            AuditPageSpec {
                kinds: vec![AuditEventKind::GrantRevoked],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: Some(grant_id("attributed")),
                target: None,
                failure_codes: vec![],
                reason_codes: vec![],
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].actor_id, Some(actor()));
    assert_eq!(
        page.records[0].endpoint_id,
        Some(EndpointId {
            value: "cli-endpoint".to_owned()
        })
    );
    assert_eq!(
        page.records[0].device_id,
        Some(DeviceId {
            value: "cli-device".to_owned()
        })
    );
}

#[tokio::test]
async fn revoking_nonexistent_grant_fails_before_write() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();

    let error = ingest_revocation(&storage, &mut registry, &domain(), revocation("missing"))
        .await
        .expect_err("unknown grants must fail fast");

    assert!(matches!(error, AuthorityError::GrantNotFound(message) if message.contains("missing")));
    assert!(events(&storage).await.is_empty());
}

#[tokio::test]
async fn committed_event_redelivery_is_projection_idempotence_only() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = AuthorityRegistry::new();
    ingest_grant(&storage, &mut registry, &domain(), grant("parent"))
        .await
        .expect("the valid grant must be ingested and warmed");
    let after_warm = registry.clone();
    let committed = events(&storage).await;

    registry
        .observe(&committed[0])
        .expect("redelivering the committed event must be idempotent");

    assert_eq!(registry, after_warm);
}
