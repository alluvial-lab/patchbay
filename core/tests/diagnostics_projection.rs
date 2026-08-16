use patchbay_contracts::patchbay::{
    AcceptedOperation, ActorEndpointRef, AdapterCapability, AdapterDiagnosticDetail,
    AdapterDiagnosticSeverity, AdapterDiagnosticState, AdapterId, AdapterRegistration,
    AdapterSnapshotSupport, AdapterTargetCategory, AuditEventKind, AuditRecord, AuthorityDomainId,
    CommandId, CommandTransition, EventId, FailureCode, Generation, GrantId, GrantRevocationEffect,
    Observation, ObservationKind, Operation, OperationKind, OperationState, PayloadContentType,
    PayloadEnvelope, ResourceCapability, ResourceKind, ResourceProjectionContract, Revocation,
    RuntimeSessionId, SchemaDescriptor, SecurityLockdownEntered, SecurityLockdownExited,
    SessionActivityState, SessionConnectivityChanged, SessionConnectivityState, SessionRegistered,
    SessionState, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    acceptance::CommandIndex,
    diagnostics::{DiagnosticsError, DiagnosticsProjection, DIAGNOSTICS_SCHEMA},
    security::events as security_events,
    session::events as session_events,
    storage::{validate_next_replay_event, RecordedEvent},
};
use prost::Message;

fn event(
    domain: &AuthorityDomainId,
    lsn: u64,
    kind: StoredEventKind,
    payload: Vec<u8>,
) -> RecordedEvent {
    stored_event(
        domain,
        lsn,
        StoredEventPayload {
            kind: kind as i32,
            payload,
        },
    )
}

fn stored_event(
    domain: &AuthorityDomainId,
    lsn: u64,
    payload: StoredEventPayload,
) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain.clone()),
            lsn: Some(patchbay_contracts::patchbay::Lsn { value: lsn }),
        },
        payload,
    }
}

fn lifecycle_event(
    domain: &AuthorityDomainId,
    lsn: u64,
    adapter_id: &str,
    kind: AuditEventKind,
) -> RecordedEvent {
    event(
        domain,
        lsn,
        StoredEventKind::AuditRecord,
        AuditRecord {
            kind: kind as i32,
            actor_id: Some(patchbay_contracts::patchbay::ActorId {
                value: adapter_id.to_owned(),
            }),
            reason_code: format!("adapter_{:?}", kind).to_lowercase(),
            ..AuditRecord::default()
        }
        .encode_to_vec(),
    )
}

fn registration_event(domain: &AuthorityDomainId, lsn: u64, adapter_id: &str) -> RecordedEvent {
    let registration = AdapterRegistration {
        adapter_id: Some(AdapterId {
            value: adapter_id.to_owned(),
        }),
        endpoint_id: Some(patchbay_contracts::patchbay::EndpointId {
            value: format!("{adapter_id}-endpoint"),
        }),
        authority_domain_id: Some(domain.clone()),
        adapter_generation: Some(Generation { value: 1 }),
        ..AdapterRegistration::default()
    };
    event(
        domain,
        lsn,
        StoredEventKind::Observation,
        Observation {
            authority_domain_id: Some(domain.clone()),
            kind: ObservationKind::Event as i32,
            payload: Some(PayloadEnvelope {
                payload: registration.encode_to_vec(),
                content_type: PayloadContentType::Protobuf as i32,
                schema_ref: "patchbay.AdapterRegistration".to_owned(),
            }),
            ..Observation::default()
        }
        .encode_to_vec(),
    )
}

