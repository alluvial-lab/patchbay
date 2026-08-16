//! Core-owned, replayable diagnostic projections and query validation.

use std::collections::HashMap;

use patchbay_contracts::patchbay::{
    diagnostics_query, spawn_claim_event, AcceptedOperation, AdapterCapabilitySummary,
    AdapterDiagnosticState, AdapterId, AdapterRegistration, AdapterStatus, AdapterStatusPage,
    AdapterStatusQuery, AuditEventKind, AuditQuery, AuditRecord, AuthorityDomainId,
    CommandHistoryEntry, CommandId, CommandInspection, CommandInspectionQuery,
    CommandInspectionResult, CommandSummary, DiagnosticsQuery, EventId, FailureCode, Observation,
    Operation, OperationKind, OperationState, Revocation, SpawnClaimDisposition, SpawnClaimEvent,
    SpawnPriorWorkDisposition, SpawnPromotionCommitted, SpawnRequest, StoredEventKind, TargetScope,
    TargetScopeKind,
};
use prost::Message;
use prost_types::Timestamp;

use crate::{
    acceptance::{CommandIndex, TargetBinding, TargetNotFound, TargetResolver},
    adapter::{AdapterRecord, CapabilityValidationContext, ValidatedAdapterCapability},
    session::{effective_connectivity, SessionRegistry},
    storage::{AuditPageSpec, RecordedEvent, StorageError, TargetKey},
};

pub mod adapter_report;
pub use adapter_report::{
    ingest_adapter_diagnostic, validate_adapter_diagnostic_report, AdapterDiagnosticReceipt,
    AdapterDiagnosticRejection, ValidatedAdapterDiagnostic,
};

pub const AUDIT_DEFAULT_LIMIT: u16 = 100;
pub const AUDIT_MAX_LIMIT: u16 = 500;
pub const COMMAND_DEFAULT_LIMIT: u16 = 50;
pub const COMMAND_MAX_LIMIT: u16 = 200;
pub const ADAPTER_DEFAULT_LIMIT: u16 = 100;
pub const ADAPTER_MAX_LIMIT: u16 = 500;
pub const MAX_RECENT_ADAPTER_DIAGNOSTICS: usize = 100;
pub const DIAGNOSTICS_SCHEMA: &str = "patchbay.DiagnosticsQuery";

/// Resolver used only by the diagnostics execution path. Ordinary Submit
/// continues to use the session resolver and cannot turn a target kind into a
/// core-local target.
#[derive(Debug, Default, Clone, Copy)]
pub struct AuthorityDomainTargetResolver;

