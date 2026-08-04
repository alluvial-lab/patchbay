use patchbay_contracts::patchbay::{
    resource_report_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, Generation,
    PayloadContentType, PayloadEnvelope, ResourceId, ResourceIdentity, ResourceKind,
    ResourceReportMutation, ResourceStateTombstone,
    ResourceStateUnknown, ResourceStateUpsert, ResourceViewReport,
};
use patchbay_core::{
    resource::{ingest_resource_report, ResourceRegistry, ResourceReportMode, ValidatedResourceReport},
    storage::RusqliteStorage,
};
use prost_types::Timestamp;

#[tokio::test]
async fn snapshot_tiers_reconcile_omissions_without_fabricating_current_state() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();

    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Authoritative,
            vec![upsert("one"), upsert("two")],
        ),
    )
    .await
    .unwrap();
    let first = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(first.touched_views, 1);
    assert_eq!(freshness(&registry, "two"), "Stale");
    assert!(registry.contains(&domain_identity("two")));

    // NONE carries no reconstructed members and keeps omitted cached state stale.
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::None,
            Vec::new(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(freshness(&registry, "one"), "Stale");

    // A later authoritative complete view terminally retires omitted members.
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Authoritative,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();
    assert!(!registry.contains(&domain_identity("two")));
    assert!(registry.get(&domain_identity("two")).unwrap().tombstoned());
}

#[tokio::test]
async fn delta_omission_is_inert_and_replacement_is_one_atomic_event() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Authoritative,
            vec![upsert("old"), upsert("untouched")],
        ),
    )
    .await
    .unwrap();
    let untouched_revision = registry.get(&domain_identity("untouched")).unwrap().revision_lsn;

    let replacement = ResourceReportMutation {
        identity: Some(wire_identity("old")),
        mutation: Some(resource_report_mutation::Mutation::Tombstone(
            ResourceStateTombstone {
                replaced_by: Some(wire_identity("new")),
            },
        )),
    };
    let result = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Authoritative,
            vec![replacement, upsert("new")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.touched_resources, 2);
    assert!(!registry.contains(&domain_identity("old")));
    assert!(registry.contains(&domain_identity("new")));
    assert_eq!(
        registry.get(&domain_identity("old")).unwrap().replaced_by.as_ref(),
        Some(&domain_identity("new"))
    );
    assert_eq!(
        registry.get(&domain_identity("untouched")).unwrap().revision_lsn,
        untouched_revision,
        "delta omission must not mutate another identity"
    );
}

#[tokio::test]
async fn newer_adapter_generation_stales_prior_unreported_state_and_old_generation_rejects() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();

    ingest_resource_report(
        &storage,
        &mut registry,
        report_for_kind(
            2,
            "usage_window",
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![ResourceReportMutation {
                identity: Some(wire_identity_for_kind("usage_window", "window")),
                mutation: Some(resource_report_mutation::Mutation::Unknown(
                    ResourceStateUnknown {},
                )),
            }],
        ),
    )
    .await
    .unwrap();
    assert_eq!(freshness(&registry, "one"), "Stale");

    let error = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert("late")],
        ),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("stale adapter generation"));
    assert!(!registry.contains(&domain_identity("late")));
}

#[tokio::test]
async fn malformed_none_and_duplicate_reports_reject_before_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    let bad_none = report(
        1,
        ResourceReportMode::Snapshot,
        AdapterSnapshotSupport::None,
        vec![upsert("one")],
    );
    assert!(ingest_resource_report(&storage, &mut registry, bad_none).await.is_err());

    let mut duplicate = report(
        1,
        ResourceReportMode::Delta,
        AdapterSnapshotSupport::Partial,
        vec![upsert("one"), upsert("one")],
    );
    duplicate.views.push(duplicate.views[0].clone());
    assert!(ingest_resource_report(&storage, &mut registry, duplicate).await.is_err());
    assert_eq!(registry.resources().count(), 0);
}

fn report(
    generation: u64,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    report_for_kind(generation, "provider_pool", mode, tier, mutations)
}

fn report_for_kind(
    generation: u64,
    kind: &str,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    ValidatedResourceReport {
        authority_domain_id: domain(),
        adapter_id: AdapterId { value: "adapter-a".into() },
        adapter_generation: Generation { value: generation },
        mode,
        views: vec![ResourceViewReport {
            resource_kind: Some(ResourceKind { value: kind.into() }),
            completeness: tier as i32,
            mutations,
        }],
        observed_at: Timestamp { seconds: 100 + generation as i64, nanos: 0 },
    }
}

fn upsert(id: &str) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(wire_identity(id)),
        mutation: Some(resource_report_mutation::Mutation::Upsert(ResourceStateUpsert {
            resource_payload: Some(envelope("resource.schema")),
            projection_payload: Some(envelope("projection.schema")),
        })),
    }
}

fn envelope(schema: &str) -> PayloadEnvelope {
    PayloadEnvelope {
        payload: vec![1],
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: schema.into(),
    }
}

fn domain() -> AuthorityDomainId {
    AuthorityDomainId { value: "authority-main".into() }
}

fn wire_identity(id: &str) -> ResourceIdentity {
    wire_identity_for_kind("provider_pool", id)
}

fn wire_identity_for_kind(kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity {
        adapter_id: Some(AdapterId { value: "adapter-a".into() }),
        resource_kind: Some(ResourceKind { value: kind.into() }),
        resource_id: Some(ResourceId { value: id.into() }),
    }
}

fn domain_identity(id: &str) -> patchbay_core::resource::ResourceIdentity {
    patchbay_core::resource::ResourceIdentity::try_from_wire(&wire_identity(id)).unwrap()
}

fn freshness(registry: &ResourceRegistry, id: &str) -> String {
    format!("{:?}", registry.get(&domain_identity(id)).unwrap().freshness)
}

