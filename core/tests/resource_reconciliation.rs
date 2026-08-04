use patchbay_contracts::patchbay::{
    resource_report_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, Generation,
    PayloadContentType, PayloadEnvelope, ResourceId, ResourceIdentity as WireResourceIdentity,
    ResourceKind, ResourceReportMutation, ResourceStateUpsert, ResourceViewReport,
};
use patchbay_core::{
    resource::{
        ingest_resource_report, rebuild_from_log, ResourceRegistry, ResourceReportMode,
        ValidatedResourceReport,
    },
    storage::RusqliteStorage,
};
use prost_types::Timestamp;
use proptest::prelude::*;

proptest! {
    #[test]
    fn reconnect_branch_replay_matches_hot_projection(
        adapter in "[a-z]{1,8}",
        kind in "[a-z]{1,8}",
        local_id in "[a-z0-9]{1,8}",
        branch in 0u8..4,
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut hot = ResourceRegistry::new();
            let identity = WireResourceIdentity {
                adapter_id: Some(AdapterId { value: adapter.clone() }),
                resource_kind: Some(ResourceKind { value: kind.clone() }),
                resource_id: Some(ResourceId { value: local_id }),
            };
            ingest_resource_report(
                &storage,
                &mut hot,
                report(
                    &adapter,
                    &kind,
                    ResourceReportMode::Snapshot,
                    AdapterSnapshotSupport::Authoritative,
                    vec![upsert(identity.clone())],
                ),
            )
            .await
            .unwrap();

            let (mode, tier) = match branch {
                0 => (ResourceReportMode::Snapshot, AdapterSnapshotSupport::Authoritative),
                1 => (ResourceReportMode::Snapshot, AdapterSnapshotSupport::Partial),
                2 => (ResourceReportMode::Snapshot, AdapterSnapshotSupport::None),
                _ => (ResourceReportMode::Delta, AdapterSnapshotSupport::Authoritative),
            };
            ingest_resource_report(
                &storage,
                &mut hot,
                report(&adapter, &kind, mode, tier, Vec::new()),
            )
            .await
            .unwrap();

            let domain_identity = patchbay_core::resource::ResourceIdentity::try_from_wire(&identity).unwrap();
            match branch {
                0 => prop_assert!(!hot.contains(&domain_identity)),
                1 | 2 => prop_assert_eq!(
                    hot.get(&domain_identity).unwrap().freshness,
                    patchbay_contracts::patchbay::ResourceFreshnessState::Stale,
                ),
                _ => prop_assert_eq!(
                    hot.get(&domain_identity).unwrap().freshness,
                    patchbay_contracts::patchbay::ResourceFreshnessState::Current,
                ),
            }
            let replayed = rebuild_from_log(&storage, &domain()).await.unwrap();
            let replayed_twice = rebuild_from_log(&storage, &domain()).await.unwrap();
            prop_assert_eq!(&hot, &replayed);
            prop_assert_eq!(&replayed, &replayed_twice);
            Ok(())
        })?;
    }
}

fn report(
    adapter: &str,
    kind: &str,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    ValidatedResourceReport {
        authority_domain_id: domain(),
        adapter_id: AdapterId { value: adapter.into() },
        adapter_generation: Generation { value: 1 },
        mode,
        views: vec![ResourceViewReport {
            resource_kind: Some(ResourceKind { value: kind.into() }),
            completeness: tier as i32,
            mutations,
        }],
        observed_at: Timestamp { seconds: 100, nanos: 0 },
    }
}

fn upsert(identity: WireResourceIdentity) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(identity),
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