fn session_lockdown_recovery_events(domain: &AuthorityDomainId) -> Vec<RecordedEvent> {
    let adapter_id = AdapterId {
        value: "adapter-1".to_owned(),
    };
    let runtime_session_id = RuntimeSessionId {
        value: "runtime-1".to_owned(),
    };
    let session_generation = Generation { value: 1 };

    vec![
        registration_event(domain, 1, &adapter_id.value),
        stored_event(
            domain,
            2,
            session_events::encode(&session_events::registered(
                domain.clone(),
                SessionRegistered {
                    adapter_id: Some(adapter_id.clone()),
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: Some(runtime_session_id.clone()),
                    session_generation: Some(session_generation),
                    initial_state: Some(SessionState {
                        connectivity: SessionConnectivityState::Live as i32,
                        activity: SessionActivityState::Idle as i32,
                    }),
                    ..SessionRegistered::default()
                },
            )),
        ),
        stored_event(
            domain,
            3,
            security_events::encode(&security_events::entered(
                domain.clone(),
                SecurityLockdownEntered {
                    reason_code: "diagnostics_replay".to_owned(),
                    affected_runtime_session_count: 1,
                    ..SecurityLockdownEntered::default()
                },
            )),
        ),
        stored_event(
            domain,
            4,
            security_events::encode(&security_events::exited(
                domain.clone(),
                SecurityLockdownExited {
                    reason_code: "operator_recovery".to_owned(),
                    ..SecurityLockdownExited::default()
                },
            )),
        ),
        stored_event(
            domain,
            5,
            session_events::encode(&session_events::connectivity_changed(
                domain.clone(),
                SessionConnectivityChanged {
                    adapter_id: Some(adapter_id),
                    deployment_scope: "machine-a".to_owned(),
                    runtime_session_id: Some(runtime_session_id),
                    session_generation: Some(session_generation),
                    from: SessionConnectivityState::Stale as i32,
                    to: SessionConnectivityState::Live as i32,
                },
            )),
        ),
    ]
}

fn adapter_session_counts(projection: &DiagnosticsProjection, as_of: u64) -> [u32; 4] {
    let page = projection
        .adapter_page(&Default::default(), as_of)
        .expect("adapter diagnostics page remains queryable");
    let status = page
        .adapters
        .first()
        .expect("the durable adapter registration is projected");
    [
        status.live_session_count,
        status.stale_session_count,
        status.offline_session_count,
        status.failed_session_count,
    ]
}

fn diagnostic_audit_event(
    domain: &AuthorityDomainId,
    lsn: u64,
    source_lsn: u64,
    adapter_id: &str,
    code: &str,
    severity: AdapterDiagnosticSeverity,
) -> RecordedEvent {
    event(
        domain,
        lsn,
        StoredEventKind::AuditRecord,
        AuditRecord {
            audit_event_id: Some(EventId {
                authority_domain_id: Some(domain.clone()),
                lsn: Some(patchbay_contracts::patchbay::Lsn { value: lsn }),
            }),
            kind: AuditEventKind::AdapterDiagnosticReported as i32,
            actor_id: Some(patchbay_contracts::patchbay::ActorId {
                value: adapter_id.to_owned(),
            }),
            reason_code: code.to_owned(),
            source_event_id: Some(EventId {
                authority_domain_id: Some(domain.clone()),
                lsn: Some(patchbay_contracts::patchbay::Lsn { value: source_lsn }),
            }),
            adapter_diagnostic: Some(AdapterDiagnosticDetail {
                adapter_id: Some(AdapterId {
                    value: adapter_id.to_owned(),
                }),
                adapter_generation: Some(Generation { value: 1 }),
                severity: severity as i32,
                operation_kind: OperationKind::Unspecified as i32,
                count: 1,
                ..AdapterDiagnosticDetail::default()
            }),
            failure_code: FailureCode::Unspecified as i32,
            ..AuditRecord::default()
        }
        .encode_to_vec(),
    )
}

