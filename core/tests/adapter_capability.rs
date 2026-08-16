use patchbay_contracts::patchbay::{
    adapter_assurance_manifest, ActorEndpointRef, ActorId, AdapterAssuranceManifest,
    AdapterAssuranceManifestV1, AdapterCapability, AdapterId, AdapterReconciliationStrength,
    AdapterRegistration, AdapterSnapshotSupport, AdapterTargetCategory, AttachmentMethod,
    AuthorityDomainId, EndpointId, FailureCode, Generation, IdempotencyStrength, Observation,
    ObservationKind, OperationKind, PayloadContentType, PayloadEnvelope, ReconciliationAction,
    ResourceCapability, ResourceId, ResourceKind, ResourceProjectionContract, SchemaDescriptor,
    StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    adapter::{
        ingest_registration, rebuild_from_log, AdapterRegistry, CapabilityValidationContext,
        ValidatedAdapterCapability, ADAPTER_REGISTRATION_SCHEMA,
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

fn assurance_v1(deduplication_strength: IdempotencyStrength) -> AdapterAssuranceManifest {
    AdapterAssuranceManifest {
        contract: Some(adapter_assurance_manifest::Contract::V1(
            AdapterAssuranceManifestV1 {
                deduplication_strength: deduplication_strength as i32,
                continuation_proof_support: Some(false),
                cursor_support: Some(false),
                generation_fence_support: Some(false),
                reconciliation_strength: AdapterReconciliationStrength::None as i32,
                unproven_outcome_action: ReconciliationAction::None as i32,
            },
        )),
    }
}

fn current_capability() -> AdapterCapability {
    AdapterCapability {
        assurance: Some(assurance_v1(IdempotencyStrength::None)),
        ..AdapterCapability::default()
    }
}

fn session_capability() -> AdapterCapability {
    AdapterCapability {
        session_snapshot_support: AdapterSnapshotSupport::Partial as i32,
        target_categories: vec![AdapterTargetCategory::RuntimeSession as i32],
        ..current_capability()
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
        ..current_capability()
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

    let payload = PayloadEnvelope {
        content_type: PayloadContentType::Protobuf as i32,
        schema_ref: "example.usage_window.payload.v1".to_owned(),
        ..PayloadEnvelope::default()
    };
    let projection = PayloadEnvelope {
        content_type: PayloadContentType::Json as i32,
        schema_ref: "example.usage_window.projection.v1".to_owned(),
        ..PayloadEnvelope::default()
    };
    for (name, mismatched_payload, mismatched_projection) in [
        (
            "payload content type",
            PayloadEnvelope {
                content_type: PayloadContentType::Json as i32,
                ..payload.clone()
            },
            projection.clone(),
        ),
        (
            "payload schema ref",
            PayloadEnvelope {
                schema_ref: "wrong.payload.v1".to_owned(),
                ..payload.clone()
            },
            projection.clone(),
        ),
        (
            "projection content type",
            payload.clone(),
            PayloadEnvelope {
                content_type: PayloadContentType::Protobuf as i32,
                ..projection.clone()
            },
        ),
        (
            "projection schema ref",
            payload.clone(),
            PayloadEnvelope {
                schema_ref: "wrong.projection.v1".to_owned(),
                ..projection.clone()
            },
        ),
    ] {
        assert!(
            registry
                .validate_resource_projection(&usage, &mismatched_payload, &mismatched_projection)
                .is_err(),
            "{name} mismatch must fail closed"
        );
    }
}

#[tokio::test]
async fn registration_redacts_attachment_descriptor_before_durable_replay() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let mut registry = AdapterRegistry::new();
    let mut capability = session_capability();
    capability.attachment_method = Some(AttachmentMethod {
        kind: "configured-local-material".to_owned(),
        descriptor: b"sentinel-attachment-secret".to_vec(),
        descriptor_content_type: PayloadContentType::Binary as i32,
    });
    ingest_registration(
        &storage,
        &mut registry,
        registration("redacted", capability),
    )
    .await
    .expect("adapter attaches");

    let domain = AuthorityDomainId {
        value: "authority-main".to_owned(),
    };
    let events = storage
        .read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .expect("events read");
    let stored_registration = events
        .iter()
        .filter(|event| event.payload.kind == StoredEventKind::Observation as i32)
        .filter_map(|event| Observation::decode(event.payload.payload.as_slice()).ok())
        .find_map(|observation| {
            observation
                .payload
                .filter(|payload| payload.schema_ref == ADAPTER_REGISTRATION_SCHEMA)
        })
        .map(|payload| {
            AdapterRegistration::decode(payload.payload.as_slice()).expect("registration decodes")
        })
        .expect("durable registration observation");
    assert!(stored_registration
        .capability
        .as_ref()
        .and_then(|capability| capability.attachment_method.as_ref())
        .is_some_and(|method| method.descriptor.is_empty()));

    let rebuilt = rebuild_from_log(&storage, &domain)
        .await
        .expect("registration replays");
    assert!(rebuilt
        .get(&AdapterId {
            value: "redacted".to_owned()
        })
        .and_then(|record| record.registration.capability.as_ref())
        .and_then(|capability| capability.attachment_method.as_ref())
        .is_some_and(|method| method.descriptor.is_empty()));
}

#[tokio::test]
async fn replay_accepts_only_the_complete_canonical_attachment_envelope() {
    let storage = RusqliteStorage::open_in_memory().expect("storage");
    let domain = AuthorityDomainId {
        value: "authority-main".to_owned(),
    };
    let mut producer_registry = AdapterRegistry::new();
    ingest_registration(
        &storage,
        &mut producer_registry,
        registration("canonical", session_capability()),
    )
    .await
    .expect("attachment produces a registration");
    let canonical = storage
        .read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 })
        .await
        .expect("events read")
        .into_iter()
        .find(|event| {
            Observation::decode(event.payload.payload.as_slice())
                .ok()
                .and_then(|observation| observation.payload)
                .is_some_and(|payload| payload.schema_ref == ADAPTER_REGISTRATION_SCHEMA)
        })
        .expect("canonical registration event");

    let mut replayed = AdapterRegistry::new();
    replayed
        .observe(&canonical)
        .expect("the actual attachment envelope replays");
    assert!(replayed
        .get(&AdapterId {
            value: "canonical".to_owned(),
        })
        .is_some());

    for mutation in [
        "event domain",
        "outer domain",
        "adapter target",
        "sender actor",
        "sender endpoint",
        "observation kind",
        "content type",
        "schema",
        "embedded adapter",
        "embedded domain",
        "embedded endpoint",
        "embedded generation",
        "embedded capability",
    ] {
        let mut candidate = canonical.clone();
        if mutation == "event domain" {
            candidate.event_id.authority_domain_id = Some(AuthorityDomainId {
                value: "authority-foreign".to_owned(),
            });
        } else {
            let mut observation =
                Observation::decode(candidate.payload.payload.as_slice()).expect("observation");
            match mutation {
                "outer domain" => {
                    observation.authority_domain_id = Some(AuthorityDomainId {
                        value: "authority-foreign".to_owned(),
                    });
                }
                "adapter target" => {
                    observation
                        .target_scope
                        .as_mut()
                        .expect("target")
                        .adapter_id = Some(AdapterId {
                        value: "adapter-foreign".to_owned(),
                    });
                }
                "sender actor" => {
                    observation.sender.as_mut().expect("sender").actor_id = Some(ActorId {
                        value: "actor-foreign".to_owned(),
                    });
                }
                "sender endpoint" => {
                    observation.sender.as_mut().expect("sender").endpoint_id = Some(EndpointId {
                        value: "endpoint-foreign".to_owned(),
                    });
                }
                "observation kind" => observation.kind = ObservationKind::Status as i32,
                "content type" => {
                    observation.payload.as_mut().expect("payload").content_type =
                        PayloadContentType::Json as i32;
                }
                "schema" => {
                    observation.payload.as_mut().expect("payload").schema_ref =
                        "example.not-registration".to_owned();
                }
                "embedded adapter"
                | "embedded domain"
                | "embedded endpoint"
                | "embedded generation"
                | "embedded capability" => {
                    let payload = observation.payload.as_mut().expect("payload");
                    let mut embedded = AdapterRegistration::decode(payload.payload.as_slice())
                        .expect("embedded registration");
                    match mutation {
                        "embedded adapter" => {
                            embedded.adapter_id = Some(AdapterId {
                                value: "adapter-foreign".to_owned(),
                            });
                        }
                        "embedded domain" => {
                            embedded.authority_domain_id = Some(AuthorityDomainId {
                                value: "authority-foreign".to_owned(),
                            });
                        }
                        "embedded endpoint" => {
                            embedded.endpoint_id = Some(EndpointId {
                                value: "endpoint-foreign".to_owned(),
                            });
                        }
                        "embedded generation" => embedded.adapter_generation = None,
                        "embedded capability" => embedded.capability = None,
                        _ => unreachable!(),
                    }
                    payload.payload = embedded.encode_to_vec();
                }
                _ => unreachable!(),
            }
            candidate.payload.payload = observation.encode_to_vec();
        }

        let mut registry = AdapterRegistry::new();
        let result = registry.observe(&candidate);
        if mutation == "schema" {
            result.expect("another schema is an unrelated Observation");
        } else {
            assert!(result.is_err(), "{mutation} must fail closed");
        }
        assert_eq!(
            registry,
            AdapterRegistry::new(),
            "{mutation} must not mutate adapter routing identity"
        );
    }
}

