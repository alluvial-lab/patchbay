use std::collections::HashMap;
use std::future::ready;

use patchbay_contracts::patchbay::{
    typed_correlation, AuthorityDomainId, CommandId, CommandTransition, FailureCode, Lsn,
    Observation, ObservationKind, OperationState, StoredEventKind, TypedCorrelation,
};
use patchbay_core::acceptance::{
    ingest_observation, AcceptanceError, CommandSnapshot, CommandStateLookup, IngestResult,
};
use patchbay_core::storage::{RusqliteStorage, Storage};
use prost::Message;

#[derive(Default)]
struct TestCommandStates {
    states: HashMap<CommandId, OperationState>,
}

impl TestCommandStates {
    fn with(command_id: CommandId, state: OperationState) -> Self {
        Self {
            states: HashMap::from([(command_id, state)]),
        }
    }
}

impl CommandStateLookup for TestCommandStates {
    fn current_state(
        &self,
        command_id: &CommandId,
    ) -> impl std::future::Future<Output = Option<CommandSnapshot>> + Send {
        ready(self.states.get(command_id).map(|state| CommandSnapshot {
            state: *state,
            correlations: vec![],
            terminal_lsn: None,
        }))
    }
}

fn authority_domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "test-domain".to_owned(),
    }
}

fn command_id() -> CommandId {
    CommandId {
        value: "command-1".to_owned(),
    }
}

fn command_correlation(command_id: CommandId) -> TypedCorrelation {
    TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::CommandId(command_id)),
    }
}

fn observation(kind: ObservationKind, failure_code: FailureCode) -> Observation {
    Observation {
        event_id: None,
        reply_id: None,
        authority_domain_id: Some(authority_domain()),
        sender: None,
        recipient: None,
        kind: kind as i32,
        correlations: vec![command_correlation(command_id())],
        target_scope: None,
        payload: None,
        lsn: None,
        observed_at: None,
        failure_code: failure_code as i32,
    }
}

async fn events(storage: &RusqliteStorage) -> Vec<patchbay_core::storage::RecordedEvent> {
    storage
        .read_after(&authority_domain(), Lsn { value: 0 })
        .await
        .unwrap()
}

fn decode_transition(event: &patchbay_core::storage::RecordedEvent) -> CommandTransition {
    assert_eq!(
        StoredEventKind::try_from(event.payload.kind).unwrap(),
        StoredEventKind::CommandTransition
    );
    CommandTransition::decode(event.payload.payload.as_slice()).unwrap()
}

#[tokio::test]
async fn every_observation_kind_is_recorded_as_observation() {
    for kind in [
        ObservationKind::Unspecified,
        ObservationKind::Event,
        ObservationKind::Status,
        ObservationKind::Delta,
        ObservationKind::Result,
    ] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let states = TestCommandStates::with(command_id(), OperationState::Delivered);
        let submitted = observation(kind, FailureCode::Unspecified);

        ingest_observation(&storage, &states, submitted.clone())
            .await
            .unwrap();

        let recorded = events(&storage).await;
        assert!(!recorded.is_empty(), "{kind:?} was not recorded");
        assert_eq!(
            StoredEventKind::try_from(recorded[0].payload.kind).unwrap(),
            StoredEventKind::Observation
        );
        assert_eq!(
            Observation::decode(recorded[0].payload.payload.as_slice()).unwrap(),
            submitted
        );
    }
}

#[tokio::test]
async fn result_without_failure_emits_completed_transition() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Delivered);

    let result = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Result, FailureCode::Unspecified),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        IngestResult::Transitioned {
            to_state: OperationState::Completed,
            ..
        }
    ));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 2);
    let transition = decode_transition(&recorded[1]);
    assert_eq!(transition.command_id, Some(command_id()));
    assert_eq!(transition.from_state, OperationState::Delivered as i32);
    assert_eq!(transition.to_state, OperationState::Completed as i32);
    assert_eq!(transition.failure_code, FailureCode::Unspecified as i32);
}

#[tokio::test]
async fn execution_failed_result_emits_failed_transition() {
    assert_failed_result_preserves_code(FailureCode::ExecutionFailed).await;
}

#[tokio::test]
async fn outcome_unknown_result_emits_failed_transition_with_ambiguity_signal() {
    assert_failed_result_preserves_code(FailureCode::ExecutionOutcomeUnknown).await;
}

#[tokio::test]
async fn other_result_failure_codes_still_emit_failed_transition() {
    assert_failed_result_preserves_code(FailureCode::AdapterUnavailable).await;
}

async fn assert_failed_result_preserves_code(failure_code: FailureCode) {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Running);

    let result = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Result, failure_code),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        IngestResult::Transitioned {
            to_state: OperationState::Failed,
            ..
        }
    ));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 2);
    let transition = decode_transition(&recorded[1]);
    assert_eq!(transition.from_state, OperationState::Running as i32);
    assert_eq!(transition.to_state, OperationState::Failed as i32);
    assert_eq!(transition.failure_code, failure_code as i32);
}

