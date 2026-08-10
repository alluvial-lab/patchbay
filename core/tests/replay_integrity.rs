use patchbay_contracts::patchbay::{
    AuthorityDomainId, EventId, IdempotencyKey, Lsn, StoredEventKind, StoredEventPayload,
};
use patchbay_core::{
    acceptance::{
        rebuild_from_log as rebuild_commands, rebuild_slots_from_log, AcceptanceError,
        CommandIndex, ElicitationSlotLayer,
    },
    adapter::{rebuild_from_log as rebuild_adapters, AdapterError, AdapterRegistry},
    authority::{
        rebuild_from_log as rebuild_authority, rebuild_operator_registry, AuthorityError,
        AuthorityRegistry, OperatorError, OperatorRegistry,
    },
    diagnostics::{DiagnosticsError, DiagnosticsProjection},
    resource::{rebuild_from_log as rebuild_resources, ResourceError, ResourceRegistry},
    security::{rebuild_from_log as rebuild_security, SecurityError, SecurityPostureProjection},
    session::{rebuild_from_log as rebuild_sessions, SessionError, SessionRegistry},
    storage::{
        validate_next_replay_event, DedupOutcome, RecordedEvent, ReplayIntegrityError, Storage,
        StorageError, StoredSnapshot, TargetKey,
    },
};
use proptest::prelude::*;

fn domain(value: &str) -> AuthorityDomainId {
    AuthorityDomainId {
        value: value.to_owned(),
    }
}

fn event(lsn: u64, kind: StoredEventKind) -> RecordedEvent {
    RecordedEvent {
        event_id: EventId {
            authority_domain_id: Some(domain("authority-main")),
            lsn: Some(Lsn { value: lsn }),
        },
        payload: StoredEventPayload {
            kind: kind as i32,
            payload: Vec::new(),
        },
    }
}

#[derive(Clone)]
struct ScriptedReplayStorage {
    events: Vec<RecordedEvent>,
}

impl ScriptedReplayStorage {
    fn new(events: Vec<RecordedEvent>) -> Self {
        Self { events }
    }
}

impl Storage for ScriptedReplayStorage {
    async fn append(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _payload: StoredEventPayload,
    ) -> Result<EventId, StorageError> {
        Err(StorageError::UnsupportedOperation)
    }

    async fn append_dedup(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _key: &IdempotencyKey,
        _target: &TargetKey,
        _payload: StoredEventPayload,
    ) -> Result<DedupOutcome, StorageError> {
        Err(StorageError::UnsupportedOperation)
    }

    async fn read_after(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        // A deliberately faulty port: replay must independently reject the
        // caller-supplied sequence instead of trusting append-side guarantees.
        Ok(self.events.clone())
    }

    async fn write_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _snapshot_lsn: Lsn,
        _snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
        Err(StorageError::UnsupportedOperation)
    }

    async fn load_latest_snapshot(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        _at_or_before: Option<Lsn>,
    ) -> Result<Option<StoredSnapshot>, StorageError> {
        Ok(None)
    }
}

fn validate_sequence(
    previous_lsn: u64,
    events: &[RecordedEvent],
) -> Result<u64, ReplayIntegrityError> {
    let expected_domain = domain("authority-main");
    events.iter().try_fold(previous_lsn, |cursor, event| {
        validate_next_replay_event(&expected_domain, cursor, event).map(|validated| validated.lsn)
    })
}

#[test]
fn complete_prefix_validator_accepts_cold_and_snapshot_tail_sequences() {
    assert_eq!(validate_sequence(0, &[]).unwrap(), 0);
    assert_eq!(
        validate_sequence(
            0,
            &[
                event(1, StoredEventKind::Grant),
                event(2, StoredEventKind::ResourceState),
                event(3, StoredEventKind::Observation),
            ],
        )
        .unwrap(),
        3
    );
    assert_eq!(
        validate_sequence(
            5,
            &[
                event(6, StoredEventKind::Grant),
                event(7, StoredEventKind::SessionState),
            ],
        )
        .unwrap(),
        7
    );

    let validated = validate_next_replay_event(
        &domain("authority-main"),
        0,
        &event(1, StoredEventKind::ResourceState),
    )
    .unwrap();
    assert_eq!(validated.lsn, 1);
    assert_eq!(validated.kind, StoredEventKind::ResourceState);
}

