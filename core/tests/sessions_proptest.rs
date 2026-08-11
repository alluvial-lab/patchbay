//! Property tests for session-registry invariants.
//!
//! `GenerationMonotonic` is the promoted property in this suite. The other
//! properties are stated-normative obligations: they exercise the same durable
//! writer and replay path but do not claim independent checked-model backing.
//!
//! The mutation test at the end is intentional evidence of non-vacuity. It
//! applies the generation oracle to a deliberately faulty registry which
//! accepts lower generations; the oracle must reject that registry.

use std::collections::{BTreeSet, HashMap};

use patchbay_contracts::patchbay::{
    AdapterId, AuthorityDomainId, Generation, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState, SessionReportSourceCursor,
};
use patchbay_core::session::{
    ingest_session_report, rebuild_from_log, IngestResult, SessionError, SessionRecord,
    SessionRegistry, SessionReport,
};
use patchbay_core::storage::RusqliteStorage;
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

/// Keep generated live generations positive. The formal model's generation 0
/// is a no-live-session sentinel, not a valid wire or durable session identity.
fn any_generation() -> impl Strategy<Value = Generation> {
    (1u64..=4).prop_map(|value| Generation { value })
}

fn any_connectivity_state() -> impl Strategy<Value = SessionConnectivityState> {
    prop_oneof![
        Just(SessionConnectivityState::Live),
        Just(SessionConnectivityState::Stale),
        Just(SessionConnectivityState::Offline),
        Just(SessionConnectivityState::Unknown),
        Just(SessionConnectivityState::Failed),
    ]
}

