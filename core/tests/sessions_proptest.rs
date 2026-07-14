//! Property tests for session-registry invariants.
//!
//! `GenerationMonotonic` is the promoted property in this suite. The other
//! properties are stated-normative obligations: they exercise the same durable
//! writer and replay path but do not claim independent checked-model backing.
//!
//! The mutation test at the end is intentional evidence of non-vacuity. It
//! applies the generation oracle to a deliberately faulty registry which
//! accepts lower generations; the oracle must reject that registry.

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, Generation, Lsn, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState,
};
use patchbay_core::session::{
    ingest_session_report, rebuild_from_log, IngestResult, SessionError, SessionRegistry,
    SessionReport,
};
use patchbay_core::storage::{RusqliteStorage, Storage};
use proptest::prelude::*;

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "sessions-proptest-domain".to_owned(),
    }
}

fn adapter() -> AdapterId {
    AdapterId {
        value: "pi-proptest".to_owned(),
    }
}

fn runtime_session(id: &str) -> RuntimeSessionId {
    RuntimeSessionId {
        value: id.to_owned(),
    }
}

/// Keep the generated generation domain aligned with the small bounded domain
/// in the session model (`GENERATIONS = 0..3`), with one extra bump boundary.
fn any_generation() -> impl Strategy<Value = Generation> {
    (0u64..=4).prop_map(|value| Generation { value })
}

fn any_connectivity_state() -> impl Strategy<Value = SessionConnectivityState> {
    prop_oneof![
        Just(SessionConnectivityState::Unspecified),
        Just(SessionConnectivityState::Live),
        Just(SessionConnectivityState::Stale),
        Just(SessionConnectivityState::Offline),
        Just(SessionConnectivityState::Unknown),
        Just(SessionConnectivityState::Failed),
    ]
}

fn any_activity_state() -> impl Strategy<Value = SessionActivityState> {
    prop_oneof![
        Just(SessionActivityState::Unspecified),
        Just(SessionActivityState::Idle),
        Just(SessionActivityState::Working),
        Just(SessionActivityState::Unknown),
    ]
}

fn any_session_report(
    adapter_id: AdapterId,
    runtime_session_id: RuntimeSessionId,
) -> impl Strategy<Value = SessionReport> {
    (
        any_generation(),
        any_connectivity_state(),
        any_activity_state(),
        "[a-z]{1,8}",
        "/[a-z]{1,8}",
        "[a-z]{1,8}",
    )
        .prop_map(
            move |(session_generation, connectivity, activity, project, cwd, name)| SessionReport {
                authority_domain_id: domain(),
                adapter_id: adapter_id.clone(),
                deployment_scope: "local".to_owned(),
                runtime_session_id: runtime_session_id.clone(),
                session_generation,
                connectivity,
                activity,
                project,
                cwd,
                name,
            },
        )
}

/// A deliberately unordered stream: it contains registrations, generation
/// bumps, equal-generation state/label reports, and stale reports. The writer
/// decides which reports become durable deltas.
fn any_session_report_sequence() -> impl Strategy<Value = Vec<SessionReport>> {
    prop::collection::vec(
        any_session_report(adapter(), runtime_session("session-1")),
        1..=15,
    )
}

fn event_lsn(result: &IngestResult) -> Option<u64> {
    let event_id = match result {
        IngestResult::Registered { event_id }
        | IngestResult::ConnectivityChanged { event_id, .. }
        | IngestResult::ActivityChanged { event_id, .. }
        | IngestResult::Relabeled { event_id } => event_id,
        IngestResult::GenerationBumped {
            new_generation_event_id,
            ..
        } => new_generation_event_id,
        IngestResult::DeltasApplied { event_ids } => {
            return event_ids
                .last()
                .and_then(|event_id| event_id.lsn.as_ref())
                .map(|lsn| lsn.value);
        }
        IngestResult::NoChange => return None,
    };
    event_id.lsn.as_ref().map(|lsn| lsn.value)
}