#[test]
fn replay_and_incremental_command_folds_match() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let command_id = CommandId {
        value: "command-1".to_owned(),
    };
    let operation = Operation {
        command_id: Some(command_id.clone()),
        authority_domain_id: Some(domain.clone()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope {
            kind: TargetScopeKind::RuntimeSession as i32,
            ..TargetScope::default()
        }),
        idempotency_key: "command-1-key".to_owned(),
        ..Operation::default()
    };
    let transition = CommandTransition {
        command_id: Some(command_id.clone()),
        from_state: OperationState::Accepted as i32,
        to_state: OperationState::Delivered as i32,
        ..CommandTransition::default()
    };
    let events = vec![
        event(
            &domain,
            1,
            StoredEventKind::Operation,
            AcceptedOperation {
                operation: Some(operation),
                authorizing_grant_id: Some(patchbay_contracts::patchbay::GrantId {
                    value: "test-grant".to_owned(),
                }),
            }
            .encode_to_vec(),
        ),
        event(
            &domain,
            2,
            StoredEventKind::CommandTransition,
            transition.encode_to_vec(),
        ),
    ];
    let mut replay = DiagnosticsProjection::new(domain.clone()).unwrap();
    for item in &events {
        replay.observe(item).unwrap();
    }
    let mut incremental = DiagnosticsProjection::new(domain.clone()).unwrap();
    incremental.observe(&events[0]).unwrap();
    incremental.observe(&events[1]).unwrap();
    assert_eq!(
        replay.result_for_query(&command_id),
        incremental.result_for_query(&command_id)
    );
    let history = replay.inspect_command(&command_id).unwrap().history;
    assert_eq!(
        history.len(),
        2,
        "accepted and delivered are both real history entries"
    );
}

#[test]
fn incremental_lockdown_replay_updates_adapter_session_counts_and_stays_exact() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let events = session_lockdown_recovery_events(&domain);
    let mut projection = DiagnosticsProjection::new(domain).unwrap();

    projection.observe(&events[0]).unwrap();
    projection.observe(&events[1]).unwrap();
    assert_eq!(adapter_session_counts(&projection, 2), [1, 0, 0, 0]);

    projection.observe(&events[2]).unwrap();
    assert_eq!(adapter_session_counts(&projection, 3), [0, 1, 0, 0]);

    projection.observe(&events[3]).unwrap();
    assert_eq!(adapter_session_counts(&projection, 4), [0, 1, 0, 0]);

    projection.observe(&events[4]).unwrap();
    assert_eq!(adapter_session_counts(&projection, 5), [1, 0, 0, 0]);

    let before_redelivery = projection.clone();
    for event in &events[1..] {
        projection
            .observe(event)
            .expect("exact owned-event redelivery remains inert");
    }
    assert_eq!(projection, before_redelivery);
}

#[test]
fn cold_restart_lockdown_prefixes_replay_without_failure_or_count_drift() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let events = session_lockdown_recovery_events(&domain);

    for (prefix_len, expected_counts) in [
        (2, [1, 0, 0, 0]),
        (3, [0, 1, 0, 0]),
        (4, [0, 1, 0, 0]),
        (5, [1, 0, 0, 0]),
    ] {
        let mut restarted = DiagnosticsProjection::new(domain.clone()).unwrap();
        let mut previous_lsn = 0;
        for event in &events[..prefix_len] {
            previous_lsn = validate_next_replay_event(&domain, previous_lsn, event)
                .expect("cold replay keeps the exact gap-free prefix contract")
                .lsn;
            restarted
                .observe(event)
                .expect("cold diagnostics replay accepts every durable event");
        }
        assert_eq!(previous_lsn, prefix_len as u64);
        restarted.reset_adapter_liveness();
        assert_eq!(
            adapter_session_counts(&restarted, prefix_len as u64),
            expected_counts,
            "cold projection at prefix LSN {prefix_len}"
        );
    }
}

