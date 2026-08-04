use patchbay_contracts::patchbay::{
    resource_state_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, Generation,
    PayloadContentType, PayloadEnvelope, ResourceId, ResourceKind, ResourceStateEvent,
    ResourceStateMutation, ResourceStateUpsert, ResourceViewStateUpdate, TargetScope,
    TargetScopeKind,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    diagnostics::AuthorityDomainTargetResolver,
    resource::{ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    storage::{event_id, RecordedEvent},
    target::{target_adapter_id, TargetRegistry},
};
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId { value: "authority-main".to_owned() }
}

fn identity(adapter: &str, kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity::new(
        AdapterId { value: adapter.to_owned() },
        ResourceKind { value: kind.to_owned() },
        ResourceId { value: id.to_owned() },
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
                mutation: Some(resource_state_mutation::Mutation::Upsert(ResourceStateUpsert {
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
                })),
            }],
            observed_at: Some(Timestamp { seconds: 1, nanos: 0 }),
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
    let targets = TargetRegistry::new(SessionRegistry::new(), resources);

    assert_eq!(
        TargetResolver::resolve(&targets, &domain(), &registered.to_scope()).await,
        Ok(TargetBinding::Resource(registered.clone()))
    );
    for unknown in [
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
        identity("adapter-a", "pool", "other"),
    ] {
        assert!(
            TargetResolver::resolve(&targets, &domain(), &unknown.to_scope())
                .await
                .is_err()
        );
    }
    assert_eq!(targets.resources().resources().count(), 1);
}

#[tokio::test]
async fn malformed_legacy_and_nonordinary_resource_targets_fail_closed() {
    let resources = registry(&[identity("adapter-a", "pool", "shared")]);
    let targets = TargetRegistry::new(SessionRegistry::new(), resources);

    let legacy = TargetScope {
        kind: TargetScopeKind::Resource as i32,
        legacy_audit_resource_id: "shared".to_owned(),
        ..TargetScope::default()
    };
    let mut mixed = identity("adapter-a", "pool", "shared").to_scope();
    mixed.adapter_id = Some(AdapterId { value: "adapter-a".to_owned() });
    let authority = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    for scope in [legacy, mixed, authority] {
        assert!(TargetResolver::resolve(&targets, &domain(), &scope).await.is_err());
    }
}

#[tokio::test]
async fn diagnostics_resolution_returns_an_honest_authority_domain_binding() {
    let scope = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    assert_eq!(
        TargetResolver::resolve(&AuthorityDomainTargetResolver, &domain(), &scope).await,
        Ok(TargetBinding::AuthorityDomain(domain()))
    );

    let resource = identity("adapter-a", "pool", "shared").to_scope();
    assert!(
        TargetResolver::resolve(&AuthorityDomainTargetResolver, &domain(), &resource)
            .await
            .is_err()
    );
}

#[test]
fn adapter_routing_uses_only_complete_canonical_resource_identity() {
    let canonical = identity("adapter-a", "pool", "shared").to_scope();
    assert_eq!(target_adapter_id(&canonical).map(|id| id.value.as_str()), Some("adapter-a"));

    let mut partial = canonical.clone();
    partial.resource.as_mut().unwrap().resource_kind = None;
    assert_eq!(target_adapter_id(&partial), None);

    let mut mixed = canonical;
    mixed.adapter_id = Some(AdapterId { value: "adapter-b".to_owned() });
    assert_eq!(target_adapter_id(&mixed), None);
}
