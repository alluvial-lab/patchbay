use patchbay_contracts::patchbay::{
    resource_state_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, Generation,
    PayloadContentType, PayloadEnvelope, ResourceId, ResourceKind, ResourceStateEvent,
    ResourceStateMutation, ResourceStateUpsert, ResourceViewStateUpdate, StoredEventKind,
    StoredEventPayload,
};
use patchbay_core::{
    resource::{replay::rebuild_from_events, ResourceIdentity},
    storage::{event_id, RecordedEvent},
};
use prost::Message;
use prost_types::Timestamp;

#[test]
fn replay_is_deterministic_and_rejects_wrong_domain_or_non_increasing_prefix() {
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let identity = ResourceIdentity::new(
        AdapterId { value: "adapter-a".into() },
        ResourceKind { value: "provider_pool".into() },
        ResourceId { value: "pool-1".into() },
    ).unwrap();
    let event = state_event(&domain, &identity);
    let prefix = vec![recorded(&domain, 1, event.clone())];
    let first = rebuild_from_events(&domain, &prefix).unwrap();
    let second = rebuild_from_events(&domain, &prefix).unwrap();
    assert_eq!(first, second);
    assert!(first.contains(&identity));

    let other = AuthorityDomainId { value: "authority-other".into() };
    assert!(rebuild_from_events(&domain, &[recorded(&other, 1, event.clone())]).is_err());
    assert!(rebuild_from_events(
        &domain,
        &[recorded(&domain, 1, event.clone()), recorded(&domain, 1, event)],
    ).is_err());
}

#[test]
fn replay_ignores_sibling_event_kinds_without_hiding_resource_state() {
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let identity = ResourceIdentity::new(
        AdapterId { value: "adapter-a".into() },
        ResourceKind { value: "provider_pool".into() },
        ResourceId { value: "pool-1".into() },
    ).unwrap();
    let prefix = vec![
        RecordedEvent {
            event_id: event_id(domain.clone(), 1),
            payload: StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: Vec::new(),
            },
        },
        recorded(&domain, 2, state_event(&domain, &identity)),
    ];
    assert!(rebuild_from_events(&domain, &prefix).unwrap().contains(&identity));
}

#[test]
fn opaque_observation_dispatch_mutant_is_killed() {
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let identity = ResourceIdentity::new(
        AdapterId { value: "adapter-a".into() },
        ResourceKind { value: "provider_pool".into() },
        ResourceId { value: "injected".into() },
    ).unwrap();
    let encoded_state = state_event(&domain, &identity).encode_to_vec();
    let observation = RecordedEvent {
        event_id: event_id(domain.clone(), 1),
        payload: StoredEventPayload {
            kind: StoredEventKind::Observation as i32,
            payload: encoded_state.clone(),
        },
    };

    let production = rebuild_from_events(&domain, std::slice::from_ref(&observation)).unwrap();
    assert!(!production.contains(&identity));

    // Claim-breaking mutant: dispatch bytes by decodability instead of by the
    // durable StoredEventKind discriminator.
    let decoded = ResourceStateEvent::decode(encoded_state.as_slice()).expect("plausible injected bytes");
    let mutant = rebuild_from_events(&domain, &[recorded(&domain, 1, decoded)]).unwrap();
    assert!(mutant.contains(&identity), "the independent no-core-state oracle kills the dispatch mutant");
}

fn state_event(domain: &AuthorityDomainId, identity: &ResourceIdentity) -> ResourceStateEvent {
    ResourceStateEvent {
        authority_domain_id: Some(domain.clone()),
        source_adapter_id: Some(identity.adapter_id().clone()),
        source_adapter_generation: Some(Generation { value: 1 }),
        views: vec![ResourceViewStateUpdate {
            resource_kind: Some(identity.resource_kind().clone()),
            completeness: AdapterSnapshotSupport::Authoritative as i32,
        }],
        mutations: vec![ResourceStateMutation {
            identity: Some(identity.to_scope().resource.unwrap()),
            from_revision_lsn: None,
            mutation: Some(resource_state_mutation::Mutation::Upsert(ResourceStateUpsert {
                resource_payload: Some(envelope("resource.schema")),
                projection_payload: Some(envelope("projection.schema")),
            })),
        }],
        observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
    }
}

fn envelope(schema: &str) -> PayloadEnvelope {
    PayloadEnvelope {
        payload: vec![1],
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: schema.into(),
    }
}

fn recorded(domain: &AuthorityDomainId, lsn: u64, state: ResourceStateEvent) -> RecordedEvent {
    RecordedEvent {
        event_id: event_id(domain.clone(), lsn),
        payload: StoredEventPayload {
            kind: StoredEventKind::ResourceState as i32,
            payload: state.encode_to_vec(),
        },
    }
}
