use patchbay_contracts::patchbay::{
    resource_state_mutation, spawn_request, AdapterCapability, AdapterId, AdapterRegistration,
    AdapterSnapshotSupport, AdapterTargetCategory, AuthorityDomainId, CommandId, EndpointId,
    ExternalRuntimeRef, FreshSpawn, Generation, LogicalTargetId, Operation, OperationKind,
    PayloadContentType, PayloadEnvelope, ResourceId, ResourceKind, ResourceStateEvent,
    ResourceStateMutation, ResourceStateUpsert, ResourceViewStateUpdate, RuntimeGenerationRef,
    RuntimeSessionId, SpawnContinuation, SpawnRequest, SpawnTargetSpec, TargetScope,
    TargetScopeKind,
};
use patchbay_core::{
    acceptance::{TargetBinding, TargetResolver},
    adapter::{ingest_registration, AdapterRegistry},
    diagnostics::AuthorityDomainTargetResolver,
    resource::{ResourceIdentity, ResourceRegistry},
    session::SessionRegistry,
    storage::{event_id, RecordedEvent, RusqliteStorage},
    target::{target_adapter_id, TargetRegistry},
};
use prost_types::Timestamp;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn operation(kind: OperationKind, target_scope: TargetScope) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: "command-a".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        kind: kind as i32,
        target_scope: Some(target_scope),
        ..Operation::default()
    }
}

fn fresh_spawn() -> SpawnRequest {
    SpawnRequest {
        intent: Some(spawn_request::Intent::Fresh(FreshSpawn {})),
        target_spec: Some(SpawnTargetSpec {
            shape: "session".to_owned(),
            ..SpawnTargetSpec::default()
        }),
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

fn registry(identities: &[ResourceIdentity]) -> ResourceRegistry {
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

#[tokio::test]
async fn registered_resource_resolves_by_the_exact_tuple() {
    let registered = identity("adapter-a", "pool", "shared");
    let resources = registry(std::slice::from_ref(&registered));
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    assert_eq!(
        TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Query, registered.to_scope()),
            None,
        )
        .await,
        Ok(TargetBinding::Resource(registered.clone()))
    );
    for unknown in [
        identity("adapter-b", "pool", "shared"),
        identity("adapter-a", "window", "shared"),
        identity("adapter-a", "pool", "other"),
    ] {
        assert!(TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Query, unknown.to_scope()),
            None,
        )
        .await
        .is_err());
    }
    assert_eq!(targets.resources().resources().count(), 1);
}