#[test]
fn complete_prefix_validator_classifies_record_and_log_corruption() {
    for (previous_lsn, candidate) in [
        (0, event(2, StoredEventKind::Grant)),
        (1, event(3, StoredEventKind::Grant)),
        (1, event(1, StoredEventKind::Grant)),
        (2, event(1, StoredEventKind::Grant)),
        (0, event(0, StoredEventKind::Grant)),
        (u64::MAX, event(u64::MAX, StoredEventKind::Grant)),
    ] {
        assert!(matches!(
            validate_next_replay_event(&domain("authority-main"), previous_lsn, &candidate),
            Err(ReplayIntegrityError::CorruptLog(_))
        ));
    }

    let mut wrong_domain = event(1, StoredEventKind::Grant);
    wrong_domain.event_id.authority_domain_id = Some(domain("authority-other"));
    assert!(matches!(
        validate_next_replay_event(&domain("authority-main"), 0, &wrong_domain),
        Err(ReplayIntegrityError::CorruptLog(_))
    ));
    assert!(matches!(
        validate_next_replay_event(
            &domain("authority-main"),
            0,
            &event(1, StoredEventKind::Unspecified),
        ),
        Err(ReplayIntegrityError::CorruptLog(_))
    ));

    let mut missing_domain = event(1, StoredEventKind::Grant);
    missing_domain.event_id.authority_domain_id = None;
    let mut empty_domain = event(1, StoredEventKind::Grant);
    empty_domain.event_id.authority_domain_id = Some(domain(""));
    let mut missing_lsn = event(1, StoredEventKind::Grant);
    missing_lsn.event_id.lsn = None;
    let mut unknown_kind = event(1, StoredEventKind::Grant);
    unknown_kind.payload.kind = i32::MAX;
    for candidate in [missing_domain, empty_domain, missing_lsn, unknown_kind] {
        assert!(matches!(
            validate_next_replay_event(&domain("authority-main"), 0, &candidate),
            Err(ReplayIntegrityError::CorruptRecord(_))
        ));
    }
}

#[tokio::test]
async fn default_bounded_read_rejects_missing_lsn_instead_of_filtering_it_out() {
    let mut missing_lsn = event(1, StoredEventKind::Grant);
    missing_lsn.event_id.lsn = None;
    let storage = ScriptedReplayStorage::new(vec![missing_lsn]);

    let error = storage
        .read_through(
            &domain("authority-main"),
            Lsn { value: 0 },
            Lsn { value: 1 },
        )
        .await
        .expect_err("missing LSN framing must fail the default bounded read");
    assert!(matches!(error, StorageError::CorruptRecord(_)));
}

proptest! {
    #[test]
    fn exact_prefix_property_accepts_one_through_n(length in 0usize..32) {
        let events: Vec<_> = (1..=length)
            .map(|lsn| event(lsn as u64, StoredEventKind::Grant))
            .collect();
        prop_assert_eq!(validate_sequence(0, &events), Ok(length as u64));
    }

    #[test]
    fn gap_mutation_witness_kills_monotonic_only_validation(
        length in 1usize..32,
        raw_gap_index in any::<usize>(),
    ) {
        let gap_index = raw_gap_index % length;
        // Independent mathematical oracle: the candidate stays strictly
        // increasing but omits exactly one member of 1..=N. Replacing exact
        // successor equality with `actual > previous` therefore makes this
        // test fail to detect the intentionally injected gap.
        let events: Vec<_> = (0..length)
            .map(|index| {
                let lsn = index + 1 + usize::from(index >= gap_index);
                event(lsn as u64, StoredEventKind::Grant)
            })
            .collect();
        prop_assert!(matches!(
            validate_sequence(0, &events),
            Err(ReplayIntegrityError::CorruptLog(_))
        ));
    }
}