#[test]
fn complete_assurance_manifest_requires_explicit_false_and_known_non_sentinel_values() {
    let complete = session_capability();
    let validated =
        ValidatedAdapterCapability::try_from_wire(&complete, CapabilityValidationContext::Attach)
            .expect("complete explicit-false V1 validates");
    assert_eq!(
        validated.assurance().to_wire_v1(),
        assurance_v1(IdempotencyStrength::None)
    );

    for missing in [
        "continuation_proof_support",
        "cursor_support",
        "generation_fence_support",
    ] {
        let mut capability = session_capability();
        let Some(adapter_assurance_manifest::Contract::V1(manifest)) = capability
            .assurance
            .as_mut()
            .and_then(|assurance| assurance.contract.as_mut())
        else {
            unreachable!()
        };
        match missing {
            "continuation_proof_support" => manifest.continuation_proof_support = None,
            "cursor_support" => manifest.cursor_support = None,
            "generation_fence_support" => manifest.generation_fence_support = None,
            _ => unreachable!(),
        }
        assert!(
            ValidatedAdapterCapability::try_from_wire(
                &capability,
                CapabilityValidationContext::Attach,
            )
            .is_err(),
            "omitted {missing} must not be inferred as false"
        );
    }

    for (field, value) in [
        ("deduplication_strength", 0),
        ("reconciliation_strength", 0),
        ("unproven_outcome_action", 0),
        ("deduplication_strength", 99),
        ("reconciliation_strength", 99),
        ("unproven_outcome_action", 99),
    ] {
        let mut capability = session_capability();
        let Some(adapter_assurance_manifest::Contract::V1(manifest)) = capability
            .assurance
            .as_mut()
            .and_then(|assurance| assurance.contract.as_mut())
        else {
            unreachable!()
        };
        match field {
            "deduplication_strength" => manifest.deduplication_strength = value,
            "reconciliation_strength" => manifest.reconciliation_strength = value,
            "unproven_outcome_action" => manifest.unproven_outcome_action = value,
            _ => unreachable!(),
        }
        assert!(
            ValidatedAdapterCapability::try_from_wire(
                &capability,
                CapabilityValidationContext::Attach,
            )
            .is_err(),
            "{field} value {value} must fail closed"
        );
    }
}

