use std::{fmt, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, CommandTransition, DescendantGrant, DescendantGrantProvenance, EventId,
    FailureCode, GrantRevocationPolicy, Lsn, OperationState, StoredEventKind, StoredEventPayload,
};
use patchbay_core::{
    acceptance::Clock,
    audit::{AuditError, AuditReceipt, AuditSink},
    authority::{
        ingest_descendant_grant, AuthorityError, AuthorityRegistry, SpawnCompletionAction,
        SpawnDescendantTail,
    },
    storage::{validate_next_replay_event, AuditRecordDraft, RecordedEvent, Storage, StorageError},
};
use prost::Message;
use tokio::time::sleep;

use crate::decision_gate::CoreDecisionGate;

const DEFAULT_SCAN_INTERVAL: Duration = Duration::from_millis(100);

/// Typed failure from the load-bearing descendant-completion owner.
#[derive(Debug)]
pub enum SpawnCompletionError {
    Storage(StorageError),
    Authority(AuthorityError),
    Audit(AuditError),
    CorruptLog(String),
}

impl fmt::Display for SpawnCompletionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "spawn completion storage failure: {error}"),
            Self::Authority(error) => {
                write!(formatter, "spawn completion authority failure: {error}")
            }
            Self::Audit(error) => write!(formatter, "spawn completion audit failure: {error}"),
            Self::CorruptLog(message) => {
                write!(formatter, "spawn completion corrupt log: {message}")
            }
        }
    }
}

impl std::error::Error for SpawnCompletionError {}

impl From<StorageError> for SpawnCompletionError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<AuthorityError> for SpawnCompletionError {
    fn from(error: AuthorityError) -> Self {
        Self::Authority(error)
    }
}

impl From<AuditError> for SpawnCompletionError {
    fn from(error: AuditError) -> Self {
        Self::Audit(error)
    }
}

/// Single fail-closed owner of live descendant-grant completion.
pub struct SpawnCompletionDriver<S> {
    storage: S,
    authority_domain_id: AuthorityDomainId,
    decision_gate: CoreDecisionGate,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
    tail: SpawnDescendantTail,
    authority: AuthorityRegistry,
    cursor: u64,
    scan_interval: Duration,
}