#[tokio::test]
async fn every_exported_complete_projection_rebuild_rejects_gap_and_unspecified() {
    let authority_domain = domain("authority-main");
    let gaps = ScriptedReplayStorage::new(vec![
        event(1, StoredEventKind::ResourceState),
        event(3, StoredEventKind::ResourceState),
    ]);

    assert!(matches!(
        rebuild_commands(&gaps, &authority_domain).await,
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_slots_from_log(&gaps, &authority_domain).await,
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_authority(&gaps, &authority_domain).await,
        Err(AuthorityError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_operator_registry(&gaps, &authority_domain).await,
        Err(OperatorError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_sessions(&gaps, &authority_domain).await,
        Err(SessionError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_security(&gaps, &authority_domain).await,
        Err(SecurityError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_adapters(&gaps, &authority_domain).await,
        Err(AdapterError::CorruptLog(_))
    ));

    // ResourceState is projection-owned, so use a harmless concrete sibling
    // kind to ensure the failure is the missing LSN rather than payload decode.
    let resource_gaps = ScriptedReplayStorage::new(vec![
        event(1, StoredEventKind::Grant),
        event(3, StoredEventKind::Grant),
    ]);
    assert!(matches!(
        rebuild_resources(&resource_gaps, &authority_domain).await,
        Err(ResourceError::CorruptLog(_))
    ));

    let unspecified = ScriptedReplayStorage::new(vec![event(1, StoredEventKind::Unspecified)]);
    assert!(matches!(
        rebuild_commands(&unspecified, &authority_domain).await,
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_slots_from_log(&unspecified, &authority_domain).await,
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_authority(&unspecified, &authority_domain).await,
        Err(AuthorityError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_operator_registry(&unspecified, &authority_domain).await,
        Err(OperatorError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_sessions(&unspecified, &authority_domain).await,
        Err(SessionError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_resources(&unspecified, &authority_domain).await,
        Err(ResourceError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_security(&unspecified, &authority_domain).await,
        Err(SecurityError::CorruptLog(_))
    ));
    assert!(matches!(
        rebuild_adapters(&unspecified, &authority_domain).await,
        Err(AdapterError::CorruptLog(_))
    ));
}

#[test]
fn unspecified_kind_mutation_witness_rejects_before_direct_projection_mutation() {
    // Removing the Unspecified rejection from either the shared boundary or a
    // direct receiver turns at least one of these fail-closed assertions into a
    // sibling no-op. This is implementation evidence, not formal promotion.
    let unspecified = event(1, StoredEventKind::Unspecified);

    let mut commands = CommandIndex::new();
    let before = commands.clone();
    assert!(matches!(
        commands.apply(&unspecified),
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert_eq!(commands, before);

    let mut elicitations = ElicitationSlotLayer::new();
    let before = elicitations.clone();
    assert!(matches!(
        elicitations.observe(&unspecified),
        Err(AcceptanceError::CorruptLog(_))
    ));
    assert_eq!(elicitations, before);

    let mut authority = AuthorityRegistry::new();
    let before = authority.clone();
    assert!(matches!(
        authority.observe(&unspecified),
        Err(AuthorityError::CorruptLog(_))
    ));
    assert_eq!(authority, before);

    let mut operators = OperatorRegistry::new();
    let before = operators.clone();
    assert!(matches!(
        operators.observe(&unspecified),
        Err(OperatorError::CorruptLog(_))
    ));
    assert_eq!(operators, before);

    let mut sessions = SessionRegistry::new();
    let before = sessions.clone();
    assert!(matches!(
        sessions.observe(&unspecified),
        Err(SessionError::CorruptLog(_))
    ));
    assert_eq!(sessions, before);

    let mut resources = ResourceRegistry::new();
    let before = resources.clone();
    assert!(matches!(
        resources.observe(&unspecified),
        Err(ResourceError::CorruptLog(_))
    ));
    assert_eq!(resources, before);

    let mut security = SecurityPostureProjection::new();
    let before = security.clone();
    assert!(matches!(
        security.observe(&unspecified),
        Err(SecurityError::CorruptLog(_))
    ));
    assert_eq!(security, before);

    let mut adapters = AdapterRegistry::new();
    let before = adapters.clone();
    assert!(matches!(
        adapters.observe(&unspecified),
        Err(AdapterError::CorruptLog(_))
    ));
    assert_eq!(adapters, before);

    let mut diagnostics = DiagnosticsProjection::new();
    assert!(matches!(
        diagnostics.observe(&unspecified),
        Err(DiagnosticsError::CorruptEvent(_))
    ));
}
