use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, typed_correlation,
    AcceptedOperation, ActorEndpointRef, ActorId,
    AdapterCapability, AdapterDiagnosticPayload, AdapterDiagnosticReport, AdapterDiagnosticSeverity, AdapterRegistration,
    AdapterSnapshotSupport, AdapterTargetCategory, AttachRequest, AuditEventKind, AuthorityDomainId,
    CommandId, CommandTransition, EndpointId, FailureCode, Generation, IdempotencyKey, Lsn,
    Observation, ObservationKind, Operation, OperationKind,
    PayloadContentType, PayloadEnvelope, ReceiveRequest, ResourceCapability,
    ResourceFreshnessState, ResourceId, ResourceIdentity, ResourceKind,
    ResourceProjectionContract, ResourceReport, ResourceReportMutation, ResourceSnapshotReport,
    ResourceStateUnknown, ResourceStateUpsert, ResourceViewReport, SchemaDescriptor,
    RuntimeSessionId, SecurityLockdownEntered, SessionActivityState,
    SessionConnectivityState, SessionReportSourceCursor, StoredEventKind, StoredEventPayload,
    TargetScope, TargetScopeKind,
    TypedCorrelation,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    resource::ResourceRegistry,
    security::events as security_events,
    session::SessionRegistry,
    storage::{
        AuditRecordDraft, AuditedBatchAppend, CoreGenerationStore, DedupOutcome, RecordedEvent,
        RusqliteStorage, Storage, StorageError, StoredSnapshot, TargetKey,
    },
    target::TargetRegistry,
};
use prost::Message;
use prost_types::Timestamp;
use tokio::sync::Notify;
use tokio_stream::StreamExt;
use tonic::Request;

use super::*;

const EVIDENCE: &str = "adapter-test-secret";

fn accepted_operation_bytes(operation: &Operation) -> Vec<u8> {
    AcceptedOperation {
        operation: Some(operation.clone()),
        authorizing_grant_id: Some(patchbay_contracts::patchbay::GrantId { value: "test-grant".to_owned() }),
    }
    .encode_to_vec()
}

#[derive(Clone)]
struct BlockingReadStorage {
    inner: RusqliteStorage,
    block_next_read: Arc<AtomicBool>,
    read_started: Arc<Notify>,
    release_read: Arc<Notify>,
}

impl BlockingReadStorage {
    fn new() -> Self {
        Self {
            inner: RusqliteStorage::open_in_memory().expect("storage opens"),
            block_next_read: Arc::new(AtomicBool::new(false)),
            read_started: Arc::new(Notify::new()),
            release_read: Arc::new(Notify::new()),
        }
    }

    fn block_next_read(&self) {
        self.block_next_read.store(true, Ordering::SeqCst);
    }

    async fn wait_for_blocked_read(&self) {
        self.read_started.notified().await;
    }

    fn release_blocked_read(&self) {
        self.release_read.notify_one();
    }
}

impl CoreGenerationStore for BlockingReadStorage {
    async fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> Result<Generation, StorageError> {
        self.inner
            .load_or_create_core_generation(authority_domain_id, candidate)
            .await
    }
}

impl Storage for BlockingReadStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        if self.block_next_read.swap(false, Ordering::SeqCst) {
            self.read_started.notify_one();
            self.release_read.notified().await;
        }
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner.append_audit(authority_domain_id, audit).await
    }
}

/// Commits the real batch, then corrupts only the returned event identity once
/// so the hot projection fold fails while durable replay remains valid.
#[derive(Clone)]
struct FailPostCommitRegistrationFoldStorage {
    inner: RusqliteStorage,
    fail_next_batch_result: Arc<AtomicBool>,
}

impl FailPostCommitRegistrationFoldStorage {
    fn new() -> Self {
        Self {
            inner: RusqliteStorage::open_in_memory().expect("storage opens"),
            fail_next_batch_result: Arc::new(AtomicBool::new(false)),
        }
    }

    fn fail_next_batch_fold(&self) {
        self.fail_next_batch_result.store(true, Ordering::SeqCst);
    }
}

impl CoreGenerationStore for FailPostCommitRegistrationFoldStorage {
    async fn load_or_create_core_generation(
        &self,
        authority_domain_id: &AuthorityDomainId,
        candidate: Generation,
    ) -> Result<Generation, StorageError> {
        self.inner
            .load_or_create_core_generation(authority_domain_id, candidate)
            .await
    }
}

impl Storage for FailPostCommitRegistrationFoldStorage {
    async fn append(
        &self,
        authority_domain_id: &AuthorityDomainId,
        payload: StoredEventPayload,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner.append(authority_domain_id, payload).await
    }

    async fn append_dedup(
        &self,
        authority_domain_id: &AuthorityDomainId,
        key: &IdempotencyKey,
        target: &TargetKey,
        payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        self.inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        self.inner
            .write_snapshot(authority_domain_id, snapshot_lsn, snapshot_payload)
            .await
    }

    async fn load_latest_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        self.inner
            .load_latest_snapshot(authority_domain_id, at_or_before)
            .await
    }

    async fn append_audit(
        &self,
        authority_domain_id: &AuthorityDomainId,
        audit: AuditRecordDraft,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner.append_audit(authority_domain_id, audit).await
    }

    async fn append_decision(
        &self,
        authority_domain_id: &AuthorityDomainId,
        source: StoredEventPayload,
        audit: AuditRecordDraft,
    ) -> Result<patchbay_contracts::patchbay::EventId, StorageError> {
        self.inner
            .append_decision(authority_domain_id, source, audit)
            .await
    }

    async fn append_batch_audited(
        &self,
        authority_domain_id: &AuthorityDomainId,
        sources: Vec<StoredEventPayload>,
        audit: AuditRecordDraft,
    ) -> Result<AuditedBatchAppend, StorageError> {
        let mut committed = self
            .inner
            .append_batch_audited(authority_domain_id, sources, audit)
            .await?;
        if self.fail_next_batch_result.swap(false, Ordering::SeqCst) {
            committed.source_event_ids[0].authority_domain_id = Some(AuthorityDomainId {
                value: "injected-wrong-domain".into(),
            });
        }
        Ok(committed)
    }
}

#[test]
fn resource_source_oracle_kills_channel_and_owner_mutants() {
    #[derive(Clone, Copy)]
    struct Attempt {
        authenticated: bool,
        current_token: bool,
        exact_owner: bool,
    }
    let oracle = |attempt: Attempt| attempt.authenticated && attempt.current_token && attempt.exact_owner;
    let trust_payload_source = |_attempt: Attempt| true;
    let skip_current_token = |attempt: Attempt| attempt.authenticated && attempt.exact_owner;
    let skip_owner = |attempt: Attempt| attempt.authenticated && attempt.current_token;

    let missing = Attempt { authenticated: false, current_token: false, exact_owner: true };
    let stale = Attempt { authenticated: true, current_token: false, exact_owner: true };
    let cross_owner = Attempt { authenticated: true, current_token: true, exact_owner: false };
    assert!(!oracle(missing));
    assert!(!oracle(stale));
    assert!(!oracle(cross_owner));
    assert!(trust_payload_source(missing), "payload-source mutant accepts unauthenticated evidence");
    assert!(skip_current_token(stale), "token-fence mutant accepts stale attachment");
    assert!(skip_owner(cross_owner), "owner-binding mutant accepts cross-adapter target");
}

