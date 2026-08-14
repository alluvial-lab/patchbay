use patchbay_contracts::patchbay::{
    resource_state_mutation, AdapterCapability, AdapterId, AdapterRegistration,
    AdapterSnapshotSupport, AdapterTargetCategory, AuthorityDomainId, EndpointId, Generation,
    OperationKind, PayloadContentType, PayloadEnvelope, ResourceId, ResourceKind,
    ResourceStateEvent, ResourceStateMutation, ResourceStateUpsert, ResourceViewStateUpdate,
    TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    adapter::{ingest_registration, AdapterRegistry},
    diagnostics::AuthorityDomainTargetResolver,
    resource::{ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    storage::{event_id, RecordedEvent, RusqliteStorage},
    target::{target_adapter_id, TargetRegistry},
};
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn identity(adapter: &str, kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity::new(
        AdapterId {
            value: adapter.to_owned(),
        },
        ResourceKind {
            value: kind.to_owned(),
        },
        ResourceId {
            value: id.to_owned(),
        },
    )
    .unwrap()
}

fn registry(identities: &[ResourceIdentity]) -> ResourceRegistry {
    let mut registry = ResourceRegistry::new();
    for (index, identity) in identities.iter().enumerate() {
        let state = ResourceStateEvent {
            authority_domain_id: Some(domain()),
            source_adapter_id: Some(identity.adapter_id().clone()),
            source_adapter_generation: Some(Generation { value: 1 }),
            views: vec![ResourceViewStateUpdate {
                resource_kind: Some(identity.resource_kind().clone()),
                completeness: AdapterSnapshotSupport::Partial as i32,
            }],
            mutations: vec![ResourceStateMutation {
                identity: Some(identity.to_scope().resource.unwrap()),
                from_revision_lsn: None,
                mutation: Some(resource_state_mutation::Mutation::Upsert(
                    ResourceStateUpsert {
                        resource_payload: Some(PayloadEnvelope {
                            payload: vec![1],
                            content_type: PayloadContentType::Protobuf as i32,
                            schema_ref: "resource.schema".into(),
                        }),
                        projection_payload: Some(PayloadEnvelope {
                            payload: vec![2],
                            content_type: PayloadContentType::Protobuf as i32,
                            schema_ref: "projection.schema".into(),
                        }),
                    },
                )),
            }],
            observed_at: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
        };
        registry
            .observe(&RecordedEvent {
                event_id: event_id(domain(), index as u64 + 1),
                payload: patchbay_core::resource::events::encode(&state),
            })
            .unwrap();
    }
    registry
}

#[tokio::test]
async fn registered_resource_resolves_by_the_exact_tuple() {
    let registered = identity("adapter-a", "pool", "shared");
    let resources = registry(std::slice::from_ref(&registered));
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    assert_eq!(
        TargetResolver::resolve(
            &targets,
            &domain(),
            OperationKind::Query,
            &registered.to_scope()
        )
        .await,
        Ok(TargetBinding::Resource(registered.clone()))
    );
    for unknown in [
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
        identity("adapter-a", "pool", "other"),
    ] {
        assert!(TargetResolver::resolve(
            &targets,
            &domain(),
            OperationKind::Query,
            &unknown.to_scope()
        )
        .await
        .is_err());
    }
    assert_eq!(targets.resources().resources().count(), 1);
}

#[tokio::test]
async fn malformed_legacy_and_nonordinary_resource_targets_fail_closed() {
    let resources = registry(&[identity("adapter-a", "pool", "shared")]);
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    let legacy = TargetScope {
        kind: TargetScopeKind::Resource as i32,
        legacy_audit_resource_id: "shared".to_owned(),
        ..TargetScope::default()
    };
    let mut mixed = identity("adapter-a", "pool", "shared").to_scope();
    mixed.adapter_id = Some(AdapterId {
        value: "adapter-a".to_owned(),
    });
    let authority = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    for scope in [legacy, mixed, authority] {
        assert!(
            TargetResolver::resolve(&targets, &domain(), OperationKind::Query, &scope)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn spawn_resolution_commits_one_attached_adapter_boundary_only() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut adapters = AdapterRegistry::new();
    ingest_registration(
        &storage,
        &mut adapters,
        AdapterRegistration {
            adapter_id: Some(AdapterId {
                value: "adapter-a".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "adapter-a-endpoint".to_owned(),
            }),
            authority_domain_id: Some(domain()),
            adapter_generation: Some(Generation { value: 1 }),
            capability: Some(AdapterCapability {
                supported_operation_kinds: vec![OperationKind::Spawn as i32],
                session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                ..AdapterCapability::default()
            }),
            ..AdapterRegistration::default()
        },
    )
    .await
    .unwrap();
    let targets = TargetRegistry::with_adapters(
        SessionRegistry::new(domain()).unwrap(),
        registry(&[identity("adapter-a", "pool", "shared")]),
        adapters,
    );
    let adapter = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "adapter-a".to_owned(),
        }),
        ..TargetScope::default()
    };
    assert_eq!(
        TargetResolver::resolve(&targets, &domain(), OperationKind::Spawn, &adapter).await,
        Ok(TargetBinding::Adapter {
            adapter_id: AdapterId {
                value: "adapter-a".to_owned(),
            },
        })
    );

    let runtime = TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: adapter.adapter_id.clone(),
        runtime_session_id: Some(patchbay_contracts::patchbay::RuntimeSessionId {
            value: "existing".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        ..TargetScope::default()
    };
    let fleet = TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    };
    let resource = identity("adapter-a", "pool", "shared").to_scope();
    let unattached = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "adapter-b".to_owned(),
        }),
        ..TargetScope::default()
    };
    let mut mixed = adapter.clone();
    mixed.project_or_group = "not-canonical".to_owned();
    for incompatible in [runtime, resource, fleet, unattached, mixed] {
        assert!(
            TargetResolver::resolve(&targets, &domain(), OperationKind::Spawn, &incompatible)
                .await
                .is_err(),
            "incompatible spawn target resolved: {incompatible:?}"
        );
    }
    assert!(
        TargetResolver::resolve(&targets, &domain(), OperationKind::Instruct, &adapter)
            .await
            .is_err(),
        "adapter scope is a spawn-only operation boundary"
    );
}

#[tokio::test]
async fn diagnostics_resolution_returns_an_honest_authority_domain_binding() {
    let scope = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    assert_eq!(
        TargetResolver::resolve(
            &AuthorityDomainTargetResolver,
            &domain(),
            OperationKind::Query,
            &scope,
        )
        .await,
        Ok(TargetBinding::AuthorityDomain(domain()))
    );

    let resource = identity("adapter-a", "pool", "shared").to_scope();
    assert!(TargetResolver::resolve(
        &AuthorityDomainTargetResolver,
        &domain(),
        OperationKind::Query,
        &resource,
    )
    .await
    .is_err());
}

#[test]
fn adapter_routing_uses_only_complete_canonical_resource_identity() {
    let canonical = identity("adapter-a", "pool", "shared").to_scope();
    assert_eq!(
        target_adapter_id(&canonical).map(|id| id.value.as_str()),
        Some("adapter-a")
    );

    let mut partial = canonical.clone();
    partial.resource.as_mut().unwrap().resource_kind = None;
    assert_eq!(target_adapter_id(&partial), None);

    let mut mixed = canonical;
    mixed.adapter_id = Some(AdapterId {
        value: "adapter-b".to_owned(),
    });
    assert_eq!(target_adapter_id(&mixed), None);
}
