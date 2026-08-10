use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    resource_report_mutation, AdapterId, AdapterSnapshotSupport, AuditEventKind,
    AuthorityDomainId, EventId, Generation, IdempotencyKey, Lsn, PayloadContentType,
    PayloadEnvelope, ResourceFreshnessState, ResourceId, ResourceIdentity, ResourceKind,
    ResourceReportMutation, ResourceStateTombstone, ResourceStateUnknown, ResourceStateUpsert,
    ResourceViewReport, StoredEventKind, StoredEventPayload,
};
use patchbay_core::{
    resource::{
        adapter_stale_event, ingest_resource_report, rebuild_from_log, ResourceRegistry,
        ResourceReportMode, ValidatedResourceReport,
    },
    storage::{
        AuditRecordDraft, DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError,
        StoredSnapshot, TargetKey,
    },
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
async fn unknown_survives_omission_generation_disconnect_tombstone_and_replay() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Partial,
            vec![unknown("mystery")],
        ),
    )
    .await
    .unwrap();
    let identity = domain_identity("mystery");
    let initial_revision = registry.get(&identity).unwrap().revision_lsn;

    for tier in [AdapterSnapshotSupport::Partial, AdapterSnapshotSupport::None] {
        ingest_resource_report(
            &storage,
            &mut registry,
            report(1, ResourceReportMode::Snapshot, tier, Vec::new()),
        )
        .await
        .unwrap();
        let omitted = registry.get(&identity).unwrap();
        assert_eq!(omitted.freshness, ResourceFreshnessState::Unknown);
        assert_eq!(omitted.revision_lsn, initial_revision);
        assert!(omitted.resource_payload.is_none());
        assert!(omitted.projection_payload.is_none());
    }

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
    assert_eq!(
        registry.get(&identity).unwrap().freshness,
        ResourceFreshnessState::Unknown,
        "new adapter generation must not invent a stale cache"
    );
    assert!(adapter_stale_event(
        &registry,
        &domain(),
        &AdapterId { value: "adapter-a".into() },
        Generation { value: 2 },
        Timestamp { seconds: 200, nanos: 0 },
    )
    .unwrap()
    .is_none());

    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            2,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Authoritative,
            Vec::new(),
        ),
    )
    .await
    .unwrap();
    let retired = registry.get(&identity).unwrap();
    assert!(retired.tombstoned());
    assert_eq!(retired.freshness, ResourceFreshnessState::Unknown);
    assert!(retired.resource_payload.is_none());
    assert!(retired.projection_payload.is_none());

    let replayed = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert_eq!(replayed, registry);
    assert_eq!(replayed.get(&identity).unwrap().freshness, ResourceFreshnessState::Unknown);
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

struct InterleavingAuditStorage {
    inner: RusqliteStorage,
    injected: AtomicBool,
    omit_committed_suffix_once: AtomicBool,
    resource_append_attempts: AtomicUsize,
}

impl InterleavingAuditStorage {
    fn new() -> Self {
        Self::with_omitted_committed_suffix(false)
    }

    fn with_omitted_committed_suffix(omit_once: bool) -> Self {
        Self {
            inner: RusqliteStorage::open_in_memory().unwrap(),
            injected: AtomicBool::new(false),
            omit_committed_suffix_once: AtomicBool::new(omit_once),
            resource_append_attempts: AtomicUsize::new(0),
        }
    }
}

impl Storage for InterleavingAuditStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        if payload.kind == StoredEventKind::ResourceState as i32 {
            self.resource_append_attempts.fetch_add(1, Ordering::SeqCst);
            if !self.injected.swap(true, Ordering::SeqCst) {
                let mut audit = AuditRecordDraft::new(
                    Timestamp {
                        seconds: 99,
                        nanos: 0,
                    },
                    AuditEventKind::AdapterDiagnosticReported,
                );
                audit.reason_code = "interleaved_audit".into();
                self.inner
                    .append_audit(authority_domain_id, audit)
                    .await?;
            }
        }
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner.append_dedup(authority_domain_id, key, target, payload).await
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn read_through(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
        as_of_lsn: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        if self.omit_committed_suffix_once.swap(false, Ordering::SeqCst) {
            return Ok(Vec::new());
        }
        self.inner.read_through(authority_domain_id, cursor, as_of_lsn).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner.write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload).await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner.load_latest_snapshot(authority_domain_id, at_or_before).await
    }
}

