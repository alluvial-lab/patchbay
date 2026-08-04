use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AdapterId, AuthorityDomainId, DescendantGrant,
    DescendantGrantProvenance, EndpointId, EventId, Generation, Grant, GrantId, GrantProvenance,
    GrantRevocationPolicy, Lsn, OperationKind, ResourceId, ResourceIdentity, ResourceKind,
    Revocation, RuntimeSessionId, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    authority::{
        grant_authorizes, target_scope_matches, AuthorityError, AuthorityRegistry, IssuerRef,
        DESCENDANT_GRANT_ALLOWED_KINDS,
    },
    storage::RecordedEvent,
};
use prost::Message;

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

fn adapter(value: &str) -> AdapterId {
    AdapterId {
        value: value.to_owned(),
    }
}

fn runtime(value: &str) -> RuntimeSessionId {
    RuntimeSessionId {
        value: value.to_owned(),
    }
}

fn generation(value: u64) -> Generation {
    Generation { value }
}

fn grant_id(value: &str) -> GrantId {
    GrantId {
        value: value.to_owned(),
    }
}

fn adapter_scope(value: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter(value)),
        ..TargetScope::default()
    }
}

fn runtime_scope(adapter_value: &str, runtime_value: &str, generation_value: u64) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(adapter(adapter_value)),
        runtime_session_id: Some(runtime(runtime_value)),
        session_generation: Some(generation(generation_value)),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    }
}

fn resource_scope(adapter_value: &str, kind: &str, id: &str) -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::Resource as i32,
        resource: Some(ResourceIdentity {
            adapter_id: Some(adapter(adapter_value)),
            resource_id: Some(ResourceId { value: id.to_owned() }),
            resource_kind: Some(ResourceKind { value: kind.to_owned() }),
        }),
        ..TargetScope::default()
    }
}

fn operator_grant() -> Grant {
    Grant {
        grant_id: Some(grant_id("grant-1")),
        authority_domain_id: Some(domain("authority-main")),
        subject_actor_id: Some(actor("operator")),
        subject_endpoint_id: None,
        subject_endpoint_class: "web".to_owned(),
        target_scope: Some(adapter_scope("pi")),
        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
        created_at: None,
        provenance: Some(GrantProvenance {
            reason: "operator setup".to_owned(),
            ..GrantProvenance::default()
        }),
        expires_at: None,
        revocation_generation: None,
        revoked_at: None,
        revocation_policy: GrantRevocationPolicy::Continue as i32,
    }
}

fn descendant_grant() -> DescendantGrant {
    DescendantGrant {
        grant_id: Some(grant_id("descendant-1")),
        authority_domain_id: Some(domain("authority-main")),
        subject_actor_id: Some(actor("operator")),
        subject_endpoint_id: Some(endpoint("browser-1")),
        subject_endpoint_class: "web".to_owned(),
        target_scope: Some(runtime_scope("pi", "session-1", 1)),
        allowed_operation_kinds: DESCENDANT_GRANT_ALLOWED_KINDS
            .iter()
            .map(|kind| *kind as i32)
            .collect(),
        provenance: Some(DescendantGrantProvenance::default()),
        created_at: None,
        expires_at: None,
        revocation_generation: None,
        revoked_at: None,
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        audit_id: None,
    }
}

fn revocation(id: &str, generation_value: u64) -> Revocation {
    Revocation {
        authority_domain_id: Some(domain("authority-main")),
        grant_id: Some(grant_id(id)),
        revoked_at: None,
        revocation_generation: Some(generation(generation_value)),
        accepted_operation_policy: GrantRevocationPolicy::Cancel as i32,
        reason: "operator revoked access".to_owned(),
        ..Revocation::default()
    }
}

fn recorded<M: Message>(lsn: u64, kind: StoredEventKind, message: &M) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain("authority-main")),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: message.encode_to_vec(),
        },
    }
}

