use std::collections::HashSet;

use patchbay_contracts::patchbay::{
    resource_report_mutation, AdapterId, AdapterSnapshotSupport, AuthorityDomainId, EventId,
    Generation, IdempotencyKey, Lsn, PayloadContentType, PayloadEnvelope, ResourceId,
    ResourceIdentity as WireResourceIdentity, ResourceKind, ResourceReportMutation,
    ResourceStateEvent, ResourceStateTombstone, ResourceStateUpsert, ResourceViewReport,
    ResourceViewStateUpdate, StoredEventKind, StoredEventPayload,
};
use patchbay_core::{
    resource::{
        events as resource_events, ingest_resource_report, rebuild_from_log,
        ResourceIdentity as DomainResourceIdentity, ResourceRegistry, ResourceReportMode,
        ValidatedResourceReport,
    },
    storage::{
        event_id, DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError,
        StoredSnapshot, TargetKey,
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
enum GenerationStep {
    Same,
    Advance,
}

#[derive(Clone, Copy, Debug)]
enum ReportAction {
    Refresh,
    Add,
}

#[derive(Clone, Copy, Debug)]
enum RetiredMutation {
    Upsert,
    Unknown,
    Tombstone,
}

#[derive(Clone, Copy, Debug)]
enum ReconciliationAction {
    Report {
        generation_step: GenerationStep,
        mode: ResourceReportMode,
        tier: AdapterSnapshotSupport,
        mutation: ReportAction,
    },
    ReplaceActive,
    RefeedCovered,
    MutateRetired(RetiredMutation),
    LowerGenerationBeyondPrefix,
}

fn arbitrary_reconciliation_trace() -> impl Strategy<Value = Vec<ReconciliationAction>> {
    let report = (
        prop_oneof![Just(GenerationStep::Same), Just(GenerationStep::Advance)],
        prop_oneof![Just(ResourceReportMode::Snapshot), Just(ResourceReportMode::Delta)],
        prop_oneof![
            Just(AdapterSnapshotSupport::Authoritative),
            Just(AdapterSnapshotSupport::Partial),
            Just(AdapterSnapshotSupport::None),
        ],
        prop_oneof![Just(ReportAction::Refresh), Just(ReportAction::Add)],
    )
        .prop_map(|(generation_step, mode, tier, mutation)| ReconciliationAction::Report {
            generation_step,
            mode,
            tier,
            mutation,
        });
    let action = prop_oneof![
        5 => report,
        2 => Just(ReconciliationAction::ReplaceActive),
        2 => Just(ReconciliationAction::RefeedCovered),
        1 => Just(ReconciliationAction::MutateRetired(RetiredMutation::Upsert)),
        1 => Just(ReconciliationAction::MutateRetired(RetiredMutation::Unknown)),
        1 => Just(ReconciliationAction::MutateRetired(RetiredMutation::Tombstone)),
        2 => Just(ReconciliationAction::LowerGenerationBeyondPrefix),
    ];
    prop::collection::vec(action, 1..=20)
}

#[derive(Clone, Debug)]
struct ReconciliationOracle {
    generation: u64,
    active: HashSet<DomainResourceIdentity>,
    retired: HashSet<DomainResourceIdentity>,
    applied_through_lsn: u64,
}

fn trace_identity(adapter: &str, kind: &str, local_id: String) -> DomainResourceIdentity {
    DomainResourceIdentity::new(
        AdapterId { value: adapter.into() },
        ResourceKind { value: kind.into() },
        ResourceId { value: local_id },
    ).unwrap()
}

fn ordered_identities(set: &HashSet<DomainResourceIdentity>) -> Vec<DomainResourceIdentity> {
    let mut identities = set.iter().cloned().collect::<Vec<_>>();
    identities.sort_by(|left, right| {
        left.resource_id().value.cmp(&right.resource_id().value)
    });
    identities
}

fn oracle_matches_projection(oracle: &ReconciliationOracle, registry: &ResourceRegistry) {
    let projected_active = registry
        .resources()
        .filter(|record| !record.tombstoned())
        .map(|record| record.identity.clone())
        .collect::<HashSet<_>>();
    assert_eq!(projected_active, oracle.active);
    for identity in &oracle.retired {
        assert!(registry.get(identity).is_some_and(|record| record.tombstoned()));
    }
}

async fn assert_prefix_convergence(
    storage: &RusqliteStorage,
    registry: &ResourceRegistry,
    oracle: &ReconciliationOracle,
) {
    let events = storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len() as u64, oracle.applied_through_lsn);
    let replay_a = rebuild_from_log(storage, &domain()).await.unwrap();
    let replay_b = rebuild_from_log(storage, &domain()).await.unwrap();
    assert_eq!(registry, &replay_a);
    assert_eq!(replay_a, replay_b);

    let mut covered_replay = replay_a.clone();
    for event in &events {
        covered_replay.observe(event).unwrap();
    }
    assert_eq!(covered_replay, replay_a);
    oracle_matches_projection(oracle, registry);
}

fn wire(identity: &DomainResourceIdentity) -> WireResourceIdentity {
    identity.to_scope().resource.expect("canonical resource identity")
}

fn replacement_tombstone(
    old: &DomainResourceIdentity,
    replacement: &DomainResourceIdentity,
) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(wire(old)),
        mutation: Some(resource_report_mutation::Mutation::Tombstone(
            ResourceStateTombstone {
                replaced_by: Some(wire(replacement)),
            },
        )),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_resource_reconciliation_trace_preserves_prefix_and_projection(
        adapter in "[a-z]{1,8}",
        kind in "[a-z]{1,8}",
        trace in arbitrary_reconciliation_trace(),
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        runtime.block_on(async move {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut hot = ResourceRegistry::new();
            let seed = trace_identity(&adapter, &kind, "seed".into());
            let first_active = trace_identity(&adapter, &kind, "active-0".into());

            ingest_resource_report(
                &storage,
                &mut hot,
                report_at_generation(
                    &adapter,
                    &kind,
                    1,
                    ResourceReportMode::Delta,
                    AdapterSnapshotSupport::Partial,
                    vec![upsert(wire(&seed))],
                ),
            ).await.unwrap();
            let mut oracle = ReconciliationOracle {
                generation: 1,
                active: HashSet::from([seed.clone()]),
                retired: HashSet::new(),
                applied_through_lsn: 1,
            };
            assert_prefix_convergence(&storage, &hot, &oracle).await;

            ingest_resource_report(
                &storage,
                &mut hot,
                report_at_generation(
                    &adapter,
                    &kind,
                    2,
                    ResourceReportMode::Delta,
                    AdapterSnapshotSupport::Partial,
                    vec![replacement_tombstone(&seed, &first_active), upsert(wire(&first_active))],
                ),
            ).await.unwrap();
            oracle.generation = 2;
            oracle.active.remove(&seed);
            oracle.active.insert(first_active);
            oracle.retired.insert(seed);
            oracle.applied_through_lsn = 2;
            assert_prefix_convergence(&storage, &hot, &oracle).await;

            for (step, action) in trace.into_iter().enumerate() {
                match action {
                    ReconciliationAction::Report { generation_step, mode, tier, mutation } => {
                        let generation = match generation_step {
                            GenerationStep::Same => oracle.generation,
                            GenerationStep::Advance => oracle.generation + 1,
                        };
                        let added = matches!(mutation, ReportAction::Add).then(|| {
                            trace_identity(&adapter, &kind, format!("added-{step}"))
                        });
                        let mut mutations = Vec::new();
                        if mode != ResourceReportMode::Snapshot
                            || tier != AdapterSnapshotSupport::None
                        {
                            mutations.extend(
                                ordered_identities(&oracle.active)
                                    .iter()
                                    .map(|identity| upsert(wire(identity))),
                            );
                            if let Some(identity) = added.as_ref() {
                                mutations.push(upsert(wire(identity)));
                            }
                        }
                        let result = ingest_resource_report(
                            &storage,
                            &mut hot,
                            report_at_generation(
                                &adapter,
                                &kind,
                                generation,
                                mode,
                                tier,
                                mutations,
                            ),
                        ).await.unwrap();
                        oracle.generation = generation;
                        if let Some(identity) = added {
                            if mode != ResourceReportMode::Snapshot
                                || tier != AdapterSnapshotSupport::None
                            {
                                oracle.active.insert(identity);
                            }
                        }
                        oracle.applied_through_lsn += 1;
                        assert_eq!(result.event_id.lsn.unwrap().value, oracle.applied_through_lsn);
                        assert_prefix_convergence(&storage, &hot, &oracle).await;
                    }
                    ReconciliationAction::ReplaceActive => {
                        let old = ordered_identities(&oracle.active).remove(0);
                        let replacement = trace_identity(&adapter, &kind, format!("replacement-{step}"));
                        let result = ingest_resource_report(
                            &storage,
                            &mut hot,
                            report_at_generation(
                                &adapter,
                                &kind,
                                oracle.generation,
                                ResourceReportMode::Delta,
                                AdapterSnapshotSupport::Partial,
                                vec![
                                    replacement_tombstone(&old, &replacement),
                                    upsert(wire(&replacement)),
                                ],
                            ),
                        ).await.unwrap();
                        oracle.active.remove(&old);
                        oracle.active.insert(replacement);
                        oracle.retired.insert(old);
                        oracle.applied_through_lsn += 1;
                        assert_eq!(result.event_id.lsn.unwrap().value, oracle.applied_through_lsn);
                        assert_prefix_convergence(&storage, &hot, &oracle).await;
                    }
                    ReconciliationAction::RefeedCovered => {
                        let before_registry = hot.clone();
                        let before_events = storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap();
                        hot.observe(&before_events[0]).unwrap();
                        assert_eq!(hot, before_registry);
                        assert_eq!(
                            storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap(),
                            before_events,
                        );
                        assert_prefix_convergence(&storage, &hot, &oracle).await;
                    }
                    ReconciliationAction::MutateRetired(mutation) => {
                        let retired = ordered_identities(&oracle.retired).remove(0);
                        let candidate = ResourceReportMutation {
                            identity: Some(wire(&retired)),
                            mutation: Some(match mutation {
                                RetiredMutation::Upsert => resource_report_mutation::Mutation::Upsert(
                                    ResourceStateUpsert {
                                        resource_payload: Some(envelope("resource.schema")),
                                        projection_payload: Some(envelope("projection.schema")),
                                    },
                                ),
                                RetiredMutation::Unknown => resource_report_mutation::Mutation::Unknown(
                                    patchbay_contracts::patchbay::ResourceStateUnknown {},
                                ),
                                RetiredMutation::Tombstone => resource_report_mutation::Mutation::Tombstone(
                                    ResourceStateTombstone { replaced_by: None },
                                ),
                            }),
                        };
                        let before_registry = hot.clone();
                        let before_events = storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap();
                        assert!(ingest_resource_report(
                            &storage,
                            &mut hot,
                            report_at_generation(
                                &adapter,
                                &kind,
                                oracle.generation,
                                ResourceReportMode::Delta,
                                AdapterSnapshotSupport::Partial,
                                vec![candidate],
                            ),
                        ).await.is_err());
                        assert_eq!(hot, before_registry);
                        assert_eq!(
                            storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap(),
                            before_events,
                        );
                        assert_prefix_convergence(&storage, &hot, &oracle).await;
                    }
                    ReconciliationAction::LowerGenerationBeyondPrefix => {
                        let candidate = RecordedEvent {
                            event_id: event_id(domain(), oracle.applied_through_lsn + 1),
                            payload: resource_events::encode(&ResourceStateEvent {
                                authority_domain_id: Some(domain()),
                                source_adapter_id: Some(AdapterId { value: adapter.clone() }),
                                source_adapter_generation: Some(Generation {
                                    value: oracle.generation - 1,
                                }),
                                views: vec![ResourceViewStateUpdate {
                                    resource_kind: Some(ResourceKind { value: kind.clone() }),
                                    completeness: AdapterSnapshotSupport::Partial as i32,
                                }],
                                mutations: Vec::new(),
                                observed_at: Some(Timestamp { seconds: 200, nanos: 0 }),
                            }),
                        };
                        assert_eq!(candidate.payload.kind, StoredEventKind::ResourceState as i32);
                        let before_registry = hot.clone();
                        let before_events = storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap();
                        assert!(hot.observe(&candidate).is_err());
                        assert_eq!(hot, before_registry);
                        assert_eq!(
                            storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap(),
                            before_events,
                        );
                        assert_prefix_convergence(&storage, &hot, &oracle).await;
                    }
                }
            }
        });
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

fn report(
    adapter: &str,
    kind: &str,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    report_at_generation(adapter, kind, 1, mode, tier, mutations)
}

fn report_at_generation(
    adapter: &str,
    kind: &str,
    generation: u64,
    mode: ResourceReportMode,
    tier: AdapterSnapshotSupport,
    mutations: Vec<ResourceReportMutation>,
) -> ValidatedResourceReport {
    ValidatedResourceReport {
        authority_domain_id: domain(),
        adapter_id: AdapterId { value: adapter.into() },
        adapter_generation: Generation { value: generation },
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
