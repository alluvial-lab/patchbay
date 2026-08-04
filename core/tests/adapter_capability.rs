use patchbay_contracts::patchbay::{
    AdapterCapability, AdapterId, AdapterRegistration, AdapterSnapshotSupport,
    AdapterTargetCategory, AuthorityDomainId, EndpointId, Generation, Observation, ObservationKind,
    PayloadContentType, PayloadEnvelope, ResourceCapability, ResourceId, ResourceKind,
    ResourceProjectionContract, SchemaDescriptor, StoredEventKind, StoredEventPayload,
};
use patchbay_core::{
    adapter::{
        ingest_registration, rebuild_from_log, AdapterRegistry, CapabilityValidationContext,
        ValidatedAdapterCapability,
    },
    resource::ResourceIdentity,
    storage::{RusqliteStorage, Storage},
};
use prost::Message;

fn descriptor(schema_ref: &str, content_type: PayloadContentType) -> SchemaDescriptor {
    SchemaDescriptor {
        schema_ref: schema_ref.to_owned(),
        content_type: content_type as i32,
    }
}

fn resource(kind: &str, snapshot_support: AdapterSnapshotSupport) -> ResourceCapability {
    ResourceCapability {
        resource_kind: Some(ResourceKind {
            value: kind.to_owned(),
        }),
        snapshot_support: snapshot_support as i32,
        projection_contract: Some(ResourceProjectionContract {
            target_category: AdapterTargetCategory::OperationalResource as i32,
            payload_schema: Some(descriptor(
                &format!("example.{kind}.payload.v1"),
                PayloadContentType::Protobuf,
            )),
            projection_schema: Some(descriptor(
                &format!("example.{kind}.projection.v1"),
                PayloadContentType::Json,
            )),
        }),
    }
}

fn session_capability() -> AdapterCapability {
    AdapterCapability {
        session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
        target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
        ..AdapterCapability::default()
    }
}

fn resource_capability() -> AdapterCapability {
    AdapterCapability {
        session_snapshot_support: AdapterSnapshotSupport::Unspecified as i32,
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![
            resource("provider_pool", AdapterSnapshotSupport::Authoritative),
            resource("usage_window", AdapterSnapshotSupport::Partial),
        ],
        ..AdapterCapability::default()
    }
}

fn registration(adapter: &str, capability: AdapterCapability) -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(AdapterId {
            value: adapter.to_owned(),
        }),
        endpoint_id: Some(EndpointId {
            value: format!("{adapter}-endpoint"),
        }),
        authority_domain_id: Some(AuthorityDomainId {
            value: "authority-main".to_owned(),
        }),
        adapter_generation: Some(Generation { value: 1 }),
        capability: Some(capability),
        ..AdapterRegistration::default()
    }
}

#[test]
fn validates_session_resource_and_mixed_category_relationships() {
    ValidatedAdapterCapability::try_from_wire(
        &session_capability(),
        CapabilityValidationContext::Attach,
    )
    .expect("session-only capability");
    ValidatedAdapterCapability::try_from_wire(
        &resource_capability(),
        CapabilityValidationContext::Attach,
    )
    .expect("resource-only capability");

    let mut mixed = resource_capability();
    mixed
        .target_categories
        .insert(0, AdapterTargetCategory::RuntimeSession as i32);
    mixed.session_snapshot_support = AdapterSnapshotSupport::Authoritative as i32;
    let validated =
        ValidatedAdapterCapability::try_from_wire(&mixed, CapabilityValidationContext::Attach)
            .expect("mixed capability");
    assert!(validated.targets(AdapterTargetCategory::RuntimeSession));
    assert!(validated.targets(AdapterTargetCategory::OperationalResource));
}

