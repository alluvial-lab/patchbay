use patchbay_contracts::patchbay::{
    resource_report_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, EventId,
    Generation, IdempotencyKey, Lsn, PayloadContentType, PayloadEnvelope, ResourceId,
    ResourceIdentity as WireResourceIdentity,
    ResourceKind, ResourceReportMutation, ResourceStateUpsert, ResourceViewReport,
    StoredEventPayload,
};
use patchbay_core::{
    resource::{
        ingest_resource_report, rebuild_from_log, ResourceRegistry, ResourceReportMode,
        ValidatedResourceReport,
    },
    storage::{
        DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot,
        TargetKey,
    },
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

#[derive(Clone)]
struct RejectAppendStorage {
    inner: RusqliteStorage,
}

impl RejectAppendStorage {
    fn new() -> Self {
        Self { inner: RusqliteStorage::open_in_memory().unwrap() }
    }
}

impl Storage for RejectAppendStorage {
    async fn append(&self, _domain: &AuthorityDomainId, _payload: StoredEventPayload) -> Result<EventId, StorageError> {
        Err(StorageError::WriteFailed { message: "injected append failure".into(), retryable: false })
    }

    async fn append_dedup(
        &self,
        domain: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner.append_dedup(domain, key, target, payload).await
    }

    async fn read_after(&self, domain: &AuthorityDomainId, cursor: Lsn) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(domain, cursor).await
    }

    async fn write_snapshot(&self, domain: &AuthorityDomainId, lsn: Lsn, payload: Vec<u8>) -> Result<(), StorageError> {
        self.inner.write_snapshot(domain, lsn, payload).await
    }

    async fn load_latest_snapshot(&self, domain: &AuthorityDomainId, bound: Option<Lsn>) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner.load_latest_snapshot(domain, bound).await
    }
}

#[tokio::test]
async fn resource_projection_never_folds_before_durable_append() {
    let storage = RejectAppendStorage::new();
    let mut registry = ResourceRegistry::new();
    let result = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            "adapter-a",
            "provider_pool",
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert(WireResourceIdentity {
                adapter_id: Some(AdapterId { value: "adapter-a".into() }),
                resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                resource_id: Some(ResourceId { value: "pool-1".into() }),
            })],
        ),
    ).await;
    assert!(result.is_err());
    assert_eq!(registry.resources().count(), 0);
    assert_eq!(registry.views().count(), 0);
}

#[derive(Clone, Copy, Debug)]
struct TraceStep {
    branch: u8,
    refresh_cached: bool,
}