#[tokio::test]
async fn resource_manifest_attach_accepts_two_kinds_and_rejects_reserved_okf_without_registration_append() {
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut valid = registration(domain.clone());
    valid.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![
            resource_declaration("provider_pool", AdapterSnapshotSupport::Authoritative),
            resource_declaration("usage_window", AdapterSnapshotSupport::Partial),
        ],
        ..AdapterCapability::default()
    });
    service
        .attach(Request::new(AttachRequest {
            registration: Some(valid),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("two-kind resource manifest attaches");
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .expect("events read")
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::Observation as i32)
            .count(),
        1
    );

    let rejected_storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let rejected_service = AdapterControlServiceImpl::new(
        rejected_storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut okf = registration(domain.clone());
    let mut declaration = resource_declaration("knowledge_bundle", AdapterSnapshotSupport::Authoritative);
    declaration.projection_contract.as_mut().expect("projection").target_category =
        AdapterTargetCategory::KnowledgeBundle as i32;
    declaration
        .projection_contract
        .as_mut()
        .expect("projection")
        .payload_schema
        .as_mut()
        .expect("schema")
        .schema_ref = "okf.v0.2.bundle".into();
    okf.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::KnowledgeBundle as i32],
        resource_capabilities: vec![declaration],
        ..AdapterCapability::default()
    });
    let error = rejected_service
        .attach(Request::new(AttachRequest {
            registration: Some(okf),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect_err("reserved knowledge bundle must reject");
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    let rejected_events = rejected_storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events read");
    assert!(!rejected_events.is_empty(), "rejection remains audited");
    assert!(rejected_events.iter().all(|event| {
        if event.payload.kind != StoredEventKind::Observation as i32 {
            return true;
        }
        Observation::decode(event.payload.payload.as_slice())
            .ok()
            .and_then(|observation| observation.payload)
            .is_none_or(|payload| payload.schema_ref != "patchbay.AdapterRegistration")
    }));
}

#[tokio::test]
async fn authenticated_resource_report_uses_manifest_admission_and_durable_projection() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut registration = registration(domain.clone());
    registration.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Partial,
        )],
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("resource adapter attaches");
    let token = attachment_token(&attached);
    let identity = ResourceIdentity {
        adapter_id: Some(adapter_id()),
        resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
        resource_id: Some(ResourceId { value: "pool-1".into() }),
    };
    let report = ResourceReport {
        adapter_id: Some(adapter_id()),
        adapter_generation: Some(Generation { value: 1 }),
        report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
            views: vec![ResourceViewReport {
                resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                completeness: AdapterSnapshotSupport::Partial as i32,
                mutations: vec![ResourceReportMutation {
                    identity: Some(identity.clone()),
                    mutation: Some(resource_report_mutation::Mutation::Upsert(
                        ResourceStateUpsert {
                            resource_payload: Some(PayloadEnvelope {
                                payload: vec![1],
                                content_type: PayloadContentType::Protobuf as i32,
                                schema_ref: "provider_pool.payload.v1".into(),
                            }),
                            projection_payload: Some(PayloadEnvelope {
                                payload: br#"{\"state\":\"ok\"}"#.to_vec(),
                                content_type: PayloadContentType::Json as i32,
                                schema_ref: "provider_pool.projection.v1".into(),
                            }),
                        },
                    )),
                }],
            }],
        })),
        observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
    };
    let mut overclaimed = report.clone();
    let Some(resource_report::Report::Snapshot(snapshot)) = overclaimed.report.as_mut() else {
        unreachable!()
    };
    snapshot.views[0].completeness = AdapterSnapshotSupport::Authoritative as i32;
    assert_eq!(
        service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::ResourceReport(
                        overclaimed,
                    )),
                },
                &token,
            ))
            .await
            .expect_err("overclaimed tier rejects")
            .code(),
        tonic::Code::InvalidArgument
    );
    let mut mismatched = report.clone();
    let Some(resource_report::Report::Snapshot(snapshot)) = mismatched.report.as_mut() else {
        unreachable!()
    };
    let Some(resource_report_mutation::Mutation::Upsert(upsert)) =
        snapshot.views[0].mutations[0].mutation.as_mut()
    else {
        unreachable!()
    };
    upsert
        .projection_payload
        .as_mut()
        .expect("projection")
        .schema_ref = "foreign.schema".into();
    assert_eq!(
        service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::ResourceReport(
                        mismatched,
                    )),
                },
                &token,
            ))
            .await
            .expect_err("schema mismatch rejects")
            .code(),
        tonic::Code::InvalidArgument
    );
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(report)),
            },
            &token,
        ))
        .await
        .expect("admitted resource report succeeds");
    let rebuilt = resource::rebuild_from_log(&storage, &domain)
        .await
        .expect("resource projection replays");
    let projected = patchbay_core::resource::ResourceIdentity::try_from_wire(&identity).unwrap();
    assert!(rebuilt.contains(&projected));
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .unwrap()
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::ResourceState as i32)
            .count(),
        1
    );

    let stream = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &token,
        ))
        .await
        .expect("resource adapter stream opens")
        .into_inner();
    drop(stream);
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let replayed = resource::rebuild_from_log(&storage, &domain).await.unwrap();
            if replayed
                .get(&projected)
                .is_some_and(|record| {
                    record.freshness
                        == patchbay_contracts::patchbay::ResourceFreshnessState::Stale
                })
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("disconnect durably stales the resource");
}

#[tokio::test]
async fn authenticated_resource_status_records_one_observation_and_fences_invalid_targets() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut resource_registration = registration(domain.clone());
    resource_registration.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Partial,
        )],
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(resource_registration),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("resource adapter attaches");
    let token = attachment_token(&attached);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(
                    resource_snapshot_report(
                        1,
                        &[("provider_pool", AdapterSnapshotSupport::Partial)],
                    ),
                )),
            },
            &token,
        ))
        .await
        .expect("resource is admitted before status ingestion");

    let status = Observation {
        authority_domain_id: Some(domain.clone()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId { value: adapter_id().value.clone() }),
            ..ActorEndpointRef::default()
        }),
        kind: ObservationKind::Status as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(resource_identity("provider_pool", "resource-1")),
            ..TargetScope::default()
        }),
        failure_code: FailureCode::Unspecified as i32,
        ..Observation::default()
    };
    let before = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events read")
        .len();
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::Event(status.clone())),
            },
            &token,
        ))
        .await
        .expect("authenticated resource status succeeds");
    let after_status = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events read");
    let appended = &after_status[before..];
    assert_eq!(appended.len(), 1, "status appends exactly one durable event");
    assert_eq!(appended[0].payload.kind, StoredEventKind::Observation as i32);
    assert!(appended
        .iter()
        .all(|event| event.payload.kind != StoredEventKind::CommandTransition as i32));

    let mut cross_adapter = status.clone();
    cross_adapter
        .target_scope
        .as_mut()
        .and_then(|target| target.resource.as_mut())
        .and_then(|resource| resource.adapter_id.as_mut())
        .expect("resource adapter id")
        .value = "other-adapter".into();
    assert_eq!(
        service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::Event(cross_adapter)),
                },
                &token,
            ))
            .await
            .expect_err("cross-adapter status rejects")
            .code(),
        tonic::Code::PermissionDenied
    );

    let mut mixed_target = status;
    mixed_target
        .target_scope
        .as_mut()
        .expect("target scope")
        .runtime_session_id = Some(RuntimeSessionId { value: "session-1".into() });
    assert_eq!(
        service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::Event(mixed_target)),
                },
                &token,
            ))
            .await
            .expect_err("mixed resource/session target rejects")
            .code(),
        tonic::Code::PermissionDenied
    );
    assert_eq!(
        storage
            .read_after(&domain, Lsn { value: 0 })
            .await
            .expect("events read")
            .len(),
        after_status.len(),
        "target fencing rejects before durable append"
    );
}