#[tokio::test]
async fn status_emits_running_transition() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Delivered);

    let result = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Status, FailureCode::Unspecified),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        IngestResult::Transitioned {
            to_state: OperationState::Running,
            ..
        }
    ));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 2);
    let transition = decode_transition(&recorded[1]);
    assert_eq!(transition.from_state, OperationState::Delivered as i32);
    assert_eq!(transition.to_state, OperationState::Running as i32);
    assert_eq!(transition.failure_code, FailureCode::Unspecified as i32);
}

#[tokio::test]
async fn accepted_to_running_status_is_rejected_without_corrupt_transition() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Accepted);

    let error = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Status, FailureCode::Unspecified),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, AcceptanceError::CorruptRecord(_)));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(recorded[0].payload.kind).unwrap(),
        StoredEventKind::Observation
    );
}

#[tokio::test]
async fn repeated_running_status_records_without_duplicate_transition() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Running);

    let result = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Status, FailureCode::Unspecified),
    )
    .await
    .unwrap();

    assert!(matches!(result, IngestResult::Recorded { .. }));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(recorded[0].payload.kind).unwrap(),
        StoredEventKind::Observation
    );
}

#[tokio::test]
async fn late_terminal_candidate_is_audit_only() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::with(command_id(), OperationState::Completed);

    let result = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Result, FailureCode::ExecutionFailed),
    )
    .await
    .unwrap();

    assert!(matches!(result, IngestResult::StaleCandidate { .. }));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(recorded[0].payload.kind).unwrap(),
        StoredEventKind::Observation
    );
}

#[tokio::test]
async fn event_and_delta_record_without_transition() {
    for kind in [ObservationKind::Event, ObservationKind::Delta] {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let states = TestCommandStates::default();

        let result = ingest_observation(
            &storage,
            &states,
            observation(kind, FailureCode::Unspecified),
        )
        .await
        .unwrap();

        assert!(matches!(result, IngestResult::Recorded { .. }));
        assert_eq!(events(&storage).await.len(), 1);
    }
}

#[tokio::test]
async fn missing_authority_domain_fails_before_recording() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::default();
    let mut submitted = observation(ObservationKind::Event, FailureCode::Unspecified);
    submitted.authority_domain_id = None;

    let error = ingest_observation(&storage, &states, submitted)
        .await
        .unwrap_err();

    assert!(matches!(error, AcceptanceError::CorruptRecord(_)));
    assert!(events(&storage).await.is_empty());
}

#[tokio::test]
async fn transition_for_unknown_command_fails_after_recording_evidence() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let states = TestCommandStates::default();

    let error = ingest_observation(
        &storage,
        &states,
        observation(ObservationKind::Result, FailureCode::Unspecified),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, AcceptanceError::CorruptRecord(_)));
    let recorded = events(&storage).await;
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        StoredEventKind::try_from(recorded[0].payload.kind).unwrap(),
        StoredEventKind::Observation
    );
}

#[tokio::test]
async fn transition_carries_command_elicitation_correlation() {
    // Blocker 5 fix: a derived CommandTransition must carry the originating
    // Operation's correlations (e.g. ElicitationId for response Operations)
    // so the Elicitation-slot layer can correlate a response terminal
    // transition back to its Elicitation.
    use patchbay_contracts::patchbay::{
        typed_correlation, CommandTransition, ElicitationId, TypedCorrelation,
    };

    let _elicitation_corr = TypedCorrelation {
        r#ref: Some(typed_correlation::Ref::ElicitationId(ElicitationId {
            value: "elicitation-from-op".to_owned(),
        })),
    };

    struct LookupWithCorrelation;
    impl CommandStateLookup for LookupWithCorrelation {
        async fn current_state(&self, _command_id: &CommandId) -> Option<CommandSnapshot> {
            Some(CommandSnapshot {
                state: OperationState::Delivered,
                correlations: vec![TypedCorrelation {
                    r#ref: Some(typed_correlation::Ref::ElicitationId(ElicitationId {
                        value: "elicitation-from-op".to_owned(),
                    })),
                }],
                terminal_lsn: None,
            })
        }
    }

    let storage = RusqliteStorage::open_in_memory().unwrap();
    let obs = observation(ObservationKind::Result, FailureCode::Unspecified);
    let result = ingest_observation(&storage, &LookupWithCorrelation, obs)
        .await
        .unwrap();

    let IngestResult::Transitioned { .. } = result else {
        panic!("expected a transition");
    };

    let events = storage
        .read_after(&authority_domain(), Lsn { value: 0 })
        .await
        .unwrap();
    let transition_event = events
        .iter()
        .find(|e| {
            StoredEventKind::try_from(e.payload.kind).unwrap() == StoredEventKind::CommandTransition
        })
        .expect("a CommandTransition event was emitted");

    let transition: CommandTransition =
        prost::Message::decode(transition_event.payload.payload.as_slice()).unwrap();
    assert!(
        transition.correlations.iter().any(|c| matches!(
            c.r#ref.as_ref(),
            Some(typed_correlation::Ref::ElicitationId(id)) if id.value == "elicitation-from-op"
        )),
        "the derived transition must carry the command's ElicitationId correlation"
    );
}
