use std::{
    num::NonZeroU64,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};
use patchbay_core::{
    storage::{Storage, StorageError},
    time::Clock,
};

use crate::{
    snapshot::{decode_compatible_session_checkpoint, encode_stored_session_checkpoint},
    state::ProjectionState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionCheckpointPolicy {
    pub events_per_checkpoint: NonZeroU64,
    pub poll_interval: Duration,
    pub retry_initial: Duration,
    pub retry_max: Duration,
}

impl Default for SessionCheckpointPolicy {
    fn default() -> Self {
        Self {
            events_per_checkpoint: NonZeroU64::new(256).expect("256 is nonzero"),
            poll_interval: Duration::from_secs(1),
            retry_initial: Duration::from_secs(1),
            retry_max: Duration::from_secs(30),
        }
    }
}

impl SessionCheckpointPolicy {
    pub fn validate(self) -> Result<Self, &'static str> {
        if self.poll_interval.is_zero()
            || self.retry_initial.is_zero()
            || self.retry_max < self.retry_initial
        {
            return Err("checkpoint intervals must be positive and retry_max >= retry_initial");
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointTickOutcome {
    EmptyLog,
    NotDue {
        checkpoint_lsn: u64,
        current_lsn: u64,
    },
    Written {
        prior_lsn: u64,
        checkpoint_lsn: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointFailureStage {
    CatchUp,
    Load,
    Materialize,
    Write,
}

pub trait CheckpointObserver: Send + Sync {
    fn observe_failure(
        &self,
        stage: CheckpointFailureStage,
        attempted_lsn: Option<u64>,
        consecutive_failures: u32,
        retryable: bool,
        error_class: &'static str,
    );
}

#[derive(Debug, Default)]
pub struct StderrCheckpointObserver;

impl CheckpointObserver for StderrCheckpointObserver {
    fn observe_failure(
        &self,
        stage: CheckpointFailureStage,
        attempted_lsn: Option<u64>,
        consecutive_failures: u32,
        retryable: bool,
        error_class: &'static str,
    ) {
        eprintln!(
            "{{\"event\":\"session_checkpoint_failed\",\"stage\":\"{}\",\"attempted_lsn\":{},\"consecutive_failures\":{},\"retryable\":{},\"error_class\":\"{}\"}}",
            stage_name(stage),
            attempted_lsn.map_or_else(|| "null".to_owned(), |lsn| lsn.to_string()),
            consecutive_failures,
            retryable,
            error_class,
        );
    }
}

#[derive(Debug, thiserror::Error)]
#[error("checkpoint {stage:?} failed: {message}")]
pub struct CheckpointWriterError {
    pub stage: CheckpointFailureStage,
    pub attempted_lsn: Option<u64>,
    pub retryable: bool,
    message: String,
}

impl CheckpointWriterError {
    fn error_class(&self) -> &'static str {
        if self.stage == CheckpointFailureStage::Materialize {
            "projection_invalid"
        } else if self.retryable {
            "storage_transient"
        } else {
            "storage_permanent"
        }
    }

    fn storage(
        stage: CheckpointFailureStage,
        attempted_lsn: Option<u64>,
        error: StorageError,
    ) -> Self {
        let retryable = matches!(
            error,
            StorageError::WriteFailed {
                retryable: true,
                ..
            } | StorageError::ReadFailed {
                retryable: true,
                ..
            } | StorageError::Unavailable(_)
        );
        Self {
            stage,
            attempted_lsn,
            retryable,
            message: error.to_string(),
        }
    }

    fn projection(
        stage: CheckpointFailureStage,
        attempted_lsn: Option<u64>,
        error: impl ToString,
    ) -> Self {
        Self {
            stage,
            attempted_lsn,
            retryable: false,
            message: error.to_string(),
        }
    }
}

pub struct SessionCheckpointWriter<S> {
    storage: S,
    state: ProjectionState,
    authority_domain_id: AuthorityDomainId,
    clock: Arc<dyn Clock>,
    policy: SessionCheckpointPolicy,
    observer: Arc<dyn CheckpointObserver>,
    consecutive_failures: AtomicU32,
    last_examined_head: AtomicU64,
    last_known_checkpoint_lsn: AtomicU64,
    force_rewrite: AtomicBool,
}

impl<S> SessionCheckpointWriter<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    pub fn new(
        storage: S,
        state: ProjectionState,
        authority_domain_id: AuthorityDomainId,
        clock: Arc<dyn Clock>,
        policy: SessionCheckpointPolicy,
        observer: Arc<dyn CheckpointObserver>,
    ) -> Result<Self, &'static str> {
        let last_known_checkpoint_lsn = state.session_recovery_checkpoint_lsn();
        let force_rewrite = state.session_checkpoint_was_rejected();
        Ok(Self {
            storage,
            state,
            authority_domain_id,
            clock,
            policy: policy.validate()?,
            observer,
            consecutive_failures: AtomicU32::new(0),
            last_examined_head: AtomicU64::new(0),
            last_known_checkpoint_lsn: AtomicU64::new(last_known_checkpoint_lsn),
            force_rewrite: AtomicBool::new(force_rewrite),
        })
    }

    pub async fn run_once(&self) -> Result<CheckpointTickOutcome, CheckpointWriterError> {
        let result = self.run_once_inner().await;
        match &result {
            Ok(outcome) => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
                match outcome {
                    CheckpointTickOutcome::EmptyLog => {}
                    CheckpointTickOutcome::NotDue {
                        checkpoint_lsn,
                        current_lsn,
                    } => {
                        self.last_known_checkpoint_lsn
                            .store(*checkpoint_lsn, Ordering::Relaxed);
                        self.last_examined_head
                            .store(*current_lsn, Ordering::Relaxed);
                    }
                    CheckpointTickOutcome::Written { checkpoint_lsn, .. } => {
                        self.last_known_checkpoint_lsn
                            .store(*checkpoint_lsn, Ordering::Relaxed);
                        self.last_examined_head
                            .store(*checkpoint_lsn, Ordering::Relaxed);
                        self.force_rewrite.store(false, Ordering::Relaxed);
                    }
                }
            }
            Err(error) => {
                let failures = self
                    .consecutive_failures
                    .fetch_add(1, Ordering::Relaxed)
                    .saturating_add(1);
                self.observer.observe_failure(
                    error.stage,
                    error.attempted_lsn,
                    failures,
                    error.retryable,
                    error.error_class(),
                );
            }
        }
        result
    }

    async fn run_once_inner(&self) -> Result<CheckpointTickOutcome, CheckpointWriterError> {
        let decision_guard = self.state.submit_guard().await;
        self.state
            .catch_up(&self.storage, &self.authority_domain_id)
            .await
            .map_err(|error| {
                CheckpointWriterError::storage(CheckpointFailureStage::CatchUp, None, error)
            })?;
        let current_lsn = self.state.current_lsn().await;
        if current_lsn == 0 {
            drop(decision_guard);
            return Ok(CheckpointTickOutcome::EmptyLog);
        }
        if !self.force_rewrite.load(Ordering::Relaxed)
            && self.last_examined_head.load(Ordering::Relaxed) == current_lsn
        {
            let checkpoint_lsn = self.last_known_checkpoint_lsn.load(Ordering::Relaxed);
            drop(decision_guard);
            return Ok(CheckpointTickOutcome::NotDue {
                checkpoint_lsn,
                current_lsn,
            });
        }

        let candidate = self
            .storage
            .load_latest_snapshot(&self.authority_domain_id, None)
            .await
            .map_err(|error| {
                CheckpointWriterError::storage(
                    CheckpointFailureStage::Load,
                    Some(current_lsn),
                    error,
                )
            })?;
        let prior_lsn = candidate
            .as_ref()
            .and_then(|stored| {
                decode_compatible_session_checkpoint(
                    stored,
                    &self.authority_domain_id,
                    self.state.core_generation(),
                )
                .ok()
            })
            .and_then(|compatible| compatible.snapshot.snapshot_lsn)
            .map_or(0, |lsn| lsn.value);

        if !self.force_rewrite.load(Ordering::Relaxed)
            && current_lsn.saturating_sub(prior_lsn) < self.policy.events_per_checkpoint.get()
        {
            drop(decision_guard);
            return Ok(CheckpointTickOutcome::NotDue {
                checkpoint_lsn: prior_lsn,
                current_lsn,
            });
        }

        let checkpoint = self
            .state
            .materialize_session_checkpoint(self.authority_domain_id.clone(), self.clock.now())
            .await;
        let checkpoint_lsn = checkpoint
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.snapshot_lsn)
            .filter(|lsn| lsn.value == current_lsn)
            .ok_or_else(|| {
                CheckpointWriterError::projection(
                    CheckpointFailureStage::Materialize,
                    Some(current_lsn),
                    "materialized session checkpoint does not match caught-up head",
                )
            })?
            .value;
        drop(decision_guard);
        let payload = encode_stored_session_checkpoint(&checkpoint);

        self.storage
            .write_snapshot(
                &self.authority_domain_id,
                Lsn {
                    value: checkpoint_lsn,
                },
                payload,
            )
            .await
            .map_err(|error| {
                CheckpointWriterError::storage(
                    CheckpointFailureStage::Write,
                    Some(checkpoint_lsn),
                    error,
                )
            })?;
        Ok(CheckpointTickOutcome::Written {
            prior_lsn,
            checkpoint_lsn,
        })
    }

    pub async fn run(self) {
        let mut retry_delay = self.policy.retry_initial;
        loop {
            match self.run_once().await {
                Ok(_) => {
                    retry_delay = self.policy.retry_initial;
                    tokio::time::sleep(self.policy.poll_interval).await;
                }
                Err(_) => {
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = retry_delay.saturating_mul(2).min(self.policy.retry_max);
                }
            }
        }
    }
}