#[tokio::test]
async fn exact_identity_lookup_and_schema_binding_select_each_resource_declaration() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let mut registry = AdapterRegistry::new();
    ingest_registration(
        &storage,
        &mut registry,
        registration("resources", resource_capability()),
    )
    .await
    .expect("resource adapter attaches");

    let pool = ResourceIdentity::new(
        AdapterId {
            value: "resources".to_owned(),
        },
        ResourceKind {
            value: "provider_pool".to_owned(),
        },
        ResourceId {
            value: "pool-a".to_owned(),
        },
    )
    .expect("identity");
    assert_eq!(
        registry
            .resource_capability(&pool)
            .expect("declared pool")
            .snapshot_support(),
        AdapterSnapshotSupport::Authoritative
    );

    let usage = ResourceIdentity::new(
        AdapterId {
            value: "resources".to_owned(),
        },
        ResourceKind {
            value: "usage_window".to_owned(),
        },
        ResourceId {
            value: "window-a".to_owned(),
        },
    )
    .expect("identity");
    let declared = registry
        .resource_capability(&usage)
        .expect("declared usage window");
    assert_eq!(declared.snapshot_support(), AdapterSnapshotSupport::Partial);
    registry
        .validate_resource_projection(
            &usage,
            &PayloadEnvelope {
                content_type: PayloadContentType::Protobuf as i32,
                schema_ref: "example.usage_window.payload.v1".to_owned(),
                ..PayloadEnvelope::default()
            },
            &PayloadEnvelope {
                content_type: PayloadContentType::Json as i32,
                schema_ref: "example.usage_window.projection.v1".to_owned(),
                ..PayloadEnvelope::default()
            },
        )
        .expect("exact descriptors bind");

    let foreign = ResourceIdentity::new(
        AdapterId {
            value: "other".to_owned(),
        },
        ResourceKind {
            value: "usage_window".to_owned(),
        },
        ResourceId {
            value: "window-a".to_owned(),
        },
    )
    .expect("identity");
    assert!(registry.resource_capability(&foreign).is_none());

    let undeclared = ResourceIdentity::new(
        AdapterId {
            value: "resources".to_owned(),
        },
        ResourceKind {
            value: "knowledge_bundle".to_owned(),
        },
        ResourceId {
            value: "bundle-a".to_owned(),
        },
    )
    .expect("identity");
    assert!(registry.resource_capability(&undeclared).is_none());

    let mismatch = registry.validate_resource_projection(
        &usage,
        &PayloadEnvelope {
            content_type: PayloadContentType::Json as i32,
            schema_ref: "example.usage_window.payload.v1".to_owned(),
            ..PayloadEnvelope::default()
        },
        &PayloadEnvelope {
            content_type: PayloadContentType::Json as i32,
            schema_ref: "example.usage_window.projection.v1".to_owned(),
            ..PayloadEnvelope::default()
        },
    );
    assert!(mismatch.is_err(), "content type mismatch must fail closed");
}