#[test]
fn concrete_sibling_redelivery_cannot_bypass_the_session_envelope_ledger() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let events = session_lockdown_recovery_events(&domain);
    let mut projection = DiagnosticsProjection::new(domain).unwrap();
    projection.observe(&events[0]).unwrap();
    projection.observe(&events[1]).unwrap();

    let before = projection.clone();
    let mut conflicting_sibling = events[1].clone();
    conflicting_sibling.payload.kind = StoredEventKind::ResourceState as i32;
    let error = projection
        .observe(&conflicting_sibling)
        .expect_err("a concrete sibling kind cannot reuse an applied session event identity");
    assert!(matches!(
        error,
        DiagnosticsError::CorruptEvent(message)
            if message.contains("conflicting durable envelopes")
    ));
    assert_eq!(projection, before);
}

#[test]
fn multi_effect_revocation_is_atomic_in_command_and_diagnostics_projections() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let grant_id = GrantId {
        value: "test-grant".to_owned(),
    };
    let operation_event = |lsn: u64, command: &str| {
        event(
            &domain,
            lsn,
            StoredEventKind::Operation,
            AcceptedOperation {
                operation: Some(Operation {
                    command_id: Some(CommandId {
                        value: command.to_owned(),
                    }),
                    authority_domain_id: Some(domain.clone()),
                    kind: OperationKind::Instruct as i32,
                    target_scope: Some(TargetScope {
                        kind: TargetScopeKind::RuntimeSession as i32,
                        ..TargetScope::default()
                    }),
                    idempotency_key: format!("key-{command}"),
                    ..Operation::default()
                }),
                authorizing_grant_id: Some(grant_id.clone()),
            }
            .encode_to_vec(),
        )
    };
    let first = operation_event(1, "command-1");
    let second = operation_event(2, "command-2");
    let revocation = event(
        &domain,
        3,
        StoredEventKind::Revocation,
        Revocation {
            authority_domain_id: Some(domain.clone()),
            grant_id: Some(grant_id),
            revocation_generation: Some(Generation { value: 1 }),
            command_effects: vec![
                GrantRevocationEffect {
                    command_id: Some(CommandId {
                        value: "command-1".to_owned(),
                    }),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Cancelled as i32,
                    failure_code: FailureCode::Cancelled as i32,
                },
                GrantRevocationEffect {
                    command_id: Some(CommandId {
                        value: "missing-command".to_owned(),
                    }),
                    from_state: OperationState::Accepted as i32,
                    to_state: OperationState::Cancelled as i32,
                    failure_code: FailureCode::Cancelled as i32,
                },
            ],
            ..Revocation::default()
        }
        .encode_to_vec(),
    );

    let mut commands = CommandIndex::new();
    let mut diagnostics = DiagnosticsProjection::new(domain.clone()).unwrap();
    for item in [&first, &second] {
        commands.apply(item).expect("accepted command projects");
        diagnostics
            .observe(item)
            .expect("accepted diagnostic timeline projects");
    }
    let commands_before = commands.clone();
    let diagnostics_before = diagnostics.clone();

    assert!(commands.apply(&revocation).is_err());
    assert!(diagnostics.observe(&revocation).is_err());
    assert_eq!(commands, commands_before);
    assert_eq!(diagnostics, diagnostics_before);
}