#[tokio::test]
async fn generic_registration_schema_ingress_cannot_register_an_embedded_adapter() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let adapter_b = AdapterId {
        value: "embedded-adapter-b".into(),
    };
    let mut embedded_b = registration(domain.clone());
    embedded_b.adapter_id = Some(adapter_b.clone());
    embedded_b.endpoint_id = Some(EndpointId {
        value: "embedded-adapter-b-endpoint".into(),
    });
    embedded_b.adapter_generation = Some(Generation { value: 9 });

    let before = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events read");
    assert_eq!(
        before
            .iter()
            .filter(|event| {
                Observation::decode(event.payload.payload.as_slice())
                    .ok()
                    .and_then(|observation| observation.payload)
                    .is_some_and(|payload| {
                        payload.schema_ref == adapter::ADAPTER_REGISTRATION_SCHEMA
                    })
            })
            .count(),
        1,
        "Attach is the only existing registration producer"
    );

    let error = service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::Event(Observation {
                    authority_domain_id: Some(domain.clone()),
                    sender: Some(ActorEndpointRef {
                        actor_id: Some(ActorId {
                            value: adapter_id().value,
                        }),
                        endpoint_id: Some(EndpointId {
                            value: "pi-adapter-endpoint".into(),
                        }),
                        ..ActorEndpointRef::default()
                    }),
                    kind: ObservationKind::Event as i32,
                    target_scope: Some(TargetScope {
                        kind: TargetScopeKind::Adapter as i32,
                        adapter_id: Some(adapter_id()),
                        ..TargetScope::default()
                    }),
                    payload: Some(PayloadEnvelope {
                        payload: embedded_b.encode_to_vec(),
                        content_type: PayloadContentType::Protobuf as i32,
                        schema_ref: adapter::ADAPTER_REGISTRATION_SCHEMA.to_owned(),
                    }),
                    ..Observation::default()
                })),
            },
            &attachment_token,
        ))
        .await
        .expect_err("generic Event ingress cannot produce adapter registration");
    assert_eq!(error.code(), tonic::Code::InvalidArgument);

    let after = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events read after rejected ingress");
    assert_eq!(
        after, before,
        "rejected generic ingress appends no registration Observation or sibling event"
    );

    let replayed = adapter::rebuild_from_log(&storage, &domain)
        .await
        .expect("canonical attachment prefix replays");
    assert_eq!(
        replayed
            .get(&adapter_id())
            .and_then(|record| record.registration.adapter_generation.as_ref())
            .map(|generation| generation.value),
        Some(1),
        "authenticated adapter A generation remains unchanged"
    );
    assert!(
        replayed.get(&adapter_b).is_none(),
        "embedded adapter B has no registration generation"
    );

    let targets = TargetRegistry::with_adapters(
        SessionRegistry::new(domain.clone()).unwrap(),
        ResourceRegistry::new(),
        replayed,
    );
    let adapter_a_scope = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter_id()),
        ..TargetScope::default()
    };
    assert_eq!(
        TargetResolver::resolve(&targets, &domain, OperationKind::Spawn, &adapter_a_scope).await,
        Ok(TargetBinding::Adapter {
            adapter_id: adapter_id(),
        })
    );
    let adapter_b_scope = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter_b),
        ..TargetScope::default()
    };
    assert!(
        TargetResolver::resolve(&targets, &domain, OperationKind::Spawn, &adapter_b_scope)
            .await
            .is_err(),
        "embedded adapter B is not spawn-resolvable"
    );
}

#[tokio::test]
async fn same_generation_manifest_redeclaration_atomically_degrades_affected_resources() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let initial_kinds = ["removed_pool", "down_tiered_pool", "schema_changed_pool"];
    let mut initial = registration(domain.clone());
    initial.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: initial_kinds
            .iter()
            .map(|kind| resource_declaration(kind, AdapterSnapshotSupport::Authoritative))
            .collect(),
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(initial),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("initial resource adapter attaches");
    let initial_token = attachment_token(&attached);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(
                    resource_snapshot_report(
                        1,
                        &initial_kinds
                            .iter()
                            .map(|kind| (*kind, AdapterSnapshotSupport::Authoritative))
                            .collect::<Vec<_>>(),
                    ),
                )),
            },
            &initial_token,
        ))
        .await
        .expect("initial authoritative resource report succeeds");

    let mut schema_changed = resource_declaration(
        "schema_changed_pool",
        AdapterSnapshotSupport::Authoritative,
    );
    schema_changed
        .projection_contract
        .as_mut()
        .expect("projection contract")
        .projection_schema
        .as_mut()
        .expect("projection schema")
        .schema_ref = "schema_changed_pool.projection.v2".into();
    let mut redeclared = registration(domain.clone());
    redeclared.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![
            resource_declaration("down_tiered_pool", AdapterSnapshotSupport::Partial),
            schema_changed,
        ],
        ..AdapterCapability::default()
    });
    let replacement = service
        .attach(Request::new(AttachRequest {
            registration: Some(redeclared),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("same-generation capability redeclaration succeeds atomically");
    let attach_lsn = replacement
        .get_ref()
        .attach_event_id
        .as_ref()
        .and_then(|event| event.lsn.as_ref())
        .expect("replacement registration LSN")
        .value;

    let replayed = resource::rebuild_from_log(&storage, &domain)
        .await
        .expect("resource degradation replays");
    for kind in initial_kinds {
        let identity = patchbay_core::resource::ResourceIdentity::try_from_wire(
            &resource_identity(kind, "resource-1"),
        )
        .unwrap();
        assert_eq!(
            replayed.get(&identity).unwrap().freshness,
            ResourceFreshnessState::Stale,
            "{kind} cached state must degrade before the replacement attachment is returned"
        );
    }
    let view_tier = |kind: &str| {
        replayed
            .views()
            .find(|view| view.key.resource_kind.value == kind)
            .expect("affected view")
            .completeness
    };
    assert_eq!(view_tier("removed_pool"), AdapterSnapshotSupport::None);
    assert_eq!(
        view_tier("down_tiered_pool"),
        AdapterSnapshotSupport::Partial
    );
    assert_eq!(
        view_tier("schema_changed_pool"),
        AdapterSnapshotSupport::None
    );
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap();
    let degradation_lsn = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::ResourceState as i32)
        .filter_map(|event| event.event_id.lsn.as_ref())
        .map(|lsn| lsn.value)
        .max()
        .expect("degradation resource event");
    assert_eq!(degradation_lsn + 1, attach_lsn);
    assert_eq!(
        service
            .receive_deliveries(authenticated_with_attachment_token(
                ReceiveRequest {
                    adapter_id: Some(adapter_id()),
                    cursor: Some(Lsn { value: 0 }),
                },
                &initial_token,
            ))
            .await
            .err()
            .expect("old token is fenced after atomic redeclaration")
            .code(),
        tonic::Code::Unauthenticated
    );
}

#[tokio::test]
async fn committed_registration_with_failed_projection_fences_prior_attachment() {
    let storage = FailPostCommitRegistrationFoldStorage::new();
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut initial = registration(domain.clone());
    initial.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Authoritative,
        )],
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(initial),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("initial attachment succeeds");
    let prior_token = attachment_token(&attached);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(
                    resource_snapshot_report(
                        1,
                        &[("provider_pool", AdapterSnapshotSupport::Authoritative)],
                    ),
                )),
            },
            &prior_token,
        ))
        .await
        .expect("initial report installs a resource view");
    let mut prior_stream = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &prior_token,
        ))
        .await
        .expect("prior delivery stream opens")
        .into_inner();

    storage.fail_next_batch_fold();
    let mut replacement = registration(domain.clone());
    replacement.adapter_generation = Some(Generation { value: 2 });
    replacement.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Authoritative,
        )],
        ..AdapterCapability::default()
    });
    service
        .attach(Request::new(AttachRequest {
            registration: Some(replacement),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect_err("post-commit projection failure returns an attach error");

    let durable = adapter::rebuild_from_log(&storage, &domain)
        .await
        .expect("committed replacement registration replays");
    assert_eq!(
        durable
            .get(&adapter_id())
            .and_then(|record| record.registration.adapter_generation.as_ref())
            .map(|generation| generation.value),
        Some(2),
        "replacement commit is the point of no return"
    );
    let stale_token_error = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &prior_token,
        ))
        .await
        .err()
        .expect("prior token must be unusable after the replacement commits");
    assert_eq!(stale_token_error.code(), tonic::Code::Unauthenticated);
    assert!(
        tokio::time::timeout(Duration::from_secs(1), prior_stream.next())
            .await
            .expect("prior stream epoch is fenced promptly")
            .is_none(),
        "delivery stream authenticated under the prior epoch must close"
    );
}