#[test]
#[allow(deprecated)]
fn attach_rejects_missing_unknown_version_and_dual_declarations() {
    let mut missing = session_capability();
    missing.assurance = None;
    assert!(ValidatedAdapterCapability::try_from_wire(
        &missing,
        CapabilityValidationContext::Attach,
    )
    .is_err());

    let mut dual = session_capability();
    dual.idempotency_strength = IdempotencyStrength::AtPatchbayBoundary as i32;
    assert!(
        ValidatedAdapterCapability::try_from_wire(&dual, CapabilityValidationContext::Attach,)
            .is_err()
    );

    let mut future_encoded = missing.encode_to_vec();
    // AdapterCapability.assurance (tag 13) containing an unknown future
    // AdapterAssuranceManifest oneof branch (tag 2). Prost preserves the known
    // wrapper but cannot select an admitted contract, which must reject.
    future_encoded.extend_from_slice(&[0x6a, 0x02, 0x12, 0x00]);
    let future = AdapterCapability::decode(future_encoded.as_slice()).expect("future wire decodes");
    assert!(future.assurance.is_some());
    assert!(future
        .assurance
        .as_ref()
        .is_some_and(|manifest| manifest.contract.is_none()));
    for context in [
        CapabilityValidationContext::Attach,
        CapabilityValidationContext::Replay,
    ] {
        assert!(
            ValidatedAdapterCapability::try_from_wire(&future, context).is_err(),
            "unknown contract version must not become current V1 or legacy replay"
        );
    }
}