#[test]
fn observing_grant_projects_a_live_record() {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(1, StoredEventKind::Grant, &operator_grant()))
        .unwrap();

    let record = registry
        .get_grant(&grant_id("grant-1"))
        .expect("the grant event must create a projected grant");
    assert_eq!(record.authority_domain_id, domain("authority-main"));
    assert_eq!(record.subject_actor_id, actor("operator"));
    assert_eq!(record.target_scope, adapter_scope("pi"));
    assert_eq!(record.allowed_operation_kinds, [OperationKind::Instruct]);
    assert!(!record.is_descendant);
    assert!(record.is_live());
    assert!(!record.is_revoked());
    assert_eq!(registry.live_grants().count(), 1);
}

#[test]
fn revocation_marks_the_grant_revoked_without_deleting_it() {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(1, StoredEventKind::Grant, &operator_grant()))
        .unwrap();
    let revoke = recorded(2, StoredEventKind::Revocation, &revocation("grant-1", 3));
    registry.observe(&revoke).unwrap();

    let record = registry
        .get_grant(&grant_id("grant-1"))
        .expect("revocation must retain the grant for audit history");
    assert!(record.is_revoked());
    assert!(!record.is_live());
    assert_eq!(record.revocation_generation, Some(generation(3)));
    assert_eq!(record.revocation_policy, GrantRevocationPolicy::Cancel);
    assert_eq!(registry.live_grants().count(), 0);

    registry.observe(&revoke).unwrap();
    assert_eq!(
        registry
            .get_grant(&grant_id("grant-1"))
            .unwrap()
            .revocation_generation,
        Some(generation(3))
    );

    // Replaying the full committed prefix is also idempotent: the original
    // creation event must not conflict with the later projected revocation.
    registry
        .observe(&recorded(1, StoredEventKind::Grant, &operator_grant()))
        .unwrap();
    assert!(registry
        .get_grant(&grant_id("grant-1"))
        .unwrap()
        .is_revoked());

    assert!(matches!(
        registry.observe(&recorded(
            3,
            StoredEventKind::Revocation,
            &revocation("grant-1", 4),
        )),
        Err(AuthorityError::CorruptLog(message))
            if message.contains("conflicting revocation generations")
    ));
    assert_eq!(
        registry
            .get_grant(&grant_id("grant-1"))
            .unwrap()
            .revocation_generation,
        Some(generation(3))
    );
}

#[test]
fn same_generation_revocations_require_identical_retained_content() {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(1, StoredEventKind::Grant, &operator_grant()))
        .unwrap();

    let mut initial_revocation = revocation("grant-1", 3);
    initial_revocation.accepted_operation_policy = GrantRevocationPolicy::Continue as i32;
    let initial_event = recorded(2, StoredEventKind::Revocation, &initial_revocation);
    registry.observe(&initial_event).unwrap();

    // Exact redelivery is idempotent.
    registry.observe(&initial_event).unwrap();

    // The same generation with a different retained policy is log corruption.
    let conflicting_event = recorded(3, StoredEventKind::Revocation, &revocation("grant-1", 3));
    assert!(matches!(
        registry.observe(&conflicting_event),
        Err(AuthorityError::CorruptLog(message))
            if message.contains("same generation, different content")
    ));
    assert_eq!(
        registry
            .get_grant(&grant_id("grant-1"))
            .unwrap()
            .revocation_policy,
        GrantRevocationPolicy::Continue
    );

    // A same-generation revocation that differs only in actor, reason, or
    // audit link is likewise corruption, not a silent redelivery.
    let base = revocation("grant-1", 3);
    let differing_actor = Revocation {
        revoked_by: Some(ActorEndpointRef {
            actor_id: Some(actor("different-revoker")),
            ..ActorEndpointRef::default()
        }),
        ..base.clone()
    };
    assert!(matches!(
        registry.observe(&recorded(4, StoredEventKind::Revocation, &differing_actor)),
        Err(AuthorityError::CorruptLog(_))
    ));

    let differing_reason = Revocation {
        reason: "a different rationale".to_owned(),
        ..base.clone()
    };
    assert!(matches!(
        registry.observe(&recorded(5, StoredEventKind::Revocation, &differing_reason)),
        Err(AuthorityError::CorruptLog(_))
    ));
}

