use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, ResourceId, ResourceKind, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    diagnostics::AuthorityDomainTargetResolver,
    resource::{ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    target::{target_adapter_id, TargetRegistry},
};

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

#[tokio::test]
async fn registered_resource_resolves_by_the_exact_tuple() {
    let registered = identity("adapter-a", "pool", "shared");
    let mut resources = ResourceRegistry::new();
    assert!(resources.register(registered.clone()));
    assert!(!resources.register(registered.clone()));
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
    let mut resources = ResourceRegistry::new();
    resources.register(identity("adapter-a", "pool", "shared"));
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
