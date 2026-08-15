use std::sync::atomic::{AtomicUsize, Ordering};

use patchbay_contracts::patchbay::{
    resource_state_mutation, ActorEndpointRef, ActorId, AdapterId, AdapterSnapshotSupport,
    AuthorityDomainId, CommandId, DeviceId, EndpointId, Generation, Grant, GrantId,
    GrantProvenance, GrantRevocationPolicy, Operation, OperationKind, PayloadContentType,
    PayloadEnvelope, ResourceId, ResourceKind, ResourceStateEvent, ResourceStateMutation,
    ResourceStateUpsert, ResourceViewStateUpdate, StoredEventKind, SubmissionOutcome, TargetScope,
    TargetScopeKind, TimeWindow,
};
use patchbay_core::{
    acceptance::{
        submit_with_clock, ActiveElicitation, Authorized, CommandIndex, ElicitationContractLookup,
        GrantCheck, GrantDenied, TargetBinding, TargetNotFound, TargetResolver,
    },
    authority::{ingest_grant, AuthorityRegistry, IssuerContext},
    resource::{ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    storage::{event_id, RecordedEvent, RusqliteStorage, Storage},
    target::TargetRegistry,
    time::TestClock,
};
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn identity(adapter: &str, kind: &str, id: &str) -> ResourceIdentity {
    ResourceIdentity::new(
        AdapterId {
            value: adapter.to_owned(),
        },
        ResourceKind {
            value: kind.to_owned(),
        },
        ResourceId {
            value: id.to_owned(),
        },
    )
    .unwrap()
}

fn resource_registry(identities: &[ResourceIdentity]) -> ResourceRegistry {
    let mut registry = ResourceRegistry::new();
    for (index, identity) in identities.iter().enumerate() {
        let state = ResourceStateEvent {
            authority_domain_id: Some(domain()),
            source_adapter_id: Some(identity.adapter_id().clone()),
            source_adapter_generation: Some(Generation { value: 1 }),
            views: vec![ResourceViewStateUpdate {
                resource_kind: Some(identity.resource_kind().clone()),
                completeness: AdapterSnapshotSupport::Partial as i32,
            }],
            mutations: vec![ResourceStateMutation {
                identity: Some(identity.to_scope().resource.unwrap()),
                from_revision_lsn: None,
                mutation: Some(resource_state_mutation::Mutation::Upsert(
                    ResourceStateUpsert {
                        resource_payload: Some(PayloadEnvelope {
                            payload: vec![1],
                            content_type: PayloadContentType::Protobuf as i32,
                            schema_ref: "resource.schema".into(),
                        }),
                        projection_payload: Some(PayloadEnvelope {
                            payload: vec![2],
                            content_type: PayloadContentType::Protobuf as i32,
                            schema_ref: "projection.schema".into(),
                        }),
                    },
                )),
            }],
            observed_at: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
        };
        registry
            .observe(&RecordedEvent {
                event_id: event_id(domain(), index as u64 + 1),
                payload: patchbay_core::resource::events::encode(&state),
            })
            .unwrap();
    }
    registry
}

fn operation(command: &str, key: &str, target: TargetScope) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command.to_owned(),
        }),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: "payload-claim".to_owned(),
            }),
            ..ActorEndpointRef::default()
        }),
        kind: OperationKind::Query as i32,
        target_scope: Some(target),
        idempotency_key: key.to_owned(),
        payload: Some(PayloadEnvelope::default()),
        validity_window: Some(TimeWindow {
            starts_at: Some(Timestamp {
                seconds: 90,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 110,
                nanos: 0,
            }),
        }),
        submitted_at: Some(Timestamp {
            seconds: 100,
            nanos: 0,
        }),
        ..Operation::default()
    }
}