#[test]
fn descendant_grants_require_the_exact_canonical_kind_set() {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(
            1,
            StoredEventKind::DescendantGrant,
            &descendant_grant(),
        ))
        .unwrap();

    let record = registry
        .get_grant(&grant_id("descendant-1"))
        .expect("a valid descendant grant must be projected");
    assert!(record.is_descendant);
    assert_eq!(
        record.allowed_operation_kinds,
        DESCENDANT_GRANT_ALLOWED_KINDS
    );

    let mut wrong = descendant_grant();
    wrong.grant_id = Some(grant_id("descendant-wrong"));
    wrong.allowed_operation_kinds.pop();
    assert!(matches!(
        registry.observe(&recorded(2, StoredEventKind::DescendantGrant, &wrong)),
        Err(AuthorityError::InvalidGrant(message))
            if message.contains("exactly the canonical")
    ));
    assert!(registry.get_grant(&grant_id("descendant-wrong")).is_none());
}

#[test]
fn grant_authorizes_only_when_every_matching_dimension_passes() {
    let mut registry = AuthorityRegistry::new();
    registry
        .observe(&recorded(1, StoredEventKind::Grant, &operator_grant()))
        .unwrap();
    let grant = registry.get_grant(&grant_id("grant-1")).unwrap().clone();
    let issuer_actor = actor("operator");
    let issuer_endpoint = endpoint("browser-1");
    let issuer_domain = domain("authority-main");
    let issuer = IssuerRef {
        actor: &issuer_actor,
        endpoint: Some(&issuer_endpoint),
        authority_domain_id: &issuer_domain,
    };

    assert!(grant_authorizes(
        &grant,
        &issuer,
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));

    let mut revoked = grant.clone();
    revoked.revocation_generation = Some(generation(1));
    assert!(!grant_authorizes(
        &revoked,
        &issuer,
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));

    let wrong_domain = domain("authority-other");
    assert!(!grant_authorizes(
        &grant,
        &IssuerRef {
            authority_domain_id: &wrong_domain,
            ..issuer
        },
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));

    let wrong_actor = actor("intruder");
    assert!(!grant_authorizes(
        &grant,
        &IssuerRef {
            actor: &wrong_actor,
            ..issuer
        },
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));
    assert!(!grant_authorizes(
        &grant,
        &issuer,
        OperationKind::Query,
        &adapter_scope("pi")
    ));
    assert!(!grant_authorizes(
        &grant,
        &issuer,
        OperationKind::Instruct,
        &adapter_scope("other")
    ));

    let mut narrowed = grant;
    narrowed.subject_endpoint_id = Some(endpoint("browser-1"));
    let other_endpoint = endpoint("browser-2");
    assert!(!grant_authorizes(
        &narrowed,
        &IssuerRef {
            endpoint: Some(&other_endpoint),
            ..issuer
        },
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));
    assert!(!grant_authorizes(
        &narrowed,
        &IssuerRef {
            endpoint: None,
            ..issuer
        },
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));
    assert!(grant_authorizes(
        &narrowed,
        &issuer,
        OperationKind::Instruct,
        &adapter_scope("pi")
    ));
}