/// Apply one report through the writer, then feed exactly the newly committed
/// events into the hot registry. This mirrors production's warm path while
/// leaving the log as the authoritative source of truth.
async fn ingest_and_observe(
    storage: &RusqliteStorage,
    registry: &mut SessionRegistry,
    cursor: &mut u64,
    report: SessionReport,
) -> Result<IngestResult, SessionError> {
    let result = ingest_session_report(storage, &*registry, report).await?;
    if let Some(appended_lsn) = event_lsn(&result) {
        let events = storage
            .read_after(&domain(), Lsn { value: *cursor })
            .await
            .expect("the in-memory test log can read committed session events");
        for event in events {
            registry.observe(&event)?;
        }
        *cursor = appended_lsn;
    }
    Ok(result)
}

fn report_at(
    generation: u64,
    connectivity: SessionConnectivityState,
    activity: SessionActivityState,
) -> SessionReport {
    SessionReport {
        authority_domain_id: domain(),
        adapter_id: adapter(),
        deployment_scope: "local".to_owned(),
        runtime_session_id: runtime_session("session-1"),
        session_generation: Generation { value: generation },
        connectivity,
        activity,
        project: "project-a".to_owned(),
        cwd: "/work/a".to_owned(),
        name: "session-a".to_owned(),
    }
}

fn live_generation(registry: &SessionRegistry) -> u64 {
    registry
        .get_live_session(&adapter(), "local", &runtime_session("session-1"))
        .expect("a generated sequence always registers its first report")
        .identity
        .session_generation
        .value
}

/// The `GenerationMonotonic` oracle, expressed independently of a particular
/// registry implementation so it can also be applied to the mutation below.
fn check_non_decreasing(previous: &mut u64, next: u64) -> Result<(), String> {
    if next < *previous {
        return Err(format!(
            "live generation decreased from {} to {next}",
            *previous
        ));
    }
    *previous = next;
    Ok(())
}

