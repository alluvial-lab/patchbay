use patchbay_contracts::patchbay::{
    ActorEndpointRef, AdapterCapability, AdapterId, AdapterRegistration, AdapterSnapshotSupport,
    AuthorityDomainId, CommandId, CommandTransition, EventId, Generation, Observation,
    ObservationKind, Operation, OperationKind, OperationState, PayloadContentType,
    PayloadEnvelope, StoredEventKind, StoredEventPayload, TargetScope, TargetScopeKind,
};
use patchbay_core::diagnostics::{DiagnosticsProjection, DIAGNOSTICS_SCHEMA};
use patchbay_core::storage::RecordedEvent;
use prost::Message;

fn event(domain: &AuthorityDomainId, lsn: u64, kind: StoredEventKind, payload: Vec<u8>) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain.clone()),
            lsn: Some(patchbay_contracts::patchbay::Lsn { value: lsn }),
        },
        payload: StoredEventPayload { kind: kind as i32, payload },
    }
}

#[test]
fn replay_and_incremental_command_folds_match() {
    let domain = AuthorityDomainId { value: "main".to_owned() };
    let command_id = CommandId { value: "command-1".to_owned() };
    let operation = Operation {
        command_id: Some(command_id.clone()),
        authority_domain_id: Some(domain.clone()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(TargetScope { kind: TargetScopeKind::RuntimeSession as i32, ..TargetScope::default() }),
        ..Operation::default()
    };
    let transition = CommandTransition {
        command_id: Some(command_id.clone()),
        from_state: OperationState::Accepted as i32,
        to_state: OperationState::Delivered as i32,
        ..CommandTransition::default()
    };
    let events = vec![
        event(&domain, 1, StoredEventKind::Operation, operation.encode_to_vec()),
        event(&domain, 2, StoredEventKind::CommandTransition, transition.encode_to_vec()),
    ];
    let mut replay = DiagnosticsProjection::new();
    for item in &events {
        replay.observe(item).unwrap();
    }
    let mut incremental = DiagnosticsProjection::new();
    incremental.observe(&events[0]).unwrap();
    incremental.observe(&events[1]).unwrap();
    assert_eq!(replay.result_for_query(&command_id), incremental.result_for_query(&command_id));
    let history = replay.inspect_command(&command_id).unwrap().history;
    assert_eq!(history.len(), 2, "accepted and delivered are both real history entries");
}

#[test]
fn adapter_projection_redacts_descriptor_and_restart_is_unknown() {
    let domain = AuthorityDomainId { value: "main".to_owned() };
    let registration = AdapterRegistration {
        adapter_id: Some(AdapterId { value: "adapter-1".to_owned() }),
        endpoint_id: Some(patchbay_contracts::patchbay::EndpointId { value: "endpoint-1".to_owned() }),
        authority_domain_id: Some(domain.clone()),
        adapter_generation: Some(Generation { value: 3 }),
        capability: Some(AdapterCapability {
            snapshot_support: AdapterSnapshotSupport::Authoritative as i32,
            attachment_method: Some(patchbay_contracts::patchbay::AttachmentMethod {
                kind: "local".to_owned(),
                descriptor: b"sentinel-secret".to_vec(),
                descriptor_content_type: PayloadContentType::Binary as i32,
            }),
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
    let record = event(&domain, 1, StoredEventKind::Observation, observation.encode_to_vec());
    let mut projection = DiagnosticsProjection::new();
    projection.observe(&record).unwrap();
    let page = projection.adapter_page(&patchbay_contracts::patchbay::AdapterStatusQuery::default(), 1).unwrap();
    assert_eq!(page.adapters.len(), 1);
    assert_eq!(page.adapters[0].state, patchbay_contracts::patchbay::AdapterDiagnosticState::Attached as i32);
    assert_eq!(page.adapters[0].capability.as_ref().unwrap().attachment_method_kind, "local");
    projection.reset_adapter_liveness();
    let restarted = projection.adapter_page(&patchbay_contracts::patchbay::AdapterStatusQuery::default(), 1).unwrap();
    assert_eq!(restarted.adapters[0].state, patchbay_contracts::patchbay::AdapterDiagnosticState::Unknown as i32);
    assert!(!restarted.adapters[0].capability.as_ref().unwrap().attachment_method_kind.contains("sentinel-secret"));
}
