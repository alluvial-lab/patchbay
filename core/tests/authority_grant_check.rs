use patchbay_contracts::patchbay::{
    ActorId, AdapterId, AuthorityDomainId, DeviceId, EndpointId, EventId, Generation, Grant,
    GrantId, GrantProvenance, GrantRevocationPolicy, Lsn, OperationKind, Revocation, TargetScope,
    TargetScopeKind,
};
use patchbay_core::{
    acceptance::{GrantCheck, GrantDenied},
    authority::{events, AuthorityRegistry, IssuerContext},
    storage::RecordedEvent,
};

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn actor(value: &str) -> ActorId {
    ActorId {
        value: value.to_owned(),
    }
}

fn endpoint(value: &str) -> EndpointId {
    EndpointId {
        value: value.to_owned(),
    }
}

fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn target(adapter: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: adapter.to_owned(),
        }),
        ..TargetScope::default()
    }
}

fn grant() -> Grant {
    Grant {
        grant_id: Some(grant_id("grant-1")),
        authority_domain_id: Some(domain("authority-main")),
        subject_actor_id: Some(actor("operator")),
        subject_endpoint_id: Some(endpoint("web-1")),
        subject_endpoint_class: "web".to_owned(),
        target_scope: Some(target("pi")),
        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
        provenance: Some(GrantProvenance {
            reason: "test fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

fn revocation() -> Revocation {
    Revocation {
        authority_domain_id: Some(domain("authority-main")),
        grant_id: Some(grant_id("grant-1")),
        revocation_generation: Some(Generation { value: 1 }),
        accepted_operation_policy: GrantRevocationPolicy::Continue as i32,
        reason: "test revocation".to_owned(),
        ..Revocation::default()
    }
}

fn recorded(lsn: u64, payload: patchbay_contracts::patchbay::StoredEventPayload) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain("authority-main")),
            lsn: Some(Lsn { value: lsn }),
        },
        payload,
    }
}

fn registry_with_live_grant() -> AuthorityRegistry {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(
            1,
            events::grant(domain("authority-main"), grant()),
        ))
        .expect("the controlled grant fixture must be valid");
    registry
}

struct TestIssuerContext {
    actor: Option<ActorId>,
    endpoint: Option<EndpointId>,
    device: Option<DeviceId>,
    endpoint_generation: Option<Generation>,
    authority_domain_id: AuthorityDomainId,
}

impl TestIssuerContext {
    fn verified() -> Self {
        Self {
            actor: Some(actor("operator")),
            endpoint: Some(endpoint("web-1")),
            device: Some(DeviceId {
                value: "device-1".to_owned(),
            }),
            endpoint_generation: Some(Generation { value: 3 }),
            authority_domain_id: domain("authority-main"),
        }
    }
}

impl IssuerContext for TestIssuerContext {
    fn verified_actor(&self) -> Option<&ActorId> {
        self.actor.as_ref()
    }

    fn verified_endpoint(&self) -> Option<&EndpointId> {
        self.endpoint.as_ref()
    }

    fn verified_device(&self) -> Option<&DeviceId> {
        self.device.as_ref()
    }

    fn endpoint_generation(&self) -> Option<Generation> {
        self.endpoint_generation
    }

    fn authority_domain_id(&self) -> &AuthorityDomainId {
        &self.authority_domain_id
    }
}

#[tokio::test]
async fn verified_issuer_with_matching_live_grant_is_authorized() {
    let registry = registry_with_live_grant();

    let authorized = registry
        .check(
            &domain("authority-main"),
            &TestIssuerContext::verified(),
            OperationKind::Instruct,
            &target("pi"),
        )
        .await
        .expect("the verified issuer has a matching live grant");

    assert_eq!(authorized.grant_id, Some(grant_id("grant-1")));
}

#[tokio::test]
async fn issuer_without_verified_actor_is_denied() {
    let registry = registry_with_live_grant();
    let mut issuer = TestIssuerContext::verified();
    issuer.actor = None;

    let result = registry
        .check(
            &domain("authority-main"),
            &issuer,
            OperationKind::Instruct,
            &target("pi"),
        )
        .await;

    assert!(matches!(result, Err(GrantDenied::NoGrant { .. })));
}

#[tokio::test]
async fn cross_domain_issuer_is_denied() {
    let registry = registry_with_live_grant();
    let mut issuer = TestIssuerContext::verified();
    issuer.authority_domain_id = domain("authority-other");

    let result = registry
        .check(
            &domain("authority-main"),
            &issuer,
            OperationKind::Instruct,
            &target("pi"),
        )
        .await;

    assert!(matches!(result, Err(GrantDenied::NoGrant { .. })));
}

#[tokio::test]
async fn revoked_grant_is_denied() {
    let mut registry = registry_with_live_grant();
    registry
        .observe(&recorded(
            2,
            events::revocation(domain("authority-main"), revocation()),
        ))
        .expect("the revocation fixture must apply");

    let result = registry
        .check(
            &domain("authority-main"),
            &TestIssuerContext::verified(),
            OperationKind::Instruct,
            &target("pi"),
        )
        .await;

    assert!(matches!(result, Err(GrantDenied::NoGrant { .. })));
}

#[tokio::test]
async fn uncovered_kind_and_target_are_denied_by_default() {
    let registry = registry_with_live_grant();
    let issuer = TestIssuerContext::verified();

    for (kind, target_scope) in [
        (OperationKind::Query, target("pi")),
        (OperationKind::Instruct, target("other-adapter")),
    ] {
        let result = registry
            .check(&domain("authority-main"), &issuer, kind, &target_scope)
            .await;
        assert!(matches!(result, Err(GrantDenied::NoGrant { .. })));
    }
}