#[test]
fn adapter_projection_redacts_descriptor_and_restart_is_unknown() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let registration = AdapterRegistration {
        adapter_id: Some(AdapterId {
            value: "adapter-1".to_owned(),
        }),
        endpoint_id: Some(patchbay_contracts::patchbay::EndpointId {
            value: "endpoint-1".to_owned(),
        }),
        authority_domain_id: Some(domain.clone()),
        adapter_generation: Some(Generation { value: 3 }),
        capability: Some(AdapterCapability {
            session_snapshot_support: AdapterSnapshotSupport::Authoritative as i32,
            attachment_method: Some(patchbay_contracts::patchbay::AttachmentMethod {
                kind: "local".to_owned(),
                descriptor: b"sentinel-secret".to_vec(),
                descriptor_content_type: PayloadContentType::Binary as i32,
            }),
            target_categories: vec![
                AdapterTargetCategory::RuntimeSession as i32,
                AdapterTargetCategory::OperationalResource as i32,
            ],
            resource_capabilities: vec![ResourceCapability {
                resource_kind: Some(ResourceKind {
                    value: "provider_pool".to_owned(),
                }),
                snapshot_support: AdapterSnapshotSupport::Partial as i32,
                projection_contract: Some(ResourceProjectionContract {
                    target_category: AdapterTargetCategory::OperationalResource as i32,
                    payload_schema: Some(SchemaDescriptor {
                        schema_ref: "pool.payload.v1".to_owned(),
                        content_type: PayloadContentType::Protobuf as i32,
                    }),
                    projection_schema: Some(SchemaDescriptor {
                        schema_ref: "pool.projection.v1".to_owned(),
                        content_type: PayloadContentType::Json as i32,
                    }),
                }),
            }],
            ..AdapterCapability::default()
        }),
        ..AdapterRegistration::default()
    };
    let observation = Observation {
        authority_domain_id: Some(domain.clone()),
        kind: ObservationKind::Event as i32,
        payload: Some(PayloadEnvelope {
            payload: registration.encode_to_vec(),
            content_type: PayloadContentType::Protobuf as i32,
            schema_ref: DIAGNOSTICS_SCHEMA.replace("DiagnosticsQuery", "AdapterRegistration"),
        }),
        ..Observation::default()
    };
    let record = event(
        &domain,
        1,
        StoredEventKind::Observation,
        observation.encode_to_vec(),
    );
    let mut projection = DiagnosticsProjection::new(domain.clone()).unwrap();
    projection.observe(&record).unwrap();
    let page = projection
        .adapter_page(
            &patchbay_contracts::patchbay::AdapterStatusQuery::default(),
            1,
        )
        .unwrap();
    assert_eq!(page.adapters.len(), 1);
    assert_eq!(
        page.adapters[0].state,
        AdapterDiagnosticState::Attached as i32
    );
    let summary = page.adapters[0].capability.as_ref().unwrap();
    assert_eq!(summary.attachment_method_kind, "local");
    assert_eq!(
        summary.session_snapshot_support,
        AdapterSnapshotSupport::Authoritative as i32
    );
    assert_eq!(
        summary.target_categories,
        [
            AdapterTargetCategory::RuntimeSession as i32,
            AdapterTargetCategory::OperationalResource as i32,
        ]
    );
    assert_eq!(summary.resource_capabilities.len(), 1);
    assert_eq!(
        summary.resource_capabilities[0]
            .resource_kind
            .as_ref()
            .unwrap()
            .value,
        "provider_pool"
    );
    assert_eq!(
        summary.resource_capabilities[0].snapshot_support,
        AdapterSnapshotSupport::Partial as i32
    );
    assert_eq!(
        summary.resource_capabilities[0]
            .projection_contract
            .as_ref()
            .unwrap()
            .projection_schema
            .as_ref()
            .unwrap()
            .schema_ref,
        "pool.projection.v1"
    );
    projection.reset_adapter_liveness();
    let restarted = projection
        .adapter_page(
            &patchbay_contracts::patchbay::AdapterStatusQuery::default(),
            1,
        )
        .unwrap();
    assert_eq!(
        restarted.adapters[0].state,
        AdapterDiagnosticState::Unknown as i32
    );
    assert!(!restarted.adapters[0]
        .capability
        .as_ref()
        .unwrap()
        .attachment_method_kind
        .contains("sentinel-secret"));
}