fn grant(id: &str, target_scope: TargetScope) -> Grant {
    Grant {
        grant_id: Some(GrantId {
            value: id.to_owned(),
        }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId {
            value: "operator".to_owned(),
        }),
        target_scope: Some(target_scope),
        allowed_operation_kinds: vec![OperationKind::Query as i32],
        provenance: Some(GrantProvenance {
            reason: "resource acceptance test".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    }
}

#[derive(Clone)]
struct Issuer;

impl IssuerContext for Issuer {
    fn verified_actor(&self) -> Option<&ActorId> {
        static ACTOR: std::sync::OnceLock<ActorId> = std::sync::OnceLock::new();
        Some(ACTOR.get_or_init(|| ActorId {
            value: "operator".to_owned(),
        }))
    }
    fn verified_endpoint(&self) -> Option<&EndpointId> {
        static ENDPOINT: std::sync::OnceLock<EndpointId> = std::sync::OnceLock::new();
        Some(ENDPOINT.get_or_init(|| EndpointId {
            value: "web".to_owned(),
        }))
    }
    fn verified_device(&self) -> Option<&DeviceId> {
        static DEVICE: std::sync::OnceLock<DeviceId> = std::sync::OnceLock::new();
        Some(DEVICE.get_or_init(|| DeviceId {
            value: "device".to_owned(),
        }))
    }
    fn endpoint_generation(&self) -> Option<Generation> {
        Some(Generation { value: 1 })
    }
    fn authority_domain_id(&self) -> &AuthorityDomainId {
        static DOMAIN: std::sync::OnceLock<AuthorityDomainId> = std::sync::OnceLock::new();
        DOMAIN.get_or_init(domain)
    }
}

struct NoContracts;
impl ElicitationContractLookup for NoContracts {
    async fn active_contract(
        &self,
        _elicitation_id: &patchbay_contracts::patchbay::ElicitationId,
    ) -> Option<ActiveElicitation> {
        None
    }
}

fn clock() -> TestClock {
    TestClock::new(Timestamp {
        seconds: 100,
        nanos: 0,
    })
}

async fn operation_count(storage: &RusqliteStorage) -> usize {
    storage
        .read_after(&domain(), patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .unwrap()
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .count()
}

#[tokio::test]
async fn exact_authorized_registered_resource_accepts_without_session_identity() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let exact = identity("adapter-a", "pool", "shared");
    let mut authority = AuthorityRegistry::new();
    ingest_grant(
        &storage,
        &mut authority,
        &domain(),
        grant("resource-grant", exact.to_scope()),
    )
    .await
    .unwrap();
    let resources = resource_registry(std::slice::from_ref(&exact));
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    let result = submit_with_clock(
        &storage,
        &authority,
        &targets,
        &CommandIndex::new(),
        &NoContracts,
        &Issuer,
        operation("resource-command", "resource-key", exact.to_scope()),
        &clock(),
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
    assert!(!result.deduplicated);
    assert_eq!(operation_count(&storage).await, 1);
}

#[tokio::test]
async fn exact_grant_fences_cross_adapter_and_kind_before_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let exact = identity("adapter-a", "pool", "shared");
    let mut authority = AuthorityRegistry::new();
    ingest_grant(
        &storage,
        &mut authority,
        &domain(),
        grant("resource-grant", exact.to_scope()),
    )
    .await
    .unwrap();
    let resources = resource_registry(&[
        exact.clone(),
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
    ]);
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    for (index, collision) in [
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
    ]
    .into_iter()
    .enumerate()
    {
        let result = submit_with_clock(
            &storage,
            &authority,
            &targets,
            &CommandIndex::new(),
            &NoContracts,
            &Issuer,
            operation(
                &format!("collision-{index}"),
                &format!("collision-key-{index}"),
                collision.to_scope(),
            ),
            &clock(),
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, SubmissionOutcome::Rejected as i32);
    }
    assert_eq!(operation_count(&storage).await, 0);
}

#[tokio::test]
async fn adapter_grant_reaches_resolution_and_target_key_scopes_the_full_tuple() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut authority = AuthorityRegistry::new();
    ingest_grant(
        &storage,
        &mut authority,
        &domain(),
        grant(
            "adapter-grant",
            TargetScope {
                kind: TargetScopeKind::Adapter as i32,
                adapter_id: Some(AdapterId {
                    value: "adapter-a".to_owned(),
                }),
                ..TargetScope::default()
            },
        ),
    )
    .await
    .unwrap();
    let first = identity("adapter-a", "pool", "shared");
    let second = identity("adapter-a", "window", "shared");
    let resources = resource_registry(&[first.clone(), second.clone()]);
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    let unknown = identity("adapter-a", "pool", "unknown");
    let missing = submit_with_clock(
        &storage,
        &authority,
        &targets,
        &CommandIndex::new(),
        &NoContracts,
        &Issuer,
        operation("unknown", "shared-key", unknown.to_scope()),
        &clock(),
    )
    .await
    .unwrap();
    assert_eq!(missing.outcome, SubmissionOutcome::Rejected as i32);
    assert_eq!(missing.reason_code, "target_not_found");

    for (command, target) in [("first", first), ("second", second)] {
        let result = submit_with_clock(
            &storage,
            &authority,
            &targets,
            &CommandIndex::new(),
            &NoContracts,
            &Issuer,
            operation(command, "shared-key", target.to_scope()),
            &clock(),
        )
        .await
        .unwrap();
        assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
        assert!(!result.deduplicated);
    }
    assert_eq!(operation_count(&storage).await, 2);
}

struct CountingGrant<'a>(&'a AtomicUsize);
impl GrantCheck for CountingGrant<'_> {
    async fn check(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _issuer: &dyn IssuerContext,
        _operation_kind: OperationKind,
        _target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(Authorized {
            grant_id: Some(GrantId {
                value: "unexpected".to_owned(),
            }),
            continuation_authority: None,
        })
    }
}

struct CountingResolver<'a>(&'a AtomicUsize);
impl TargetResolver for CountingResolver<'_> {
    async fn resolve(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _operation: &Operation,
        _spawn_request: Option<&patchbay_contracts::patchbay::SpawnRequest>,
    ) -> Result<TargetBinding, TargetNotFound> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(TargetBinding::AuthorityDomain(domain()))
    }
}

#[tokio::test]
async fn malformed_resource_rejects_before_grant_resolution_or_append() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let grants = AtomicUsize::new(0);
    let resolutions = AtomicUsize::new(0);
    let mut malformed = identity("adapter-a", "pool", "shared").to_scope();
    malformed.resource.as_mut().unwrap().resource_kind = None;

    let result = submit_with_clock(
        &storage,
        &CountingGrant(&grants),
        &CountingResolver(&resolutions),
        &CommandIndex::new(),
        &NoContracts,
        &Issuer,
        operation("malformed", "malformed-key", malformed),
        &clock(),
    )
    .await
    .unwrap();
    assert_eq!(result.outcome, SubmissionOutcome::Rejected as i32);
    assert_eq!(result.reason_code, "validation_failed");
    assert_eq!(grants.load(Ordering::Relaxed), 0);
    assert_eq!(resolutions.load(Ordering::Relaxed), 0);
    assert_eq!(operation_count(&storage).await, 0);
}