#[tokio::test]
async fn newer_generation_attachment_degrades_cached_resources_without_a_report() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut initial = registration(domain.clone());
    initial.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Authoritative,
        )],
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(initial),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("initial attachment succeeds");
    let token = attachment_token(&attached);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(
                    resource_snapshot_report(
                        1,
                        &[("provider_pool", AdapterSnapshotSupport::Authoritative)],
                    ),
                )),
            },
            &token,
        ))
        .await
        .expect("initial report succeeds");

    let mut replacement = registration(domain.clone());
    replacement.adapter_generation = Some(Generation { value: 2 });
    replacement.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Authoritative,
        )],
        ..AdapterCapability::default()
    });
    service
        .attach(Request::new(AttachRequest {
            registration: Some(replacement),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("newer attachment succeeds without a resource report");

    let replayed = resource::rebuild_from_log(&storage, &domain).await.unwrap();
    let identity = patchbay_core::resource::ResourceIdentity::try_from_wire(
        &resource_identity("provider_pool", "resource-1"),
    )
    .unwrap();
    let record = replayed.get(&identity).unwrap();
    assert_eq!(record.freshness, ResourceFreshnessState::Stale);
    assert_eq!(record.source_adapter_generation, Generation { value: 2 });
    let view = replayed.views().next().expect("resource view");
    assert_eq!(view.completeness, AdapterSnapshotSupport::None);
    assert_eq!(view.source_adapter_generation, Generation { value: 2 });
}

#[tokio::test]
async fn authoritative_snapshot_unknown_rejects_before_resource_append() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let mut registration = registration(domain.clone());
    registration.capability = Some(AdapterCapability {
        target_categories: vec![AdapterTargetCategory::OperationalResource as i32],
        resource_capabilities: vec![resource_declaration(
            "provider_pool",
            AdapterSnapshotSupport::Authoritative,
        )],
        ..AdapterCapability::default()
    });
    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("resource adapter attaches");
    let token = attachment_token(&attached);
    let events_before = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap()
        .len();
    let report = ResourceReport {
        adapter_id: Some(adapter_id()),
        adapter_generation: Some(Generation { value: 1 }),
        report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
            views: vec![ResourceViewReport {
                resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                completeness: AdapterSnapshotSupport::Authoritative as i32,
                mutations: vec![ResourceReportMutation {
                    identity: Some(ResourceIdentity {
                        adapter_id: Some(adapter_id()),
                        resource_kind: Some(ResourceKind { value: "provider_pool".into() }),
                        resource_id: Some(ResourceId { value: "unknown-pool".into() }),
                    }),
                    mutation: Some(resource_report_mutation::Mutation::Unknown(
                        ResourceStateUnknown {},
                    )),
                }],
            }],
        })),
        observed_at: Some(Timestamp { seconds: 100, nanos: 0 }),
    };

    let error = service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::ResourceReport(report)),
            },
            &token,
        ))
        .await
        .expect_err("authoritative unknown must reject");
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        storage.read_after(&domain, Lsn { value: 0 }).await.unwrap().len(),
        events_before,
        "rejection must happen before durable append"
    );
    assert_eq!(service.resources.lock().await.resources().count(), 0);
}

fn resource_snapshot_report(
    generation: u64,
    views: &[(&str, AdapterSnapshotSupport)],
) -> ResourceReport {
    ResourceReport {
        adapter_id: Some(adapter_id()),
        adapter_generation: Some(Generation { value: generation }),
        report: Some(resource_report::Report::Snapshot(ResourceSnapshotReport {
            views: views
                .iter()
                .map(|(kind, tier)| ResourceViewReport {
                    resource_kind: Some(ResourceKind {
                        value: (*kind).into(),
                    }),
                    completeness: *tier as i32,
                    mutations: vec![ResourceReportMutation {
                        identity: Some(resource_identity(kind, "resource-1")),
                        mutation: Some(resource_report_mutation::Mutation::Upsert(
                            ResourceStateUpsert {
                                resource_payload: Some(PayloadEnvelope {
                                    payload: vec![1],
                                    content_type: PayloadContentType::Protobuf as i32,
                                    schema_ref: format!("{kind}.payload.v1"),
                                }),
                                projection_payload: Some(PayloadEnvelope {
                                    payload: vec![2],
                                    content_type: PayloadContentType::Json as i32,
                                    schema_ref: format!("{kind}.projection.v1"),
                                }),
                            },
                        )),
                    }],
                })
                .collect(),
        })),
        observed_at: Some(Timestamp {
            seconds: 100 + generation as i64,
            nanos: 0,
        }),
    }
}

fn resource_identity(kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity {
        adapter_id: Some(adapter_id()),
        resource_kind: Some(ResourceKind { value: kind.into() }),
        resource_id: Some(ResourceId { value: id.into() }),
    }
}

fn resource_declaration(kind: &str, tier: AdapterSnapshotSupport) -> ResourceCapability {
    ResourceCapability {
        resource_kind: Some(ResourceKind { value: kind.into() }),
        snapshot_support: tier as i32,
        projection_contract: Some(ResourceProjectionContract {
            target_category: AdapterTargetCategory::OperationalResource as i32,
            payload_schema: Some(SchemaDescriptor {
                schema_ref: format!("{kind}.payload.v1"),
                content_type: PayloadContentType::Protobuf as i32,
            }),
            projection_schema: Some(SchemaDescriptor {
                schema_ref: format!("{kind}.projection.v1"),
                content_type: PayloadContentType::Json as i32,
            }),
        }),
    }
}

#[tokio::test]
async fn authenticated_diagnostic_report_appends_source_and_audit_atomically() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;
    let response = service
        .report_diagnostics(authenticated_with_attachment_token(
            AdapterDiagnosticReport {
                authority_domain_id: Some(domain.clone()),
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::Adapter as i32,
                    adapter_id: Some(adapter_id()),
                    ..Default::default()
                }),
                observed_at: Some(prost_types::Timestamp { seconds: 2, nanos: 0 }),
                payload: Some(PayloadEnvelope {
                    payload: AdapterDiagnosticPayload {
                        code: "pi_adapter_started".into(),
                        severity: AdapterDiagnosticSeverity::Info as i32,
                        adapter_generation: Some(Generation { value: 1 }),
                        count: 1,
                        ..Default::default()
                    }
                    .encode_to_vec(),
                    content_type: PayloadContentType::Protobuf as i32,
                    schema_ref: "patchbay.AdapterDiagnosticPayload".into(),
                }),
                ..Default::default()
            },
            &attachment_token,
        ))
        .await
        .expect("report succeeds")
        .into_inner();
    assert!(response.accepted);
    let events = storage.read_after(&domain, Lsn { value: 0 }).await.expect("events read");
    assert_eq!(events.iter().filter(|event| event.payload.kind == StoredEventKind::Observation as i32).count(), 2, "registration plus diagnostic source");
    let diagnostic_source = response.observation_event_id.expect("source id");
    let audit_id = response.audit_event_id.expect("audit id");
    let audit = events.iter().find(|event| event.event_id == audit_id).expect("audit event");
    let audit = patchbay_contracts::patchbay::AuditRecord::decode(audit.payload.payload.as_slice()).expect("audit decodes");
    assert_eq!(audit.kind, AuditEventKind::AdapterDiagnosticReported as i32);
    assert_eq!(audit.source_event_id, Some(diagnostic_source));
    assert_eq!(audit.reason_code, "pi_adapter_started");
    assert!(audit.adapter_diagnostic.is_some());
}

#[tokio::test]
async fn command_projection_catch_up_is_atomic_on_late_fold_failure() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let operation = |command: &str| Operation {
        command_id: Some(CommandId { value: command.into() }),
        authority_domain_id: Some(domain.clone()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            ..TargetScope::default()
        }),
        idempotency_key: format!("key-{command}"),
        ..Operation::default()
    };
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation("command-1")),
            },
        )
        .await
        .expect("initial command appends");
    let mut projection = rebuild_command_projection(&storage, &domain)
        .await
        .expect("initial command projection rebuilds");
    let before = projection.clone();

    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation("command-2")),
            },
        )
        .await
        .expect("valid leading tail event appends");
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::CommandTransition as i32,
                payload: CommandTransition {
                    command_id: Some(CommandId { value: "missing-command".into() }),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Delivered as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    ..CommandTransition::default()
                }
                .encode_to_vec(),
            },
        )
        .await
        .expect("faulty durable transition appends");

    catch_up_command_projection(&storage, &domain, &mut projection)
        .await
        .expect_err("a later invalid fold must reject the complete tail");
    assert_eq!(projection, before);
}

#[test]
fn resource_delivery_routes_only_to_the_nested_owning_adapter() {
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let operation = Operation {
        command_id: Some(CommandId { value: "resource-command".into() }),
        authority_domain_id: Some(domain.clone()),
        kind: OperationKind::Query as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(ResourceIdentity {
                adapter_id: Some(AdapterId { value: "adapter-a".into() }),
                resource_id: Some(ResourceId { value: "shared".into() }),
                resource_kind: Some(ResourceKind { value: "pool".into() }),
            }),
            ..TargetScope::default()
        }),
        idempotency_key: "resource-command-key".into(),
        ..Operation::default()
    };
    let event = RecordedEvent {
        event_id: patchbay_contracts::patchbay::EventId {
            authority_domain_id: Some(domain),
            lsn: Some(Lsn { value: 1 }),
        },
        payload: StoredEventPayload {
            kind: StoredEventKind::Operation as i32,
            payload: accepted_operation_bytes(&operation),
        },
    };
    let mut commands = CommandIndex::new();
    commands.apply(&event).expect("accepted operation projects");

    let adapter_a = AdapterId { value: "adapter-a".into() };
    let adapter_b = AdapterId { value: "adapter-b".into() };
    assert_eq!(deliveries_for_events(std::slice::from_ref(&event), &commands, &adapter_a, 0).len(), 1);
    assert!(deliveries_for_events(std::slice::from_ref(&event), &commands, &adapter_b, 0).is_empty());

    let mut malformed = operation;
    malformed.target_scope.as_mut().unwrap().resource.as_mut().unwrap().resource_kind = None;
    let malformed_event = RecordedEvent {
        payload: StoredEventPayload {
            kind: StoredEventKind::Operation as i32,
            payload: accepted_operation_bytes(&malformed),
        },
        ..event
    };
    assert!(deliveries_for_events(&[malformed_event], &commands, &adapter_a, 0).is_empty());
}