#[test]
fn fresh_detach_and_failure_replace_attached_projection_state() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    for (lsn, kind, expected) in [
        (
            2,
            AuditEventKind::AdapterDetached,
            AdapterDiagnosticState::Detached,
        ),
        (
            2,
            AuditEventKind::AdapterFailed,
            AdapterDiagnosticState::Failed,
        ),
    ] {
        let mut projection = DiagnosticsProjection::new(domain.clone()).unwrap();
        projection
            .observe(&registration_event(&domain, 1, "adapter-1"))
            .unwrap();
        assert_eq!(
            projection
                .adapter_page(&Default::default(), lsn - 1)
                .unwrap()
                .adapters[0]
                .state,
            AdapterDiagnosticState::Attached as i32
        );
        projection
            .observe(&lifecycle_event(&domain, lsn, "adapter-1", kind))
            .unwrap();
        assert_eq!(
            projection
                .adapter_page(&Default::default(), lsn)
                .unwrap()
                .adapters[0]
                .state,
            expected as i32
        );
    }
}

#[test]
fn historical_lifecycle_does_not_establish_state_after_restart_but_fresh_attach_does() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let mut projection = DiagnosticsProjection::new(domain.clone()).unwrap();
    projection
        .observe(&registration_event(&domain, 1, "adapter-1"))
        .unwrap();
    projection
        .observe(&lifecycle_event(
            &domain,
            2,
            "adapter-1",
            AuditEventKind::AdapterDetached,
        ))
        .unwrap();
    projection.reset_adapter_liveness();
    assert_eq!(
        projection
            .adapter_page(&Default::default(), 2)
            .unwrap()
            .adapters[0]
            .state,
        AdapterDiagnosticState::Unknown as i32
    );

    projection
        .observe(&registration_event(&domain, 3, "adapter-1"))
        .unwrap();
    projection
        .observe(&lifecycle_event(
            &domain,
            4,
            "adapter-1",
            AuditEventKind::AdapterFailed,
        ))
        .unwrap();
    assert_eq!(
        projection
            .adapter_page(&Default::default(), 4)
            .unwrap()
            .adapters[0]
            .state,
        AdapterDiagnosticState::Failed as i32
    );
}

#[test]
fn recent_diagnostics_are_replayed_newest_first_and_bounded_by_query() {
    let domain = AuthorityDomainId {
        value: "main".to_owned(),
    };
    let mut projection = DiagnosticsProjection::new(domain.clone()).unwrap();
    projection
        .observe(&registration_event(&domain, 1, "adapter-1"))
        .unwrap();
    // The source observations are intentionally minimal: the projection only
    // needs their prior EventIds to validate the correlated audit records.
    for (source_lsn, audit_lsn, code, severity) in [
        (2, 3, "pi_old", AdapterDiagnosticSeverity::Info),
        (4, 5, "pi_new", AdapterDiagnosticSeverity::Warning),
    ] {
        projection
            .observe(&event(
                &domain,
                source_lsn,
                StoredEventKind::Observation,
                Vec::new(),
            ))
            .unwrap();
        projection
            .observe(&diagnostic_audit_event(
                &domain,
                audit_lsn,
                source_lsn,
                "adapter-1",
                code,
                severity,
            ))
            .unwrap();
    }
    let page = projection
        .adapter_page(
            &patchbay_contracts::patchbay::AdapterStatusQuery {
                adapter_ids: vec![AdapterId {
                    value: "adapter-1".to_owned(),
                }],
                recent_diagnostic_limit: Some(1),
                ..Default::default()
            },
            5,
        )
        .unwrap();
    assert_eq!(page.adapters[0].recent_diagnostics.len(), 1);
    assert_eq!(page.adapters[0].recent_diagnostics[0].reason_code, "pi_new");

    let all = projection
        .adapter_page(
            &patchbay_contracts::patchbay::AdapterStatusQuery {
                adapter_ids: vec![AdapterId {
                    value: "adapter-1".to_owned(),
                }],
                recent_diagnostic_limit: Some(2),
                ..Default::default()
            },
            5,
        )
        .unwrap();
    assert_eq!(
        all.adapters[0]
            .recent_diagnostics
            .iter()
            .map(|record| record.reason_code.as_str())
            .collect::<Vec<_>>(),
        ["pi_new", "pi_old"]
    );
}