impl<S> SpawnCompletionDriver<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    /// Rebuild the complete durable prefix and repair every incomplete spawn
    /// before returning control to the composition root.
    pub async fn bootstrap(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        decision_gate: CoreDecisionGate,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, SpawnCompletionError> {
        if authority_domain_id.value.is_empty() {
            return Err(SpawnCompletionError::CorruptLog(
                "authority_domain_id is empty".to_owned(),
            ));
        }
        let mut driver = Self {
            storage,
            authority_domain_id,
            decision_gate,
            audit,
            clock,
            tail: SpawnDescendantTail::new(),
            authority: AuthorityRegistry::new(),
            cursor: 0,
            scan_interval: DEFAULT_SCAN_INTERVAL,
        };
        let gate = driver.decision_gate.clone();
        let _guard = gate.acquire().await;
        driver.catch_up_locked().await?;
        Ok(driver)
    }

    /// Catch up and execute durable actions until both log tail and action fold
    /// are quiescent. The shared decision gate covers the complete cycle.
    pub async fn catch_up_to_quiescence(&mut self) -> Result<(), SpawnCompletionError> {
        let gate = self.decision_gate.clone();
        let _guard = gate.acquire().await;
        self.catch_up_locked().await
    }

    /// Continuously consume the durable tail. Any failure terminates the
    /// future so the composition root can fail the serving set.
    pub async fn run(mut self) -> Result<(), SpawnCompletionError> {
        loop {
            self.catch_up_to_quiescence().await?;
            // catch_up_to_quiescence returns only after an empty durable read
            // and no pending action; never sleep while holding the gate.
            sleep(self.scan_interval).await;
        }
    }

    async fn catch_up_locked(&mut self) -> Result<(), SpawnCompletionError> {
        loop {
            let events = self
                .storage
                .read_after(&self.authority_domain_id, Lsn { value: self.cursor })
                .await?;
            let empty_read = events.is_empty();
            for event in events {
                self.fold_event(&event)?;
            }

            if let Some(action) = self.tail.next_action()? {
                self.execute(action).await?;
                // Never mutate the tail optimistically. Read the committed
                // result back through the same fold on the next iteration.
                continue;
            }
            if empty_read {
                return Ok(());
            }
        }
    }

    fn fold_event(&mut self, event: &RecordedEvent) -> Result<(), SpawnCompletionError> {
        let validated = validate_next_replay_event(&self.authority_domain_id, self.cursor, event)
            .map_err(|error| SpawnCompletionError::CorruptLog(error.to_string()))?;

        self.tail.observe(event)?;
        self.authority.observe(event)?;
        self.cursor = validated.lsn;
        Ok(())
    }

    async fn execute(&mut self, action: SpawnCompletionAction) -> Result<(), SpawnCompletionError> {
        match action {
            SpawnCompletionAction::RecordAudit(completion) => {
                let mut draft = AuditRecordDraft::new(
                    self.clock.now(),
                    patchbay_contracts::patchbay::AuditEventKind::CommandCompleted,
                );
                draft.actor_id = Some(completion.subject_actor_id);
                draft.endpoint_id = completion.subject_endpoint_id;
                draft.device_id = completion.subject_device_id;
                draft.command_id = Some(completion.spawn_operation_id);
                draft.grant_id = Some(completion.spawning_grant_id);
                draft.target_scope = Some(completion.spawned_session_scope);
                draft.reason_code = "spawn_completion".to_owned();
                draft.source_event_id = Some(completion.completion_source_event_id);
                match self.audit.record(draft).await? {
                    AuditReceipt::Durable(event_id) => {
                        validate_written_event_id(&event_id, &self.authority_domain_id)?;
                    }
                    AuditReceipt::DiagnosticOnly => {
                        return Err(SpawnCompletionError::Audit(AuditError::NotDurable));
                    }
                }
            }
            SpawnCompletionAction::IssueDescendantGrant(issuance) => {
                let grant = DescendantGrant {
                    grant_id: Some(issuance.descendant_grant_id),
                    authority_domain_id: Some(issuance.authority_domain_id.clone()),
                    subject_actor_id: Some(issuance.subject_actor_id),
                    subject_endpoint_id: issuance.subject_endpoint_id,
                    target_scope: Some(issuance.spawned_session_scope),
                    allowed_operation_kinds: issuance
                        .allowed_operation_kinds
                        .iter()
                        .map(|kind| *kind as i32)
                        .collect(),
                    provenance: Some(DescendantGrantProvenance {
                        spawn_operation_id: Some(issuance.spawn_operation_id),
                        spawning_grant_id: Some(issuance.spawning_grant_id),
                    }),
                    created_at: Some(issuance.created_at),
                    revocation_policy: GrantRevocationPolicy::Continue as i32,
                    audit_id: Some(issuance.audit_id),
                    ..DescendantGrant::default()
                };
                ingest_descendant_grant(
                    &self.storage,
                    &mut self.authority,
                    &self.authority_domain_id,
                    grant,
                )
                .await?;
            }
            SpawnCompletionAction::CommitCompleted(completion) => {
                let command_id = completion.spawn_operation_id;
                let transition = CommandTransition {
                    command_id: Some(command_id.clone()),
                    from_state: completion.from_state as i32,
                    to_state: OperationState::Completed as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    correlations: completion.correlations,
                    ..CommandTransition::default()
                };
                let event_id = self
                    .storage
                    .append(
                        &self.authority_domain_id,
                        StoredEventPayload {
                            kind: StoredEventKind::CommandTransition as i32,
                            payload: transition.encode_to_vec(),
                        },
                    )
                    .await?;
                validate_written_event_id(&event_id, &self.authority_domain_id)?;
                eprintln!(
                    "patchbay-core-server: spawn completion finalized authority_domain_id={} command_id={}",
                    self.authority_domain_id.value, command_id.value
                );
            }
        }
        Ok(())
    }
}

fn validate_written_event_id(
    event_id: &EventId,
    expected_domain: &AuthorityDomainId,
) -> Result<(), SpawnCompletionError> {
    if event_id.authority_domain_id.as_ref() != Some(expected_domain)
        || event_id.lsn.as_ref().is_none_or(|lsn| lsn.value == 0)
    {
        return Err(SpawnCompletionError::CorruptLog(format!(
            "writer returned invalid event id {event_id:?}"
        )));
    }
    Ok(())
}