#[tokio::test]
async fn authenticated_resource_result_cannot_cross_kind_or_id_within_one_adapter() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;
    let operation = Operation {
        command_id: Some(CommandId { value: "resource-observation-command".into() }),
        authority_domain_id: Some(domain.clone()),
        kind: OperationKind::Query as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(ResourceIdentity {
                adapter_id: Some(adapter_id()),
                resource_id: Some(ResourceId { value: "expected".into() }),
                resource_kind: Some(ResourceKind { value: "pool".into() }),
            }),
            ..TargetScope::default()
        }),
        idempotency_key: "resource-observation-key".into(),
        ..Operation::default()
    };
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation),
            },
        )
        .await
        .expect("operation appends");
    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("exact delivery acknowledgement succeeds");

    for (kind, id) in [("window", "expected"), ("pool", "other")] {
        let before = storage.read_after(&domain, Lsn { value: 0 }).await.unwrap().len();
        let mismatched = ObservationRequest {
            authority_domain_id: Some(domain.clone()),
            observation: Some(observation_request::Observation::Event(Observation {
                authority_domain_id: Some(domain.clone()),
                kind: ObservationKind::Result as i32,
                target_scope: Some(TargetScope {
                    kind: TargetScopeKind::Resource as i32,
                    resource: Some(ResourceIdentity {
                        adapter_id: Some(adapter_id()),
                        resource_id: Some(ResourceId { value: id.into() }),
                        resource_kind: Some(ResourceKind { value: kind.into() }),
                    }),
                    ..TargetScope::default()
                }),
                correlations: vec![TypedCorrelation {
                    r#ref: Some(typed_correlation::Ref::CommandId(
                        operation.command_id.clone().unwrap(),
                    )),
                }],
                ..Observation::default()
            })),
        };
        let error = service
            .ingest_observation(authenticated_with_attachment_token(
                mismatched,
                &attachment_token,
            ))
            .await
            .expect_err("same-adapter tuple mismatch must reject");
        assert!(error.message().contains("target does not match"));
        assert_eq!(storage.read_after(&domain, Lsn { value: 0 }).await.unwrap().len(), before);
    }
}

#[tokio::test]
async fn adapter_attaches_reports_session_and_receives_targeted_operation() {
    let directory = tempfile::tempdir().expect("temp directory");
    let database = directory.path().join("core.sqlite3");
    let storage =
        RusqliteStorage::open(database.to_str().expect("utf8 path")).expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");

    let attached = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration(domain.clone())),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach succeeds");
    let attachment_token = attachment_token(&attached);
    let attached = attached.into_inner();
    assert!(attached.accepted);
    assert!(attached.attach_event_id.is_some());

    let report = session_report(SessionConnectivityState::Live);
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(report)),
            },
            &attachment_token,
        ))
        .await
        .expect("session report succeeds");

    let operation = Operation {
        command_id: Some(CommandId {
            value: "command-1".into(),
        }),
        authority_domain_id: Some(domain.clone()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id()),
            deployment_scope: "machine-a".into(),
            runtime_session_id: Some(RuntimeSessionId {
                value: "session-1".into(),
            }),
            session_generation: Some(Generation { value: 1 }),
            ..Default::default()
        }),
        idempotency_key: "command-1-key".into(),
        ..Default::default()
    };
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation),
            },
        )
        .await
        .expect("operation appends");

    let mut deliveries = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &attachment_token,
        ))
        .await
        .expect("delivery stream opens")
        .into_inner();
    let delivery = deliveries
        .next()
        .await
        .expect("one delivery")
        .expect("valid delivery");
    assert_eq!(
        delivery.operation.expect("operation").kind,
        OperationKind::Instruct as i32
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(20), deliveries.next())
            .await
            .is_err(),
        "an idle delivery subscription remains pending"
    );
}

#[tokio::test]
async fn lockdown_entry_then_live_report_catches_up_adapter_projection_before_derivation() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(
                    session_report(SessionConnectivityState::Live),
                )),
            },
            &attachment_token,
        ))
        .await
        .expect("initial live report succeeds");

    // Simulate a lockdown committed by the control service after this adapter
    // service's independent projection was last used.
    let lockdown = security_events::entered(
        domain.clone(),
        SecurityLockdownEntered {
            reason_code: "test_lockdown".into(),
            occurred_at: Some(prost_types::Timestamp { seconds: 2, nanos: 0 }),
            entered_by: Some(ActorEndpointRef {
                actor_id: Some(ActorId { value: "operator".into() }),
                ..Default::default()
            }),
            invalidated_through_operator_session_generation: Some(Generation { value: 1 }),
            affected_runtime_session_count: 1,
        },
    );
    storage
        .append(&domain, security_events::encode(&lockdown))
        .await
        .expect("lockdown source event appends");

    // The report is still live adapter evidence, but the catch-up fold must
    // make the stale clamp visible before ingest derives its transition.
    let mut post_lockdown_report = session_report(SessionConnectivityState::Live);
    post_lockdown_report.source_cursor.as_mut().unwrap().revision = 2;
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(
                    post_lockdown_report,
                )),
            },
            &attachment_token,
        ))
        .await
        .expect("live report is reconciled as stale during lockdown");

    let replayed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("entry then live report remains replayable");
    let current = replayed
        .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
        .expect("session remains present");
    assert_eq!(current.state.connectivity(), SessionConnectivityState::Stale);
}

#[tokio::test]
async fn concurrent_increasing_model_reports_leave_a_replayable_log() {
    let storage = BlockingReadStorage::new();
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;

    let mut initial = session_report(SessionConnectivityState::Live);
    initial.model = "provider/model-a".into();
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(initial)),
            },
            &attachment_token,
        ))
        .await
        .expect("initial session report succeeds");

    storage.block_next_read();
    let first_service = service.clone();
    let first_domain = domain.clone();
    let first_token = attachment_token.clone();
    let first = tokio::spawn(async move {
        let mut report = session_report(SessionConnectivityState::Live);
        report.source_cursor.as_mut().unwrap().revision = 2;
        report.model = "provider/model-b".into();
        first_service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(first_domain),
                    observation: Some(observation_request::Observation::SessionReport(report)),
                },
                &first_token,
            ))
            .await
    });
    storage.wait_for_blocked_read().await;

    let second_service = service.clone();
    let second_domain = domain.clone();
    let second_token = attachment_token.clone();
    let second = tokio::spawn(async move {
        let mut report = session_report(SessionConnectivityState::Live);
        report.source_cursor.as_mut().unwrap().revision = 3;
        report.model = "provider/model-c".into();
        second_service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(second_domain),
                    observation: Some(observation_request::Observation::SessionReport(report)),
                },
                &second_token,
            ))
            .await
    });

    tokio::task::yield_now().await;
    storage.release_blocked_read();
    first
        .await
        .expect("first report task joins")
        .expect("first model report succeeds");
    second
        .await
        .expect("second report task joins")
        .expect("second model report succeeds");

    let replayed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("concurrent model-report log remains replayable");
    assert_eq!(
        replayed
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains live")
            .model,
        "provider/model-c"
    );
}