fn any_activity_state() -> impl Strategy<Value = SessionActivityState> {
    prop_oneof![
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
        "[a-z]{1,8}/[a-z]{1,8}",
    )
        .prop_map(
            move |(session_generation, connectivity, activity, project, cwd, name, model)| {
                SessionReport {
                    adapter_id: Some(adapter_id.clone()),
                    deployment_scope: "local".to_owned(),
                    runtime_session_id: Some(runtime_session_id.clone()),
                    session_generation: Some(session_generation),
                    connectivity: connectivity as i32,
                    activity: activity as i32,
                    project,
                    cwd,
                    name,
                    model,
                    spawn_origin: None,
                    source_cursor: Some(SessionReportSourceCursor {
                        adapter_generation: Some(Generation { value: 1 }),
                        revision: 1,
                    }),
                }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OracleSessionKey {
    adapter: String,
    deployment_scope: String,
    runtime_session: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleSessionState {
    live_generation: u64,
    tombstoned_generations: BTreeSet<u64>,
}

fn collision_keys() -> [OracleSessionKey; 4] {
    [
        OracleSessionKey {
            adapter: "adapter-a".to_owned(),
            deployment_scope: "scope-a".to_owned(),
            runtime_session: "runtime-shared".to_owned(),
        },
        OracleSessionKey {
            adapter: "adapter-b".to_owned(),
            deployment_scope: "scope-a".to_owned(),
            runtime_session: "runtime-shared".to_owned(),
        },
        OracleSessionKey {
            adapter: "adapter-a".to_owned(),
            deployment_scope: "scope-b".to_owned(),
            runtime_session: "runtime-shared".to_owned(),
        },
        OracleSessionKey {
            adapter: "adapter-a".to_owned(),
            deployment_scope: "scope-a".to_owned(),
            runtime_session: "runtime-other".to_owned(),
        },
    ]
}

fn any_multi_identity_sequence() -> impl Strategy<Value = Vec<(usize, u64)>> {
    prop::collection::vec((0usize..4, 1u64..=4), 1..=20)
}

fn report_for_key(key: &OracleSessionKey, generation: u64) -> SessionReport {
    SessionReport {
        adapter_id: Some(AdapterId {
            value: key.adapter.clone(),
        }),
        deployment_scope: key.deployment_scope.clone(),
        runtime_session_id: Some(RuntimeSessionId {
            value: key.runtime_session.clone(),
        }),
        session_generation: Some(Generation { value: generation }),
        connectivity: SessionConnectivityState::Live as i32,
        activity: SessionActivityState::Idle as i32,
        project: format!("project-{}", key.deployment_scope),
        cwd: format!("/work/{}", key.deployment_scope),
        name: format!("{}-{}", key.adapter, key.runtime_session),
        model: "provider/model".to_owned(),
        spawn_origin: None,
        source_cursor: Some(SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        }),
    }
}

fn projection_by_oracle_key(
    registry: &SessionRegistry,
) -> Result<HashMap<OracleSessionKey, SessionRecord>, String> {
    let mut records = HashMap::new();
    for record in registry.sessions() {
        let key = OracleSessionKey {
            adapter: record.identity.adapter_id.value.clone(),
            deployment_scope: record.identity.deployment_scope.clone(),
            runtime_session: record.identity.runtime_session_id.value.clone(),
        };
        if records.insert(key.clone(), record.clone()).is_some() {
            return Err(format!(
                "projection exposed duplicate independent key {key:?}"
            ));
        }
    }
    Ok(records)
}

/// Apply one report through the production append-then-fold writer. A success
/// must already be visible in `registry`; cold replay is compared separately.
async fn ingest_hot(
    storage: &RusqliteStorage,
    registry: &mut SessionRegistry,
    report: SessionReport,
) -> Result<IngestResult, SessionError> {
    ingest_session_report(storage, registry, &domain(), report).await
}

fn report_at(
    generation: u64,
    connectivity: SessionConnectivityState,
    activity: SessionActivityState,
) -> SessionReport {
    SessionReport {
        adapter_id: Some(adapter()),
        deployment_scope: "local".to_owned(),
        runtime_session_id: Some(runtime_session("session-1")),
        session_generation: Some(Generation { value: generation }),
        connectivity: connectivity as i32,
        activity: activity as i32,
        project: "project-a".to_owned(),
        cwd: "/work/a".to_owned(),
        name: "session-a".to_owned(),
        model: "provider/model-a".to_owned(),
        spawn_origin: None,
        source_cursor: Some(SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceAttempt {
    session_generation: u64,
    adapter_generation: u64,
    revision: u64,
    model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceOracle {
    session_generation: u64,
    adapter_generation: u64,
    revision: u64,
    model: String,
}

fn any_source_attempt_sequence() -> impl Strategy<Value = Vec<SourceAttempt>> {
    prop::collection::vec((1u64..=3, 1u64..=3, 1u64..=5, "[A-C]"), 1..=20).prop_map(|attempts| {
        attempts
            .into_iter()
            .map(
                |(session_generation, adapter_generation, revision, model)| SourceAttempt {
                    session_generation,
                    adapter_generation,
                    revision,
                    model,
                },
            )
            .collect()
    })
}

fn source_report(attempt: &SourceAttempt) -> SessionReport {
    let mut report = report_at(
        attempt.session_generation,
        SessionConnectivityState::Live,
        SessionActivityState::Idle,
    );
    report.model.clone_from(&attempt.model);
    report.source_cursor = Some(SessionReportSourceCursor {
        adapter_generation: Some(Generation {
            value: attempt.adapter_generation,
        }),
        revision: attempt.revision,
    });
    report
}

fn source_oracle_accepts(live: Option<&SourceOracle>, attempt: &SourceAttempt) -> bool {
    let Some(live) = live else {
        return true;
    };
    attempt.session_generation > live.session_generation
        || (attempt.session_generation == live.session_generation
            && (attempt.adapter_generation > live.adapter_generation
                || (attempt.adapter_generation == live.adapter_generation
                    && attempt.revision > live.revision)))
}

fn source_mutant_accepts(live: Option<&SourceOracle>, attempt: &SourceAttempt) -> bool {
    let Some(live) = live else {
        return true;
    };
    attempt.session_generation > live.session_generation
        || (attempt.session_generation == live.session_generation
            && (attempt.adapter_generation > live.adapter_generation
                || (attempt.adapter_generation == live.adapter_generation && attempt.revision > 0)))
}

fn source_oracle_state(attempt: &SourceAttempt) -> SourceOracle {
    SourceOracle {
        session_generation: attempt.session_generation,
        adapter_generation: attempt.adapter_generation,
        revision: attempt.revision,
        model: attempt.model.clone(),
    }
}

async fn run_source_ordering_check(attempts: &[SourceAttempt]) -> Result<(), String> {
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut registry = SessionRegistry::new(domain()).map_err(|error| error.to_string())?;
    let mut oracle: Option<SourceOracle> = None;

    for attempt in attempts {
        let before = registry.clone();
        let expected_acceptance = source_oracle_accepts(oracle.as_ref(), attempt);
        let result = ingest_hot(&storage, &mut registry, source_report(attempt)).await;
        if expected_acceptance {
            if result.is_err() {
                return Err(format!(
                    "oracle-current report was rejected: {attempt:?}: {result:?}"
                ));
            }
            oracle = Some(source_oracle_state(attempt));
        } else {
            if !matches!(
                &result,
                Err(SessionError::StaleGeneration { .. } | SessionError::StaleSourceCursor { .. })
            ) {
                return Err(format!(
                    "oracle-stale report had wrong result: {attempt:?}: {result:?}"
                ));
            }
            if registry != before {
                return Err(format!(
                    "oracle-stale report mutated the registry: {attempt:?}"
                ));
            }
        }

        let expected = oracle.as_ref().expect("the first attempt always registers");
        let live = registry
            .get_live_session(&adapter(), "local", &runtime_session("session-1"))
            .ok_or("source-order projection lost the live session")?;
        let cursor = live
            .last_source_cursor
            .as_ref()
            .ok_or("accepted source-order report omitted its cursor")?;
        if live.identity.session_generation.value != expected.session_generation
            || cursor
                .adapter_generation
                .as_ref()
                .map(|generation| generation.value)
                != Some(expected.adapter_generation)
            || cursor.revision != expected.revision
            || live.model != expected.model
        {
            return Err(format!(
                "production projection {live:?} disagreed with independent oracle {expected:?}"
            ));
        }
    }

    let rebuilt = rebuild_from_log(&storage, &domain())
        .await
        .map_err(|error| format!("source-order replay failed: {error}"))?;
    if rebuilt != registry {
        return Err("source-order hot registry differed from cold replay".to_owned());
    }
    Ok(())
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
    let mut registry = SessionRegistry::new(domain()).unwrap();
    let mut expected_live_generation = 0;

    for report in reports.iter().cloned() {
        let reported_generation = report.session_generation.unwrap().value;
        let result = ingest_hot(&storage, &mut registry, report).await;
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
            Err(SessionError::StaleGeneration { .. } | SessionError::StaleSourceCursor { .. }) => {}
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

async fn run_multi_identity_check(actions: &[(usize, u64)]) -> Result<(), String> {
    let storage = RusqliteStorage::open_in_memory().map_err(|error| error.to_string())?;
    let mut registry = SessionRegistry::new(domain()).map_err(|error| error.to_string())?;
    let keys = collision_keys();
    let mut oracle: HashMap<OracleSessionKey, OracleSessionState> = HashMap::new();

    for &(key_index, reported_generation) in actions {
        let addressed = keys[key_index].clone();
        let before_registry = registry.clone();
        let before = projection_by_oracle_key(&registry)?;
        let previous = oracle.get(&addressed).cloned();
        let result = ingest_hot(
            &storage,
            &mut registry,
            report_for_key(&addressed, reported_generation),
        )
        .await;

        match previous {
            None => {
                if !matches!(&result, Ok(IngestResult::Registered { .. })) {
                    return Err(format!(
                        "first report for {addressed:?} did not register: {result:?}"
                    ));
                }
                oracle.insert(
                    addressed.clone(),
                    OracleSessionState {
                        live_generation: reported_generation,
                        tombstoned_generations: BTreeSet::new(),
                    },
                );
            }
            Some(mut expected) if reported_generation > expected.live_generation => {
                if !matches!(
                    &result,
                    Ok(IngestResult::GenerationBumped {
                        from_generation,
                        to_generation,
                        ..
                    }) if from_generation.value == expected.live_generation
                        && to_generation.value == reported_generation
                ) {
                    return Err(format!(
                        "strict bump for {addressed:?} had wrong result: {result:?}"
                    ));
                }
                expected
                    .tombstoned_generations
                    .insert(expected.live_generation);
                expected.live_generation = reported_generation;
                oracle.insert(addressed.clone(), expected);
            }
            Some(expected) if reported_generation == expected.live_generation => {
                if !matches!(&result, Err(SessionError::StaleSourceCursor { .. })) {
                    return Err(format!(
                        "equal report for {addressed:?} was not source-stale: {result:?}"
                    ));
                }
                if registry != before_registry {
                    return Err(format!(
                        "equal report for {addressed:?} mutated the exact registry"
                    ));
                }
            }
            Some(expected) => {
                if !matches!(
                    &result,
                    Err(SessionError::StaleGeneration { live, reported })
                        if live.value == expected.live_generation
                            && reported.value == reported_generation
                ) {
                    return Err(format!(
                        "lower report for {addressed:?} had wrong result: {result:?}"
                    ));
                }
                if registry != before_registry {
                    return Err(format!(
                        "lower report for {addressed:?} mutated the exact registry"
                    ));
                }
            }
        }

        let after = projection_by_oracle_key(&registry)?;
        if after.len() != oracle.len() {
            return Err(format!(
                "projection key count {} differs from oracle count {}",
                after.len(),
                oracle.len()
            ));
        }
        for (other_key, before_record) in &before {
            if other_key != &addressed && after.get(other_key) != Some(before_record) {
                return Err(format!(
                    "report for {addressed:?} interfered with {other_key:?}"
                ));
            }
        }
        for (key, expected) in &oracle {
            let record = after
                .get(key)
                .ok_or_else(|| format!("missing live record for {key:?}"))?;
            if record.identity.session_generation.value != expected.live_generation {
                return Err(format!(
                    "live generation for {key:?} is {}, expected {}",
                    record.identity.session_generation.value, expected.live_generation
                ));
            }
            for tombstoned_generation in &expected.tombstoned_generations {
                if registry
                    .get_tombstone(
                        &AdapterId {
                            value: key.adapter.clone(),
                        },
                        &key.deployment_scope,
                        &RuntimeSessionId {
                            value: key.runtime_session.clone(),
                        },
                        &Generation {
                            value: *tombstoned_generation,
                        },
                    )
                    .is_none()
                {
                    return Err(format!(
                        "missing retained tombstone for {key:?} generation {tombstoned_generation}"
                    ));
                }
            }
        }
    }

    let rebuilt = rebuild_from_log(&storage, &domain())
        .await
        .map_err(|error| format!("multi-identity replay failed: {error}"))?;
    if rebuilt != registry {
        return Err("multi-identity hot registry differed from cold replay".to_owned());
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

    /// SessionReportSourceOrdering implementation evidence: an independent raw
    /// tuple oracle decides acceptance without calling the production comparator.
    #[test]
    fn session_report_source_ordering_matches_independent_oracle(
        attempts in any_source_attempt_sequence(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            run_source_ordering_check(&attempts)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// SessionIdentityTuple / isolation implementation evidence: reports vary
    /// every canonical identity dimension across deliberate one-field
    /// collisions. The independent oracle checks only the addressed tuple,
    /// retained tombstones, generation non-decrease, and hot/cold equality.
    #[test]
    fn multi_identity_sequences_preserve_isolation_and_replay(
        actions in any_multi_identity_sequence(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            run_multi_identity_check(&actions)
                .await
                .map_err(TestCaseError::fail)?;
            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Strict supersession: an identical equal-generation source cursor is
    /// rejected; a lower runtime generation is also rejected, and neither can
    /// alter the live generation.
    #[test]
    fn equal_generation_is_noop_lower_is_rejected(
        generation in 2u64..=4,
        connectivity in any_connectivity_state(),
        activity in any_activity_state(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new(domain()).unwrap();
            let original = report_at(generation, connectivity, activity);
            let registered = ingest_hot(&storage, &mut registry, original.clone()).await.unwrap();
            prop_assert!(matches!(registered, IngestResult::Registered { .. }), "first report must register");
            let before_equal = live_generation(&registry);

            let equal = ingest_hot(&storage, &mut registry, original.clone()).await;
            prop_assert!(
                matches!(equal, Err(SessionError::StaleSourceCursor { .. })),
                "equal source cursor must be rejected"
            );
            prop_assert_eq!(live_generation(&registry), before_equal);

            let mut lower = original;
            lower.session_generation = Some(Generation { value: generation - 1 });
            let rejected = ingest_hot(&storage, &mut registry, lower).await;
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
        start in 1u64..=3,
        connectivity in any_connectivity_state(),
        activity in any_activity_state(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new(domain()).unwrap();
            let first = report_at(start, connectivity, activity);
            ingest_hot(&storage, &mut registry, first.clone()).await.unwrap();
            let bumped = report_at(start + 1, connectivity, activity);
            let result = ingest_hot(&storage, &mut registry, bumped).await.unwrap();
            prop_assert!(matches!(result, IngestResult::GenerationBumped { .. }), "newer report must bump generation");
            let live_before_late = live_generation(&registry);

            let late = ingest_hot(&storage, &mut registry, first).await;
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
            let mut registry = SessionRegistry::new(domain()).unwrap();
            let original = report_at(
                generation.value,
                SessionConnectivityState::Live,
                SessionActivityState::Idle,
            );
            ingest_hot(&storage, &mut registry, original.clone()).await.unwrap();
            let identity_before = registry
                .get_live_session(&adapter(), "local", &runtime_session("session-1"))
                .unwrap()
                .identity
                .clone();

            let mut relabeled = original;
            relabeled.source_cursor.as_mut().unwrap().revision = 2;
            relabeled.project = "project-b".to_owned();
            relabeled.cwd = "/work/b".to_owned();
            relabeled.name = "session-b".to_owned();
            let result = ingest_hot(&storage, &mut registry, relabeled).await.unwrap();
            prop_assert!(matches!(result, IngestResult::ReportApplied { .. }), "metadata-only report must apply atomically");
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
        start in 1u64..=2,
        first_increment in 1u64..=2,
        second_increment in 1u64..=2,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let storage = RusqliteStorage::open_in_memory().unwrap();
            let mut registry = SessionRegistry::new(domain()).unwrap();
            let generations = [
                start,
                start + first_increment,
                start + first_increment + second_increment,
            ];
            for generation in generations {
                ingest_hot(
                    &storage,
                    &mut registry,
                    report_at(
                        generation,
                        SessionConnectivityState::Live,
                        SessionActivityState::Idle,
                    ),
                )
                .await
                .unwrap();
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
            let mut live = SessionRegistry::new(domain()).unwrap();
            for report in reports {
                match ingest_hot(&storage, &mut live, report).await {
                    Ok(_)
                    | Err(SessionError::StaleGeneration { .. })
                    | Err(SessionError::StaleSourceCursor { .. })
                    | Err(SessionError::InvalidTransition { .. }) => {},
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

#[test]
fn session_source_oracle_kills_positive_revision_comparison_mutant() {
    let attempts = [
        SourceAttempt {
            session_generation: 1,
            adapter_generation: 1,
            revision: 1,
            model: "A".to_owned(),
        },
        SourceAttempt {
            session_generation: 1,
            adapter_generation: 1,
            revision: 3,
            model: "B".to_owned(),
        },
        SourceAttempt {
            session_generation: 1,
            adapter_generation: 1,
            revision: 2,
            model: "A".to_owned(),
        },
    ];
    let mut oracle = None;
    let mut mutant = None;
    for attempt in &attempts {
        if source_oracle_accepts(oracle.as_ref(), attempt) {
            oracle = Some(source_oracle_state(attempt));
        }
        if source_mutant_accepts(mutant.as_ref(), attempt) {
            mutant = Some(source_oracle_state(attempt));
        }
    }

    assert_eq!(oracle.as_ref().unwrap().model, "B");
    assert_eq!(oracle.as_ref().unwrap().revision, 3);
    assert_eq!(mutant.as_ref().unwrap().model, "A");
    assert_eq!(mutant.as_ref().unwrap().revision, 2);
    assert_ne!(
        mutant, oracle,
        "independent oracle failed to kill stale-guard mutant"
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FaultySessionKey {
    adapter: String,
    deployment_scope: String,
    runtime_session: String,
}

#[derive(Debug, Clone, Copy)]
enum OmittedIdentityDimension {
    Adapter,
    DeploymentScope,
    RuntimeSession,
}

fn faulty_key(key: &OracleSessionKey, omitted: OmittedIdentityDimension) -> FaultySessionKey {
    FaultySessionKey {
        adapter: if matches!(omitted, OmittedIdentityDimension::Adapter) {
            String::new()
        } else {
            key.adapter.clone()
        },
        deployment_scope: if matches!(omitted, OmittedIdentityDimension::DeploymentScope) {
            String::new()
        } else {
            key.deployment_scope.clone()
        },
        runtime_session: if matches!(omitted, OmittedIdentityDimension::RuntimeSession) {
            String::new()
        } else {
            key.runtime_session.clone()
        },
    }
}

fn independent_keys_are_injective<K: std::hash::Hash + Eq + std::fmt::Debug>(
    keys: &[OracleSessionKey],
    key_for: impl Fn(&OracleSessionKey) -> K,
) -> Result<(), String> {
    let mut seen = HashMap::new();
    for key in keys {
        let candidate = key_for(key);
        if let Some(previous) = seen.insert(candidate, key) {
            return Err(format!("identity alias between {previous:?} and {key:?}"));
        }
    }
    Ok(())
}

#[test]
fn independent_identity_oracle_kills_each_dimension_omission_mutant() {
    let keys = collision_keys();
    assert!(independent_keys_are_injective(&keys, Clone::clone).is_ok());
    for omitted in [
        OmittedIdentityDimension::Adapter,
        OmittedIdentityDimension::DeploymentScope,
        OmittedIdentityDimension::RuntimeSession,
    ] {
        assert!(
            independent_keys_are_injective(&keys, |key| faulty_key(key, omitted)).is_err(),
            "oracle failed to detect {omitted:?} omission"
        );
    }
}

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
        let generation = report.session_generation.unwrap();
        self.live_generation = Some(generation);
        generation
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