#[tokio::test]
async fn malformed_legacy_and_nonordinary_resource_targets_fail_closed() {
    let resources = registry(&[identity("adapter-a", "pool", "shared")]);
    let targets = TargetRegistry::new(SessionRegistry::new(domain()).unwrap(), resources);

    let legacy = TargetScope {
        kind: TargetScopeKind::Resource as i32,
        legacy_audit_resource_id: "shared".to_owned(),
        ..TargetScope::default()
    };
    let mut mixed = identity("adapter-a", "pool", "shared").to_scope();
    mixed.adapter_id = Some(AdapterId {
        value: "adapter-a".to_owned(),
    });
    let authority = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    for scope in [legacy, mixed, authority] {
        assert!(TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Query, scope),
            None,
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn spawn_resolution_commits_one_attached_adapter_boundary_only() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut adapters = AdapterRegistry::new();
    ingest_registration(
        &storage,
        &mut adapters,
        AdapterRegistration {
            adapter_id: Some(AdapterId {
                value: "adapter-a".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "adapter-a-endpoint".to_owned(),
            }),
            authority_domain_id: Some(domain()),
            adapter_generation: Some(Generation { value: 1 }),
            capability: Some(AdapterCapability {
                supported_operation_kinds: vec![OperationKind::Spawn as i32],
                session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                ..AdapterCapability::default()
            }),
            ..AdapterRegistration::default()
        },
    )
    .await
    .unwrap();
    ingest_registration(
        &storage,
        &mut adapters,
        AdapterRegistration {
            adapter_id: Some(AdapterId {
                value: "adapter-b".to_owned(),
            }),
            endpoint_id: Some(EndpointId {
                value: "adapter-b-endpoint".to_owned(),
            }),
            authority_domain_id: Some(domain()),
            adapter_generation: Some(Generation { value: 1 }),
            capability: Some(AdapterCapability {
                supported_operation_kinds: vec![OperationKind::Spawn as i32],
                session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
                target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
                ..AdapterCapability::default()
            }),
            ..AdapterRegistration::default()
        },
    )
    .await
    .unwrap();
    let mut targets = TargetRegistry::with_adapters(
        SessionRegistry::new(domain()).unwrap(),
        registry(&[identity("adapter-a", "pool", "shared")]),
        adapters,
    );
    let adapter = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "adapter-a".to_owned(),
        }),
        ..TargetScope::default()
    };
    let fresh = fresh_spawn();
    assert_eq!(
        TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Spawn, adapter.clone()),
            Some(&fresh),
        )
        .await,
        Ok(TargetBinding::SpawnAdapter {
            adapter_id: AdapterId {
                value: "adapter-a".to_owned(),
            },
            claim: Box::new(patchbay_contracts::patchbay::SpawnGenerationClaim {
                authority_domain_id: Some(domain()),
                claim_operation_id: Some(CommandId {
                    value: "command-a".to_owned(),
                }),
                logical_target_id: Some(patchbay_contracts::patchbay::LogicalTargetId {
                    value: "command-a".to_owned(),
                }),
                expected_prior: None,
                claimed_generation: Some(Generation { value: 1 }),
            }),
            continuation_authority: None,
        })
    );

    let logical_target_id = LogicalTargetId {
        value: "logical-a".to_owned(),
    };
    let prior_external = ExternalRuntimeRef {
        adapter_id: Some(AdapterId {
            value: "adapter-a".to_owned(),
        }),
        deployment_scope: "local".to_owned(),
        runtime_session_id: Some(RuntimeSessionId {
            value: "runtime-a".to_owned(),
        }),
        generation: Some(Generation { value: 7 }),
    };
    targets
        .sessions_mut()
        .logical_targets_mut()
        .create(
            logical_target_id.clone(),
            AdapterId {
                value: "adapter-a".to_owned(),
            },
            "local".to_owned(),
        )
        .unwrap();
    targets
        .sessions_mut()
        .logical_targets_mut()
        .assign_initial_current(&logical_target_id, prior_external.clone())
        .unwrap();
    let prior = RuntimeGenerationRef {
        logical_target_id: Some(logical_target_id.clone()),
        external_runtime: Some(prior_external),
    };
    let continuation = SpawnRequest {
        intent: Some(spawn_request::Intent::Continuation(SpawnContinuation {
            prior: Some(prior.clone()),
        })),
        target_spec: fresh.target_spec.clone(),
    };
    let continuation_binding = TargetResolver::resolve(
        &targets,
        &domain(),
        &operation(OperationKind::Spawn, adapter.clone()),
        Some(&continuation),
    )
    .await
    .expect("the exact current generation resolves");
    assert!(matches!(
        continuation_binding,
        TargetBinding::SpawnAdapter {
            claim,
            continuation_authority: None,
            ..
        } if claim.logical_target_id == Some(logical_target_id)
            && claim.expected_prior == Some(prior.clone())
            && claim.claimed_generation == Some(Generation { value: 8 })
    ));
    let mut stale_prior = prior.clone();
    stale_prior.external_runtime.as_mut().unwrap().generation = Some(Generation { value: 6 });
    let stale_continuation = SpawnRequest {
        intent: Some(spawn_request::Intent::Continuation(SpawnContinuation {
            prior: Some(stale_prior),
        })),
        target_spec: fresh.target_spec.clone(),
    };
    assert!(TargetResolver::resolve(
        &targets,
        &domain(),
        &operation(OperationKind::Spawn, adapter.clone()),
        Some(&stale_continuation),
    )
    .await
    .is_err());
    let mut prior_mutations = Vec::new();
    let mut wrong_logical_target = prior.clone();
    wrong_logical_target.logical_target_id = Some(LogicalTargetId {
        value: "logical-other".to_owned(),
    });
    prior_mutations.push(wrong_logical_target);
    let mut wrong_runtime_adapter = prior.clone();
    wrong_runtime_adapter
        .external_runtime
        .as_mut()
        .unwrap()
        .adapter_id = Some(AdapterId {
        value: "adapter-b".to_owned(),
    });
    prior_mutations.push(wrong_runtime_adapter);
    let mut wrong_deployment = prior.clone();
    wrong_deployment
        .external_runtime
        .as_mut()
        .unwrap()
        .deployment_scope = "remote".to_owned();
    prior_mutations.push(wrong_deployment);
    let mut wrong_runtime = prior.clone();
    wrong_runtime
        .external_runtime
        .as_mut()
        .unwrap()
        .runtime_session_id = Some(RuntimeSessionId {
        value: "runtime-other".to_owned(),
    });
    prior_mutations.push(wrong_runtime);
    for mutated_prior in prior_mutations {
        let mutated = SpawnRequest {
            intent: Some(spawn_request::Intent::Continuation(SpawnContinuation {
                prior: Some(mutated_prior),
            })),
            target_spec: fresh.target_spec.clone(),
        };
        assert!(TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Spawn, adapter.clone()),
            Some(&mutated),
        )
        .await
        .is_err());
    }

    let other_adapter = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "adapter-b".to_owned(),
        }),
        ..TargetScope::default()
    };
    assert!(TargetResolver::resolve(
        &targets,
        &domain(),
        &operation(OperationKind::Spawn, other_adapter),
        Some(&continuation),
    )
    .await
    .is_err());

    let runtime = TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: adapter.adapter_id.clone(),
        runtime_session_id: Some(patchbay_contracts::patchbay::RuntimeSessionId {
            value: "existing".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        ..TargetScope::default()
    };
    let fleet = TargetScope {
        kind: TargetScopeKind::FleetSupervisor as i32,
        ..TargetScope::default()
    };
    let resource = identity("adapter-a", "pool", "shared").to_scope();
    let unattached = TargetScope {
        kind: TargetScopeKind::Adapter as i32,
        adapter_id: Some(AdapterId {
            value: "adapter-c".to_owned(),
        }),
        ..TargetScope::default()
    };
    let mut mixed = adapter.clone();
    mixed.project_or_group = "not-canonical".to_owned();
    for incompatible in [runtime, resource, fleet, unattached, mixed] {
        assert!(
            TargetResolver::resolve(
                &targets,
                &domain(),
                &operation(OperationKind::Spawn, incompatible.clone()),
                Some(&fresh),
            )
            .await
            .is_err(),
            "incompatible spawn target resolved: {incompatible:?}"
        );
    }
    assert!(
        TargetResolver::resolve(
            &targets,
            &domain(),
            &operation(OperationKind::Instruct, adapter),
            None,
        )
        .await
        .is_err(),
        "adapter scope is a spawn-only operation boundary"
    );
}