#[tokio::test]
async fn authenticated_session_ingress_fences_delayed_and_old_generation_cursors_with_audit() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain.clone(), 1).await;

    for (revision, model) in [(1, "provider/model-a"), (3, "provider/model-b")] {
        let mut report = session_report(SessionConnectivityState::Live);
        report.source_cursor.as_mut().unwrap().revision = revision;
        report.model = model.into();
        service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::SessionReport(report)),
                },
                &attachment_token,
            ))
            .await
            .expect("increasing source cursor succeeds");
    }

    let session_events_before = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::SessionState as i32)
        .count();
    assert_eq!(session_events_before, 2);

    let mut delayed = session_report(SessionConnectivityState::Live);
    delayed.source_cursor.as_mut().unwrap().revision = 2;
    delayed.model = "provider/rollback".into();
    let mut old_generation = session_report(SessionConnectivityState::Live);
    old_generation
        .source_cursor
        .as_mut()
        .unwrap()
        .adapter_generation = Some(Generation { value: 0 });
    for stale in [delayed, old_generation] {
        let status = service
            .ingest_observation(authenticated_with_attachment_token(
                ObservationRequest {
                    authority_domain_id: Some(domain.clone()),
                    observation: Some(observation_request::Observation::SessionReport(stale)),
                },
                &attachment_token,
            ))
            .await
            .expect_err("stale source cursor must reject");
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }

    let events = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::SessionState as i32)
            .count(),
        session_events_before,
        "stale reports cannot append session-state mutations"
    );
    let stale_audits = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::AuditRecord as i32)
        .filter_map(|event| {
            patchbay_contracts::patchbay::AuditRecord::decode(event.payload.payload.as_slice()).ok()
        })
        .filter(|audit| audit.reason_code == "session_report_source_cursor_stale")
        .collect::<Vec<_>>();
    assert_eq!(stale_audits.len(), 2);
    assert!(stale_audits.iter().all(|audit| {
        audit.kind == AuditEventKind::StaleEventIgnored as i32
            && audit.failure_code == FailureCode::StaleEvent as i32
    }));

    let replayed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("stale attempts leave replay valid");
    let live = replayed
        .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
        .unwrap();
    assert_eq!(live.model, "provider/model-b");
    assert_eq!(live.last_source_cursor.as_ref().unwrap().revision, 3);
}

#[tokio::test]
async fn adapter_attachment_evidence_cannot_cross_adapter_identity() {
    const VICTIM_EVIDENCE: &str = "token-commune-test-secret";

    assert!(
        AdapterEvidenceVerifier::new([
            (adapter_id().value, EVIDENCE),
            ("token-commune".to_owned(), EVIDENCE),
        ])
        .is_err(),
        "the core must reject a shared credential across adapter identities"
    );

    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage.clone(),
        domain.clone(),
        AdapterEvidenceVerifier::new([
            (adapter_id().value, EVIDENCE),
            ("token-commune".to_owned(), VICTIM_EVIDENCE),
        ])
        .expect("valid per-adapter evidence"),
    )
    .await
    .expect("service initializes");
    let victim_id = AdapterId {
        value: "token-commune".into(),
    };

    let mut victim_registration = registration(domain.clone());
    victim_registration.adapter_id = Some(victim_id.clone());
    victim_registration.endpoint_id = Some(EndpointId {
        value: "token-commune-endpoint".into(),
    });
    let victim_attachment = service
        .attach(Request::new(AttachRequest {
            registration: Some(victim_registration.clone()),
            attachment_evidence: VICTIM_EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("victim adapter attaches");
    let victim_token = attachment_token(&victim_attachment);
    let attacker_token = attach_generation(&service, domain.clone(), 1).await;

    victim_registration.adapter_generation = Some(Generation { value: u64::MAX });
    let forged_attach = service
        .attach(Request::new(AttachRequest {
            registration: Some(victim_registration.clone()),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect_err("one adapter's credential cannot replace another adapter");
    assert_eq!(forged_attach.code(), tonic::Code::Unauthenticated);

    let mut victim_report = session_report(SessionConnectivityState::Live);
    victim_report.adapter_id = Some(victim_id.clone());
    service
        .ingest_observation(authenticated_as_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(
                    victim_report.clone(),
                )),
            },
            &victim_id,
            VICTIM_EVIDENCE,
            &victim_token,
        ))
        .await
        .expect("rejected replacement leaves the victim's ingestion channel current");

    victim_report.source_cursor.as_mut().unwrap().revision = 2;
    let forged_ingest = service
        .ingest_observation(authenticated_as_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(victim_report)),
            },
            &victim_id,
            EVIDENCE,
            &attacker_token,
        ))
        .await
        .expect_err("one adapter's attachment cannot ingest as another adapter");
    assert_eq!(forged_ingest.code(), tonic::Code::Unauthenticated);

    let forged_subscription = service
        .receive_deliveries(authenticated_as_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(victim_id.clone()),
                cursor: Some(Lsn { value: 0 }),
            },
            &victim_id,
            EVIDENCE,
            &attacker_token,
        ))
        .await
        .err()
        .expect("one adapter's attachment cannot subscribe as another adapter");
    assert_eq!(forged_subscription.code(), tonic::Code::Unauthenticated);

    let _victim_subscription = service
        .receive_deliveries(authenticated_as_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(victim_id.clone()),
                cursor: Some(Lsn { value: 0 }),
            },
            &victim_id,
            VICTIM_EVIDENCE,
            &victim_token,
        ))
        .await
        .expect("rejected replacement leaves the victim's subscription current");

    let durable = adapter::rebuild_from_log(&storage, &domain)
        .await
        .expect("rejected forgery leaves adapter registration replayable");
    assert_eq!(
        durable
            .get(&victim_id)
            .and_then(|record| record.registration.adapter_generation)
            .map(|generation| generation.value),
        Some(1),
        "forged maximal generation must not fence legitimate reattachment"
    );

    victim_registration.adapter_generation = Some(Generation { value: 2 });
    service
        .attach(Request::new(AttachRequest {
            registration: Some(victim_registration),
            attachment_evidence: VICTIM_EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("the legitimate adapter can reattach at its next generation");
}

#[tokio::test]
async fn newer_attachment_fences_stale_adapter_process() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");

    let stale_token = attach_generation(&service, domain.clone(), 1).await;
    let current_token = attach_generation(&service, domain, 2).await;

    let stale = service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &stale_token,
        ))
        .await
        .err()
        .expect("superseded attachment token must be rejected");
    assert_eq!(stale.code(), tonic::Code::Unauthenticated);

    service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &current_token,
        ))
        .await
        .expect("current attachment token remains valid");
}

#[tokio::test]
async fn stale_attachment_is_rejected_before_observation_or_diagnostic_decision() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId { value: "authority-main".into() };
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let stale_token = attach_generation(&service, domain.clone(), 1).await;
    let _current_token = attach_generation(&service, domain.clone(), 2).await;

    let observation = service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::SessionReport(
                    session_report(SessionConnectivityState::Live),
                )),
            },
            &stale_token,
        ))
        .await
        .expect_err("stale observation attachment must be rejected");
    assert_eq!(observation.code(), tonic::Code::Unauthenticated);

    let diagnostics = service
        .report_diagnostics(authenticated_with_attachment_token(
            AdapterDiagnosticReport::default(),
            &stale_token,
        ))
        .await
        .expect_err("stale diagnostic attachment must be rejected");
    assert_eq!(diagnostics.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn post_attach_rpc_without_attachment_token_is_rejected() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, _attachment_token) = attached_service(storage, domain).await;

    let status = service
        .receive_deliveries(authenticated(ReceiveRequest {
            adapter_id: Some(adapter_id()),
            cursor: Some(Lsn { value: 0 }),
        }))
        .await
        .err()
        .expect("missing attachment token must be rejected");
    assert_eq!(status.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn rebuilt_core_forgets_attachment_tokens_until_adapter_reattaches() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (before_restart, stale_token) = attached_service(storage.clone(), domain.clone()).await;
    drop(before_restart);

    let after_restart = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service rebuilds");
    let status = after_restart
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &stale_token,
        ))
        .await
        .err()
        .expect("in-memory attachment tokens must not survive restart");
    assert_eq!(status.code(), tonic::Code::Unauthenticated);

    let refreshed_token = attach_generation(&after_restart, domain, 2).await;
    after_restart
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            &refreshed_token,
        ))
        .await
        .expect("reattachment restores access");
}

#[tokio::test]
async fn delivered_command_is_redelivered_and_reacknowledged_without_double_transition() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let operation = targeted_operation(domain.clone(), "command-redelivery");
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation),
            },
        )
        .await
        .expect("operation appends");

    let mut first_tail = receive_from_start(&service, &attachment_token).await;
    assert!(first_tail.next().await.unwrap().is_ok());

    // Treat the first successful core call as a response lost after the
    // delivered checkpoint committed: the simulated adapter does not execute.
    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("first acknowledgement commits delivered");
    drop(first_tail);
    let mut executions = 0;

    let mut redelivery_tail = receive_from_start(&service, &attachment_token).await;
    let redelivery = redelivery_tail
        .next()
        .await
        .expect("delivered command is re-offered")
        .expect("redelivery is valid");
    assert_eq!(redelivery.operation, Some(operation.clone()));

    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("delivered command re-acknowledges idempotently");
    drop(redelivery_tail);
    executions += 1;
    assert_eq!(executions, 1, "adapter begins execution exactly once");

    let events = storage
        .read_after(&domain, Lsn { value: 0 })
        .await
        .expect("events remain readable");
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload.kind == StoredEventKind::CommandTransition as i32)
            .count(),
        1,
        "a re-ack must not append delivered -> delivered"
    );
}

