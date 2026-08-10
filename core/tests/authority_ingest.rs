use patchbay_contracts::patchbay::{
    typed_correlation, ActorEndpointRef, ActorId, AdapterId, AuditEventKind, AuthorityDomainId,
    CommandId, DescendantGrant, DescendantGrantProvenance, DeviceId, EndpointId, FailureCode,
    Generation, Grant, GrantId, GrantProvenance, GrantRevocationPolicy, Lsn, Observation,
    ObservationKind, OperationKind, Revocation, RuntimeSessionId, StoredEventKind,
    StoredEventPayload, TargetScope, TargetScopeKind, TypedCorrelation,
};
use patchbay_core::{
    authority::{
        ingest_descendant_grant, ingest_grant, ingest_revocation, AuthorityError,
        AuthorityRegistry, DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::{AuditPageSpec, AuditRecordDraft, RecordedEvent, RusqliteStorage, Storage},
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

async fn ingest_valid_descendant(
    storage: &RusqliteStorage,
    registry: &mut AuthorityRegistry,
    parent_id: &str,
) -> Result<(patchbay_contracts::patchbay::EventId, GrantId), AuthorityError> {
    let command_id = CommandId {
        value: "spawn-1".to_owned(),
    };
    let source = Observation {
        authority_domain_id: Some(domain()),
        kind: ObservationKind::Result as i32,
        correlations: vec![TypedCorrelation {
            r#ref: Some(typed_correlation::Ref::CommandId(command_id.clone())),
        }],
        target_scope: Some(fleet_scope()),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let source_event_id = storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: source.encode_to_vec(),
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
    });
    descendant.created_at = Some(occurred_at);
    descendant.audit_id = Some(audit_id);
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
    assert_eq!(committed.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(committed[0].payload.kind).unwrap(),
        StoredEventKind::Grant
    );
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

    let (event_id, descendant_id) = ingest_valid_descendant(&storage, &mut registry, "parent")
        .await
        .expect("the canonical descendant grant must be ingested");

    assert_eq!(event_id.lsn, Some(Lsn { value: 3 }));
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
    assert_eq!(events(&storage).await.len(), 6);
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
async fn committed_event_redelivery_is_consistent_after_warm() {
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