#[test]
fn target_scope_matching_covers_the_full_containment_matrix() {
    let requested_session = TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        actor_id: Some(actor("operator")),
        adapter_id: Some(adapter("pi")),
        runtime_session_id: Some(runtime("session-1")),
        session_generation: Some(generation(7)),
        deployment_scope: "machine-a".to_owned(),
        project_or_group: "patchbay".to_owned(),
        ..TargetScope::default()
    };

    for wildcard_kind in [
        TargetScopeKind::FleetSupervisor,
        TargetScopeKind::AuthorityDomain,
    ] {
        assert!(target_scope_matches(
            &TargetScope {
                kind: wildcard_kind as i32,
                ..TargetScope::default()
            },
            &requested_session
        ));
    }

    assert!(target_scope_matches(
        &adapter_scope("pi"),
        &requested_session
    ));
    assert!(!target_scope_matches(
        &adapter_scope("other"),
        &requested_session
    ));
    assert!(target_scope_matches(
        &adapter_scope("pi"),
        &resource_scope("pi", "pool", "resource-1")
    ));
    assert!(!target_scope_matches(
        &adapter_scope("pi"),
        &resource_scope("other", "pool", "resource-1")
    ));

    let exact_session = runtime_scope("pi", "session-1", 7);
    assert!(target_scope_matches(&exact_session, &requested_session));
    for non_matching in [
        runtime_scope("other", "session-1", 7),
        runtime_scope("pi", "session-2", 7),
        runtime_scope("pi", "session-1", 8),
    ] {
        assert!(!target_scope_matches(&non_matching, &requested_session));
    }

    let cross_deployment_request = TargetScope {
        deployment_scope: "machine-b".to_owned(),
        ..requested_session.clone()
    };
    assert!(!target_scope_matches(
        &exact_session,
        &cross_deployment_request
    ));

    let empty_deployment_grant = TargetScope {
        deployment_scope: String::new(),
        ..exact_session
    };
    assert!(!target_scope_matches(
        &empty_deployment_grant,
        &requested_session
    ));

    let project = TargetScope {
        kind: TargetScopeKind::ProjectSessionGroup as i32,
        project_or_group: "patchbay".to_owned(),
        ..TargetScope::default()
    };
    assert!(target_scope_matches(&project, &requested_session));
    assert!(!target_scope_matches(
        &TargetScope {
            project_or_group: "other".to_owned(),
            ..project.clone()
        },
        &requested_session
    ));

    let actor_scope = TargetScope {
        kind: TargetScopeKind::Actor as i32,
        actor_id: Some(actor("operator")),
        ..TargetScope::default()
    };
    assert!(target_scope_matches(&actor_scope, &requested_session));
    assert!(!target_scope_matches(
        &TargetScope {
            actor_id: Some(actor("intruder")),
            ..actor_scope.clone()
        },
        &requested_session
    ));

    let resource = resource_scope("pi", "pool", "resource-1");
    assert!(target_scope_matches(&resource, &resource));
    assert!(!target_scope_matches(&resource, &requested_session));
    for non_matching in [
        resource_scope("other", "pool", "resource-1"),
        resource_scope("pi", "window", "resource-1"),
        resource_scope("pi", "pool", "resource-2"),
    ] {
        assert!(!target_scope_matches(&resource, &non_matching));
    }

    assert!(!target_scope_matches(
        &TargetScope::default(),
        &requested_session
    ));
    assert!(!target_scope_matches(
        &TargetScope {
            kind: 999,
            ..TargetScope::default()
        },
        &requested_session
    ));
}

#[test]
fn descendant_allowed_kinds_are_exactly_the_protocol_set() {
    let actual: HashSet<_> = DESCENDANT_GRANT_ALLOWED_KINDS.iter().copied().collect();
    let expected = HashSet::from([
        OperationKind::Instruct,
        OperationKind::Cancel,
        OperationKind::Interrupt,
        OperationKind::Query,
        OperationKind::ApprovalResponse,
        OperationKind::ElicitationResponse,
        OperationKind::Reconfigure,
        OperationKind::SessionManagement,
    ]);

    assert_eq!(DESCENDANT_GRANT_ALLOWED_KINDS.len(), 8);
    assert_eq!(actual, expected);
    assert!(!actual.contains(&OperationKind::Spawn));
    assert!(!actual.contains(&OperationKind::Attach));
}