#[tokio::test]
async fn deferred_spawn_success_suppresses_redelivery_after_restart_and_reattach() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let mut operation = targeted_operation(domain.clone(), "spawn-non-idempotent");
    operation.kind = OperationKind::Spawn as i32;
    operation.target_scope = Some(TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(adapter_id()),
        ..TargetScope::default()
    });
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation),
            },
        )
        .await
        .expect("spawn operation appends");

    let mut first_tail = receive_from_start(&service, &attachment_token).await;
    let first_delivery = first_tail
        .next()
        .await
        .expect("spawn is initially offered")
        .expect("initial delivery is valid");
    assert_eq!(first_delivery.operation, Some(operation.clone()));
    service
        .ingest_observation(authenticated_with_attachment_token(
            delivery_acknowledgement(domain.clone(), &operation),
            &attachment_token,
        ))
        .await
        .expect("spawn delivery acknowledgement commits");
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain.clone()),
                observation: Some(observation_request::Observation::Event(Observation {
                    authority_domain_id: Some(domain.clone()),
                    kind: ObservationKind::Result as i32,
                    correlations: vec![TypedCorrelation {
                        r#ref: Some(typed_correlation::Ref::CommandId(
                            operation.command_id.clone().unwrap(),
                        )),
                    }],
                    target_scope: operation.target_scope.clone(),
                    failure_code: FailureCode::Unspecified as i32,
                    ..Observation::default()
                })),
            },
            &attachment_token,
        ))
        .await
        .expect("successful spawn result is durably deferred");
    drop(first_tail);
    drop(service);

    let restarted = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("adapter service rebuilds from the durable result");
    let restarted_token = attach_generation(&restarted, domain, 2).await;
    let mut restarted_tail = receive_from_start(&restarted, &restarted_token).await;
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(250), restarted_tail.next())
            .await
            .is_err(),
        "a durable successful result must suppress non-idempotent spawn redelivery"
    );
}

#[tokio::test]
async fn abnormal_delivery_stream_drop_marks_adapter_sessions_stale() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        1,
        &attachment_token,
    )
    .await;

    let tail = receive_from_start(&service, &attachment_token).await;
    drop(tail); // no terminal `None`: models transport loss / process death

    let mut became_stale = false;
    for _ in 0..100 {
        let rebuilt = session::rebuild_from_log(&storage, &domain)
            .await
            .expect("session log rebuilds");
        let connectivity = rebuilt
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains registered")
            .state
            .connectivity();
        if connectivity == SessionConnectivityState::Stale {
            became_stale = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        became_stale,
        "abnormal stream drop did not durably mark the session stale"
    );

    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        2,
        &attachment_token,
    )
    .await;
    let refreshed = session::rebuild_from_log(&storage, &domain)
        .await
        .expect("session log rebuilds after reconnect report");
    assert_eq!(
        refreshed
            .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
            .expect("session remains registered")
            .state
            .connectivity(),
        SessionConnectivityState::Live,
        "a fresh adapter report restores authoritative liveness"
    );
}

#[tokio::test]
async fn idle_delivery_subscription_receives_an_operation_accepted_later() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let mut subscription = receive_from_start(&service, &attachment_token).await;

    assert!(
        tokio::time::timeout(Duration::from_millis(20), subscription.next())
            .await
            .is_err(),
        "empty subscription must not complete"
    );

    let operation = targeted_operation(domain.clone(), "command-after-open");
    storage
        .append(
            &domain,
            StoredEventPayload {
                kind: StoredEventKind::Operation as i32,
                payload: accepted_operation_bytes(&operation),
            },
        )
        .await
        .expect("operation appends");

    let delivered = tokio::time::timeout(Duration::from_secs(1), subscription.next())
        .await
        .expect("subscription observes the durable tail")
        .expect("subscription remains open")
        .expect("delivery is valid");
    assert_eq!(delivered.operation, Some(operation));
}

#[tokio::test]
async fn obsolete_stream_drop_is_inert_but_current_stream_drop_marks_stale() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    report_session(
        &service,
        domain.clone(),
        SessionConnectivityState::Live,
        1,
        &attachment_token,
    )
    .await;

    let obsolete = receive_from_start(&service, &attachment_token).await;
    let current = receive_from_start(&service, &attachment_token).await;
    drop(obsolete);
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        session_connectivity(&storage, &domain).await,
        SessionConnectivityState::Live,
        "the newer stream epoch fences the obsolete drop"
    );

    drop(current);
    wait_for_connectivity(&storage, &domain, SessionConnectivityState::Stale).await;
}

#[tokio::test]
async fn stream_loss_fails_running_once_and_leaves_delivered_redeliverable() {
    let storage = RusqliteStorage::open_in_memory().expect("storage opens");
    let domain = AuthorityDomainId {
        value: "authority-main".into(),
    };
    let (service, attachment_token) = attached_service(storage.clone(), domain.clone()).await;
    let running = targeted_operation(domain.clone(), "command-running");
    let resource_running = resource_targeted_operation(domain.clone(), "resource-command-running");
    let delivered = targeted_operation(domain.clone(), "command-delivered");
    for operation in [&running, &resource_running, &delivered] {
        storage
            .append(
                &domain,
                StoredEventPayload {
                    kind: StoredEventKind::Operation as i32,
                    payload: accepted_operation_bytes(operation),
                },
            )
            .await
            .expect("operation appends");
    }

    let mut subscription = receive_from_start(&service, &attachment_token).await;
    for operation in [&running, &resource_running, &delivered] {
        subscription
            .next()
            .await
            .expect("delivery")
            .expect("valid delivery");
        service
            .ingest_observation(authenticated_with_attachment_token(
                delivery_acknowledgement(domain.clone(), operation),
                &attachment_token,
            ))
            .await
            .expect("delivery acknowledgement");
    }
    for operation in [&running, &resource_running] {
        service
            .ingest_observation(authenticated_with_attachment_token(
                lifecycle_observation(
                    domain.clone(),
                    operation,
                    ObservationKind::Status,
                    FailureCode::Unspecified,
                ),
                &attachment_token,
            ))
            .await
            .expect("running observation");
    }

    drop(subscription);
    wait_for_command_state(&storage, &domain, "command-running", OperationState::Failed).await;
    wait_for_command_state(
        &storage,
        &domain,
        "resource-command-running",
        OperationState::Failed,
    )
    .await;

    let rebuilt = acceptance::rebuild_from_log(&storage, &domain)
        .await
        .expect("command log rebuilds");
    let running_record = rebuilt
        .get_command(&CommandId {
            value: "command-running".into(),
        })
        .expect("running command remains indexed");
    assert_eq!(
        running_record.failure_code,
        Some(FailureCode::ExecutionOutcomeUnknown)
    );
    assert_eq!(
        rebuilt
            .get_command(&CommandId {
                value: "resource-command-running".into(),
            })
            .expect("running resource command remains indexed")
            .failure_code,
        Some(FailureCode::ExecutionOutcomeUnknown)
    );
    assert_eq!(
        rebuilt
            .get_command(&CommandId {
                value: "command-delivered".into(),
            })
            .expect("delivered command remains indexed")
            .state,
        OperationState::Delivered
    );

    service
        .ingest_observation(authenticated_with_attachment_token(
            lifecycle_observation(
                domain.clone(),
                &running,
                ObservationKind::Result,
                FailureCode::Unspecified,
            ),
            &attachment_token,
        ))
        .await
        .expect("late completion is accepted as audit evidence");
    let after_late = acceptance::rebuild_from_log(&storage, &domain)
        .await
        .expect("command log rebuilds after late terminal");
    assert_eq!(
        after_late
            .get_command(&CommandId {
                value: "command-running".into(),
            })
            .expect("command")
            .state,
        OperationState::Failed,
        "first durable terminal outcome remains final"
    );

    let mut redelivery = receive_from_start(&service, &attachment_token).await;
    let operation = redelivery
        .next()
        .await
        .expect("delivered command is re-offered")
        .expect("redelivery is valid")
        .operation
        .expect("redelivery carries operation");
    assert_eq!(operation.command_id, delivered.command_id);
}

