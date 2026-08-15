use std::{collections::HashSet, fmt, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, CommandId, CommandTransition, DescendantGrant, DescendantGrantProvenance,
    EventId, FailureCode, GrantRevocationPolicy, Lsn, OperationState, StoredEventKind,
    StoredEventPayload,
};
use patchbay_core::{
    acceptance::Clock,
    audit::{AuditError, AuditReceipt, AuditSink},
    authority::{
        ingest_descendant_grant, AuthorityError, AuthorityRegistry, SpawnCompletionAction,
        SpawnDescendantTail,
    },
    session::next_spawn_promotion_excluding,
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
    history: Vec<RecordedEvent>,
    suppressed_promotions: HashSet<CommandId>,
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
            history: Vec::new(),
            suppressed_promotions: HashSet::new(),
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

            if let Some(promotion) = next_spawn_promotion_excluding(
                &self.authority_domain_id,
                &self.history,
                self.clock.now(),
                &self.suppressed_promotions,
            )
            .map_err(|error| SpawnCompletionError::CorruptLog(error.to_string()))?
            {
                let command_id = promotion
                    .accepted_claim
                    .as_ref()
                    .and_then(|accepted| accepted.claim.as_ref())
                    .and_then(|claim| claim.claim_operation_id.clone())
                    .ok_or_else(|| {
                        SpawnCompletionError::CorruptLog(
                            "promotion producer returned no claim operation id".to_owned(),
                        )
                    })?;
                if let Some(action) = self.tail.managed_promotion_action(promotion)? {
                    self.execute(action).await?;
                    // Never mutate a projection optimistically. Read the atomic
                    // promotion+audit pair through the same durable fold next.
                    continue;
                }
                // Exact accepted authority that was revoked or expired before
                // promotion suppresses the decision. Preserve the staged
                // candidate for explicit reconciliation; do not substitute a
                // new Grant id or ask storage to fail the transaction for us.
                // Exclude this permanently dead provenance from later scans so
                // it cannot head-of-line block an unrelated ready spawn.
                if !self.suppressed_promotions.insert(command_id) {
                    return Err(SpawnCompletionError::CorruptLog(
                        "promotion producer returned an excluded command".to_owned(),
                    ));
                }
                continue;
            }
            // Compatibility repair for durable pre-promotion histories only.
            // Current managed-spawn ingress never emits the SessionState fact
            // required by this leaf; it stages exact successor evidence above.
            if let Some(action) = self.tail.next_action()? {
                self.execute(action).await?;
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

        // The tail classifies ownership per command: managed SpawnClaim
        // histories are promotion-only, while unrelated pre-managed prefixes
        // remain eligible for one-way compatibility repair.
        self.tail.observe(event)?;
        self.authority.observe(event)?;
        self.history.push(event.clone());
        self.cursor = validated.lsn;
        Ok(())
    }

    async fn execute_promotion(
        &mut self,
        promotion: patchbay_contracts::patchbay::SpawnPromotionCommitted,
    ) -> Result<(), SpawnCompletionError> {
        let accepted = promotion
            .accepted_claim
            .as_ref()
            .and_then(|accepted| accepted.accepted_operation.as_ref())
            .ok_or_else(|| {
                SpawnCompletionError::CorruptLog(
                    "promotion producer returned no accepted operation".to_owned(),
                )
            })?;
        let operation = accepted.operation.as_ref().ok_or_else(|| {
            SpawnCompletionError::CorruptLog(
                "promotion producer returned no spawning operation".to_owned(),
            )
        })?;
        let command_id = operation.command_id.clone().ok_or_else(|| {
            SpawnCompletionError::CorruptLog(
                "promotion producer returned no spawning command id".to_owned(),
            )
        })?;
        let sender = operation.sender.as_ref().ok_or_else(|| {
            SpawnCompletionError::CorruptLog(
                "promotion producer returned no spawning sender".to_owned(),
            )
        })?;
        let descendant_target = promotion
            .authority
            .as_ref()
            .and_then(|authority| authority.descendant_grant.as_ref())
            .and_then(|grant| grant.target_scope.clone())
            .ok_or_else(|| {
                SpawnCompletionError::CorruptLog(
                    "promotion producer returned no descendant target".to_owned(),
                )
            })?;
        let occurred_at = promotion.committed_at.ok_or_else(|| {
            SpawnCompletionError::CorruptLog(
                "promotion producer returned no committed_at".to_owned(),
            )
        })?;
        let mut audit = AuditRecordDraft::new(
            occurred_at,
            patchbay_contracts::patchbay::AuditEventKind::CommandCompleted,
        );
        audit.actor_id = sender.actor_id.clone();
        audit.endpoint_id = sender.endpoint_id.clone();
        audit.device_id = sender.device_id.clone();
        audit.command_id = Some(command_id.clone());
        audit.grant_id = accepted.authorizing_grant_id.clone();
        audit.target_scope = Some(descendant_target);
        audit.reason_code = "spawn_completion".to_owned();
        let committed = self
            .storage
            .append_spawn_promotion_audited(&self.authority_domain_id, promotion, audit)
            .await?;
        validate_written_event_id(&committed.source_event_id, &self.authority_domain_id)?;
        validate_written_event_id(&committed.audit_event_id, &self.authority_domain_id)?;
        eprintln!(
            "patchbay-core-server: spawn promotion committed authority_domain_id={} command_id={}",
            self.authority_domain_id.value, command_id.value
        );
        Ok(())
    }

    async fn execute(&mut self, action: SpawnCompletionAction) -> Result<(), SpawnCompletionError> {
        match action {
            SpawnCompletionAction::CommitPromotion(promotion) => {
                self.execute_promotion(*promotion).await?;
            }
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
                        continuation_authority: None,
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