fn stage_name(stage: CheckpointFailureStage) -> &'static str {
    match stage {
        CheckpointFailureStage::CatchUp => "catch_up",
        CheckpointFailureStage::Load => "load",
        CheckpointFailureStage::Materialize => "materialize",
        CheckpointFailureStage::Write => "write",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::AtomicBool, Mutex as StdMutex};

    use patchbay_contracts::patchbay::{
        resource_state_mutation, ActorEndpointRef, ActorId, AdapterId, AdapterSnapshotSupport,
        EventId, Generation, IdempotencyKey, Observation, PayloadContentType, PayloadEnvelope,
        ResourceId, ResourceIdentity, ResourceKind, ResourceStateEvent, ResourceStateMutation,
        ResourceStateUpsert, ResourceViewStateUpdate, RuntimeSessionId, SecurityLockdownEntered,
        SessionActivityState, SessionConnectivityState, SessionGenerationBumped, SessionRegistered,
        SessionReport, SessionReportApplied, SessionReportSourceCursor, SessionState,
        StoredEventKind, StoredEventPayload,
    };
    use patchbay_core::{
        session::{events as session_events, rebuild_from_log as rebuild_sessions_from_log},
        storage::{
            CoreGenerationStore, DedupOutcome, RecordedEvent, RusqliteStorage, StoredSnapshot,
            TargetKey,
        },
        time::TestClock,
    };
    use prost::Message;

    fn domain() -> AuthorityDomainId {
        AuthorityDomainId {
            value: "authority-main".to_owned(),
        }
    }

    async fn append_events<S: Storage>(storage: &S, count: usize) {
        for _ in 0..count {
            storage
                .append(
                    &domain(),
                    StoredEventPayload {
                        kind: StoredEventKind::Observation as i32,
                        payload: Observation::default().encode_to_vec(),
                    },
                )
                .await
                .unwrap();
        }
    }

    fn test_policy() -> SessionCheckpointPolicy {
        SessionCheckpointPolicy {
            events_per_checkpoint: NonZeroU64::new(2).unwrap(),
            poll_interval: Duration::from_millis(1),
            retry_initial: Duration::from_millis(1),
            retry_max: Duration::from_millis(4),
        }
    }

    type FailureObservation = (CheckpointFailureStage, Option<u64>, u32, bool, &'static str);

    #[derive(Default)]
    struct RecordingObserver {
        failures: StdMutex<Vec<FailureObservation>>,
    }

    impl CheckpointObserver for RecordingObserver {
        fn observe_failure(
            &self,
            stage: CheckpointFailureStage,
            attempted_lsn: Option<u64>,
            consecutive_failures: u32,
            retryable: bool,
            error_class: &'static str,
        ) {
            self.failures.lock().expect("observer lock").push((
                stage,
                attempted_lsn,
                consecutive_failures,
                retryable,
                error_class,
            ));
        }
    }

    #[tokio::test]
    async fn threshold_writes_the_caught_up_head_and_recovery_applies_only_tail() {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let state = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        let observer = Arc::new(RecordingObserver::default());
        let writer = SessionCheckpointWriter::new(
            storage.clone(),
            state,
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 5,
                nanos: 0,
            })),
            test_policy(),
            observer,
        )
        .unwrap();

        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::EmptyLog
        );
        append_events(&storage, 1).await;
        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::NotDue {
                checkpoint_lsn: 0,
                current_lsn: 1
            }
        );
        append_events(&storage, 1).await;
        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                prior_lsn: 0,
                checkpoint_lsn: 2
            }
        );
        append_events(&storage, 1).await;
        let generation = storage
            .load_or_create_core_generation(&domain(), Generation { value: 99 })
            .await
            .unwrap();
        let recovered = crate::snapshot::recover_session_registry(&storage, &domain(), &generation)
            .await
            .unwrap();
        assert_eq!(recovered.checkpoint_lsn, 2);
        assert_eq!(recovered.replayed_event_count, 1);
        assert_eq!(recovered.recovered_through_lsn, 3);
    }

    #[tokio::test]
    async fn incompatible_checkpoint_is_repaired_below_the_normal_gap_threshold() {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        append_events(&storage, 1).await;
        storage
            .write_snapshot(&domain(), Lsn { value: 1 }, b"legacy-format-one".to_vec())
            .await
            .unwrap();
        let state = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        assert!(state.session_checkpoint_was_rejected());
        let writer = SessionCheckpointWriter::new(
            storage.clone(),
            state,
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 4,
                nanos: 0,
            })),
            SessionCheckpointPolicy::default(),
            Arc::new(RecordingObserver::default()),
        )
        .unwrap();

        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                prior_lsn: 0,
                checkpoint_lsn: 1,
            }
        );
        let stored = storage
            .load_latest_snapshot(&domain(), None)
            .await
            .unwrap()
            .unwrap();
        assert!(decode_compatible_session_checkpoint(
            &stored,
            &domain(),
            writer.state.core_generation(),
        )
        .is_ok());
    }

    #[tokio::test]
    async fn file_restart_recovers_both_session_consumers_and_full_replays_siblings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint-restart.sqlite3");
        let storage = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let runtime_session_id = RuntimeSessionId {
            value: "session-file".to_owned(),
        };
        let cursor_one = SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        };
        storage
            .append(
                &domain(),
                session_events::encode(&session_events::registered(
                    domain(),
                    SessionRegistered {
                        adapter_id: Some(adapter_id.clone()),
                        deployment_scope: "machine-a".to_owned(),
                        runtime_session_id: Some(runtime_session_id.clone()),
                        session_generation: Some(Generation { value: 1 }),
                        initial_state: Some(SessionState {
                            connectivity: SessionConnectivityState::Live as i32,
                            activity: SessionActivityState::Idle as i32,
                        }),
                        project: "patchbay".to_owned(),
                        cwd: "/work/patchbay".to_owned(),
                        name: "file".to_owned(),
                        model: "provider/a".to_owned(),
                        source_cursor: Some(cursor_one),
                        ..SessionRegistered::default()
                    },
                )),
            )
            .await
            .unwrap();
        let resource_identity = ResourceIdentity {
            adapter_id: Some(adapter_id.clone()),
            resource_kind: Some(ResourceKind {
                value: "pool".to_owned(),
            }),
            resource_id: Some(ResourceId {
                value: "pool-1".to_owned(),
            }),
        };
        storage
            .append(
                &domain(),
                patchbay_core::resource::events::encode(&ResourceStateEvent {
                    authority_domain_id: Some(domain()),
                    source_adapter_id: Some(adapter_id.clone()),
                    source_adapter_generation: Some(Generation { value: 1 }),
                    views: vec![ResourceViewStateUpdate {
                        resource_kind: Some(ResourceKind {
                            value: "pool".to_owned(),
                        }),
                        completeness: AdapterSnapshotSupport::Authoritative as i32,
                    }],
                    mutations: vec![ResourceStateMutation {
                        identity: Some(resource_identity),
                        from_revision_lsn: None,
                        mutation: Some(resource_state_mutation::Mutation::Upsert(
                            ResourceStateUpsert {
                                resource_payload: Some(PayloadEnvelope {
                                    payload: vec![1],
                                    content_type: PayloadContentType::Protobuf as i32,
                                    schema_ref: "pool.payload.v1".to_owned(),
                                }),
                                projection_payload: Some(PayloadEnvelope {
                                    payload: vec![2],
                                    content_type: PayloadContentType::Json as i32,
                                    schema_ref: "pool.projection.v1".to_owned(),
                                }),
                            },
                        )),
                    }],
                    observed_at: Some(prost_types::Timestamp {
                        seconds: 2,
                        nanos: 0,
                    }),
                }),
            )
            .await
            .unwrap();
        storage
            .append(
                &domain(),
                patchbay_core::security::events::encode(&patchbay_core::security::events::entered(
                    domain(),
                    SecurityLockdownEntered {
                        reason_code: "checkpoint_restart".to_owned(),
                        occurred_at: Some(prost_types::Timestamp {
                            seconds: 3,
                            nanos: 0,
                        }),
                        entered_by: Some(ActorEndpointRef {
                            actor_id: Some(ActorId {
                                value: "operator".to_owned(),
                            }),
                            ..ActorEndpointRef::default()
                        }),
                        invalidated_through_operator_session_generation: Some(Generation {
                            value: 1,
                        }),
                        affected_runtime_session_count: 1,
                    },
                )),
            )
            .await
            .unwrap();
        let state = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        let writer = SessionCheckpointWriter::new(
            storage.clone(),
            state.clone(),
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 4,
                nanos: 0,
            })),
            SessionCheckpointPolicy {
                events_per_checkpoint: NonZeroU64::new(1).unwrap(),
                ..test_policy()
            },
            Arc::new(RecordingObserver::default()),
        )
        .unwrap();
        assert!(matches!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                checkpoint_lsn: 3,
                ..
            }
        ));
        storage
            .append(
                &domain(),
                session_events::encode(&session_events::report_applied(
                    domain(),
                    SessionReportApplied {
                        report: Some(SessionReport {
                            adapter_id: Some(adapter_id),
                            deployment_scope: "machine-a".to_owned(),
                            runtime_session_id: Some(runtime_session_id),
                            session_generation: Some(Generation { value: 1 }),
                            connectivity: SessionConnectivityState::Stale as i32,
                            activity: SessionActivityState::Working as i32,
                            project: "patchbay".to_owned(),
                            cwd: "/work/patchbay".to_owned(),
                            name: "file".to_owned(),
                            model: "provider/b".to_owned(),
                            source_cursor: Some(SessionReportSourceCursor {
                                adapter_generation: Some(Generation { value: 1 }),
                                revision: 2,
                            }),
                            ..SessionReport::default()
                        }),
                        previous_source_cursor: Some(cursor_one),
                    },
                )),
            )
            .await
            .unwrap();
        drop(writer);
        drop(state);
        drop(storage);

        let reopened = RusqliteStorage::open(path.to_str().unwrap()).unwrap();
        let full = rebuild_sessions_from_log(&reopened, &domain())
            .await
            .unwrap();
        let aggregate = ProjectionState::rebuild(&reopened, &domain())
            .await
            .unwrap();
        assert_eq!(aggregate.session_recovery_checkpoint_lsn(), 3);
        assert_eq!(aggregate.session_replayed_event_count(), 1);
        let aggregate_snapshot = aggregate
            .materialize_session_snapshot(
                domain(),
                prost_types::Timestamp {
                    seconds: 4,
                    nanos: 0,
                },
            )
            .await;
        assert_eq!(aggregate_snapshot.sessions.len(), 1);
        assert_eq!(
            aggregate_snapshot.sessions[0].last_source_cursor,
            Some(SessionReportSourceCursor {
                adapter_generation: Some(Generation { value: 1 }),
                revision: 2,
            })
        );
        let resource_snapshot = aggregate
            .materialize_resource_snapshot(
                domain(),
                prost_types::Timestamp {
                    seconds: 4,
                    nanos: 0,
                },
            )
            .await;
        assert_eq!(resource_snapshot.resources.len(), 1);
        assert_eq!(resource_snapshot.snapshot_lsn, Some(Lsn { value: 4 }));
        assert!(aggregate.lockdown_state().await.active);

        let adapter_service = crate::adapter_service::AdapterControlServiceImpl::new(
            reopened,
            domain(),
            crate::adapter_service::AdapterEvidenceVerifier::new([("pi", "evidence")]).unwrap(),
        )
        .await
        .unwrap();
        let adapter_sessions = adapter_service.conformance_session_registry().await;
        assert_eq!(adapter_sessions.covered_through_lsn(), Some(3));
        assert_eq!(
            adapter_sessions.sessions().collect::<Vec<_>>(),
            full.sessions().collect::<Vec<_>>()
        );
        assert_eq!(
            adapter_sessions.tombstones().collect::<Vec<_>>(),
            full.tombstones().collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn complete_checkpoint_round_trips_tombstones_source_cursor_and_tail() {
        let storage = RusqliteStorage::open_in_memory().unwrap();
        let cursor_one = SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 1,
        };
        let adapter_id = AdapterId {
            value: "pi".to_owned(),
        };
        let runtime_session_id = RuntimeSessionId {
            value: "session-1".to_owned(),
        };
        storage
            .append(
                &domain(),
                session_events::encode(&session_events::registered(
                    domain(),
                    SessionRegistered {
                        adapter_id: Some(adapter_id.clone()),
                        deployment_scope: "machine-a".to_owned(),
                        runtime_session_id: Some(runtime_session_id.clone()),
                        session_generation: Some(Generation { value: 1 }),
                        initial_state: Some(SessionState {
                            connectivity: SessionConnectivityState::Live as i32,
                            activity: SessionActivityState::Idle as i32,
                        }),
                        project: "patchbay".to_owned(),
                        cwd: "/work/patchbay".to_owned(),
                        name: "one".to_owned(),
                        model: "provider/a".to_owned(),
                        source_cursor: Some(cursor_one),
                        ..SessionRegistered::default()
                    },
                )),
            )
            .await
            .unwrap();
        storage
            .append(
                &domain(),
                session_events::encode(&session_events::generation_bumped(
                    domain(),
                    SessionGenerationBumped {
                        adapter_id: Some(adapter_id.clone()),
                        deployment_scope: "machine-a".to_owned(),
                        runtime_session_id: Some(runtime_session_id.clone()),
                        from_generation: Some(Generation { value: 1 }),
                        to_generation: Some(Generation { value: 2 }),
                        initial_state: Some(SessionState {
                            connectivity: SessionConnectivityState::Live as i32,
                            activity: SessionActivityState::Idle as i32,
                        }),
                        project: "patchbay".to_owned(),
                        cwd: "/work/patchbay".to_owned(),
                        name: "two".to_owned(),
                        model: "provider/b".to_owned(),
                        source_cursor: Some(cursor_one),
                        ..SessionGenerationBumped::default()
                    },
                )),
            )
            .await
            .unwrap();
        let state = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        let writer = SessionCheckpointWriter::new(
            storage.clone(),
            state,
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 5,
                nanos: 0,
            })),
            SessionCheckpointPolicy {
                events_per_checkpoint: NonZeroU64::new(1).unwrap(),
                ..test_policy()
            },
            Arc::new(RecordingObserver::default()),
        )
        .unwrap();
        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                prior_lsn: 0,
                checkpoint_lsn: 2
            }
        );

        let generation = storage
            .load_or_create_core_generation(&domain(), Generation { value: 99 })
            .await
            .unwrap();
        let stored = storage
            .load_latest_snapshot(&domain(), None)
            .await
            .unwrap()
            .unwrap();
        let compatible =
            crate::snapshot::decode_compatible_session_checkpoint(&stored, &domain(), &generation)
                .unwrap();
        assert_eq!(compatible.registry.tombstones().count(), 1);
        let mut zero_generation = compatible.snapshot.clone();
        zero_generation.sessions[0].session_generation = Some(Generation { value: 0 });
        let zero_generation_stored = StoredSnapshot {
            event_id: stored.event_id.clone(),
            payload: encode_stored_session_checkpoint(
                &patchbay_contracts::patchbay::StoredSessionCheckpoint {
                    snapshot: Some(zero_generation),
                    tombstones: Vec::new(),
                },
            ),
        };
        assert!(crate::snapshot::decode_compatible_session_checkpoint(
            &zero_generation_stored,
            &domain(),
            &generation,
        )
        .is_err());
        let live = compatible.registry.sessions().next().unwrap();
        assert_eq!(live.identity.session_generation, Generation { value: 2 });
        assert_eq!(live.last_source_cursor, Some(cursor_one));
        let covered = storage
            .read_after(&domain(), Lsn { value: 0 })
            .await
            .unwrap()[0]
            .clone();
        assert!(compatible.registry.clone().observe(&covered).is_err());

        let cursor_two = SessionReportSourceCursor {
            adapter_generation: Some(Generation { value: 1 }),
            revision: 2,
        };
        storage
            .append(
                &domain(),
                session_events::encode(&session_events::report_applied(
                    domain(),
                    SessionReportApplied {
                        report: Some(SessionReport {
                            adapter_id: Some(adapter_id),
                            deployment_scope: "machine-a".to_owned(),
                            runtime_session_id: Some(runtime_session_id),
                            session_generation: Some(Generation { value: 2 }),
                            connectivity: SessionConnectivityState::Live as i32,
                            activity: SessionActivityState::Idle as i32,
                            project: "patchbay".to_owned(),
                            cwd: "/work/patchbay".to_owned(),
                            name: "two".to_owned(),
                            model: "provider/c".to_owned(),
                            source_cursor: Some(cursor_two),
                            ..SessionReport::default()
                        }),
                        previous_source_cursor: Some(cursor_one),
                    },
                )),
            )
            .await
            .unwrap();
        let recovered = crate::snapshot::recover_session_registry(&storage, &domain(), &generation)
            .await
            .unwrap();
        assert_eq!(recovered.checkpoint_lsn, 2);
        assert_eq!(recovered.replayed_event_count, 1);
        let fresh = rebuild_sessions_from_log(&storage, &domain())
            .await
            .unwrap();
        assert_eq!(
            recovered.registry.sessions().collect::<Vec<_>>(),
            fresh.sessions().collect::<Vec<_>>()
        );
        assert_eq!(
            recovered.registry.tombstones().collect::<Vec<_>>(),
            fresh.tombstones().collect::<Vec<_>>()
        );
        assert_eq!(
            recovered.registry.lockdown_active(),
            fresh.lockdown_active()
        );

        // Mutate an internally well-formed checkpoint so it disagrees with
        // the authoritative tail's previous cursor. Recovery must discard the
        // checkpoint, replay from zero, and force a prompt replacement even
        // though the normal event gap is below the policy threshold.
        let mut inconsistent = compatible.snapshot;
        inconsistent.sessions[0]
            .last_source_cursor
            .as_mut()
            .unwrap()
            .revision = 99;
        storage
            .write_snapshot(
                &domain(),
                Lsn { value: 2 },
                encode_stored_session_checkpoint(
                    &patchbay_contracts::patchbay::StoredSessionCheckpoint {
                        snapshot: Some(inconsistent),
                        tombstones: compatible
                            .registry
                            .tombstones()
                            .map(|tombstone| {
                                patchbay_contracts::patchbay::SessionCheckpointTombstone {
                                    adapter_id: Some(tombstone.adapter_id.clone()),
                                    deployment_scope: tombstone.deployment_scope.clone(),
                                    runtime_session_id: Some(tombstone.runtime_session_id.clone()),
                                    generation: Some(tombstone.superseded_generation),
                                    superseded_at_lsn: Some(Lsn {
                                        value: tombstone.superseded_at_lsn,
                                    }),
                                }
                            })
                            .collect(),
                    },
                ),
            )
            .await
            .unwrap();
        let fallback = crate::snapshot::recover_session_registry(&storage, &domain(), &generation)
            .await
            .unwrap();
        assert!(fallback.checkpoint_rejected);
        assert_eq!(fallback.checkpoint_lsn, 0);
        assert_eq!(fallback.replayed_event_count, 3);
        assert_eq!(
            fallback.registry.sessions().collect::<Vec<_>>(),
            fresh.sessions().collect::<Vec<_>>()
        );

        let restarted = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        assert_eq!(restarted.session_recovery_checkpoint_lsn(), 0);
        assert_eq!(restarted.session_replayed_event_count(), 3);
        assert!(restarted.session_checkpoint_was_rejected());
        let repair_writer = SessionCheckpointWriter::new(
            storage.clone(),
            restarted,
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 6,
                nanos: 0,
            })),
            SessionCheckpointPolicy::default(),
            Arc::new(RecordingObserver::default()),
        )
        .unwrap();
        assert_eq!(
            repair_writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                prior_lsn: 2,
                checkpoint_lsn: 3,
            }
        );
    }

    #[derive(Clone)]
    struct FailFirstSnapshot {
        inner: RusqliteStorage,
        fail: Arc<AtomicBool>,
    }

    impl CoreGenerationStore for FailFirstSnapshot {
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

    impl Storage for FailFirstSnapshot {
        async fn append(
            &self,
            authority_domain_id: &AuthorityDomainId,
            payload: StoredEventPayload,
        ) -> Result<EventId, StorageError> {
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
            if self.fail.swap(false, Ordering::SeqCst) {
                return Err(StorageError::WriteFailed {
                    message: "injected checkpoint failure".to_owned(),
                    retryable: true,
                });
            }
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
    }

    #[tokio::test]
    async fn failed_write_preserves_log_observes_failure_and_retries_without_restart() {
        let inner = RusqliteStorage::open_in_memory().unwrap();
        let storage = FailFirstSnapshot {
            inner: inner.clone(),
            fail: Arc::new(AtomicBool::new(true)),
        };
        append_events(&storage, 2).await;
        let state = ProjectionState::rebuild(&storage, &domain()).await.unwrap();
        let guard = state.submit_guard().await;
        let prior_payload = encode_stored_session_checkpoint(
            &state
                .materialize_session_checkpoint(
                    domain(),
                    prost_types::Timestamp {
                        seconds: 4,
                        nanos: 0,
                    },
                )
                .await,
        );
        drop(guard);
        inner
            .write_snapshot(&domain(), Lsn { value: 2 }, prior_payload)
            .await
            .unwrap();
        let prior = inner
            .load_latest_snapshot(&domain(), None)
            .await
            .unwrap()
            .unwrap();
        append_events(&storage, 2).await;
        let observer = Arc::new(RecordingObserver::default());
        let writer = SessionCheckpointWriter::new(
            storage.clone(),
            state,
            domain(),
            Arc::new(TestClock::new(prost_types::Timestamp {
                seconds: 5,
                nanos: 0,
            })),
            test_policy(),
            observer.clone(),
        )
        .unwrap();

        let error = writer.run_once().await.unwrap_err();
        assert_eq!(error.stage, CheckpointFailureStage::Write);
        assert_eq!(
            inner
                .read_after(&domain(), Lsn { value: 0 })
                .await
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            inner
                .load_latest_snapshot(&domain(), None)
                .await
                .unwrap()
                .unwrap(),
            prior,
            "a failed replacement must preserve the prior checkpoint byte-for-byte",
        );
        assert_eq!(
            observer.failures.lock().unwrap().as_slice(),
            &[(
                CheckpointFailureStage::Write,
                Some(4),
                1,
                true,
                "storage_transient",
            )]
        );
        assert_eq!(
            writer.run_once().await.unwrap(),
            CheckpointTickOutcome::Written {
                prior_lsn: 2,
                checkpoint_lsn: 4
            }
        );
        assert_eq!(
            inner
                .read_after(&domain(), Lsn { value: 0 })
                .await
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            inner
                .load_latest_snapshot(&domain(), None)
                .await
                .unwrap()
                .unwrap()
                .event_id
                .lsn,
            Some(Lsn { value: 4 })
        );
    }
}