fn any_trace() -> impl Strategy<Value = (Vec<TraceStep>, Option<bool>)> {
    (
        prop::collection::vec((0u8..4, any::<bool>()), 1..20),
        prop::option::of(any::<bool>()),
    )
        .prop_map(|(steps, authoritative_final)| {
            (
                steps
                    .into_iter()
                    .map(|(branch, refresh_cached)| TraceStep { branch, refresh_cached })
                    .collect(),
                authoritative_final,
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_resource_report_trace_matches_independent_truth_table(
        adapter in "[a-z]{1,8}",
        kind in "[a-z]{1,8}",
        cached_id in "[a-z0-9]{1,8}",
        unknown_id in "[a-z0-9]{1,8}",
        (trace, authoritative_final) in any_trace(),
    ) {
        prop_assume!(cached_id != unknown_id);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async move {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut hot = ResourceRegistry::new();
            let cached = WireResourceIdentity {
                adapter_id: Some(AdapterId { value: adapter.clone() }),
                resource_kind: Some(ResourceKind { value: kind.clone() }),
                resource_id: Some(ResourceId { value: cached_id }),
            };
            let unknown = WireResourceIdentity {
                adapter_id: Some(AdapterId { value: adapter.clone() }),
                resource_kind: Some(ResourceKind { value: kind.clone() }),
                resource_id: Some(ResourceId { value: unknown_id }),
            };
            ingest_resource_report(
                &storage,
                &mut hot,
                report(
                    &adapter,
                    &kind,
                    ResourceReportMode::Delta,
                    AdapterSnapshotSupport::Partial,
                    vec![upsert(cached.clone()), unknown_mutation(unknown.clone())],
                ),
            ).await.unwrap();

            let cached_domain = patchbay_core::resource::ResourceIdentity::try_from_wire(&cached).unwrap();
            let unknown_domain = patchbay_core::resource::ResourceIdentity::try_from_wire(&unknown).unwrap();
            let mut expected_cached = patchbay_contracts::patchbay::ResourceFreshnessState::Current;
            for step in trace {
                let (mode, tier) = match step.branch {
                    0 => (ResourceReportMode::Snapshot, AdapterSnapshotSupport::Partial),
                    1 => (ResourceReportMode::Snapshot, AdapterSnapshotSupport::None),
                    2 => (ResourceReportMode::Delta, AdapterSnapshotSupport::Partial),
                    _ => (ResourceReportMode::Delta, AdapterSnapshotSupport::Authoritative),
                };
                let explicitly_listed = step.refresh_cached && tier != AdapterSnapshotSupport::None;
                let mutations = if explicitly_listed { vec![upsert(cached.clone())] } else { vec![] };
                ingest_resource_report(&storage, &mut hot, report(&adapter, &kind, mode, tier, mutations)).await.unwrap();

                // Independent truth table over raw mode/tier/explicit presence.
                expected_cached = if explicitly_listed {
                    patchbay_contracts::patchbay::ResourceFreshnessState::Current
                } else if mode == ResourceReportMode::Snapshot {
                    patchbay_contracts::patchbay::ResourceFreshnessState::Stale
                } else {
                    expected_cached
                };
                prop_assert_eq!(hot.get(&cached_domain).unwrap().freshness, expected_cached);
                prop_assert_eq!(
                    hot.get(&unknown_domain).unwrap().freshness,
                    patchbay_contracts::patchbay::ResourceFreshnessState::Unknown,
                );
                prop_assert!(!hot.get(&cached_domain).unwrap().tombstoned());
                prop_assert!(!hot.get(&unknown_domain).unwrap().tombstoned());
            }

            if let Some(include_cached) = authoritative_final {
                let mutations = if include_cached { vec![upsert(cached.clone())] } else { vec![] };
                ingest_resource_report(
                    &storage,
                    &mut hot,
                    report(
                        &adapter,
                        &kind,
                        ResourceReportMode::Snapshot,
                        AdapterSnapshotSupport::Authoritative,
                        mutations,
                    ),
                ).await.unwrap();
                prop_assert_eq!(hot.get(&cached_domain).unwrap().tombstoned(), !include_cached);
                if include_cached {
                    prop_assert_eq!(
                        hot.get(&cached_domain).unwrap().freshness,
                        patchbay_contracts::patchbay::ResourceFreshnessState::Current,
                    );
                }
                prop_assert!(hot.get(&unknown_domain).unwrap().tombstoned());
                prop_assert_eq!(
                    hot.get(&unknown_domain).unwrap().freshness,
                    patchbay_contracts::patchbay::ResourceFreshnessState::Unknown,
                );
            }

            let replayed = rebuild_from_log(&storage, &domain()).await.unwrap();
            let replayed_twice = rebuild_from_log(&storage, &domain()).await.unwrap();
            prop_assert_eq!(&hot, &replayed);
            prop_assert_eq!(&replayed, &replayed_twice);
            Ok(())
        })?;
    }
}

#[test]
fn completeness_truth_table_kills_omission_mutants() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Expected { Current, Stale, Tombstoned }
    let oracle = |mode: ResourceReportMode, tier: AdapterSnapshotSupport| match (mode, tier) {
        (ResourceReportMode::Snapshot, AdapterSnapshotSupport::Authoritative) => Expected::Tombstoned,
        (ResourceReportMode::Snapshot, AdapterSnapshotSupport::Partial | AdapterSnapshotSupport::None) => Expected::Stale,
        (ResourceReportMode::Delta, _) => Expected::Current,
        _ => Expected::Current,
    };
    let authoritative_as_partial = |_mode: ResourceReportMode, _tier: AdapterSnapshotSupport| Expected::Stale;
    let weak_as_authoritative = |_mode: ResourceReportMode, _tier: AdapterSnapshotSupport| Expected::Tombstoned;
    let delta_as_snapshot = |_mode: ResourceReportMode, _tier: AdapterSnapshotSupport| Expected::Stale;

    assert_eq!(oracle(ResourceReportMode::Snapshot, AdapterSnapshotSupport::Authoritative), Expected::Tombstoned);
    assert_ne!(authoritative_as_partial(ResourceReportMode::Snapshot, AdapterSnapshotSupport::Authoritative), Expected::Tombstoned);
    assert_eq!(oracle(ResourceReportMode::Snapshot, AdapterSnapshotSupport::Partial), Expected::Stale);
    assert_ne!(weak_as_authoritative(ResourceReportMode::Snapshot, AdapterSnapshotSupport::Partial), Expected::Stale);
    assert_eq!(oracle(ResourceReportMode::Delta, AdapterSnapshotSupport::Partial), Expected::Current);
    assert_ne!(delta_as_snapshot(ResourceReportMode::Delta, AdapterSnapshotSupport::Partial), Expected::Current);
}

fn unknown_mutation(identity: WireResourceIdentity) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(identity),
        mutation: Some(resource_report_mutation::Mutation::Unknown(
            patchbay_contracts::patchbay::ResourceStateUnknown {},
        )),
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
