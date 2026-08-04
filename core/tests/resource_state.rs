use patchbay_contracts::patchbay::{
    resource_state_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, Generation, Lsn,
    PayloadContentType, PayloadEnvelope, ResourceFreshnessChanged, ResourceFreshnessState,
    ResourceId, ResourceKind, ResourceStateEvent,
    ResourceStateMutation, ResourceStateTombstone, ResourceStateUnknown, ResourceStateUpsert,
    ResourceViewStateUpdate,
};
use patchbay_core::{
    resource::{events, ResourceError, ResourceIdentity, ResourceRegistry, ResourceViewKey},
    storage::{event_id, RecordedEvent},
};
use prost_types::Timestamp;

#[test]
fn durable_fold_preserves_exact_identity_revisions_and_terminal_replacement() {
    let domain = domain();
    let old = identity("adapter-a", "provider_pool", "pool-1");
    let replacement = identity("adapter-a", "usage_window", "window-1");
    let collision = identity("adapter-b", "provider_pool", "pool-1");
    let mut registry = ResourceRegistry::new();

    registry.observe(&recorded(
        &domain,
        1,
        state_event(
            &domain,
            "adapter-a",
            4,
            vec![view("provider_pool", AdapterSnapshotSupport::Authoritative)],
            vec![upsert(&old, None)],
        ),
    )).unwrap();
    registry.observe(&recorded(
        &domain,
        2,
        state_event(
            &domain,
            "adapter-a",
            4,
            vec![
                view("provider_pool", AdapterSnapshotSupport::Authoritative),
                view("usage_window", AdapterSnapshotSupport::Partial),
            ],
            vec![tombstone(&old, 1, Some(&replacement)), upsert(&replacement, None)],
        ),
    )).unwrap();

    assert!(!registry.contains(&old));
    assert!(registry.contains(&replacement));
    assert!(!registry.contains(&collision));
    let retired = registry.get(&old).unwrap();
    assert_eq!(retired.revision_lsn, 2);
    assert_eq!(retired.tombstoned_at_lsn, Some(2));
    assert_eq!(retired.replaced_by.as_ref(), Some(&replacement));
    let window = registry.get(&replacement).unwrap();
    assert_eq!(window.freshness, ResourceFreshnessState::Current);
    assert_eq!(window.revision_lsn, 2);
    let key = ResourceViewKey {
        adapter_id: AdapterId { value: "adapter-a".into() },
        resource_kind: ResourceKind { value: "usage_window".into() },
    };
    let projected_view = registry.views().find(|view| view.key == key).unwrap();
    assert_eq!(projected_view.completeness, AdapterSnapshotSupport::Partial);
    assert_eq!(projected_view.revision_lsn, 2);

    let resurrection = registry.observe(&recorded(
        &domain,
        3,
        state_event(
            &domain,
            "adapter-a",
            4,
            vec![view("provider_pool", AdapterSnapshotSupport::Authoritative)],
            vec![upsert(&old, Some(2))],
        ),
    )).unwrap_err();
    assert!(matches!(resurrection, ResourceError::TerminalTombstone(_)));
    assert!(!registry.contains(&old));
}

#[test]
fn unknown_and_stale_are_reconciliation_freshness_not_payload_health() {
    let domain = domain();
    let id = identity("adapter-a", "provider_pool", "pool-1");
    let mut registry = ResourceRegistry::new();
    registry.observe(&recorded(
        &domain,
        1,
        state_event(
            &domain,
            "adapter-a",
            1,
            vec![view("provider_pool", AdapterSnapshotSupport::None)],
            vec![ResourceStateMutation {
                identity: Some(id.to_scope().resource.unwrap()),
                from_revision_lsn: None,
                mutation: Some(resource_state_mutation::Mutation::Unknown(ResourceStateUnknown {})),
            }],
        ),
    )).unwrap();
    let record = registry.get(&id).unwrap();
    assert_eq!(record.freshness, ResourceFreshnessState::Unknown);
    assert!(record.resource_payload.is_none());
    assert!(record.projection_payload.is_none());

    registry.observe(&recorded(
        &domain,
        2,
        state_event(
            &domain,
            "adapter-a",
            1,
            vec![view("provider_pool", AdapterSnapshotSupport::Partial)],
            vec![ResourceStateMutation {
                identity: Some(id.to_scope().resource.unwrap()),
                from_revision_lsn: Some(Lsn { value: 1 }),
                mutation: Some(resource_state_mutation::Mutation::FreshnessChanged(
                    ResourceFreshnessChanged {
                        from: ResourceFreshnessState::Unknown as i32,
                        to: ResourceFreshnessState::Stale as i32,
                    },
                )),
            }],
        ),
    )).unwrap();
    assert_eq!(registry.get(&id).unwrap().freshness, ResourceFreshnessState::Stale);
}