async fn session_connectivity(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
) -> SessionConnectivityState {
    session::rebuild_from_log(storage, domain)
        .await
        .expect("session log rebuilds")
        .get_live_session(&adapter_id(), "machine-a", &runtime_session_id())
        .expect("session remains registered")
        .state
        .connectivity()
}

async fn wait_for_connectivity(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
    expected: SessionConnectivityState,
) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if session_connectivity(storage, domain).await == expected {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("session connectivity reaches expected state");
}

async fn wait_for_command_state(
    storage: &RusqliteStorage,
    domain: &AuthorityDomainId,
    command_id: &str,
    expected: OperationState,
) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let commands = acceptance::rebuild_from_log(storage, domain)
                .await
                .expect("command log rebuilds");
            if commands
                .get_command(&CommandId {
                    value: command_id.into(),
                })
                .is_some_and(|record| record.state == expected)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("command reaches expected state");
}

async fn attached_service(
    storage: RusqliteStorage,
    domain: AuthorityDomainId,
) -> (AdapterControlServiceImpl<RusqliteStorage>, String) {
    let service = AdapterControlServiceImpl::new(
        storage,
        domain.clone(),
        evidence_verifier(),
    )
    .await
    .expect("service initializes");
    let attachment_token = attach_generation(&service, domain, 1).await;
    (service, attachment_token)
}

async fn receive_from_start(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    attachment_token: &str,
) -> DeliveryStream {
    service
        .receive_deliveries(authenticated_with_attachment_token(
            ReceiveRequest {
                adapter_id: Some(adapter_id()),
                cursor: Some(Lsn { value: 0 }),
            },
            attachment_token,
        ))
        .await
        .expect("delivery stream opens")
        .into_inner()
}

async fn report_session(
    service: &AdapterControlServiceImpl<RusqliteStorage>,
    domain: AuthorityDomainId,
    connectivity: SessionConnectivityState,
    revision: u64,
    attachment_token: &str,
) {
    service
        .ingest_observation(authenticated_with_attachment_token(
            ObservationRequest {
                authority_domain_id: Some(domain),
                observation: Some(observation_request::Observation::SessionReport({
                    let mut report = session_report(connectivity);
                    report.source_cursor.as_mut().unwrap().revision = revision;
                    report
                })),
            },
            attachment_token,
        ))
        .await
        .expect("session report succeeds");
}

fn session_report(
    connectivity: SessionConnectivityState,
) -> patchbay_contracts::patchbay::SessionReport {
    patchbay_contracts::patchbay::SessionReport {
        adapter_id: Some(adapter_id()),
        deployment_scope: "machine-a".into(),
        runtime_session_id: Some(runtime_session_id()),
        session_generation: Some(Generation { value: 1 }),
        connectivity: connectivity as i32,
        activity: SessionActivityState::Idle as i32,
        project: "patchbay".into(),
        cwd: "/work/patchbay".into(),
        name: "test".into(),
        model: "provider/model".into(),
        spawn_origin: None,
        source_cursor: Some(SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        }),
    }
}

fn targeted_operation(domain: AuthorityDomainId, command: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.into(),
        }),
        authority_domain_id: Some(domain),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            adapter_id: Some(adapter_id()),
            deployment_scope: "machine-a".into(),
            runtime_session_id: Some(runtime_session_id()),
            session_generation: Some(Generation { value: 1 }),
            ..Default::default()
        }),
        idempotency_key: format!("{command}-key"),
        ..Default::default()
    }
}

fn resource_targeted_operation(domain: AuthorityDomainId, command: &str) -> Operation {
    Operation {
        command_id: Some(CommandId { value: command.into() }),
        authority_domain_id: Some(domain),
        kind: OperationKind::Query as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Resource as i32,
            resource: Some(ResourceIdentity {
                adapter_id: Some(adapter_id()),
                resource_id: Some(ResourceId { value: "shared".into() }),
                resource_kind: Some(ResourceKind { value: "pool".into() }),
            }),
            ..TargetScope::default()
        }),
        idempotency_key: format!("{command}-key"),
        ..Operation::default()
    }
}

fn lifecycle_observation(
    domain: AuthorityDomainId,
    operation: &Operation,
    kind: ObservationKind,
    failure_code: FailureCode,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::Event(Observation {
            authority_domain_id: Some(domain),
            kind: kind as i32,
            target_scope: operation.target_scope.clone(),
            failure_code: failure_code as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(
                    operation.command_id.clone().expect("command id"),
                )),
            }],
            ..Default::default()
        })),
    }
}

fn delivery_acknowledgement(
    domain: AuthorityDomainId,
    operation: &Operation,
) -> ObservationRequest {
    ObservationRequest {
        authority_domain_id: Some(domain.clone()),
        observation: Some(observation_request::Observation::Event(Observation {
            authority_domain_id: Some(domain),
            kind: ObservationKind::Event as i32,
            target_scope: operation.target_scope.clone(),
            payload: Some(PayloadEnvelope {
                schema_ref: adapter::DELIVERY_ACKNOWLEDGEMENT_SCHEMA.to_owned(),
                ..Default::default()
            }),
            failure_code: FailureCode::Unspecified as i32,
            correlations: vec![TypedCorrelation {
                r#ref: Some(typed_correlation::Ref::CommandId(
                    operation.command_id.clone().expect("command id"),
                )),
            }],
            ..Default::default()
        })),
    }
}

fn registration(domain: AuthorityDomainId) -> AdapterRegistration {
    AdapterRegistration {
        adapter_id: Some(adapter_id()),
        endpoint_id: Some(EndpointId {
            value: "pi-adapter-endpoint".into(),
        }),
        authority_domain_id: Some(domain),
        adapter_generation: Some(Generation { value: 1 }),
        capability: Some(AdapterCapability {
            supported_operation_kinds: vec![OperationKind::Instruct as i32],
            streaming_support: true,
            session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
            cancellation_support: true,
            session_replacement_support: true,
            target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn adapter_id() -> AdapterId {
    AdapterId { value: "pi".into() }
}

fn evidence_verifier() -> AdapterEvidenceVerifier {
    AdapterEvidenceVerifier::new([(adapter_id().value, EVIDENCE)])
        .expect("valid per-adapter evidence")
}

fn runtime_session_id() -> RuntimeSessionId {
    RuntimeSessionId {
        value: "session-1".into(),
    }
}

async fn attach_generation<S>(
    service: &AdapterControlServiceImpl<S>,
    domain: AuthorityDomainId,
    generation: u64,
) -> String
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    let mut registration = registration(domain);
    registration.adapter_generation = Some(Generation { value: generation });
    let response = service
        .attach(Request::new(AttachRequest {
            registration: Some(registration),
            attachment_evidence: EVIDENCE.as_bytes().to_vec(),
        }))
        .await
        .expect("attach succeeds");
    attachment_token(&response)
}

fn attachment_token<T>(response: &Response<T>) -> String {
    response
        .metadata()
        .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
        .expect("attach response carries attachment token")
        .to_str()
        .expect("attachment token is ASCII")
        .to_owned()
}

fn authenticated_with_attachment_token<T>(message: T, attachment_token: &str) -> Request<T> {
    let mut request = authenticated(message);
    request.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        attachment_token.parse().expect("metadata"),
    );
    request
}

fn authenticated<T>(message: T) -> Request<T> {
    authenticated_as(message, &adapter_id(), EVIDENCE)
}

fn authenticated_as_with_attachment_token<T>(
    message: T,
    adapter_id: &AdapterId,
    evidence: &str,
    attachment_token: &str,
) -> Request<T> {
    let mut request = authenticated_as(message, adapter_id, evidence);
    request.metadata_mut().insert(
        ADAPTER_ATTACHMENT_TOKEN_HEADER,
        attachment_token.parse().expect("metadata"),
    );
    request
}

fn authenticated_as<T>(message: T, adapter_id: &AdapterId, evidence: &str) -> Request<T> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(
        ADAPTER_ID_HEADER,
        adapter_id.value.parse().expect("metadata"),
    );
    request.metadata_mut().insert(
        ADAPTER_EVIDENCE_HEADER,
        evidence.parse().expect("metadata"),
    );
    request
}