#[test]
fn current_v1_cannot_enter_legacy_category_normalization_on_replay() {
    let mut categoryless_v1 = session_capability();
    categoryless_v1.target_categories.clear();

    for context in [
        CapabilityValidationContext::Attach,
        CapabilityValidationContext::Replay,
    ] {
        assert!(
            ValidatedAdapterCapability::try_from_wire(&categoryless_v1, context).is_err(),
            "categoryless current V1 must reject in {context:?} rather than enter legacy normalization"
        );
    }

    let legacy = AdapterCapability::default();
    let replayed =
        ValidatedAdapterCapability::try_from_wire(&legacy, CapabilityValidationContext::Replay)
            .expect("genuinely pre-assurance categoryless registration remains replayable");
    assert!(replayed.targets(AdapterTargetCategory::RuntimeSession));
    assert!(
        ValidatedAdapterCapability::try_from_wire(&legacy, CapabilityValidationContext::Attach)
            .is_err(),
        "legacy category normalization remains replay-only"
    );
}

#[test]
#[allow(deprecated)]
fn replay_only_legacy_assurance_normalizes_conservatively() {
    for (wire, expected) in [
        (
            IdempotencyStrength::Unspecified as i32,
            IdempotencyStrength::None,
        ),
        (IdempotencyStrength::None as i32, IdempotencyStrength::None),
        (
            IdempotencyStrength::AtPatchbayBoundary as i32,
            IdempotencyStrength::AtPatchbayBoundary,
        ),
        (
            IdempotencyStrength::EndToEnd as i32,
            IdempotencyStrength::EndToEnd,
        ),
        (99, IdempotencyStrength::None),
    ] {
        let mut legacy = session_capability();
        legacy.assurance = None;
        legacy.idempotency_strength = wire;
        let replayed =
            ValidatedAdapterCapability::try_from_wire(&legacy, CapabilityValidationContext::Replay)
                .expect("historical v0.2 manifest normalizes on replay");
        assert_eq!(
            replayed.assurance().to_wire_v1(),
            assurance_v1(expected),
            "legacy deduplication value {wire} maps to one complete conservative V1"
        );
        assert!(
            ValidatedAdapterCapability::try_from_wire(
                &legacy,
                CapabilityValidationContext::Attach,
            )
            .is_err(),
            "the same historical bytes must not use replay normalization at attach"
        );
    }
}

#[test]
fn invalid_manifest_shapes_fail_closed() {
    let mut cases = Vec::new();

    cases.push(("missing categories", current_capability()));

    let mut unspecified_operation = session_capability();
    unspecified_operation.supported_operation_kinds = vec![OperationKind::Unspecified as i32];
    cases.push(("unspecified supported operation", unspecified_operation));

    let mut unknown_operation = session_capability();
    unknown_operation.supported_operation_kinds = vec![99];
    cases.push(("unknown supported operation", unknown_operation));

    let mut duplicate_operation = session_capability();
    duplicate_operation.supported_operation_kinds = vec![
        OperationKind::Instruct as i32,
        OperationKind::Instruct as i32,
    ];
    cases.push(("duplicate supported operation", duplicate_operation));

    let mut unspecified_failure = session_capability();
    unspecified_failure.known_failure_modes = vec![FailureCode::Unspecified as i32];
    cases.push(("unspecified known failure", unspecified_failure));

    let mut unknown_failure = session_capability();
    unknown_failure.known_failure_modes = vec![99];
    cases.push(("unknown known failure", unknown_failure));

    let mut duplicate_failure = session_capability();
    duplicate_failure.known_failure_modes = vec![
        FailureCode::ExecutionFailed as i32,
        FailureCode::ExecutionFailed as i32,
    ];
    cases.push(("duplicate known failure", duplicate_failure));

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
    let adapter_id = registration.adapter_id.clone().expect("adapter id");
    let endpoint_id = registration.endpoint_id.clone().expect("endpoint id");
    let observation = Observation {
        authority_domain_id: Some(domain.clone()),
        sender: Some(ActorEndpointRef {
            actor_id: Some(ActorId {
                value: adapter_id.value.clone(),
            }),
            endpoint_id: Some(endpoint_id),
            ..ActorEndpointRef::default()
        }),
        kind: ObservationKind::Event as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::Adapter as i32,
            adapter_id: Some(adapter_id),
            ..TargetScope::default()
        }),
        payload: Some(PayloadEnvelope {
            payload: registration.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: ADAPTER_REGISTRATION_SCHEMA.to_owned(),
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