#[test]
fn contradictory_prior_revision_is_rejected_without_partial_fold() {
    let domain = domain();
    let first = identity("adapter-a", "pool", "one");
    let second = identity("adapter-a", "pool", "two");
    let mut registry = ResourceRegistry::new();
    registry.observe(&recorded(
        &domain,
        1,
        state_event(
            &domain,
            "adapter-a",
            1,
            vec![view("pool", AdapterSnapshotSupport::Partial)],
            vec![upsert(&first, None)],
        ),
    )).unwrap();
    let before = registry.clone();
    let error = registry.observe(&recorded(
        &domain,
        2,
        state_event(
            &domain,
            "adapter-a",
            1,
            vec![view("pool", AdapterSnapshotSupport::Partial)],
            vec![upsert(&second, None), upsert(&first, Some(99))],
        ),
    )).unwrap_err();
    assert!(matches!(error, ResourceError::CorruptLog(_)));
    assert_eq!(registry, before);
}

pub(crate) fn domain() -> AuthorityDomainId {
    AuthorityDomainId { value: "authority-main".into() }
}

pub(crate) fn identity(adapter: &str, kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity::new(
        AdapterId { value: adapter.into() },
        ResourceKind { value: kind.into() },
        ResourceId { value: id.into() },
    ).unwrap()
}

pub(crate) fn view(kind: &str, tier: AdapterSnapshotSupport) -> ResourceViewStateUpdate {
    ResourceViewStateUpdate {
        resource_kind: Some(ResourceKind { value: kind.into() }),
        completeness: tier as i32,
    }
}

pub(crate) fn upsert(identity: &ResourceIdentity, from: Option<u64>) -> ResourceStateMutation {
    ResourceStateMutation {
        identity: Some(identity.to_scope().resource.unwrap()),
        from_revision_lsn: from.map(|value| Lsn { value }),
        mutation: Some(resource_state_mutation::Mutation::Upsert(ResourceStateUpsert {
            resource_payload: Some(envelope("resource.schema")),
            projection_payload: Some(envelope("projection.schema")),
        })),
    }
}

pub(crate) fn tombstone(
    identity: &ResourceIdentity,
    from: u64,
    replacement: Option<&ResourceIdentity>,
) -> ResourceStateMutation {
    ResourceStateMutation {
        identity: Some(identity.to_scope().resource.unwrap()),
        from_revision_lsn: Some(Lsn { value: from }),
        mutation: Some(resource_state_mutation::Mutation::Tombstone(ResourceStateTombstone {
            replaced_by: replacement.map(|id| id.to_scope().resource.unwrap()),
        })),
    }
}

pub(crate) fn envelope(schema: &str) -> PayloadEnvelope {
    PayloadEnvelope {
        payload: br#"{"ok":true}"#.to_vec(),
        content_type: PayloadContentType::Json as i32,
        schema_ref: schema.into(),
    }
}

pub(crate) fn state_event(
    domain: &AuthorityDomainId,
    adapter: &str,
    generation: u64,
    views: Vec<ResourceViewStateUpdate>,
    mutations: Vec<ResourceStateMutation>,
) -> ResourceStateEvent {
    ResourceStateEvent {
        authority_domain_id: Some(domain.clone()),
        source_adapter_id: Some(AdapterId { value: adapter.into() }),
        source_adapter_generation: Some(Generation { value: generation }),
        views,
        mutations,
        observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
    }
}

pub(crate) fn recorded(
    domain: &AuthorityDomainId,
    lsn: u64,
    state: ResourceStateEvent,
) -> RecordedEvent {
    RecordedEvent {
        event_id: event_id(domain.clone(), lsn),
        payload: events::encode(&state),
    }
}