#[tokio::test]
async fn report_ingest_folds_an_interleaved_ungated_audit_and_returns_committed_success() {
    let storage = InterleavingAuditStorage::new();
    let mut registry = ResourceRegistry::new();

    let result = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .expect("a committed report must not become Internal/retry-ambiguous because an audit interleaved");

    assert_eq!(result.event_id.lsn.as_ref().map(|lsn| lsn.value), Some(2));
    assert_eq!(
        storage.resource_append_attempts.load(Ordering::SeqCst),
        1,
        "success must not require a duplicate report retry",
    );
    let events = storage.read_after(&domain(), Lsn { value: 0 }).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].payload.kind, StoredEventKind::AuditRecord as i32);
    assert_eq!(events[1].event_id, result.event_id);
    assert_eq!(events[1].payload.kind, StoredEventKind::ResourceState as i32);
    assert_eq!(registry.get(&domain_identity("one")).unwrap().revision_lsn, 2);
    assert_eq!(registry, rebuild_from_log(&storage, &domain()).await.unwrap());
}

#[tokio::test]
async fn report_ingest_fails_closed_when_the_committed_suffix_is_missing() {
    let storage = InterleavingAuditStorage::with_omitted_committed_suffix(true);
    let mut registry = ResourceRegistry::new();

    let error = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("does not end with the exact committed report"));
    assert_eq!(storage.resource_append_attempts.load(Ordering::SeqCst), 1);
    let replayed = rebuild_from_log(&storage, &domain()).await.unwrap();
    assert_eq!(registry, replayed, "failure recovery must reinstall durable authority");
    assert_eq!(registry.get(&domain_identity("one")).unwrap().revision_lsn, 2);
}

#[tokio::test]
async fn report_ingest_catches_up_the_durable_prefix_before_normalizing() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    storage
        .append(
            &domain(),
            StoredEventPayload {
                kind: StoredEventKind::Observation as i32,
                payload: Vec::new(),
            },
        )
        .await
        .unwrap();

    let mut writer = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut writer,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();

    let mut stale = ResourceRegistry::new();
    let result = ingest_resource_report(
        &storage,
        &mut stale,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::Partial,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(result.event_id.lsn.unwrap().value, 3);
    assert_eq!(stale.get(&domain_identity("one")).unwrap().revision_lsn, 3);
    assert_eq!(stale, rebuild_from_log(&storage, &domain()).await.unwrap());
}

#[tokio::test]
async fn none_tier_live_delta_can_mutate_an_explicit_identity() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Delta,
            AdapterSnapshotSupport::None,
            vec![upsert("one")],
        ),
    )
    .await
    .unwrap();
    assert!(registry.contains(&domain_identity("one")));
}

#[tokio::test]
async fn authoritative_snapshot_unknown_rejects_before_append_or_projection() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = ResourceRegistry::new();
    let before = registry.clone();

    let error = ingest_resource_report(
        &storage,
        &mut registry,
        report(
            1,
            ResourceReportMode::Snapshot,
            AdapterSnapshotSupport::Authoritative,
            vec![unknown("unclassified")],
        ),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("authoritative snapshot cannot list"));
    assert_eq!(registry, before);
    assert!(storage
        .read_after(&domain(), patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .unwrap()
        .is_empty());
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

fn unknown(id: &str) -> ResourceReportMutation {
    ResourceReportMutation {
        identity: Some(wire_identity(id)),
        mutation: Some(resource_report_mutation::Mutation::Unknown(
            ResourceStateUnknown {},
        )),
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