#[test]
fn invalid_manifest_shapes_fail_closed() {
    let mut cases = Vec::new();

    cases.push(("missing categories", AdapterCapability::default()));

    let mut duplicate_categories = session_capability();
    duplicate_categories
        .target_categories
        .push(AdapterTargetCategory::RuntimeSession as i32);
    cases.push(("duplicate categories", duplicate_categories));

    let mut unknown_category = session_capability();
    unknown_category.target_categories = vec![99];
    cases.push(("unknown category", unknown_category));

    let mut reserved_category = session_capability();
    reserved_category.target_categories = vec![AdapterTargetCategory::KnowledgeBundle as i32];
    cases.push(("reserved category", reserved_category));

    let mut unspecified_session_tier = session_capability();
    unspecified_session_tier.session_snapshot_support = AdapterSnapshotSupport::Unspecified as i32;
    cases.push(("unspecified session tier", unspecified_session_tier));

    let mut resource_without_category = resource_capability();
    resource_without_category.target_categories =
        vec![AdapterTargetCategory::RuntimeSession as i32];
    resource_without_category.session_snapshot_support = AdapterSnapshotSupport::Partial as i32;
    cases.push(("resource without category", resource_without_category));

    let mut category_without_resource = resource_capability();
    category_without_resource.resource_capabilities.clear();
    cases.push(("category without resource", category_without_resource));

    let mut duplicate_kind = resource_capability();
    duplicate_kind
        .resource_capabilities
        .push(resource("provider_pool", AdapterSnapshotSupport::None));
    cases.push(("duplicate kind", duplicate_kind));

    let mut unspecified_resource_tier = resource_capability();
    unspecified_resource_tier.resource_capabilities[0].snapshot_support =
        AdapterSnapshotSupport::Unspecified as i32;
    cases.push(("unspecified resource tier", unspecified_resource_tier));

    let mut unknown_resource_tier = resource_capability();
    unknown_resource_tier.resource_capabilities[0].snapshot_support = 99;
    cases.push(("unknown resource tier", unknown_resource_tier));

    let mut missing_kind = resource_capability();
    missing_kind.resource_capabilities[0].resource_kind = None;
    cases.push(("missing resource kind", missing_kind));

    let mut missing_projection = resource_capability();
    missing_projection.resource_capabilities[0].projection_contract = None;
    cases.push(("missing projection", missing_projection));

    let mut category_mismatch = resource_capability();
    category_mismatch.resource_capabilities[0]
        .projection_contract
        .as_mut()
        .expect("projection")
        .target_category = AdapterTargetCategory::KnowledgeBundle as i32;
    cases.push(("projection category mismatch", category_mismatch));

    let mut missing_descriptor = resource_capability();
    missing_descriptor.resource_capabilities[0]
        .projection_contract
        .as_mut()
        .expect("projection")
        .payload_schema = None;
    cases.push(("missing descriptor", missing_descriptor));

    let mut invalid_ref = resource_capability();
    invalid_ref.resource_capabilities[0]
        .projection_contract
        .as_mut()
        .expect("projection")
        .payload_schema
        .as_mut()
        .expect("descriptor")
        .schema_ref = "contains whitespace".to_owned();
    cases.push(("invalid schema ref", invalid_ref));

    let mut unspecified_content = resource_capability();
    unspecified_content.resource_capabilities[0]
        .projection_contract
        .as_mut()
        .expect("projection")
        .projection_schema
        .as_mut()
        .expect("descriptor")
        .content_type = PayloadContentType::Unspecified as i32;
    cases.push(("unspecified content type", unspecified_content));

    let mut unknown_content = resource_capability();
    unknown_content.resource_capabilities[0]
        .projection_contract
        .as_mut()
        .expect("projection")
        .projection_schema
        .as_mut()
        .expect("descriptor")
        .content_type = 99;
    cases.push(("unknown content type", unknown_content));

    for (name, capability) in cases {
        assert!(
            ValidatedAdapterCapability::try_from_wire(
                &capability,
                CapabilityValidationContext::Attach,
            )
            .is_err(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn replay_normalizes_only_legacy_resource_empty_manifest() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let domain = AuthorityDomainId {
        value: "authority-main".to_owned(),
    };
    append_registration_observation(
        &storage,
        &domain,
        registration("legacy", AdapterCapability::default()),
    )
    .await;

    let rebuilt = rebuild_from_log(&storage, &domain)
        .await
        .expect("legacy session registration replays");
    let record = rebuilt
        .get(&AdapterId {
            value: "legacy".to_owned(),
        })
        .expect("legacy record");
    assert!(record
        .validated_capability
        .targets(AdapterTargetCategory::RuntimeSession));
    assert!(record
        .validated_capability
        .resource(&ResourceKind {
            value: "provider_pool".to_owned(),
        })
        .is_none());

    let fresh_storage = RusqliteStorage::open_in_memory().expect("storage");
    let mut fresh_registry = AdapterRegistry::new();
    assert!(ingest_registration(
        &fresh_storage,
        &mut fresh_registry,
        registration("legacy", AdapterCapability::default()),
    )
    .await
    .is_err());
    assert!(fresh_storage
        .read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .expect("read")
        .is_empty());
}

#[tokio::test]
async fn replay_rejects_categoryless_resource_declarations_as_corrupt() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let domain = AuthorityDomainId {
        value: "authority-main".to_owned(),
    };
    let mut categoryless = resource_capability();
    categoryless.target_categories.clear();
    append_registration_observation(&storage, &domain, registration("corrupt", categoryless)).await;
    assert!(rebuild_from_log(&storage, &domain).await.is_err());
}

async fn append_registration_observation(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
    registration: AdapterRegistration,
) {
    let observation = Observation {
        authority_domain_id: Some(domain.clone()),
        kind: ObservationKind::Event as i32,
        payload: Some(PayloadEnvelope {
            payload: registration.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: "patchbay.AdapterRegistration".to_owned(),
        }),
        ..Observation::default()
    };
    storage
        .append(
            domain,
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: observation.encode_to_vec(),
            },
        )
        .await
        .expect("append registration observation");
}