#[test]
fn exact_grant_redelivery_is_idempotent_but_conflicting_duplicate_is_corrupt() {
    let event = recorded(1, StoredEventKind::Grant, &operator_grant());
    let mut registry = AuthorityRegistry::new();
    registry.observe(&event).unwrap();
    let once = registry.clone();
    registry.observe(&event).unwrap();
    assert_eq!(registry, once);

    let mut conflicting = operator_grant();
    conflicting.allowed_operation_kinds = vec![OperationKind::Query as i32];
    assert!(matches!(
        registry.observe(&recorded(2, StoredEventKind::Grant, &conflicting)),
        Err(AuthorityError::CorruptLog(_))
    ));
}

#[test]
fn revoking_an_unknown_grant_fails_fast() {
    let mut registry = AuthorityRegistry::new();
    assert!(matches!(
        registry.observe(&recorded(
            1,
            StoredEventKind::Revocation,
            &revocation("missing", 1),
        )),
        Err(AuthorityError::GrantNotFound(message)) if message.contains("unknown grant")
    ));
}

#[test]
fn malformed_grants_and_cross_domain_records_are_rejected() {
    let mut missing_actor = operator_grant();
    missing_actor.subject_actor_id = None;
    let mut registry = AuthorityRegistry::new();
    assert!(matches!(
        registry.observe(&recorded(1, StoredEventKind::Grant, &missing_actor)),
        Err(AuthorityError::InvalidGrant(message)) if message.contains("subject_actor_id")
    ));

    let mut wrong_domain = operator_grant();
    wrong_domain.authority_domain_id = Some(domain("authority-other"));
    assert!(matches!(
        registry.observe(&recorded(2, StoredEventKind::Grant, &wrong_domain)),
        Err(AuthorityError::CorruptLog(message)) if message.contains("does not match")
    ));

    let mut unknown_kind = operator_grant();
    unknown_kind.allowed_operation_kinds = vec![999];
    assert!(matches!(
        registry.observe(&recorded(3, StoredEventKind::Grant, &unknown_kind)),
        Err(AuthorityError::CorruptRecord(message)) if message.contains("unknown operation kind")
    ));

    let mut partial_resource = operator_grant();
    partial_resource.target_scope = Some(resource_scope("pi", "pool", "resource-1"));
    partial_resource
        .target_scope
        .as_mut()
        .unwrap()
        .resource
        .as_mut()
        .unwrap()
        .resource_kind = None;
    assert!(matches!(
        registry.observe(&recorded(4, StoredEventKind::Grant, &partial_resource)),
        Err(AuthorityError::InvalidGrant(message)) if message.contains("incomplete Resource")
    ));

    let mut legacy_resource = operator_grant();
    legacy_resource.target_scope = Some(TargetScope {
        kind: TargetScopeKind::Resource as i32,
        legacy_audit_resource_id: "audit-only".to_owned(),
        ..TargetScope::default()
    });
    assert!(matches!(
        registry.observe(&recorded(5, StoredEventKind::Grant, &legacy_resource)),
        Err(AuthorityError::InvalidGrant(message)) if message.contains("incomplete Resource")
    ));

    let mut mixed_resource = operator_grant();
    mixed_resource.target_scope = Some(resource_scope("pi", "pool", "resource-1"));
    mixed_resource.target_scope.as_mut().unwrap().adapter_id = Some(adapter("pi"));
    assert!(matches!(
        registry.observe(&recorded(6, StoredEventKind::Grant, &mixed_resource)),
        Err(AuthorityError::InvalidGrant(message)) if message.contains("incomplete Resource")
    ));
}

#[test]
fn non_authority_events_are_ignored() {
    let event = RecordedEvent {
        event_id: EventId::default(),
        payload: StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: vec![0xff],
        },
    };
    let mut registry = AuthorityRegistry::new();
    registry.observe(&event).unwrap();
    assert_eq!(registry.live_grants().count(), 0);
}