#[tokio::test]
async fn diagnostics_resolution_returns_an_honest_authority_domain_binding() {
    let scope = TargetScope {
        kind: TargetScopeKind::AuthorityDomain as i32,
        ..TargetScope::default()
    };
    assert_eq!(
        TargetResolver::resolve(
            &AuthorityDomainTargetResolver,
            &domain(),
            &operation(OperationKind::Query, scope),
            None,
        )
        .await,
        Ok(TargetBinding::AuthorityDomain(domain()))
    );

    let resource = identity("adapter-a", "pool", "shared").to_scope();
    assert!(TargetResolver::resolve(
        &AuthorityDomainTargetResolver,
        &domain(),
        &operation(OperationKind::Query, resource),
        None,
    )
    .await
    .is_err());
}

#[test]
fn adapter_routing_uses_only_complete_canonical_resource_identity() {
    let canonical = identity("adapter-a", "pool", "shared").to_scope();
    assert_eq!(
        target_adapter_id(&canonical).map(|id| id.value.as_str()),
        Some("adapter-a")
    );

    let mut partial = canonical.clone();
    partial.resource.as_mut().unwrap().resource_kind = None;
    assert_eq!(target_adapter_id(&partial), None);

    let mut mixed = canonical;
    mixed.adapter_id = Some(AdapterId {
        value: "adapter-b".to_owned(),
    });
    assert_eq!(target_adapter_id(&mixed), None);
}