impl TargetResolver for AuthorityDomainTargetResolver {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        operation: &Operation,
        _spawn_request: Option<&SpawnRequest>,
    ) -> Result<TargetBinding, TargetNotFound> {
        let target_scope =
            operation
                .target_scope
                .as_ref()
                .ok_or_else(|| TargetNotFound::NotFound {
                    target: "diagnostics operation is missing target_scope".to_owned(),
                })?;
        if OperationKind::try_from(operation.kind).ok() != Some(OperationKind::Query)
            || TargetScopeKind::try_from(target_scope.kind).ok()
                != Some(TargetScopeKind::AuthorityDomain)
        {
            return Err(TargetNotFound::NotFound {
                target: "diagnostics target is not an authority domain".to_owned(),
            });
        }
        Ok(TargetBinding::AuthorityDomain(authority_domain_id.clone()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticsError {
    #[error("invalid diagnostics query: {0}")]
    InvalidQuery(String),
    #[error("corrupt diagnostics event: {0}")]
    CorruptEvent(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

#[derive(Debug, Clone)]
pub enum ValidatedDiagnosticsQuery {
    Audit(AuditPageSpec),
    Command(CommandInspectionQuery),
    Adapters(patchbay_contracts::patchbay::AdapterStatusQuery),
}

#[derive(Debug, Clone, PartialEq)]
struct CommandTimeline {
    summary: CommandSummary,
    accepted_event_id: EventId,
    grant_id: Option<patchbay_contracts::patchbay::GrantId>,
    current_state: OperationState,
    failure_code: FailureCode,
    terminal_event_id: Option<EventId>,
    claim_disposition: Option<SpawnClaimDisposition>,
    history: Vec<CommandHistoryEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticsProjection {
    commands: HashMap<CommandId, CommandTimeline>,
    /// Canonical command projection retained so generated promotion envelopes
    /// use the same exact pre-state validation and terminal fold as acceptance.
    command_index: CommandIndex,
    adapters: HashMap<AdapterId, AdapterRecord>,
    /// Lifecycle state observed since the current process started. Historical
    /// lifecycle records are deliberately not copied into this map: they are
    /// evidence, not proof of current attachment after a restart.
    current_process_adapters: HashMap<AdapterId, AdapterDiagnosticState>,
    lifecycle: HashMap<AdapterId, AuditRecord>,
    sessions: SessionRegistry,
    recent_diagnostics: HashMap<AdapterId, Vec<patchbay_contracts::patchbay::AuditRecord>>,
}

impl DiagnosticsProjection {
    pub fn new(authority_domain_id: AuthorityDomainId) -> Result<Self, DiagnosticsError> {
        let sessions = SessionRegistry::new(authority_domain_id)
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        Ok(Self::with_session_registry(sessions))
    }

    /// Seed the embedded session view from the same accepted checkpoint as the
    /// canonical session projection while replaying all diagnostics-owned
    /// sibling state from the authoritative log.
    #[must_use]
    pub fn with_session_registry(sessions: SessionRegistry) -> Self {
        Self {
            commands: HashMap::new(),
            command_index: CommandIndex::new(),
            adapters: HashMap::new(),
            current_process_adapters: HashMap::new(),
            lifecycle: HashMap::new(),
            sessions,
            recent_diagnostics: HashMap::new(),
        }
    }

    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError> {
        let event_domain = event.event_id.authority_domain_id.as_ref().ok_or_else(|| {
            DiagnosticsError::CorruptEvent("event has no authority domain".to_owned())
        })?;
        if event_domain.value.is_empty() {
            return Err(DiagnosticsError::CorruptEvent(
                "event has an empty authority domain".to_owned(),
            ));
        }
        if event.event_id.lsn.is_none() {
            return Err(DiagnosticsError::CorruptEvent(
                "event has no LSN".to_owned(),
            ));
        }
        let kind = StoredEventKind::try_from(event.payload.kind)
            .map_err(|_| DiagnosticsError::CorruptEvent("unknown stored event kind".to_owned()))?;
        if kind == StoredEventKind::Unspecified {
            return Err(DiagnosticsError::CorruptEvent(
                "diagnostics replay event kind is unspecified".to_owned(),
            ));
        }
        // Apply command semantics through the canonical projection first. In
        // particular, SpawnPromotionCommitted must pass CommandIndex's exact
        // operation/grant/pre-state/deferred-result checks before diagnostics
        // can publish a completed timeline entry.
        let mut command_index = self.command_index.clone();
        command_index
            .apply(event)
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;

        // The embedded session view owns exact-envelope classification for its
        // session and lockdown events. A compact checkpoint cannot retain that
        // envelope ledger for the covered prefix, so skip only the already
        // validated prefix and continue strict classification for every tail
        // event. Diagnostics-owned sibling state still folds the full log.
        let lsn = event.event_id.lsn.as_ref().expect("validated above").value;
        if self
            .sessions
            .covered_through_lsn()
            .is_none_or(|covered| lsn > covered)
        {
            self.sessions
                .observe(event)
                .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        }
        match kind {
            StoredEventKind::Operation => {
                let accepted = AcceptedOperation::decode(event.payload.payload.as_slice())
                    .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
                self.observe_accepted_operation(event, accepted)?;
            }
            StoredEventKind::CommandTransition => {
                let transition = patchbay_contracts::patchbay::CommandTransition::decode(
                    event.payload.payload.as_slice(),
                )
                .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
                let command_id = transition.command_id.ok_or_else(|| {
                    DiagnosticsError::CorruptEvent("transition has no command id".to_owned())
                })?;
                let timeline = self.commands.get_mut(&command_id).ok_or_else(|| {
                    DiagnosticsError::CorruptEvent(
                        "transition precedes command operation".to_owned(),
                    )
                })?;
                let from = OperationState::try_from(transition.from_state).map_err(|_| {
                    DiagnosticsError::CorruptEvent("unknown transition from_state".to_owned())
                })?;
                let to = OperationState::try_from(transition.to_state).map_err(|_| {
                    DiagnosticsError::CorruptEvent("unknown transition to_state".to_owned())
                })?;
                if timeline.current_state != from {
                    return Err(DiagnosticsError::CorruptEvent(
                        "transition from_state does not match projection".to_owned(),
                    ));
                }
                timeline.current_state = to;
                timeline.failure_code =
                    FailureCode::try_from(transition.failure_code).map_err(|_| {
                        DiagnosticsError::CorruptEvent("unknown transition failure_code".to_owned())
                    })?;
                if to != OperationState::Accepted {
                    timeline.history.push(CommandHistoryEntry {
                        event_id: Some(event.event_id.clone()),
                        state: to as i32,
                        failure_code: timeline.failure_code as i32,
                        occurred_at: transition.committed_at,
                        correlations: transition.correlations,
                    });
                }
                if is_terminal(to) {
                    timeline.terminal_event_id = Some(event.event_id.clone());
                }
            }
            StoredEventKind::SpawnClaim => self.observe_spawn_claim(event)?,
            StoredEventKind::SpawnPromotionCommitted => {
                self.observe_spawn_promotion(event, &command_index)?;
            }
            StoredEventKind::Observation => self.observe_observation(event)?,
            StoredEventKind::AuditRecord => self.observe_audit(event)?,
            StoredEventKind::Revocation => self.observe_revocation(event)?,
            StoredEventKind::SessionState
            | StoredEventKind::Elicitation
            | StoredEventKind::ResourceState
            | StoredEventKind::SpawnExecutionEvidence
            | StoredEventKind::SpawnSuccessorEvidenceStaged
            | StoredEventKind::QuarantinedRuntimeEvidence
            | StoredEventKind::Grant
            | StoredEventKind::DescendantGrant
            | StoredEventKind::OperatorRecord
            | StoredEventKind::ControlSurfacePrincipal
            | StoredEventKind::OperatorSessionRevocation
            | StoredEventKind::ControlSurfaceRevocation
            | StoredEventKind::SecurityLockdown => {}
            StoredEventKind::Unspecified => unreachable!("rejected before dispatch"),
        }
        self.command_index = command_index;
        Ok(())
    }

    fn observe_revocation(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError> {
        // A revocation is one durable event even when it carries multiple
        // command effects. Install none of its diagnostic changes unless all
        // effects validate.
        let mut staged = self.clone();
        staged.observe_revocation_in_place(event)?;
        *self = staged;
        Ok(())
    }

    fn observe_revocation_in_place(
        &mut self,
        event: &RecordedEvent,
    ) -> Result<(), DiagnosticsError> {
        let revocation = Revocation::decode(event.payload.payload.as_slice())
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        let grant_id = revocation.grant_id.ok_or_else(|| {
            DiagnosticsError::CorruptEvent("revocation has no grant id".to_owned())
        })?;
        for effect in revocation.command_effects {
            let command_id = effect.command_id.clone().ok_or_else(|| {
                DiagnosticsError::CorruptEvent("revocation effect has no command id".to_owned())
            })?;
            let timeline = self.commands.get_mut(&command_id).ok_or_else(|| {
                DiagnosticsError::CorruptEvent(
                    "revocation effect precedes command operation".to_owned(),
                )
            })?;
            if timeline.grant_id.as_ref() != Some(&grant_id) {
                return Err(DiagnosticsError::CorruptEvent(
                    "revocation effect grant does not match command provenance".to_owned(),
                ));
            }
            if is_terminal(timeline.current_state) {
                return Err(DiagnosticsError::CorruptEvent(
                    "revocation effect targets terminal command".to_owned(),
                ));
            }
            let from = OperationState::try_from(effect.from_state).map_err(|_| {
                DiagnosticsError::CorruptEvent("unknown revocation from_state".to_owned())
            })?;
            let to = OperationState::try_from(effect.to_state).map_err(|_| {
                DiagnosticsError::CorruptEvent("unknown revocation to_state".to_owned())
            })?;
            let failure = FailureCode::try_from(effect.failure_code).map_err(|_| {
                DiagnosticsError::CorruptEvent("unknown revocation failure_code".to_owned())
            })?;
            if timeline.current_state != from
                || !matches!(
                    (from, to, failure),
                    (
                        OperationState::Accepted
                            | OperationState::Delivered
                            | OperationState::Running,
                        OperationState::Cancelled,
                        FailureCode::Cancelled
                    ) | (
                        OperationState::Accepted,
                        OperationState::Rejected,
                        FailureCode::AuthorizationDenied
                    )
                )
            {
                return Err(DiagnosticsError::CorruptEvent(
                    "invalid revocation effect adjacency".to_owned(),
                ));
            }
            timeline.current_state = to;
            timeline.failure_code = failure;
            timeline.history.push(CommandHistoryEntry {
                event_id: Some(event.event_id.clone()),
                state: to as i32,
                failure_code: failure as i32,
                occurred_at: revocation.revoked_at,
                correlations: Vec::new(),
            });
            timeline.terminal_event_id = Some(event.event_id.clone());
        }
        Ok(())
    }

    fn observe_audit(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError> {
        let record = AuditRecord::decode(event.payload.payload.as_slice())
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        let kind = AuditEventKind::try_from(record.kind)
            .map_err(|_| DiagnosticsError::CorruptEvent("unknown audit event kind".to_owned()))?;
        if matches!(
            kind,
            AuditEventKind::AdapterAttached
                | AuditEventKind::AdapterDetached
                | AuditEventKind::AdapterFailed
        ) {
            if let Some(adapter_id) = record
                .actor_id
                .clone()
                .filter(|id| !id.value.is_empty())
                .map(|id| AdapterId { value: id.value })
            {
                self.lifecycle.insert(adapter_id.clone(), record.clone());
                // This map is empty after a rebuild. Therefore only lifecycle
                // records observed by the running process can establish a
                // current state; replayed pre-restart records remain history.
                let state = match kind {
                    AuditEventKind::AdapterAttached => AdapterDiagnosticState::Attached,
                    AuditEventKind::AdapterDetached => AdapterDiagnosticState::Detached,
                    AuditEventKind::AdapterFailed => AdapterDiagnosticState::Failed,
                    _ => unreachable!("lifecycle kind was checked above"),
                };
                self.current_process_adapters.insert(adapter_id, state);
            }
            return Ok(());
        }
        if kind != AuditEventKind::AdapterDiagnosticReported {
            return Ok(());
        }
        let detail = record.adapter_diagnostic.as_ref().ok_or_else(|| {
            DiagnosticsError::CorruptEvent(
                "adapter diagnostic audit is missing safe detail".to_owned(),
            )
        })?;
        let adapter_id = detail.adapter_id.clone().ok_or_else(|| {
            DiagnosticsError::CorruptEvent(
                "adapter diagnostic detail is missing adapter id".to_owned(),
            )
        })?;
        let severity =
            patchbay_contracts::patchbay::AdapterDiagnosticSeverity::try_from(detail.severity)
                .map_err(|_| {
                    DiagnosticsError::CorruptEvent(
                        "adapter diagnostic severity is unknown".to_owned(),
                    )
                })?;
        let operation_kind = patchbay_contracts::patchbay::OperationKind::try_from(
            detail.operation_kind,
        )
        .map_err(|_| {
            DiagnosticsError::CorruptEvent(
                "adapter diagnostic operation kind is unknown".to_owned(),
            )
        })?;
        if adapter_id.value.is_empty()
            || detail.adapter_generation.is_none()
            || detail.count == 0
            || detail.count > 1000
            || severity == patchbay_contracts::patchbay::AdapterDiagnosticSeverity::Unspecified
            || record.reason_code.is_empty()
        {
            return Err(DiagnosticsError::CorruptEvent(
                "adapter diagnostic audit detail is invalid".to_owned(),
            ));
        }
        let source = record.source_event_id.as_ref().ok_or_else(|| {
            DiagnosticsError::CorruptEvent(
                "adapter diagnostic audit is missing source event".to_owned(),
            )
        })?;
        let source_lsn = source.lsn.as_ref().ok_or_else(|| {
            DiagnosticsError::CorruptEvent("adapter diagnostic audit source has no LSN".to_owned())
        })?;
        if source.authority_domain_id.as_ref() != event.event_id.authority_domain_id.as_ref()
            || source_lsn.value >= event_lsn(event)
        {
            return Err(DiagnosticsError::CorruptEvent(
                "adapter diagnostic source is not prior in the same domain".to_owned(),
            ));
        }
        let records = self.recent_diagnostics.entry(adapter_id).or_default();
        records.push(record);
        if records.len() > MAX_RECENT_ADAPTER_DIAGNOSTICS {
            let remove = records.len() - MAX_RECENT_ADAPTER_DIAGNOSTICS;
            records.drain(0..remove);
        }
        let _ = operation_kind;
        Ok(())
    }

    fn observe_accepted_operation(
        &mut self,
        event: &RecordedEvent,
        accepted: AcceptedOperation,
    ) -> Result<(), DiagnosticsError> {
        let operation = accepted.operation.ok_or_else(|| {
            DiagnosticsError::CorruptEvent("accepted operation has no operation".to_owned())
        })?;
        let Some(command_id) = operation.command_id.clone() else {
            return Ok(());
        };
        let summary = CommandSummary {
            command_id: Some(command_id.clone()),
            sender: operation.sender,
            recipient: operation.recipient,
            kind: operation.kind,
            target_scope: operation.target_scope,
            correlations: operation.correlations,
            validity_window: operation.validity_window,
            submitted_at: operation.submitted_at,
        };
        if self.commands.contains_key(&command_id) {
            return Err(DiagnosticsError::CorruptEvent(
                "duplicate accepted command identity".to_owned(),
            ));
        }
        self.commands.insert(
            command_id,
            CommandTimeline {
                summary: summary.clone(),
                accepted_event_id: event.event_id.clone(),
                grant_id: accepted.authorizing_grant_id,
                current_state: OperationState::Accepted,
                failure_code: FailureCode::Unspecified,
                terminal_event_id: None,
                claim_disposition: None,
                history: vec![CommandHistoryEntry {
                    event_id: Some(event.event_id.clone()),
                    state: OperationState::Accepted as i32,
                    failure_code: FailureCode::Unspecified as i32,
                    occurred_at: summary.submitted_at,
                    correlations: summary.correlations.clone(),
                }],
            },
        );
        Ok(())
    }

    fn observe_spawn_claim(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError> {
        let claim = SpawnClaimEvent::decode(event.payload.payload.as_slice())
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        match claim.mutation.ok_or_else(|| {
            DiagnosticsError::CorruptEvent("spawn claim event has no mutation".to_owned())
        })? {
            spawn_claim_event::Mutation::Accepted(accepted) => {
                for effect in &accepted.prior_work_effects {
                    let command_id = effect.command_id.as_ref().ok_or_else(|| {
                        DiagnosticsError::CorruptEvent(
                            "prior-work effect has no command id".to_owned(),
                        )
                    })?;
                    let timeline = self.commands.get_mut(command_id).ok_or_else(|| {
                        DiagnosticsError::CorruptEvent(
                            "prior-work effect precedes command operation".to_owned(),
                        )
                    })?;
                    let prior = OperationState::try_from(effect.prior_state).map_err(|_| {
                        DiagnosticsError::CorruptEvent(
                            "prior-work effect has unknown state".to_owned(),
                        )
                    })?;
                    if timeline.current_state != prior {
                        return Err(DiagnosticsError::CorruptEvent(
                            "prior-work effect state does not match projection".to_owned(),
                        ));
                    }
                    match SpawnPriorWorkDisposition::try_from(effect.disposition).ok() {
                        Some(SpawnPriorWorkDisposition::SupersededBeforeOffer) => {
                            timeline.current_state = OperationState::Superseded;
                            timeline.failure_code = FailureCode::Superseded;
                            timeline.terminal_event_id = Some(event.event_id.clone());
                            timeline.history.push(CommandHistoryEntry {
                                event_id: Some(event.event_id.clone()),
                                state: OperationState::Superseded as i32,
                                failure_code: FailureCode::Superseded as i32,
                                occurred_at: None,
                                correlations: Vec::new(),
                            });
                        }
                        Some(SpawnPriorWorkDisposition::QuiesceOutcomeReconciliation) => {}
                        Some(SpawnPriorWorkDisposition::Unspecified) | None => {
                            return Err(DiagnosticsError::CorruptEvent(
                                "prior-work effect has unknown disposition".to_owned(),
                            ));
                        }
                    }
                }
                let command_id = accepted
                    .claim
                    .as_ref()
                    .and_then(|claim| claim.claim_operation_id.as_ref())
                    .cloned()
                    .ok_or_else(|| {
                        DiagnosticsError::CorruptEvent(
                            "accepted spawn claim has no command id".to_owned(),
                        )
                    })?;
                let accepted_operation = accepted.accepted_operation.ok_or_else(|| {
                    DiagnosticsError::CorruptEvent(
                        "accepted spawn claim has no operation".to_owned(),
                    )
                })?;
                self.observe_accepted_operation(event, accepted_operation)?;
                self.commands
                    .get_mut(&command_id)
                    .expect("accepted command was just projected")
                    .claim_disposition = Some(SpawnClaimDisposition::Active);
                Ok(())
            }
            spawn_claim_event::Mutation::DispositionChanged(change) => {
                let command_id = change.claim_operation_id.as_ref().ok_or_else(|| {
                    DiagnosticsError::CorruptEvent(
                        "spawn claim disposition has no command id".to_owned(),
                    )
                })?;
                let from =
                    SpawnClaimDisposition::try_from(change.from_disposition).map_err(|_| {
                        DiagnosticsError::CorruptEvent(
                            "spawn claim disposition has unknown source".to_owned(),
                        )
                    })?;
                let to = SpawnClaimDisposition::try_from(change.to_disposition).map_err(|_| {
                    DiagnosticsError::CorruptEvent(
                        "spawn claim disposition has unknown target".to_owned(),
                    )
                })?;
                let timeline = self.commands.get_mut(command_id).ok_or_else(|| {
                    DiagnosticsError::CorruptEvent(
                        "spawn claim disposition precedes command operation".to_owned(),
                    )
                })?;
                if timeline.claim_disposition != Some(from)
                    || !crate::session::allowed_spawn_claim_transition(from, to)
                {
                    return Err(DiagnosticsError::CorruptEvent(
                        "spawn claim disposition disagrees with projected claim state".to_owned(),
                    ));
                }
                timeline.claim_disposition = Some(to);
                timeline.history.push(CommandHistoryEntry {
                    event_id: Some(event.event_id.clone()),
                    state: timeline.current_state as i32,
                    failure_code: match to {
                        SpawnClaimDisposition::PoisonedPendingReconciliation
                        | SpawnClaimDisposition::TargetAbandoned => {
                            FailureCode::ExecutionOutcomeUnknown as i32
                        }
                        SpawnClaimDisposition::ReleasedNoExternalEffect
                        | SpawnClaimDisposition::Promoted => FailureCode::Unspecified as i32,
                        SpawnClaimDisposition::Unspecified | SpawnClaimDisposition::Active => {
                            unreachable!("illegal claim target was rejected by the shared registry")
                        }
                    },
                    occurred_at: None,
                    correlations: Vec::new(),
                });
                Ok(())
            }
        }
    }

    fn observe_spawn_promotion(
        &mut self,
        event: &RecordedEvent,
        command_index: &CommandIndex,
    ) -> Result<(), DiagnosticsError> {
        // CommandIndex already decoded and validated this envelope. Decode only
        // to identify the completed timeline and carry its committed timestamp.
        let promotion = SpawnPromotionCommitted::decode(event.payload.payload.as_slice())
            .expect("canonical command fold decoded the promotion envelope");
        let accepted_operation = promotion
            .accepted_claim
            .as_ref()
            .and_then(|accepted| accepted.accepted_operation.as_ref())
            .expect("canonical command fold validated the accepted operation");
        let operation = accepted_operation
            .operation
            .as_ref()
            .expect("canonical command fold validated the operation");
        let command_id = operation
            .command_id
            .as_ref()
            .expect("canonical command fold validated the command id");
        let canonical = command_index.get_command(command_id).ok_or_else(|| {
            DiagnosticsError::CorruptEvent(
                "spawn promotion completed an unknown command".to_owned(),
            )
        })?;
        let timeline = self.commands.get_mut(command_id).ok_or_else(|| {
            DiagnosticsError::CorruptEvent("spawn promotion precedes command operation".to_owned())
        })?;
        if canonical.state != OperationState::Completed
            || canonical.terminal_lsn != event.event_id.lsn.as_ref().map(|lsn| lsn.value)
            || !matches!(
                timeline.current_state,
                OperationState::Delivered | OperationState::Running
            )
            || timeline.terminal_event_id.is_some()
        {
            return Err(DiagnosticsError::CorruptEvent(
                "spawn promotion disagrees with diagnostic command pre-state".to_owned(),
            ));
        }
        timeline.current_state = OperationState::Completed;
        timeline.failure_code = FailureCode::Unspecified;
        timeline.terminal_event_id = Some(event.event_id.clone());
        timeline.claim_disposition = Some(SpawnClaimDisposition::Promoted);
        timeline.history.push(CommandHistoryEntry {
            event_id: Some(event.event_id.clone()),
            state: OperationState::Completed as i32,
            failure_code: FailureCode::Unspecified as i32,
            occurred_at: promotion.committed_at,
            correlations: operation.correlations.clone(),
        });
        Ok(())
    }

    fn observe_observation(&mut self, event: &RecordedEvent) -> Result<(), DiagnosticsError> {
        let observation = Observation::decode(event.payload.payload.as_slice())
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        let Some(payload) = observation.payload.as_ref() else {
            return Ok(());
        };
        if payload.schema_ref != "patchbay.AdapterRegistration" {
            return Ok(());
        }
        let registration = AdapterRegistration::decode(payload.payload.as_slice())
            .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        let adapter_id = registration.adapter_id.clone().ok_or_else(|| {
            DiagnosticsError::CorruptEvent("adapter registration has no adapter id".to_owned())
        })?;
        let capability = registration.capability.as_ref().ok_or_else(|| {
            DiagnosticsError::CorruptEvent("adapter registration has no capability".to_owned())
        })?;
        let validated_capability = ValidatedAdapterCapability::try_from_wire(
            capability,
            CapabilityValidationContext::Replay,
        )
        .map_err(|error| DiagnosticsError::CorruptEvent(error.to_string()))?;
        self.adapters.insert(
            adapter_id.clone(),
            AdapterRecord {
                registration,
                validated_capability,
                attach_event_id: event.event_id.clone(),
            },
        );
        self.current_process_adapters
            .insert(adapter_id, AdapterDiagnosticState::Attached);
        Ok(())
    }

    /// Rebuilds intentionally do not infer current liveness from historical
    /// registration or lifecycle records. A live process calls `observe` for
    /// fresh attachment/lifecycle evidence after startup.
    pub fn reset_adapter_liveness(&mut self) {
        self.current_process_adapters.clear();
    }

    /// The server uses this set to carry current-process evidence into an
    /// as-of projection. It includes fresh detached/failed states as well as
    /// attached states; the name is retained for the existing server port.
    pub fn live_adapter_ids(&self) -> impl Iterator<Item = &AdapterId> {
        self.current_process_adapters.keys()
    }

    pub fn mark_adapter_live(&mut self, adapter_id: AdapterId) {
        // `diagnostics_at` resets freshness before copying the set from the
        // hot projection. Re-read the lifecycle state from the selected
        // durable prefix so an as-of query preserves a fresh detach/failure
        // instead of blindly turning every copied adapter into ATTACHED.
        let state = self
            .lifecycle
            .get(&adapter_id)
            .and_then(|record| AuditEventKind::try_from(record.kind).ok())
            .map(|kind| match kind {
                AuditEventKind::AdapterAttached => AdapterDiagnosticState::Attached,
                AuditEventKind::AdapterDetached => AdapterDiagnosticState::Detached,
                AuditEventKind::AdapterFailed => AdapterDiagnosticState::Failed,
                _ => AdapterDiagnosticState::Attached,
            })
            .unwrap_or(AdapterDiagnosticState::Attached);
        self.current_process_adapters.insert(adapter_id, state);
    }

    pub fn adapter_page(
        &self,
        query: &AdapterStatusQuery,
        as_of: u64,
    ) -> Result<AdapterStatusPage, DiagnosticsError> {
        let limit = query.limit.unwrap_or(u32::from(ADAPTER_DEFAULT_LIMIT));
        let recent_limit = validate_recent_diagnostic_limit(query.recent_diagnostic_limit)?;
        if limit == 0 || limit > u32::from(ADAPTER_MAX_LIMIT) {
            return Err(DiagnosticsError::InvalidQuery(
                "adapter limit is out of bounds".to_owned(),
            ));
        }
        let requested: std::collections::HashSet<_> = query.adapter_ids.iter().cloned().collect();
        let mut records: Vec<_> = self
            .adapters
            .iter()
            .filter(|(id, _)| {
                (requested.is_empty() || requested.contains(id))
                    && id.value.as_str() > query.after_adapter_id.as_str()
            })
            .collect();
        records.sort_by(|(left, _), (right, _)| left.value.cmp(&right.value));
        let has_more = records.len() > limit as usize;
        records.truncate(limit as usize);
        let next_after_adapter_id = if has_more {
            records
                .last()
                .map_or_else(String::new, |(id, _)| id.value.clone())
        } else {
            String::new()
        };
        let adapters =
            records
                .into_iter()
                .map(|(adapter_id, record)| {
                    let registration = &record.registration;
                    let capability = registration.capability.as_ref().map(|capability| {
                        AdapterCapabilitySummary {
                            supported_operation_kinds: capability.supported_operation_kinds.clone(),
                            supported_target_spec_shapes: capability
                                .supported_target_spec_shapes
                                .clone(),
                            streaming_support: capability.streaming_support,
                            session_snapshot_support: capability.session_snapshot_support,
                            cancellation_support: capability.cancellation_support,
                            session_replacement_support: capability.session_replacement_support,
                            attachment_method_kind: capability
                                .attachment_method
                                .as_ref()
                                .map_or_else(String::new, |method| method.kind.clone()),
                            attachment_descriptor_content_type: capability
                                .attachment_method
                                .as_ref()
                                .map_or(0, |method| method.descriptor_content_type),
                            known_failure_modes: capability.known_failure_modes.clone(),
                            diagnostic_reporting: capability.diagnostic_reporting.clone(),
                            target_categories: capability.target_categories.clone(),
                            resource_capabilities: capability.resource_capabilities.clone(),
                            assurance: Some(record.validated_capability.assurance().to_wire_v1()),
                        }
                    });
                    let recent_diagnostics = if recent_limit == 0 {
                        Vec::new()
                    } else {
                        self.recent_diagnostics
                            .get(adapter_id)
                            .into_iter()
                            .flat_map(|records| records.iter().rev())
                            .filter(|record| {
                                record
                                    .audit_event_id
                                    .as_ref()
                                    .and_then(|id| id.lsn.as_ref())
                                    .is_some_and(|lsn| lsn.value <= as_of)
                            })
                            .take(recent_limit)
                            .cloned()
                            .collect()
                    };
                    let (
                        live_session_count,
                        stale_session_count,
                        offline_session_count,
                        failed_session_count,
                    ) = self
                        .sessions
                        .sessions()
                        .filter(|session| session.identity.adapter_id == *adapter_id)
                        .fold((0_u32, 0_u32, 0_u32, 0_u32), |mut counts, session| {
                            match effective_connectivity(session.state) {
                            patchbay_contracts::patchbay::SessionConnectivityState::Live => {
                                counts.0 += 1
                            }
                            patchbay_contracts::patchbay::SessionConnectivityState::Stale => {
                                counts.1 += 1
                            }
                            patchbay_contracts::patchbay::SessionConnectivityState::Offline => {
                                counts.2 += 1
                            }
                            patchbay_contracts::patchbay::SessionConnectivityState::Failed => {
                                counts.3 += 1
                            }
                            patchbay_contracts::patchbay::SessionConnectivityState::Unknown
                            | patchbay_contracts::patchbay::SessionConnectivityState::Unspecified =>
                                {}
                        }
                            counts
                        });
                    // Only current-process evidence may establish a live adapter
                    // state. Historical lifecycle records remain visible as audit
                    // history but are UNKNOWN after reset/restart.
                    let state = self
                        .current_process_adapters
                        .get(adapter_id)
                        .copied()
                        .unwrap_or(AdapterDiagnosticState::Unknown);
                    AdapterStatus {
                        adapter_id: Some(adapter_id.clone()),
                        endpoint_id: registration.endpoint_id.clone(),
                        adapter_generation: registration.adapter_generation,
                        state: state as i32,
                        attach_event_id: Some(record.attach_event_id.clone()),
                        attached_at: registration.attached_at,
                        capability,
                        last_lifecycle_record: self.lifecycle.get(adapter_id).cloned(),
                        live_session_count,
                        stale_session_count,
                        offline_session_count,
                        failed_session_count,
                        recent_diagnostics,
                    }
                })
                .collect();
        Ok(AdapterStatusPage {
            adapters,
            next_after_adapter_id,
            has_more,
        })
    }

    #[must_use]
    pub fn inspect_command(&self, id: &CommandId) -> Option<CommandInspection> {
        self.commands.get(id).map(|timeline| CommandInspection {
            command: Some(timeline.summary.clone()),
            accepted_event_id: Some(timeline.accepted_event_id.clone()),
            current_state: timeline.current_state as i32,
            failure_code: timeline.failure_code as i32,
            terminal_event_id: timeline.terminal_event_id.clone(),
            history: timeline.history.clone(),
            audit: None,
            spawn_claim_disposition: timeline
                .claim_disposition
                .unwrap_or(SpawnClaimDisposition::Unspecified)
                as i32,
        })
    }

    #[must_use]
    pub fn result_for_query(&self, id: &CommandId) -> Option<CommandInspectionResult> {
        Some(CommandInspectionResult {
            found: self.commands.contains_key(id),
            inspection: self.inspect_command(id),
        })
    }
}

fn is_terminal(state: OperationState) -> bool {
    matches!(
        state,
        OperationState::Completed
            | OperationState::Rejected
            | OperationState::Failed
            | OperationState::Expired
            | OperationState::Cancelled
            | OperationState::Superseded
    )
}

pub fn validate_query(
    operation: &Operation,
    current_lsn: u64,
) -> Result<ValidatedDiagnosticsQuery, DiagnosticsError> {
    if OperationKind::try_from(operation.kind).ok() != Some(OperationKind::Query) {
        return Err(DiagnosticsError::InvalidQuery(
            "operation kind must be QUERY".to_owned(),
        ));
    }
    let domain = operation
        .authority_domain_id
        .as_ref()
        .filter(|id| !id.value.is_empty())
        .ok_or_else(|| {
            DiagnosticsError::InvalidQuery("operation is missing authority domain".to_owned())
        })?;
    let target = operation.target_scope.as_ref().ok_or_else(|| {
        DiagnosticsError::InvalidQuery("query operation is missing target scope".to_owned())
    })?;
    if TargetScopeKind::try_from(target.kind).ok() != Some(TargetScopeKind::AuthorityDomain) {
        return Err(DiagnosticsError::InvalidQuery(
            "diagnostics target must be an authority domain".to_owned(),
        ));
    }
    let payload = operation.payload.as_ref().ok_or_else(|| {
        DiagnosticsError::InvalidQuery("query operation is missing payload".to_owned())
    })?;
    if payload.schema_ref != DIAGNOSTICS_SCHEMA {
        return Err(DiagnosticsError::InvalidQuery(
            "query payload has the wrong schema_ref".to_owned(),
        ));
    }
    if payload.content_type != patchbay_contracts::patchbay::PayloadContentType::Protobuf as i32 {
        return Err(DiagnosticsError::InvalidQuery(
            "diagnostics payload must be protobuf".to_owned(),
        ));
    }
    let query = DiagnosticsQuery::decode(payload.payload.as_slice()).map_err(|error| {
        DiagnosticsError::InvalidQuery(format!("query payload is malformed: {error}"))
    })?;
    match query
        .query
        .ok_or_else(|| DiagnosticsError::InvalidQuery("diagnostics query is empty".to_owned()))?
    {
        diagnostics_query::Query::Audit(query) => validate_audit_query(query, domain, current_lsn),
        diagnostics_query::Query::Command(query) => {
            let command_id = query
                .command_id
                .as_ref()
                .filter(|id| !id.value.is_empty())
                .ok_or_else(|| {
                    DiagnosticsError::InvalidQuery("command query is missing command_id".to_owned())
                })?;
            if let Some(cursor) = query.audit_before_event_id.as_ref() {
                validate_cursor(cursor, domain, current_lsn)?;
            }
            validate_limit(
                query.audit_limit,
                COMMAND_DEFAULT_LIMIT,
                COMMAND_MAX_LIMIT,
                "command audit",
            )?;
            Ok(ValidatedDiagnosticsQuery::Command(CommandInspectionQuery {
                command_id: Some(command_id.clone()),
                ..query
            }))
        }
        diagnostics_query::Query::Adapters(query) => {
            if query.adapter_ids.iter().any(|id| id.value.is_empty()) {
                return Err(DiagnosticsError::InvalidQuery(
                    "adapter filter contains an empty id".to_owned(),
                ));
            }
            validate_limit(
                query.limit,
                ADAPTER_DEFAULT_LIMIT,
                ADAPTER_MAX_LIMIT,
                "adapter",
            )?;
            validate_recent_diagnostic_limit(query.recent_diagnostic_limit)?;
            Ok(ValidatedDiagnosticsQuery::Adapters(query))
        }
    }
}

fn validate_audit_query(
    query: AuditQuery,
    domain: &AuthorityDomainId,
    current_lsn: u64,
) -> Result<ValidatedDiagnosticsQuery, DiagnosticsError> {
    for kind in &query.kinds {
        let kind = patchbay_contracts::patchbay::AuditEventKind::try_from(*kind).map_err(|_| {
            DiagnosticsError::InvalidQuery("audit filter contains an unknown kind".to_owned())
        })?;
        if kind == patchbay_contracts::patchbay::AuditEventKind::Unspecified {
            return Err(DiagnosticsError::InvalidQuery(
                "audit filter contains unspecified kind".to_owned(),
            ));
        }
    }
    for code in &query.failure_codes {
        let code = FailureCode::try_from(*code).map_err(|_| {
            DiagnosticsError::InvalidQuery(
                "audit filter contains an unknown failure code".to_owned(),
            )
        })?;
        if code == FailureCode::Unspecified {
            return Err(DiagnosticsError::InvalidQuery(
                "audit filter contains unspecified failure code".to_owned(),
            ));
        }
    }
    if query.reason_codes.iter().any(|reason| {
        reason.is_empty()
            || reason.len() > 64
            || !reason
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    }) {
        return Err(DiagnosticsError::InvalidQuery(
            "reason_codes must match [a-z0-9_]{1,64}".to_owned(),
        ));
    }
    validate_interval(
        query.occurred_from_inclusive.as_ref(),
        query.occurred_before_exclusive.as_ref(),
    )?;
    if query
        .grant_id
        .as_ref()
        .is_some_and(|id| id.value.is_empty())
    {
        return Err(DiagnosticsError::InvalidQuery(
            "grant_id filter must not be empty".to_owned(),
        ));
    }
    if let Some(cursor) = query.before_event_id.as_ref() {
        validate_cursor(cursor, domain, current_lsn)?;
    }
    validate_limit(query.limit, AUDIT_DEFAULT_LIMIT, AUDIT_MAX_LIMIT, "audit")?;
    let target = query
        .target_scope
        .as_ref()
        .map(|scope| {
            TargetKey::new(hex_scope(scope)).ok_or_else(|| {
                DiagnosticsError::InvalidQuery("target filter has no canonical fields".to_owned())
            })
        })
        .transpose()?;
    Ok(ValidatedDiagnosticsQuery::Audit(AuditPageSpec {
        kinds: query
            .kinds
            .into_iter()
            .map(|kind| {
                patchbay_contracts::patchbay::AuditEventKind::try_from(kind)
                    .expect("validated kind")
            })
            .collect(),
        actor_id: query.actor_id,
        endpoint_id: query.endpoint_id,
        command_id: query.command_id,
        grant_id: query.grant_id,
        target,
        failure_codes: query
            .failure_codes
            .into_iter()
            .map(|code| FailureCode::try_from(code).expect("validated failure"))
            .collect(),
        reason_codes: query.reason_codes,
        occurred_from: query.occurred_from_inclusive,
        occurred_before: query.occurred_before_exclusive,
        before_lsn: query
            .before_event_id
            .and_then(|id| id.lsn)
            .map(|lsn| lsn.value),
        limit: query
            .limit
            .map_or(AUDIT_DEFAULT_LIMIT, |limit| limit as u16),
    }))
}

fn hex_scope(scope: &TargetScope) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    scope
        .encode_to_vec()
        .iter()
        .flat_map(|byte| {
            [
                HEX[(byte >> 4) as usize] as char,
                HEX[(byte & 0x0f) as usize] as char,
            ]
        })
        .collect()
}

fn validate_cursor(
    cursor: &EventId,
    domain: &AuthorityDomainId,
    current_lsn: u64,
) -> Result<(), DiagnosticsError> {
    if cursor.authority_domain_id.as_ref() != Some(domain) {
        return Err(DiagnosticsError::InvalidQuery(
            "cursor belongs to another authority domain".to_owned(),
        ));
    }
    let lsn = cursor
        .lsn
        .as_ref()
        .ok_or_else(|| DiagnosticsError::InvalidQuery("cursor is missing LSN".to_owned()))?
        .value;
    if lsn > current_lsn {
        return Err(DiagnosticsError::InvalidQuery(
            "cursor is beyond current LSN".to_owned(),
        ));
    }
    Ok(())
}

pub fn validate_recent_diagnostic_limit(value: Option<u32>) -> Result<usize, DiagnosticsError> {
    match value {
        None => Ok(0),
        Some(value) if (1..=100).contains(&value) => Ok(value as usize),
        Some(_) => Err(DiagnosticsError::InvalidQuery(
            "recent diagnostic limit must be between 1 and 100".to_owned(),
        )),
    }
}

fn event_lsn(event: &RecordedEvent) -> u64 {
    event.event_id.lsn.as_ref().map_or(0, |lsn| lsn.value)
}

fn validate_limit(
    limit: Option<u32>,
    default: u16,
    maximum: u16,
    name: &str,
) -> Result<(), DiagnosticsError> {
    if let Some(limit) = limit {
        if limit == 0 || limit > u32::from(maximum) {
            return Err(DiagnosticsError::InvalidQuery(format!(
                "{name} limit must be between 1 and {maximum}"
            )));
        }
    }
    let _ = default;
    Ok(())
}

fn validate_interval(
    from: Option<&Timestamp>,
    before: Option<&Timestamp>,
) -> Result<(), DiagnosticsError> {
    const MIN_SECONDS: i64 = -62_135_596_800;
    const MAX_SECONDS: i64 = 253_402_300_799;
    for timestamp in [from, before].into_iter().flatten() {
        if !(MIN_SECONDS..=MAX_SECONDS).contains(&timestamp.seconds)
            || !(0..1_000_000_000).contains(&timestamp.nanos)
        {
            return Err(DiagnosticsError::InvalidQuery(
                "query timestamp is invalid".to_owned(),
            ));
        }
    }
    if let (Some(from), Some(before)) = (from, before) {
        if (from.seconds, from.nanos) >= (before.seconds, before.nanos) {
            return Err(DiagnosticsError::InvalidQuery(
                "query time interval is empty or reversed".to_owned(),
            ));
        }
    }
    Ok(())
}