async fn run_generation_monotonic_check(reports: &[SessionReport]) -> Result<(), String> {
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut registry = SessionRegistry::new();
    let mut cursor = 0;
    let mut expected_live_generation = 0;

    for report in reports.iter().cloned() {
        let reported_generation = report.session_generation.value;
        let result = ingest_and_observe(&storage, &mut registry, &mut cursor, report).await;
        match result {
            Ok(IngestResult::GenerationBumped { to_generation, .. }) => {
                if to_generation.value <= expected_live_generation {
                    return Err(format!(
                        "generation bump did not strictly advance: {} -> {}",
                        expected_live_generation, to_generation.value
                    ));
                }
                check_non_decreasing(&mut expected_live_generation, to_generation.value)?;
            }
            Ok(IngestResult::Registered { .. }) => {
                check_non_decreasing(&mut expected_live_generation, reported_generation)?;
            }
            Ok(_) => {}
            Err(SessionError::StaleGeneration { .. }) => {}
            Err(error) => {
                // Random state-axis reports may be invalid transitions. They
                // are rejected before append and therefore cannot alter the
                // live generation.
                if !matches!(error, SessionError::InvalidTransition { .. }) {
                    return Err(format!("unexpected report rejection: {error}"));
                }
            }
        }

        let current = live_generation(&registry);
        if current != expected_live_generation {
            return Err(format!(
                "live registry generation {current} disagreed with tracked generation {expected_live_generation}"
            ));
        }
    }

    let rebuilt = rebuild_from_log(&storage, &domain())
        .await
        .map_err(|error| format!("replay failed: {error}"))?;
    if live_generation(&rebuilt) != expected_live_generation {
        return Err(format!(
            "replay generation {} disagreed with live generation {expected_live_generation}",
            live_generation(&rebuilt)
        ));
    }
    if rebuilt != registry {
        return Err("replay registry differed from the live registry".to_owned());
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        ..ProptestConfig::default()
    })]

    /// GenerationMonotonic (promoted): no committed session report can lower
    /// the live generation, and durable replay agrees with the hot registry.
    #[test]
    fn generation_never_decreases(reports in any_session_report_sequence()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            run_generation_monotonic_check(&reports)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Strict supersession: an identical equal-generation report is a no-op;
    /// a lower-generation report is rejected and leaves the live generation
    /// unchanged.
    #[test]
    fn equal_generation_is_noop_lower_is_rejected(
        generation in 1u64..=4,
        connectivity in any_connectivity_state(),
        activity in any_activity_state(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new();
            let mut cursor = 0;
            let original = report_at(generation, connectivity, activity);
            let registered = ingest_and_observe(&storage, &mut registry, &mut cursor, original.clone()).await.unwrap();
            prop_assert!(matches!(registered, IngestResult::Registered { .. }), "first report must register");
            let before_equal = live_generation(&registry);

            let equal = ingest_and_observe(&storage, &mut registry, &mut cursor, original.clone()).await.unwrap();
            prop_assert!(matches!(equal, IngestResult::NoChange));
            prop_assert_eq!(live_generation(&registry), before_equal);

            let mut lower = original;
            lower.session_generation = Generation { value: generation - 1 };
            let rejected = ingest_and_observe(&storage, &mut registry, &mut cursor, lower).await;
            prop_assert!(matches!(
                rejected,
                Err(SessionError::StaleGeneration { live, reported })
                    if live.value == generation && reported.value == generation - 1
            ), "lower report must be rejected as StaleGeneration");
            prop_assert_eq!(live_generation(&registry), before_equal);
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// LateGenerationInert: after a bump tombstones the old generation, a
    /// report for that generation is rejected and cannot mutate the live one.
    #[test]
    fn late_generation_is_inert(
        start in 0u64..=3,
        connectivity in any_connectivity_state(),
        activity in any_activity_state(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new();
            let mut cursor = 0;
            let first = report_at(start, connectivity, activity);
            ingest_and_observe(&storage, &mut registry, &mut cursor, first.clone()).await.unwrap();
            let bumped = report_at(start + 1, connectivity, activity);
            let result = ingest_and_observe(&storage, &mut registry, &mut cursor, bumped).await.unwrap();
            prop_assert!(matches!(result, IngestResult::GenerationBumped { .. }), "newer report must bump generation");
            let live_before_late = live_generation(&registry);

            let late = ingest_and_observe(&storage, &mut registry, &mut cursor, first).await;
            prop_assert!(
                matches!(late, Err(SessionError::StaleGeneration { .. })),
                "late report must be stale"
            );
            prop_assert_eq!(live_generation(&registry), live_before_late);
            prop_assert!(registry.is_tombstoned(
                &adapter(),
                "local",
                &runtime_session("session-1"),
                &Generation { value: start },
            ), "bumped generation must have a tombstone");
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Labels are presentation metadata, not identity fields. Relabeling an
    /// existing session changes its labels while retaining its one live slot
    /// and canonical identity tuple.
    #[test]
    fn relabel_preserves_identity(generation in any_generation()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new();
            let mut cursor = 0;
            let original = report_at(
                generation.value,
                SessionConnectivityState::Live,
                SessionActivityState::Idle,
            );
            ingest_and_observe(&storage, &mut registry, &mut cursor, original.clone()).await.unwrap();
            let identity_before = registry
                .get_live_session(&adapter(), "local", &runtime_session("session-1"))
                .unwrap()
                .identity
                .clone();

            let mut relabeled = original;
            relabeled.project = "project-b".to_owned();
            relabeled.cwd = "/work/b".to_owned();
            relabeled.name = "session-b".to_owned();
            let result = ingest_and_observe(&storage, &mut registry, &mut cursor, relabeled).await.unwrap();
            prop_assert!(matches!(result, IngestResult::Relabeled { .. }), "metadata-only report must relabel");
            let record = registry
                .get_live_session(&adapter(), "local", &runtime_session("session-1"))
                .unwrap();
            prop_assert_eq!(&record.identity, &identity_before);
            prop_assert_eq!(&record.project, "project-b");
            prop_assert_eq!(&record.cwd, "/work/b");
            prop_assert_eq!(&record.name, "session-b");
            prop_assert!(registry.get_session(&identity_before).is_some());
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Tombstones are durable audit facts: every superseded generation remains
    /// queryable after later generations replace it.
    #[test]
    fn tombstones_retained(
        start in 0u64..=1,
        first_increment in 1u64..=2,
        second_increment in 1u64..=2,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new();
            let mut cursor = 0;
            let generations = [start, start + first_increment, start + first_increment + second_increment];
            for generation in generations {
                ingest_and_observe(
                    &storage,
                    &mut registry,
                    &mut cursor,
                    report_at(generation, SessionConnectivityState::Live, SessionActivityState::Idle),
                ).await.unwrap();
            }
            prop_assert_eq!(live_generation(&registry), generations[2]);
            for generation in &generations[..2] {
                let tombstone = registry.get_tombstone(
                    &adapter(),
                    "local",
                    &runtime_session("session-1"),
                    &Generation { value: *generation },
                ).ok_or_else(|| TestCaseError::fail(format!("missing tombstone for generation {generation}")))?;
                prop_assert_eq!(tombstone.superseded_generation.value, *generation);
            }
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// IdempotentLogReplay: the live registry that observes committed events
    /// and a fresh rebuild from those same durable events are identical.
    #[test]
    fn replay_matches_live(reports in any_session_report_sequence()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut live = SessionRegistry::new();
            let mut cursor = 0;
            for report in reports {
                match ingest_and_observe(&storage, &mut live, &mut cursor, report).await {
                    Ok(_) | Err(SessionError::StaleGeneration { .. }) | Err(SessionError::InvalidTransition { .. }) => {},
                    Err(error) => return Err(TestCaseError::fail(format!("unexpected report rejection: {error}"))),
                }
            }
            let rebuilt = rebuild_from_log(&storage, &domain())
                .await
                .map_err(|error| TestCaseError::fail(format!("replay failed: {error}")))?;
            prop_assert_eq!(rebuilt, live);
            Ok::<(), TestCaseError>(())
        })?;
    }
}

// ===== Mutation discipline =====

/// Mutant: models a registry whose generation-update action overwrites the
/// live generation for every report, including an older report. This is the
/// strict-supersession guard removed: the exact defect
/// `GenerationMonotonic` is meant to catch.
#[derive(Default)]
struct DecreasingGenerationRegistry {
    live_generation: Option<Generation>,
}

impl DecreasingGenerationRegistry {
    fn ingest(&mut self, report: &SessionReport) -> Generation {
        self.live_generation = Some(report.session_generation);
        report.session_generation
    }
}

fn run_mutated_generation_monotonic_check(reports: &[SessionReport]) -> Result<(), String> {
    let mut registry = DecreasingGenerationRegistry::default();
    let mut live_generation = 0;
    for report in reports {
        let next = registry.ingest(report).value;
        check_non_decreasing(&mut live_generation, next)?;
    }
    Ok(())
}

#[test]
fn generation_monotonic_catches_injected_decrease() {
    let reports = vec![
        report_at(
            1,
            SessionConnectivityState::Live,
            SessionActivityState::Idle,
        ),
        report_at(
            3,
            SessionConnectivityState::Live,
            SessionActivityState::Idle,
        ),
        report_at(
            1,
            SessionConnectivityState::Live,
            SessionActivityState::Idle,
        ),
    ];
    let rt = tokio::runtime::Runtime::new().unwrap();
    let real_passes = rt
        .block_on(run_generation_monotonic_check(&reports))
        .is_ok();
    assert!(
        real_passes,
        "the production registry must satisfy GenerationMonotonic"
    );

    let mutant_fails = run_mutated_generation_monotonic_check(&reports).is_err();
    assert!(
        mutant_fails,
        "the generation-monotonic property did not catch a lower-generation overwrite"
    );
}
