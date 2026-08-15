use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, runtime_generation_disposition,
    spawn_claim_event, AcceptedOperation, ActorEndpointRef, ActorId, AdapterDiagnosticReport,
    AdapterDiagnosticReportResult, AdapterId, AdapterSnapshotSupport, AttachRequest, AttachResult,
    AuthorityDomainId, Delivery, EventId, ExternalEffectDisposition, FailureCode, Generation,
    Observation, ObservationKind, ObservationRequest, ObservationResult, OperationKind,
    OperationState, ReceiveRequest, RuntimeEvidenceQuarantineReason,
    RuntimeEvidenceSourceAttachment, RuntimeGenerationDisposition, RuntimeGenerationRef,
    SpawnClaimAccepted, SpawnClaimDisposition, SpawnClaimEvent, SpawnEvidenceAttachment,
    SpawnExecutionEvidence, SpawnExecutionEvidenceProducer, SpawnExecutionPhase,
    SpawnSuccessorEvidenceStaged, StoredEventKind, TargetScopeKind,
};
use patchbay_core::{
    acceptance::{self, CommandIndex, OperationStateExt},
    adapter::{self, AdapterRecord, AdapterRegistry},
    audit::{AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    authority::hash_principal_credential,
    diagnostics::{ingest_adapter_diagnostic, validate_adapter_diagnostic_report},
    resource::{
        self, ResourceIdentity, ResourceRegistry, ResourceReportMode, ValidatedResourceReport,
    },
    session::{self, SessionRegistry},
    storage::{
        validate_next_replay_event, AuditRecordDraft, CoreGenerationStore, RecordedEvent, Storage,
    },
    target::target_adapter_id,
};
use prost::Message;
use tokio::{
    sync::{mpsc, Mutex},
    time::sleep,
};
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status};

#[cfg(test)]
mod tests;

use crate::{
    decision_gate::CoreDecisionGate,
    identity::{random_core_generation, random_token},
    rpc::adapter_control_service_server::AdapterControlService,
    service::{map_acceptance_error_to_status, map_storage_error_to_status},
    snapshot::recover_session_registry,
};

pub const ADAPTER_ID_HEADER: &str = "x-patchbay-adapter-id";
pub const ADAPTER_EVIDENCE_HEADER: &str = "x-patchbay-adapter-evidence";
pub const ADAPTER_ATTACHMENT_TOKEN_HEADER: &str = "x-patchbay-adapter-attachment-token";

#[derive(Clone)]
pub struct AdapterEvidenceVerifier {
    expected_by_adapter: Arc<HashMap<AdapterId, Arc<[u8]>>>,
}

impl AdapterEvidenceVerifier {
    pub fn new<I, K, V>(credentials: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut expected_by_adapter: HashMap<AdapterId, Arc<[u8]>> = HashMap::new();
        for (adapter_id, evidence) in credentials {
            let adapter_id = adapter_id.into();
            let evidence = evidence.into();
            if adapter_id.is_empty() {
                return Err("adapter attachment credential id must not be empty".into());
            }
            if evidence.is_empty() {
                return Err(format!(
                    "adapter attachment credential for {adapter_id:?} must not be empty"
                ));
            }
            if !evidence.is_ascii() {
                return Err(format!(
                    "adapter attachment credential for {adapter_id:?} must be ASCII"
                ));
            }
            if expected_by_adapter
                .values()
                .any(|expected| constant_time_eq(evidence.as_bytes(), expected))
            {
                return Err("adapter attachment credentials must be unique per adapter".into());
            }
            let replaced = expected_by_adapter.insert(
                AdapterId {
                    value: adapter_id.clone(),
                },
                Arc::from(evidence.into_bytes()),
            );
            if replaced.is_some() {
                return Err(format!(
                    "adapter attachment credential id {adapter_id:?} is duplicated"
                ));
            }
        }
        if expected_by_adapter.is_empty() {
            return Err("at least one adapter attachment credential must be configured".into());
        }
        Ok(Self {
            expected_by_adapter: Arc::new(expected_by_adapter),
        })
    }

    fn verify_attach(&self, adapter_id: &AdapterId, evidence: &[u8]) -> Result<(), Status> {
        let valid = self
            .expected_by_adapter
            .get(adapter_id)
            .is_some_and(|expected| constant_time_eq(evidence, expected));
        if valid {
            Ok(())
        } else {
            Err(Status::unauthenticated(
                "invalid adapter attachment evidence",
            ))
        }
    }

    fn verify_request<T>(&self, request: &Request<T>) -> Result<AdapterId, Status> {
        let adapter_id = request
            .metadata()
            .get(ADAPTER_ID_HEADER)
            .ok_or_else(|| Status::unauthenticated("missing adapter id"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid adapter id"))?;
        if adapter_id.is_empty() {
            return Err(Status::unauthenticated("adapter id must not be empty"));
        }
        let adapter_id = AdapterId {
            value: adapter_id.to_owned(),
        };
        let evidence = request
            .metadata()
            .get(ADAPTER_EVIDENCE_HEADER)
            .map(|value| value.as_encoded_bytes())
            .ok_or_else(|| Status::unauthenticated("missing adapter attachment evidence"))?;
        self.verify_attach(&adapter_id, evidence)?;
        Ok(adapter_id)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[doc(hidden)]
pub enum AdapterServiceConformanceFault {
    #[default]
    None,
    #[cfg(feature = "conformance-fault-injection")]
    AcceptPriorAttachmentToken,
    #[cfg(feature = "conformance-fault-injection")]
    IgnoreResourceGeneration,
    #[cfg(feature = "conformance-fault-injection")]
    NormalizeResourceOwnerToAuthenticatedAdapter,
    #[cfg(feature = "conformance-fault-injection")]
    IgnoreEmptyPartialResourceReport,
    #[cfg(feature = "conformance-fault-injection")]
    KeepResourcesCurrentOnDisconnect,
    #[cfg(feature = "conformance-fault-injection")]
    AcceptNonIncreasingSessionRevision,
}

#[derive(Clone)]
pub struct AdapterControlServiceImpl<S> {
    storage: S,
    authority_domain_id: AuthorityDomainId,
    core_generation: Generation,
    evidence: AdapterEvidenceVerifier,
    audit: Arc<dyn AuditSink>,
    adapters: Arc<Mutex<AdapterRegistry>>,
    commands: Arc<Mutex<CommandProjection>>,
    sessions: Arc<Mutex<SessionRegistry>>,
    resources: Arc<Mutex<ResourceRegistry>>,
    attachment_tokens: Arc<Mutex<HashMap<AdapterId, Vec<u8>>>>,
    delivery_stream_epochs: Arc<Mutex<HashMap<AdapterId, u64>>>,
    decision_gate: CoreDecisionGate,
    #[cfg(feature = "conformance-fault-injection")]
    conformance_fault: AdapterServiceConformanceFault,
}

impl<S> AdapterControlServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    pub async fn new(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        evidence: AdapterEvidenceVerifier,
    ) -> Result<Self, String> {
        Self::new_with_decision_gate(
            storage,
            authority_domain_id,
            evidence,
            CoreDecisionGate::default(),
        )
        .await
    }

    pub async fn new_with_decision_gate(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        evidence: AdapterEvidenceVerifier,
        decision_gate: CoreDecisionGate,
    ) -> Result<Self, String> {
        Self::new_with_decision_gate_and_conformance_fault(
            storage,
            authority_domain_id,
            evidence,
            decision_gate,
            AdapterServiceConformanceFault::None,
        )
        .await
    }

    #[cfg(feature = "conformance-fault-injection")]
    #[doc(hidden)]
    pub async fn new_with_conformance_fault(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        evidence: AdapterEvidenceVerifier,
        conformance_fault: AdapterServiceConformanceFault,
    ) -> Result<Self, String> {
        Self::new_with_decision_gate_and_conformance_fault(
            storage,
            authority_domain_id,
            evidence,
            CoreDecisionGate::default(),
            conformance_fault,
        )
        .await
    }

    #[cfg(any(test, feature = "conformance-fault-injection"))]
    #[doc(hidden)]
    pub async fn conformance_session_registry(&self) -> SessionRegistry {
        self.sessions.lock().await.clone()
    }

    async fn new_with_decision_gate_and_conformance_fault(
        storage: S,
        authority_domain_id: AuthorityDomainId,
        evidence: AdapterEvidenceVerifier,
        decision_gate: CoreDecisionGate,
        _conformance_fault: AdapterServiceConformanceFault,
    ) -> Result<Self, String> {
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".into());
        }
        let core_generation = storage
            .load_or_create_core_generation(&authority_domain_id, random_core_generation())
            .await
            .map_err(|error| error.to_string())?;
        let adapters = adapter::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let commands = rebuild_command_projection(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let sessions = recover_session_registry(&storage, &authority_domain_id, &core_generation)
            .await
            .map_err(|error| error.to_string())?
            .registry;
        let resources = resource::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let audit: Arc<dyn AuditSink> = Arc::new(RequiredAuditFanout::new(
            Arc::new(DurableAuditSink::new(
                storage.clone(),
                authority_domain_id.clone(),
            )),
            vec![Arc::new(StderrAuditSink)],
        ));
        Ok(Self {
            storage,
            authority_domain_id,
            core_generation,
            evidence,
            audit,
            adapters: Arc::new(Mutex::new(adapters)),
            commands: Arc::new(Mutex::new(commands)),
            sessions: Arc::new(Mutex::new(sessions)),
            resources: Arc::new(Mutex::new(resources)),
            attachment_tokens: Arc::new(Mutex::new(HashMap::new())),
            delivery_stream_epochs: Arc::new(Mutex::new(HashMap::new())),
            decision_gate,
            #[cfg(feature = "conformance-fault-injection")]
            conformance_fault: _conformance_fault,
        })
    }

    async fn authenticate_request<T>(&self, request: &Request<T>) -> Result<AdapterId, Status> {
        let adapter_id = self.evidence.verify_request(request)?;
        let token = request
            .metadata()
            .get(ADAPTER_ATTACHMENT_TOKEN_HEADER)
            .ok_or_else(|| Status::unauthenticated("missing adapter attachment token"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid adapter attachment token"))?;
        #[cfg(feature = "conformance-fault-injection")]
        if self.conformance_fault == AdapterServiceConformanceFault::AcceptPriorAttachmentToken {
            return Ok(adapter_id);
        }
        let actual_hash = hash_principal_credential(token);
        let tokens = self.attachment_tokens.lock().await;
        let expected_hash = tokens.get(&adapter_id).ok_or_else(|| {
            Status::unauthenticated("adapter attachment is not current; reattach required")
        })?;
        if !constant_time_eq(&actual_hash, expected_hash) {
            return Err(Status::unauthenticated(
                "stale adapter attachment token; reattach required",
            ));
        }
        Ok(adapter_id)
    }

    fn require_domain(&self, domain: &AuthorityDomainId) -> Result<(), Status> {
        if domain != &self.authority_domain_id {
            return Err(Status::invalid_argument(
                "request authority domain does not match this core",
            ));
        }
        Ok(())
    }

    async fn require_attached(&self, adapter_id: &AdapterId) -> Result<AuthorityDomainId, Status> {
        let adapters = self.adapters.lock().await;
        let record = adapters
            .get(adapter_id)
            .ok_or_else(|| Status::permission_denied("adapter is not attached"))?;
        let domain = record
            .registration
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::internal("attached adapter has no authority domain"))?;
        self.require_domain(&domain)?;
        Ok(domain)
    }

    /// Fail closed after a replacement registration has durably committed but
    /// its in-memory projection could not be refreshed. The old token and any
    /// delivery stream authenticated under its epoch must become inert even
    /// though the attach RPC returns an error.
    async fn fence_attachment_after_commit(&self, adapter_id: &AdapterId) -> Result<(), Status> {
        self.attachment_tokens.lock().await.remove(adapter_id);
        let mut epochs = self.delivery_stream_epochs.lock().await;
        let epoch = epochs.entry(adapter_id.clone()).or_default();
        *epoch = epoch
            .checked_add(1)
            .ok_or_else(|| Status::internal("delivery stream epoch overflow"))?;
        Ok(())
    }
}

const DELIVERY_SCAN_INTERVAL: Duration = Duration::from_millis(100);

type DeliveryStream = Pin<Box<dyn Stream<Item = Result<Delivery, Status>> + Send + 'static>>;
type DisconnectCallback = Box<dyn FnOnce() + Send + 'static>;

/// Deferred single-adapter materialization handle.
///
/// Holds only the shared registry `Arc` (O(1) clone) so gate-held retry
/// reconciliation never pays for whole-registry copying; the full record is
/// cloned lazily per adapter, only after an indexed retry miss.
#[derive(Clone)]
struct AdapterRegistryLookup {
    adapters: Arc<Mutex<AdapterRegistry>>,
    adapter_id: AdapterId,
}

impl AdapterRegistryLookup {
    async fn point_record(&self) -> Option<AdapterRecord> {
        #[cfg(test)]
        ADAPTER_PROJECTION_MATERIALIZATIONS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let adapters = self.adapters.lock().await;
        adapters.get(&self.adapter_id).cloned()
    }
}

/// Test seam: counts single-adapter projection materializations through the
/// deferred lookup. Exact staged retries must return before ANY materialization
/// (delta 0); only a fresh classification after an indexed miss pays for it.
#[cfg(test)]
pub(crate) static ADAPTER_PROJECTION_MATERIALIZATIONS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq)]
struct CommandProjection {
    index: CommandIndex,
    cursor: u64,
}

async fn append_quarantined_session_report<S: Storage>(
    storage: &S,
    domain: &AuthorityDomainId,
    report: patchbay_contracts::patchbay::SessionReport,
    disposition: patchbay_contracts::patchbay::RuntimeGenerationDisposition,
    reason: patchbay_contracts::patchbay::RuntimeEvidenceQuarantineReason,
    source_attachment: RuntimeEvidenceSourceAttachment,
    projections: (&SessionRegistry, &session::SpawnClaimRegistry),
) -> Result<EventId, Status> {
    let (sessions, claims) = projections;
    let quarantined = session::quarantined_session_report(
        domain,
        report,
        disposition,
        reason,
        source_attachment,
        sessions,
        claims,
    )
    .map_err(|error| Status::failed_precondition(error.to_string()))?;
    let mut audit = AuditRecordDraft::new(
        crate::identity::now_timestamp().map_err(|error| Status::internal(error.to_string()))?,
        patchbay_contracts::patchbay::AuditEventKind::StaleEventIgnored,
    );
    audit.failure_code = Some(FailureCode::StaleEvent);
    audit.reason_code = session::quarantine_reason_code(reason).to_owned();
    audit.target_scope = Some(
        session::quarantined_candidate_scope(&quarantined)
            .map_err(|error| Status::failed_precondition(error.to_string()))?,
    );
    storage
        .append_quarantined_runtime_evidence_audited(domain, quarantined, audit)
        .await
        .map(|committed| committed.source_event_id)
        .map_err(map_storage_error_to_status)
}

async fn existing_staged_successor_retry<S: Storage>(
    storage: &S,
    domain: &AuthorityDomainId,
    claim_operation_id: patchbay_contracts::patchbay::CommandId,
    report: &patchbay_contracts::patchbay::SessionReport,
    source_attachment: &RuntimeEvidenceSourceAttachment,
) -> Result<Option<EventId>, Status> {
    storage
        .reconcile_spawn_successor_staged_retry(
            domain,
            claim_operation_id,
            report.clone(),
            source_attachment.clone(),
        )
        .await
        .map_err(map_storage_error_to_status)
}

fn staged_successor_for_claim(
    domain: &AuthorityDomainId,
    report: patchbay_contracts::patchbay::SessionReport,
    source_attachment: RuntimeEvidenceSourceAttachment,
    claim_record: &session::SpawnClaimRecord,
    disposition: RuntimeGenerationDisposition,
) -> Result<SpawnSuccessorEvidenceStaged, Status> {
    if !matches!(
        disposition.disposition.as_ref(),
        Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
    ) {
        return Err(Status::failed_precondition(
            "only the shared ClaimedSuccessor classification may stage a managed report",
        ));
    }
    let claim = claim_record.claim.clone();
    let external = patchbay_contracts::patchbay::ExternalRuntimeRef {
        adapter_id: report.adapter_id.clone(),
        deployment_scope: report.deployment_scope.clone(),
        runtime_session_id: report.runtime_session_id.clone(),
        generation: report.session_generation,
    };
    let staged = SpawnSuccessorEvidenceStaged {
        authority_domain_id: Some(domain.clone()),
        exact_claim: Some(claim.clone()),
        report: Some(report),
        classified_target: Some(RuntimeGenerationRef {
            logical_target_id: claim.logical_target_id.clone(),
            external_runtime: Some(external.clone()),
        }),
        disposition: Some(disposition),
        source_attachment: Some(source_attachment),
        external_runtime_reservation: Some(external),
    };
    session::validate_staged_successor(&staged)
        .map_err(|error| Status::failed_precondition(error.to_string()))?;
    Ok(staged)
}

fn session_report_claim_operation(
    report: &patchbay_contracts::patchbay::SessionReport,
) -> Option<&patchbay_contracts::patchbay::CommandId> {
    match report.spawn_origin.as_ref()?.r#ref.as_ref()? {
        patchbay_contracts::patchbay::typed_correlation::Ref::CommandId(command_id)
            if !command_id.value.is_empty() =>
        {
            Some(command_id)
        }
        _ => None,
    }
}

fn session_report_has_stale_source_order(
    report: &patchbay_contracts::patchbay::SessionReport,
    sessions: &SessionRegistry,
) -> bool {
    let Some((adapter_id, runtime_id, generation, reported)) = report
        .adapter_id
        .as_ref()
        .zip(report.runtime_session_id.as_ref())
        .zip(report.session_generation.as_ref())
        .zip(report.source_cursor.as_ref())
        .map(|(((adapter_id, runtime_id), generation), reported)| {
            (adapter_id, runtime_id, generation, reported)
        })
    else {
        return false;
    };
    let Some(current) = sessions
        .get_live_session(adapter_id, &report.deployment_scope, runtime_id)
        .filter(|current| current.identity.session_generation == *generation)
    else {
        return false;
    };
    let Some(last) = current.last_source_cursor.as_ref() else {
        return false;
    };
    let Some((reported_generation, last_generation)) = reported
        .adapter_generation
        .as_ref()
        .zip(last.adapter_generation.as_ref())
    else {
        return false;
    };
    reported_generation.value < last_generation.value
        || (reported_generation == last_generation && reported.revision <= last.revision)
}

async fn poison_ambiguous_spawn_result<S: Storage>(
    storage: &S,
    commands: &CommandIndex,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
    source_attachment: &RuntimeEvidenceSourceAttachment,
    observation: &Observation,
) -> Result<(), Status> {
    if ObservationKind::try_from(observation.kind).ok() != Some(ObservationKind::Result)
        || !matches!(
            FailureCode::try_from(observation.failure_code).ok(),
            Some(
                FailureCode::Cancelled
                    | FailureCode::Expired
                    | FailureCode::ExecutionOutcomeUnknown
            )
        )
    {
        return Ok(());
    }
    let Some(command_id) = acceptance::exact_command_correlation(&observation.correlations) else {
        return Ok(());
    };
    let Some(command) = commands.get_command(&command_id) else {
        return Ok(());
    };
    if OperationKind::try_from(command.operation.kind).ok() != Some(OperationKind::Spawn)
        || !matches!(
            command.state,
            OperationState::Accepted | OperationState::Delivered | OperationState::Running
        )
        || command.operation.target_scope != observation.target_scope
    {
        return Ok(());
    }
    let claims = session::rebuild_spawn_claims_from_log(storage, authority_domain_id)
        .await
        .map_err(map_spawn_claim_error)?;
    let record = session::SpawnClaimQuery::claim_for_operation(&claims, &command_id)
        .ok_or_else(|| Status::failed_precondition("ambiguous spawn result has no exact claim"))?;
    if record.adapter_id != *adapter_id
        || !matches!(
            record.disposition,
            SpawnClaimDisposition::Active | SpawnClaimDisposition::PoisonedPendingReconciliation
        )
    {
        return Ok(());
    }
    storage
        .append_spawn_execution_evidence_reconciled(
            authority_domain_id,
            SpawnExecutionEvidence {
                authority_domain_id: Some(authority_domain_id.clone()),
                exact_claim: Some(record.claim.clone()),
                phase: SpawnExecutionPhase::LaunchAttempted as i32,
                external_effect_disposition: ExternalEffectDisposition::MayExist as i32,
                producer: SpawnExecutionEvidenceProducer::CurrentAdapter as i32,
                source_attachment: Some(SpawnEvidenceAttachment {
                    adapter_id: source_attachment.adapter_id.clone(),
                    adapter_generation: source_attachment.adapter_generation,
                    attachment_event_id: source_attachment.attachment_event_id.clone(),
                }),
                failure_code: observation.failure_code,
                no_external_effect_proof: None,
                external_runtime: None,
            },
        )
        .await
        .map_err(map_storage_error_to_status)?;
    Ok(())
}

async fn poison_ambiguous_spawn_claims_for_adapter<S: Storage>(
    storage: &S,
    commands: &CommandIndex,
    authority_domain_id: &AuthorityDomainId,
    adapter_id: &AdapterId,
    source_attachment: SpawnEvidenceAttachment,
) -> Result<(), adapter::AdapterError> {
    let claims = session::rebuild_spawn_claims_from_log(storage, authority_domain_id)
        .await
        .map_err(|error| adapter::AdapterError::CorruptRecord(error.to_string()))?;
    let candidates: Vec<_> = claims
        .records()
        .filter(|record| {
            record.disposition == SpawnClaimDisposition::Active && record.adapter_id == *adapter_id
        })
        .filter_map(|record| {
            let command_id = record.claim.claim_operation_id.as_ref()?;
            let command = commands.get_command(command_id)?;
            matches!(
                command.state,
                OperationState::Accepted | OperationState::Delivered | OperationState::Running
            )
            .then(|| (record.claim.clone(), command.state))
        })
        .collect();

    for (claim, state) in candidates {
        storage
            .append_spawn_execution_evidence_reconciled(
                authority_domain_id,
                SpawnExecutionEvidence {
                    authority_domain_id: Some(authority_domain_id.clone()),
                    exact_claim: Some(claim),
                    phase: if state == OperationState::Running {
                        SpawnExecutionPhase::LaunchAttempted as i32
                    } else {
                        SpawnExecutionPhase::Offered as i32
                    },
                    external_effect_disposition: ExternalEffectDisposition::MayExist as i32,
                    producer: SpawnExecutionEvidenceProducer::Core as i32,
                    source_attachment: Some(source_attachment.clone()),
                    failure_code: FailureCode::ExecutionOutcomeUnknown as i32,
                    no_external_effect_proof: None,
                    external_runtime: None,
                },
            )
            .await?;
    }
    Ok(())
}

async fn rebuild_command_projection<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<CommandProjection, acceptance::AcceptanceError> {
    let events = storage
        .read_after(
            authority_domain_id,
            patchbay_contracts::patchbay::Lsn { value: 0 },
        )
        .await?;
    command_projection_from_events(&events, authority_domain_id)
}

fn command_projection_from_events(
    events: &[RecordedEvent],
    authority_domain_id: &AuthorityDomainId,
) -> Result<CommandProjection, acceptance::AcceptanceError> {
    let mut index = CommandIndex::new();
    let mut cursor = 0;
    for event in events {
        let validated =
            validate_next_replay_event(authority_domain_id, cursor, event).map_err(|error| {
                error.map(
                    acceptance::AcceptanceError::CorruptRecord,
                    acceptance::AcceptanceError::CorruptLog,
                )
            })?;
        index.apply(event)?;
        cursor = validated.lsn;
    }
    Ok(CommandProjection { index, cursor })
}

async fn catch_up_command_projection<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
    projection: &mut CommandProjection,
) -> Result<Vec<RecordedEvent>, acceptance::AcceptanceError> {
    let events = storage
        .read_after(
            authority_domain_id,
            patchbay_contracts::patchbay::Lsn {
                value: projection.cursor,
            },
        )
        .await?;
    let mut staged = projection.clone();
    for event in &events {
        let validated = validate_next_replay_event(authority_domain_id, staged.cursor, event)
            .map_err(|error| {
                error.map(
                    acceptance::AcceptanceError::CorruptRecord,
                    acceptance::AcceptanceError::CorruptLog,
                )
            })?;
        staged.index.apply(event)?;
        staged.cursor = validated.lsn;
    }
    *projection = staged;
    Ok(events)
}

enum DeliveryAcceptance {
    Operation(Box<AcceptedOperation>),
    ManagedSpawn(Box<SpawnClaimAccepted>),
}

impl DeliveryAcceptance {
    fn accepted_operation(&self) -> Option<&AcceptedOperation> {
        match self {
            Self::Operation(accepted) => Some(accepted),
            Self::ManagedSpawn(accepted) => accepted.accepted_operation.as_ref(),
        }
    }
}

fn acceptance_for_delivery(event: &RecordedEvent) -> Result<Option<DeliveryAcceptance>, Status> {
    match StoredEventKind::try_from(event.payload.kind).ok() {
        Some(StoredEventKind::Operation) => {
            AcceptedOperation::decode(event.payload.payload.as_slice())
                .map(Box::new)
                .map(DeliveryAcceptance::Operation)
                .map(Some)
                .map_err(|error| {
                    Status::internal(format!("cannot decode accepted operation: {error}"))
                })
        }
        Some(StoredEventKind::SpawnClaim) => {
            let claim =
                SpawnClaimEvent::decode(event.payload.payload.as_slice()).map_err(|error| {
                    Status::internal(format!("cannot decode accepted spawn claim: {error}"))
                })?;
            let event_domain = event
                .event_id
                .authority_domain_id
                .as_ref()
                .filter(|domain| !domain.value.is_empty())
                .ok_or_else(|| Status::internal("accepted spawn claim event has no domain"))?;
            if claim.authority_domain_id.as_ref() != Some(event_domain) {
                return Err(Status::internal(
                    "accepted spawn claim payload domain differs from its event",
                ));
            }
            match claim.mutation {
                Some(spawn_claim_event::Mutation::Accepted(accepted)) => {
                    session::validate_spawn_claim_accepted(event_domain, &accepted).map_err(
                        |error| {
                            Status::internal(format!(
                                "accepted spawn delivery envelope is invalid: {error}"
                            ))
                        },
                    )?;
                    Ok(Some(DeliveryAcceptance::ManagedSpawn(Box::new(accepted))))
                }
                Some(spawn_claim_event::Mutation::DispositionChanged(_)) => Ok(None),
                None => Err(Status::internal("spawn claim event has no mutation")),
            }
        }
        _ => Ok(None),
    }
}

fn deliveries_for_events(
    events: &[RecordedEvent],
    commands: &CommandIndex,
    adapter_id: &AdapterId,
    after_cursor: u64,
) -> Vec<Result<Delivery, Status>> {
    events
        .iter()
        .filter(|event| {
            event
                .event_id
                .lsn
                .as_ref()
                .is_some_and(|lsn| lsn.value > after_cursor)
        })
        .filter_map(|event| match acceptance_for_delivery(event) {
            Ok(Some(acceptance)) => {
                let operation = match acceptance
                    .accepted_operation()
                    .and_then(|accepted| accepted.operation.clone())
                {
                    Some(operation) => operation,
                    None => {
                        return Some(Err(Status::internal("accepted operation has no operation")))
                    }
                };
                let targets_adapter =
                    operation.target_scope.as_ref().and_then(target_adapter_id) == Some(adapter_id);
                let remains_deliverable = operation
                    .command_id
                    .as_ref()
                    .and_then(|command_id| commands.get_command(command_id))
                    .is_some_and(|record| {
                        matches!(
                            record.state,
                            OperationState::Accepted | OperationState::Delivered
                        ) && !commands.delivery_is_suppressed(&record.command_id)
                    });
                (targets_adapter && remains_deliverable).then_some(Ok(Delivery {
                    operation: Some(operation),
                    delivery_event_id: Some(event.event_id.clone()),
                    accepted_spawn: match acceptance {
                        DeliveryAcceptance::Operation(_) => None,
                        DeliveryAcceptance::ManagedSpawn(accepted) => Some(*accepted),
                    },
                }))
            }
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

struct DeliverySubscriptionContext<S> {
    storage: S,
    authority_domain_id: AuthorityDomainId,
    adapter_id: AdapterId,
    commands: Arc<Mutex<CommandProjection>>,
    delivery_stream_epochs: Arc<Mutex<HashMap<AdapterId, u64>>>,
    stream_epoch: u64,
    decision_gate: CoreDecisionGate,
}

fn delivery_subscription<S>(
    context: DeliverySubscriptionContext<S>,
    initial_cursor: u64,
    initial_events: Vec<RecordedEvent>,
    initial_projection: CommandProjection,
) -> DeliveryStream
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let DeliverySubscriptionContext {
        storage,
        authority_domain_id,
        adapter_id,
        commands,
        delivery_stream_epochs,
        stream_epoch,
        decision_gate,
    } = context;
    let (sender, receiver) = mpsc::channel(16);
    tokio::spawn(async move {
        let mut delivery_cursor = initial_cursor;
        let mut scan_cursor = initial_projection.cursor;
        let mut subscription_commands = initial_projection.index;
        let mut pending_initial = Some(initial_events);

        loop {
            if sender.is_closed() {
                return;
            }
            // Claim acceptance and an offer share one linearization gate. A
            // delivery that wins enqueues before the fence; a claim that wins
            // is folded before eligibility is evaluated. Catching up again
            // under this guard closes the stream-establishment race too.
            let _decision_guard = decision_gate.acquire().await;
            let epochs = delivery_stream_epochs.lock().await;
            if epochs.get(&adapter_id) != Some(&stream_epoch) {
                return;
            }
            let batch = match storage
                .read_after(
                    &authority_domain_id,
                    patchbay_contracts::patchbay::Lsn { value: scan_cursor },
                )
                .await
            {
                Ok(events) => {
                    let applied = events.iter().try_for_each(|event| {
                        let validated =
                            validate_next_replay_event(&authority_domain_id, scan_cursor, event)
                                .map_err(|error| {
                                    error.map(
                                        acceptance::AcceptanceError::CorruptRecord,
                                        acceptance::AcceptanceError::CorruptLog,
                                    )
                                })?;
                        subscription_commands.apply(event)?;
                        scan_cursor = validated.lsn;
                        Ok::<(), acceptance::AcceptanceError>(())
                    });
                    match applied {
                        Ok(()) => {
                            // Keep unary acknowledgement/observation ingestion's
                            // shared projection current without borrowing its
                            // cursor as the subscription's delivery cursor.
                            let global_catch_up = {
                                let mut projection = commands.lock().await;
                                catch_up_command_projection(
                                    &storage,
                                    &authority_domain_id,
                                    &mut projection,
                                )
                                .await
                            };
                            match global_catch_up {
                                Ok(_) => {
                                    let mut delivery_events =
                                        pending_initial.take().unwrap_or_default();
                                    delivery_events.extend(events);
                                    let deliveries = deliveries_for_events(
                                        &delivery_events,
                                        &subscription_commands,
                                        &adapter_id,
                                        delivery_cursor,
                                    );
                                    delivery_cursor = delivery_cursor.max(scan_cursor);
                                    Ok((delivery_events.is_empty(), deliveries))
                                }
                                Err(error) => Err(map_acceptance_error_to_status(error)),
                            }
                        }
                        Err(error) => Err(map_acceptance_error_to_status(error)),
                    }
                }
                Err(error) => Err(map_storage_error_to_status(error)),
            };
            let (empty, deliveries) = match batch {
                Ok(batch) => batch,
                Err(status) => {
                    let _ = sender.send(Err(status)).await;
                    return;
                }
            };
            for delivery in deliveries {
                if sender.send(delivery).await.is_err() {
                    return;
                }
            }
            drop(epochs);
            drop(_decision_guard);
            if empty {
                sleep(DELIVERY_SCAN_INTERVAL).await;
            }
        }
    });
    Box::pin(ReceiverStream::new(receiver))
}

/// A long-lived delivery subscription treats every end or error as a lost
/// connection. Obsolete streams are made inert by the callback's epoch fence.
struct DeliveryTail {
    inner: DeliveryStream,
    on_abnormal_disconnect: Option<DisconnectCallback>,
}

impl DeliveryTail {
    fn new(inner: DeliveryStream, on_abnormal_disconnect: DisconnectCallback) -> Self {
        Self {
            inner,
            on_abnormal_disconnect: Some(on_abnormal_disconnect),
        }
    }

    fn mark_abnormal_disconnect(&mut self) {
        if let Some(callback) = self.on_abnormal_disconnect.take() {
            callback();
        }
    }
}

impl Stream for DeliveryTail {
    type Item = Result<Delivery, Status>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(context) {
            Poll::Ready(None) => {
                self.mark_abnormal_disconnect();
                Poll::Ready(None)
            }
            result @ Poll::Ready(Some(Err(_))) => {
                self.mark_abnormal_disconnect();
                result
            }
            result => result,
        }
    }
}

impl Drop for DeliveryTail {
    fn drop(&mut self) {
        self.mark_abnormal_disconnect();
    }
}

#[tonic::async_trait]
impl<S> AdapterControlService for AdapterControlServiceImpl<S>
where
    S: Storage + CoreGenerationStore + Clone + Send + Sync + 'static,
{
    async fn attach(
        &self,
        request: Request<AttachRequest>,
    ) -> Result<Response<AttachResult>, Status> {
        let request = request.into_inner();
        let registration = request
            .registration
            .ok_or_else(|| Status::invalid_argument("attach request is missing registration"))?;
        let adapter_id = registration
            .adapter_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("registration is missing adapter_id"))?;
        // The registration identity is an untrusted claim until its own
        // configured credential verifies. Only then may its generation replace
        // that adapter's durable registration and process-local token.
        self.evidence
            .verify_attach(&adapter_id, &request.attachment_evidence)?;
        let domain = registration.authority_domain_id.as_ref().ok_or_else(|| {
            Status::invalid_argument("registration is missing authority_domain_id")
        })?;
        self.require_domain(domain)?;
        let attachment_token = random_token();
        let attachment_token_hash = hash_principal_credential(&attachment_token);
        // Registration, token replacement, and every adapter decision are
        // ordered by the composition-root gate. A request that authenticated
        // against the old token before this point must not establish a
        // decision after this replacement commits.
        let _decision_guard = self.decision_gate.acquire().await;
        let rebuilt_adapters = adapter::rebuild_from_log(&self.storage, &self.authority_domain_id)
            .await
            .map_err(map_adapter_error)?;
        let rebuilt_resources =
            resource::rebuild_from_log(&self.storage, &self.authority_domain_id)
                .await
                .map_err(map_resource_error)?;
        let mut adapters = self.adapters.lock().await;
        *adapters = rebuilt_adapters;
        let mut resources = self.resources.lock().await;
        *resources = rebuilt_resources;
        let event_id = match adapter::ingest_registration_with_resources(
            &self.storage,
            &mut adapters,
            &mut resources,
            registration,
        )
        .await
        {
            Ok(event_id) => event_id,
            Err(error) => {
                let committed = error.committed();
                let error = error.into_adapter_error();
                if committed {
                    // Durable replacement is the point of no return. Never let
                    // projection failure preserve authority for the attachment
                    // that the log has already superseded.
                    self.fence_attachment_after_commit(&adapter_id).await?;
                }
                let kind = if matches!(error, adapter::AdapterError::StaleGeneration { .. }) {
                    patchbay_contracts::patchbay::AuditEventKind::TargetGenerationMismatch
                } else {
                    patchbay_contracts::patchbay::AuditEventKind::AdapterFailed
                };
                record_adapter_audit(
                    self.audit.as_ref(),
                    kind,
                    &adapter_id,
                    None,
                    "adapter_attach_rejected",
                )
                .await?;
                return Err(map_adapter_error(error));
            }
        };
        // Keep registration acceptance and token replacement in one critical
        // section so a slower older attach cannot overwrite a newer fence.
        self.attachment_tokens
            .lock()
            .await
            .insert(adapter_id.clone(), attachment_token_hash);
        // Replacing an attachment also fences delivery streams established by
        // the previous process, even when that process never disconnects
        // cleanly.
        let mut epochs = self.delivery_stream_epochs.lock().await;
        let epoch = epochs.entry(adapter_id.clone()).or_default();
        *epoch = epoch
            .checked_add(1)
            .ok_or_else(|| Status::internal("delivery stream epoch overflow"))?;
        drop(epochs);
        drop(resources);
        drop(adapters);

        let mut response = Response::new(AttachResult {
            accepted: true,
            attach_event_id: Some(event_id),
            failure_code: String::new(),
        });
        response.metadata_mut().insert(
            ADAPTER_ATTACHMENT_TOKEN_HEADER,
            attachment_token
                .parse()
                .map_err(|_| Status::internal("generated invalid adapter attachment token"))?,
        );
        Ok(response)
    }

    async fn report_diagnostics(
        &self,
        request: Request<AdapterDiagnosticReport>,
    ) -> Result<Response<AdapterDiagnosticReportResult>, Status> {
        // Authenticate only after acquiring the shared gate. Attachment
        // replacement can otherwise commit between token verification and
        // diagnostic append.
        let _decision_guard = self.decision_gate.acquire().await;
        let authenticated_adapter = self.authenticate_request(&request).await?;
        let report = request.into_inner();
        let registration = {
            let adapters = self.adapters.lock().await;
            adapters
                .get(&authenticated_adapter)
                .ok_or_else(|| {
                    Status::unauthenticated("adapter attachment is not current; reattach required")
                })?
                .registration
                .clone()
        };
        let validated = match validate_adapter_diagnostic_report(
            report,
            &authenticated_adapter,
            &registration,
            crate::identity::now_timestamp()?,
        ) {
            Ok(validated) => validated,
            Err(_) => {
                return Ok(Response::new(AdapterDiagnosticReportResult {
                    accepted: false,
                    failure_code: FailureCode::ValidationFailed as i32,
                    ..AdapterDiagnosticReportResult::default()
                }));
            }
        };
        let receipt =
            ingest_adapter_diagnostic(&self.storage, &self.authority_domain_id, validated)
                .await
                .map_err(map_storage_error_to_status)?;
        Ok(Response::new(AdapterDiagnosticReportResult {
            accepted: true,
            observation_event_id: Some(receipt.observation_event_id),
            audit_event_id: Some(receipt.audit_event_id),
            failure_code: FailureCode::Unspecified as i32,
        }))
    }

    async fn ingest_observation(
        &self,
        request: Request<ObservationRequest>,
    ) -> Result<Response<ObservationResult>, Status> {
        // Re-authentication is deliberately inside the shared gate: a stale
        // adapter token that was valid before an attach replacement must not
        // establish an observation decision afterwards.
        let _decision_guard = self.decision_gate.acquire().await;
        let authenticated_adapter = self.authenticate_request(&request).await?;
        let request = request.into_inner();
        let domain = request
            .authority_domain_id
            .ok_or_else(|| Status::invalid_argument("missing authority_domain_id"))?;
        self.require_domain(&domain)?;
        self.require_attached(&authenticated_adapter).await?;

        // Every adapter transition shares the composition-root gate with
        // Submit/RevokeGrant. This keeps revocation's catch-up/effect plan
        // adjacent to the append that establishes its LSN boundary.
        let event_id = match request.observation {
            Some(observation_request::Observation::SessionReport(mut report)) => {
                require_same_adapter(report.adapter_id.as_ref(), &authenticated_adapter)?;
                let source_cursor = report.source_cursor.as_mut().ok_or_else(|| {
                    Status::invalid_argument("session report is missing source_cursor")
                })?;
                if source_cursor.revision == 0 {
                    return Err(Status::invalid_argument(
                        "session report source_cursor revision is zero",
                    ));
                }
                let reported_adapter_generation =
                    source_cursor.adapter_generation.ok_or_else(|| {
                        Status::invalid_argument(
                            "session report source_cursor is missing adapter_generation",
                        )
                    })?;
                // Extract only the bounded point-lookup fields under the
                // attachment lock: the indexed exact-retry reconciliation below
                // needs just the current generation + attachment evidence, so
                // cloning the whole registry (O(adapters x manifest)) here would
                // serialize unrelated decisions behind unbounded work. The full
                // adapter projection is materialized only after an indexed miss.
                let (current_adapter_generation, source_attachment, adapter_lookup) = {
                    let adapters = self.adapters.lock().await;
                    let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?;
                    let generation = record
                        .registration
                        .adapter_generation
                        .ok_or_else(|| Status::internal("attached adapter has no generation"))?;
                    (
                        generation,
                        RuntimeEvidenceSourceAttachment {
                            adapter_id: Some(authenticated_adapter.clone()),
                            adapter_generation: Some(generation),
                            attachment_event_id: Some(record.attach_event_id.clone()),
                        },
                        AdapterRegistryLookup {
                            adapters: self.adapters.clone(),
                            adapter_id: authenticated_adapter.clone(),
                        },
                    )
                };
                // Replace producer identity with the authenticated adapter. The
                // candidate's reported producer generation is retained until
                // classification so stale-producer evidence can be quarantined
                // atomically rather than escaping as a standalone audit.
                report.adapter_id = Some(authenticated_adapter.clone());

                // Exact staged retries are a read-only response reconciliation,
                // not a fresh classification decision. Authentication, current
                // attachment lookup, domain/adapter binding, and required source
                // cursor framing have already run under the decision gate. The
                // indexed durable envelope was fully validated before staging;
                // complete report equality therefore re-proves every report and
                // claim-correlation check, while source-attachment equality
                // proves the same authenticated attachment generation/event.
                // No session/claim projection is needed to return its immutable
                // event id, including after claim poison or promotion.
                if let Some(claim_operation_id) = session_report_claim_operation(&report).cloned() {
                    if let Some(event_id) = existing_staged_successor_retry(
                        &self.storage,
                        &domain,
                        claim_operation_id,
                        &report,
                        &source_attachment,
                    )
                    .await?
                    {
                        return Ok(Response::new(ObservationResult {
                            event_id: Some(event_id),
                        }));
                    }
                }

                // Indexed miss: this is a fresh classification decision, so the
                // full adapter projection (single-adapter point materialization,
                // never a whole-registry clone) is fetched only now. The
                // classifier receives a single-entry registry view of exactly
                // this authenticated adapter (it only ever reads that record).
                let adapter_projection = AdapterRegistry::from_single(
                    authenticated_adapter.clone(),
                    adapter_lookup.point_record().await.ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?,
                );

                // The adapter owns an independent session projection. Rebuild
                // it at the gate boundary before deriving the next report
                // delta; otherwise a lockdown (or any core-side append) can
                // leave this writer with a stale pre-event view and produce a
                // live registration/transition that replay correctly rejects.
                let rebuilt =
                    recover_session_registry(&self.storage, &domain, &self.core_generation)
                        .await
                        .map_err(map_session_error)?
                        .registry;
                #[cfg(feature = "conformance-fault-injection")]
                if self.conformance_fault
                    == AdapterServiceConformanceFault::AcceptNonIncreasingSessionRevision
                {
                    // Deliberate compiled-graph mutant: when the production
                    // comparison would reject a same-epoch revision, synthesize
                    // an advancing cursor so the stale payload reaches the real
                    // atomic fold. The vector's independent status/audit/state
                    // oracle must detect the rollback.
                    let forced_revision = report
                        .adapter_id
                        .as_ref()
                        .zip(report.runtime_session_id.as_ref())
                        .zip(report.session_generation.as_ref())
                        .zip(report.source_cursor.as_ref())
                        .and_then(
                            |(((adapter_id, runtime_session_id), session_generation), cursor)| {
                                rebuilt
                                    .get_live_session(
                                        adapter_id,
                                        &report.deployment_scope,
                                        runtime_session_id,
                                    )
                                    .filter(|live| {
                                        live.identity.session_generation == *session_generation
                                    })
                                    .and_then(|live| live.last_source_cursor.as_ref())
                                    .filter(|last| {
                                        cursor.adapter_generation == last.adapter_generation
                                            && cursor.revision <= last.revision
                                    })
                                    .and_then(|last| last.revision.checked_add(1))
                            },
                        );
                    if let Some(revision) = forced_revision {
                        report
                            .source_cursor
                            .as_mut()
                            .expect("validated session source cursor")
                            .revision = revision;
                    }
                }
                let claims = session::rebuild_spawn_claims_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_spawn_claim_error)?;
                let disposition = session::classify_session_report(
                    &domain,
                    &report,
                    &source_attachment,
                    &adapter_projection,
                    &claims,
                    &rebuilt,
                );

                if reported_adapter_generation != current_adapter_generation {
                    let event_id = append_quarantined_session_report(
                        &self.storage,
                        &domain,
                        report,
                        disposition,
                        RuntimeEvidenceQuarantineReason::StaleAttachment,
                        source_attachment,
                        (&rebuilt, &claims),
                    )
                    .await?;
                    return Ok(Response::new(ObservationResult {
                        event_id: Some(event_id),
                    }));
                }

                if matches!(
                    disposition.disposition.as_ref(),
                    Some(runtime_generation_disposition::Disposition::Current(_))
                ) && session_report_has_stale_source_order(&report, &rebuilt)
                {
                    let event_id = append_quarantined_session_report(
                        &self.storage,
                        &domain,
                        report,
                        disposition,
                        RuntimeEvidenceQuarantineReason::StaleSourceOrder,
                        source_attachment,
                        (&rebuilt, &claims),
                    )
                    .await?;
                    return Ok(Response::new(ObservationResult {
                        event_id: Some(event_id),
                    }));
                }

                let correlated_claim =
                    session_report_claim_operation(&report).and_then(|command_id| {
                        session::SpawnClaimQuery::claim_for_operation(&claims, command_id)
                    });
                let claimed_successor = matches!(
                    disposition.disposition.as_ref(),
                    Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_))
                );
                if claimed_successor {
                    let claim_record = correlated_claim.ok_or_else(|| {
                        Status::internal("managed successor report lost its durable claim")
                    })?;
                    let staged = staged_successor_for_claim(
                        &domain,
                        report,
                        source_attachment,
                        claim_record,
                        disposition,
                    )?;
                    let event_id = self
                        .storage
                        .append_spawn_successor_staged_idempotent(&domain, staged)
                        .await
                        .map_err(map_storage_error_to_status)?;
                    *self.sessions.lock().await =
                        recover_session_registry(&self.storage, &domain, &self.core_generation)
                            .await
                            .map_err(map_session_error)?
                            .registry;
                    return Ok(Response::new(ObservationResult {
                        event_id: Some(event_id),
                    }));
                }

                match disposition.disposition.as_ref() {
                    Some(runtime_generation_disposition::Disposition::ClaimedSuccessor(_)) => {
                        unreachable!("claimed successor was handled by idempotent staging")
                    }
                    Some(runtime_generation_disposition::Disposition::Current(_)) => {}
                    Some(runtime_generation_disposition::Disposition::Unknown(_))
                        if report.spawn_origin.is_none() => {}
                    _ => {
                        let reason = session::quarantine_reason_for(&disposition);
                        let event_id = append_quarantined_session_report(
                            &self.storage,
                            &domain,
                            report,
                            disposition,
                            reason,
                            source_attachment,
                            (&rebuilt, &claims),
                        )
                        .await?;
                        return Ok(Response::new(ObservationResult {
                            event_id: Some(event_id),
                        }));
                    }
                }
                let ordinary_report = report.clone();
                let ordinary_disposition = disposition.clone();
                let ordinary_source = source_attachment.clone();
                let mut sessions = self.sessions.lock().await;
                *sessions = rebuilt;
                let result = match session::ingest_session_report(
                    &self.storage,
                    &mut *sessions,
                    &domain,
                    report,
                )
                .await
                {
                    Ok(result) => result,
                    Err(
                        error @ (session::SessionError::StaleSourceCursor { .. }
                        | session::SessionError::StaleGeneration { .. }),
                    ) => {
                        let reason = match error {
                            session::SessionError::StaleSourceCursor { .. } => {
                                RuntimeEvidenceQuarantineReason::StaleSourceOrder
                            }
                            session::SessionError::StaleGeneration { .. } => {
                                session::quarantine_reason_for(&ordinary_disposition)
                            }
                            _ => unreachable!("matched stale session-ingress error"),
                        };
                        let event_id = append_quarantined_session_report(
                            &self.storage,
                            &domain,
                            ordinary_report,
                            ordinary_disposition,
                            reason,
                            ordinary_source,
                            (&sessions, &claims),
                        )
                        .await?;
                        return Ok(Response::new(ObservationResult {
                            event_id: Some(event_id),
                        }));
                    }
                    Err(error) => {
                        record_adapter_audit(
                            self.audit.as_ref(),
                            patchbay_contracts::patchbay::AuditEventKind::AdapterFailed,
                            &authenticated_adapter,
                            None,
                            "session_report_rejected",
                        )
                        .await?;
                        return Err(map_session_error(error));
                    }
                };
                let rebuilt =
                    recover_session_registry(&self.storage, &domain, &self.core_generation)
                        .await
                        .map_err(map_session_error)?
                        .registry;
                *sessions = rebuilt;
                session_result_event_id(result)
            }
            Some(observation_request::Observation::SpawnExecutionEvidence(mut evidence)) => {
                if evidence.authority_domain_id.as_ref() != Some(&domain) {
                    return Err(Status::invalid_argument(
                        "spawn execution evidence authority domain does not match request",
                    ));
                }
                let (adapter_generation, attachment_event_id) = {
                    let adapters = self.adapters.lock().await;
                    let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?;
                    (
                        record.registration.adapter_generation.ok_or_else(|| {
                            Status::internal("attached adapter has no generation")
                        })?,
                        record.attach_event_id.clone(),
                    )
                };
                // Producer and attachment provenance are canonical facts from
                // the authenticated current attachment, never payload claims.
                evidence.producer = SpawnExecutionEvidenceProducer::CurrentAdapter as i32;
                evidence.source_attachment = Some(SpawnEvidenceAttachment {
                    adapter_id: Some(authenticated_adapter.clone()),
                    adapter_generation: Some(adapter_generation),
                    attachment_event_id: Some(attachment_event_id),
                });
                let claims = session::rebuild_spawn_claims_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_spawn_claim_error)?;
                claims
                    .validate_execution_evidence_candidate(&evidence)
                    .map_err(map_spawn_claim_error)?;
                Some(
                    self.storage
                        .append_spawn_execution_evidence_reconciled(&domain, evidence)
                        .await
                        .map_err(map_storage_error_to_status)?
                        .evidence_event_id,
                )
            }
            Some(observation_request::Observation::ResourceReport(report)) => {
                require_same_adapter(report.adapter_id.as_ref(), &authenticated_adapter)?;
                let generation = report.adapter_generation.ok_or_else(|| {
                    Status::invalid_argument("resource report is missing adapter_generation")
                })?;
                let observed_at = report.observed_at.ok_or_else(|| {
                    Status::invalid_argument("resource report is missing observed_at")
                })?;
                let (mode, views) = match report.report {
                    Some(resource_report::Report::Snapshot(snapshot)) => {
                        (ResourceReportMode::Snapshot, snapshot.views)
                    }
                    Some(resource_report::Report::Delta(delta)) => {
                        (ResourceReportMode::Delta, delta.views)
                    }
                    None => {
                        return Err(Status::invalid_argument(
                            "resource report is missing report variant",
                        ));
                    }
                };
                #[cfg(feature = "conformance-fault-injection")]
                let mut views = views;
                {
                    let adapters = self.adapters.lock().await;
                    let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?;
                    if record.registration.adapter_generation.as_ref() != Some(&generation) {
                        #[cfg(feature = "conformance-fault-injection")]
                        if self.conformance_fault
                            == AdapterServiceConformanceFault::IgnoreResourceGeneration
                        {
                            // Deliberate compiled-graph fault used only by the conformance runner.
                        } else {
                            return Err(Status::failed_precondition(
                                "resource report adapter generation is stale",
                            ));
                        }
                        #[cfg(not(feature = "conformance-fault-injection"))]
                        return Err(Status::failed_precondition(
                            "resource report adapter generation is stale",
                        ));
                    }
                    #[cfg(feature = "conformance-fault-injection")]
                    if self.conformance_fault
                        == AdapterServiceConformanceFault::NormalizeResourceOwnerToAuthenticatedAdapter
                    {
                        for view in &mut views {
                            for mutation in &mut view.mutations {
                                if let Some(identity) = mutation.identity.as_mut() {
                                    identity.adapter_id = Some(authenticated_adapter.clone());
                                }
                            }
                        }
                    }
                    validate_resource_views(&adapters, &authenticated_adapter, mode, &views)?;
                }
                #[cfg(feature = "conformance-fault-injection")]
                if self.conformance_fault
                    == AdapterServiceConformanceFault::IgnoreEmptyPartialResourceReport
                    && mode == ResourceReportMode::Snapshot
                    && views.iter().all(|view| {
                        view.completeness == AdapterSnapshotSupport::Partial as i32
                            && view.mutations.is_empty()
                    })
                {
                    return Ok(Response::new(ObservationResult { event_id: None }));
                }
                let rebuilt = resource::rebuild_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_resource_error)?;
                let mut resources = self.resources.lock().await;
                *resources = rebuilt;
                let result = resource::ingest_resource_report(
                    &self.storage,
                    &mut resources,
                    ValidatedResourceReport {
                        authority_domain_id: domain.clone(),
                        adapter_id: authenticated_adapter.clone(),
                        adapter_generation: generation,
                        mode,
                        views,
                        observed_at,
                    },
                )
                .await
                .map_err(map_resource_error)?;
                Some(result.event_id)
            }
            Some(observation_request::Observation::Event(mut observation)) => {
                if observation.authority_domain_id.as_ref() != Some(&domain) {
                    return Err(Status::invalid_argument(
                        "observation authority domain does not match request",
                    ));
                }
                require_same_adapter(
                    observation
                        .target_scope
                        .as_ref()
                        .and_then(target_adapter_id),
                    &authenticated_adapter,
                )?;
                let (canonical_sender, source_attachment, adapter_projection) = {
                    let adapters = self.adapters.lock().await;
                    let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?;
                    let registration = &record.registration;
                    let generation = registration
                        .adapter_generation
                        .ok_or_else(|| Status::internal("attached adapter has no generation"))?;
                    (
                        ActorEndpointRef {
                            actor_id: Some(ActorId {
                                value: authenticated_adapter.value.clone(),
                            }),
                            endpoint_id: Some(registration.endpoint_id.clone().ok_or_else(
                                || Status::internal("attached adapter has no registered endpoint"),
                            )?),
                            ..ActorEndpointRef::default()
                        },
                        RuntimeEvidenceSourceAttachment {
                            adapter_id: Some(authenticated_adapter.clone()),
                            adapter_generation: Some(generation),
                            attachment_event_id: Some(record.attach_event_id.clone()),
                        },
                        adapters.clone(),
                    )
                };
                canonicalize_observation_sender(&mut observation, canonical_sender)?;
                if adapter::is_adapter_registration(&observation) {
                    return Err(Status::invalid_argument(
                        "adapter registration is accepted only through Attach",
                    ));
                }
                let mut commands = self.commands.lock().await;
                catch_up_command_projection(&self.storage, &domain, &mut commands)
                    .await
                    .map_err(map_acceptance_error_to_status)?;

                let runtime_target = observation.target_scope.as_ref().and_then(|scope| {
                    (TargetScopeKind::try_from(scope.kind).ok()
                        == Some(TargetScopeKind::RuntimeSession))
                    .then(|| patchbay_contracts::patchbay::ExternalRuntimeRef {
                        adapter_id: scope.adapter_id.clone(),
                        deployment_scope: scope.deployment_scope.clone(),
                        runtime_session_id: scope.runtime_session_id.clone(),
                        generation: scope.session_generation,
                    })
                });
                if let Some(runtime_target) = runtime_target {
                    let sessions =
                        recover_session_registry(&self.storage, &domain, &self.core_generation)
                            .await
                            .map_err(map_session_error)?
                            .registry;
                    let claims = session::rebuild_spawn_claims_from_log(&self.storage, &domain)
                        .await
                        .map_err(map_spawn_claim_error)?;
                    let disposition = session::classify_runtime_target(
                        &domain,
                        &runtime_target,
                        &source_attachment,
                        &adapter_projection,
                        &sessions,
                    );
                    let late_terminal =
                        acceptance::exact_command_correlation(&observation.correlations)
                            .and_then(|command_id| commands.index.get_command(&command_id))
                            .is_some_and(|record| record.state.is_terminal());
                    if late_terminal && acceptance::derive_transition(&observation).is_some() {
                        if let Some(event_id) = self
                            .storage
                            .reconcile_observation_retry(&domain, observation.clone())
                            .await
                            .map_err(map_storage_error_to_status)?
                        {
                            return Ok(Response::new(ObservationResult {
                                event_id: Some(event_id),
                            }));
                        }
                    }
                    let is_current = matches!(
                        disposition.disposition.as_ref(),
                        Some(runtime_generation_disposition::Disposition::Current(_))
                    );
                    if !is_current || late_terminal {
                        let reason = if late_terminal && is_current {
                            RuntimeEvidenceQuarantineReason::StaleSourceOrder
                        } else {
                            session::quarantine_reason_for(&disposition)
                        };
                        let quarantined = session::quarantined_observation(
                            &domain,
                            observation,
                            disposition,
                            reason,
                            source_attachment,
                            &sessions,
                            &claims,
                        )
                        .map_err(|error| Status::failed_precondition(error.to_string()))?;
                        let mut audit = AuditRecordDraft::new(
                            crate::identity::now_timestamp()
                                .map_err(|error| Status::internal(error.to_string()))?,
                            patchbay_contracts::patchbay::AuditEventKind::StaleEventIgnored,
                        );
                        audit.failure_code = Some(FailureCode::StaleEvent);
                        audit.reason_code = session::quarantine_reason_code(reason).to_owned();
                        audit.target_scope = Some(
                            session::quarantined_candidate_scope(&quarantined)
                                .map_err(|error| Status::failed_precondition(error.to_string()))?,
                        );
                        audit.command_id = match quarantined.candidate.as_ref() {
                            Some(
                                patchbay_contracts::patchbay::quarantined_runtime_evidence::Candidate::Observation(
                                    nested,
                                ),
                            ) => acceptance::exact_command_correlation(&nested.correlations),
                            _ => None,
                        };
                        let committed = self
                            .storage
                            .append_quarantined_runtime_evidence_audited(
                                &domain,
                                quarantined,
                                audit,
                            )
                            .await
                            .map_err(map_storage_error_to_status)?;
                        catch_up_command_projection(&self.storage, &domain, &mut commands)
                            .await
                            .map_err(map_acceptance_error_to_status)?;
                        return Ok(Response::new(ObservationResult {
                            event_id: Some(committed.source_event_id),
                        }));
                    }
                }
                poison_ambiguous_spawn_result(
                    &self.storage,
                    &commands.index,
                    &domain,
                    &authenticated_adapter,
                    &source_attachment,
                    &observation,
                )
                .await?;
                catch_up_command_projection(&self.storage, &domain, &mut commands)
                    .await
                    .map_err(map_acceptance_error_to_status)?;
                let event_id = if adapter::is_delivery_acknowledgement(&observation) {
                    adapter::ingest_delivery_acknowledgement(
                        &self.storage,
                        &commands.index,
                        observation,
                    )
                    .await
                    .map_err(map_adapter_error)?
                    .observation_event_id
                } else {
                    match acceptance::ingest_observation(
                        &self.storage,
                        &commands.index,
                        observation,
                    )
                    .await
                    .map_err(map_acceptance_error_to_status)?
                    {
                        acceptance::IngestResult::Recorded { event_id }
                        | acceptance::IngestResult::CompletionDeferred {
                            observation_event_id: event_id,
                        }
                        | acceptance::IngestResult::StaleCandidate {
                            observation_event_id: event_id,
                        }
                        | acceptance::IngestResult::Transitioned {
                            observation_event_id: event_id,
                            ..
                        } => event_id,
                    }
                };
                catch_up_command_projection(&self.storage, &domain, &mut commands)
                    .await
                    .map_err(map_acceptance_error_to_status)?;
                Some(event_id)
            }
            None => return Err(Status::invalid_argument("missing observation")),
        };

        Ok(Response::new(ObservationResult { event_id }))
    }

    type ReceiveDeliveriesStream = DeliveryStream;

    async fn receive_deliveries(
        &self,
        request: Request<ReceiveRequest>,
    ) -> Result<Response<Self::ReceiveDeliveriesStream>, Status> {
        // Stream establishment is a decision too. Authenticate and re-read
        // the current attachment while holding the shared gate, before
        // taking the projection prefix used by the stream.
        let _decision_guard = self.decision_gate.acquire().await;
        let authenticated_adapter = self.authenticate_request(&request).await?;
        let request = request.into_inner();
        require_same_adapter(request.adapter_id.as_ref(), &authenticated_adapter)?;
        let domain = self.require_attached(&authenticated_adapter).await?;
        let stream_epoch = {
            let mut epochs = self.delivery_stream_epochs.lock().await;
            let epoch = epochs.entry(authenticated_adapter.clone()).or_default();
            *epoch = epoch
                .checked_add(1)
                .ok_or_else(|| Status::internal("delivery stream epoch overflow"))?;
            *epoch
        };
        let initial_cursor = request.cursor.map_or(0, |cursor| cursor.value);
        let (initial_events, initial_projection) = {
            // Establish one complete projection and remember its durable prefix.
            // Every later scan applies only the tail beyond this projection cursor.
            let mut live_commands = self.commands.lock().await;
            let events = self
                .storage
                .read_after(&domain, patchbay_contracts::patchbay::Lsn { value: 0 })
                .await
                .map_err(map_storage_error_to_status)?;
            *live_commands = command_projection_from_events(&events, &domain)
                .map_err(map_acceptance_error_to_status)?;
            (events, live_commands.clone())
        };

        let stale_spawn_source = {
            let adapters = self.adapters.lock().await;
            let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                Status::failed_precondition("adapter is no longer durably attached")
            })?;
            SpawnEvidenceAttachment {
                adapter_id: Some(authenticated_adapter.clone()),
                adapter_generation: record.registration.adapter_generation,
                attachment_event_id: Some(record.attach_event_id.clone()),
            }
        };
        let storage = self.storage.clone();
        let commands = Arc::clone(&self.commands);
        let adapters = Arc::clone(&self.adapters);
        let sessions = Arc::clone(&self.sessions);
        let resources = Arc::clone(&self.resources);
        let delivery_stream_epochs = Arc::clone(&self.delivery_stream_epochs);
        let audit = Arc::clone(&self.audit);
        let decision_gate = self.decision_gate.clone();
        let stale_domain = domain.clone();
        let stale_adapter = authenticated_adapter.clone();
        let core_generation = self.core_generation;
        #[cfg(feature = "conformance-fault-injection")]
        let conformance_fault = self.conformance_fault;
        let on_abnormal_disconnect: DisconnectCallback = Box::new(move || {
            let task = async move {
                // The shared decision gate prevents revocation from planning
                // against a projection that disconnect reconciliation can
                // transition before its durable append. Holding it before the
                // epoch guard also gives every command writer one order.
                let _decision_guard = decision_gate.acquire().await;
                // Holding the epoch guard through reconciliation establishes a
                // total order with a replacement stream. An obsolete stream's
                // delayed drop cannot mutate the replacement attachment.
                let epochs = delivery_stream_epochs.lock().await;
                if epochs.get(&stale_adapter) != Some(&stream_epoch) {
                    return;
                }

                let command_result = {
                    let mut projection = commands.lock().await;
                    let caught_up =
                        catch_up_command_projection(&storage, &stale_domain, &mut projection).await;
                    let reconciled = match caught_up {
                        Ok(_) => {
                            let poisoned = poison_ambiguous_spawn_claims_for_adapter(
                                &storage,
                                &projection.index,
                                &stale_domain,
                                &stale_adapter,
                                stale_spawn_source,
                            )
                            .await;
                            let failed = adapter::fail_running_commands_for_adapter(
                                &storage,
                                &projection.index,
                                &stale_domain,
                                &stale_adapter,
                            )
                            .await
                            .map(|_| ());
                            poisoned.and(failed)
                        }
                        Err(error) => Err(adapter::AdapterError::CorruptRecord(error.to_string())),
                    };
                    let rebuilt =
                        catch_up_command_projection(&storage, &stale_domain, &mut projection).await;
                    reconciled.and_then(|()| {
                        rebuilt.map(|_| ()).map_err(|error| {
                            adapter::AdapterError::CorruptRecord(error.to_string())
                        })
                    })
                };

                let state_result: Result<(), String> = async {
                    let rebuilt_sessions =
                        recover_session_registry(&storage, &stale_domain, &core_generation)
                            .await
                            .map_err(|error| error.to_string())?
                            .registry;
                    let rebuilt_resources = resource::rebuild_from_log(&storage, &stale_domain)
                        .await
                        .map_err(|error| error.to_string())?;
                    let adapter_generation = adapters
                        .lock()
                        .await
                        .get(&stale_adapter)
                        .and_then(|record| record.registration.adapter_generation)
                        .ok_or_else(|| "detached adapter has no current generation".to_owned())?;
                    let mut sources = session::adapter_stale_events(
                        &rebuilt_sessions,
                        &stale_domain,
                        &stale_adapter,
                    )
                    .map_err(|error| error.to_string())?;
                    #[cfg(feature = "conformance-fault-injection")]
                    let keep_resources_current = conformance_fault
                        == AdapterServiceConformanceFault::KeepResourcesCurrentOnDisconnect;
                    #[cfg(not(feature = "conformance-fault-injection"))]
                    let keep_resources_current = false;
                    if !keep_resources_current {
                        if let Some(source) = resource::adapter_stale_event(
                            &rebuilt_resources,
                            &stale_domain,
                            &stale_adapter,
                            adapter_generation,
                            crate::identity::now_timestamp().map_err(|error| error.to_string())?,
                        )
                        .map_err(|error| error.to_string())?
                        {
                            sources.push(source);
                        }
                    }
                    let mut audit_draft = AuditRecordDraft::new(
                        crate::identity::now_timestamp().map_err(|error| error.to_string())?,
                        patchbay_contracts::patchbay::AuditEventKind::AdapterDetached,
                    );
                    audit_draft.actor_id = Some(patchbay_contracts::patchbay::ActorId {
                        value: stale_adapter.value.clone(),
                    });
                    audit_draft.reason_code = "adapter_detached".to_owned();
                    if sources.is_empty() {
                        storage
                            .append_audit(&stale_domain, audit_draft)
                            .await
                            .map_err(|error| error.to_string())?;
                    } else {
                        storage
                            .append_batch_audited(&stale_domain, sources, audit_draft)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    *sessions.lock().await =
                        recover_session_registry(&storage, &stale_domain, &core_generation)
                            .await
                            .map_err(|error| error.to_string())?
                            .registry;
                    *resources.lock().await = resource::rebuild_from_log(&storage, &stale_domain)
                        .await
                        .map_err(|error| error.to_string())?;
                    Ok(())
                }
                .await;
                drop(epochs);

                let reconciliation_failed = command_result.is_err() || state_result.is_err();
                // A successful detach writes session/resource degradation and
                // the one ADAPTER_DETACHED audit in the same batch. Only the
                // failure path has no trustworthy paired source and needs a
                // standalone audit.
                if reconciliation_failed {
                    if let Err(error) = record_adapter_audit(
                        audit.as_ref(),
                        patchbay_contracts::patchbay::AuditEventKind::AdapterFailed,
                        &stale_adapter,
                        None,
                        "adapter_disconnect_reconciliation_failed",
                    )
                    .await
                    {
                        eprintln!("patchbay-core-server: failed to record adapter lifecycle audit: {error}");
                    }
                }
                if let Err(error) = command_result {
                    eprintln!(
                        "patchbay-core-server: failed to reconcile running commands after adapter disconnect: {error}"
                    );
                }
                if let Err(error) = state_result {
                    eprintln!(
                        "patchbay-core-server: failed to mark adapter state stale after disconnect: {error}"
                    );
                }
            };
            match tokio::runtime::Handle::try_current() {
                Ok(runtime) => {
                    runtime.spawn(task);
                }
                Err(error) => {
                    eprintln!(
                        "patchbay-core-server: cannot record abnormal adapter disconnect outside a Tokio runtime: {error}"
                    );
                }
            }
        });

        drop(_decision_guard);
        let subscription = delivery_subscription(
            DeliverySubscriptionContext {
                storage: self.storage.clone(),
                authority_domain_id: domain,
                adapter_id: authenticated_adapter,
                commands: Arc::clone(&self.commands),
                delivery_stream_epochs: Arc::clone(&self.delivery_stream_epochs),
                stream_epoch,
                decision_gate: self.decision_gate.clone(),
            },
            initial_cursor,
            initial_events,
            initial_projection,
        );
        let tail = DeliveryTail::new(subscription, on_abnormal_disconnect);
        Ok(Response::new(Box::pin(tail)))
    }
}

fn validate_resource_views(
    adapters: &AdapterRegistry,
    authenticated_adapter: &AdapterId,
    mode: ResourceReportMode,
    views: &[patchbay_contracts::patchbay::ResourceViewReport],
) -> Result<(), Status> {
    for view in views {
        let kind = view
            .resource_kind
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("resource view is missing resource_kind"))?;
        let declared = adapters
            .get(authenticated_adapter)
            .and_then(|record| record.validated_capability.resource(kind))
            .ok_or_else(|| Status::invalid_argument("resource kind is not declared"))?;
        let reported = AdapterSnapshotSupport::try_from(view.completeness)
            .map_err(|_| Status::invalid_argument("resource view has unknown completeness"))?;
        if reported == AdapterSnapshotSupport::Unspecified
            || snapshot_strength(reported) > snapshot_strength(declared.snapshot_support())
        {
            return Err(Status::invalid_argument(
                "resource report completeness exceeds the declared tier",
            ));
        }
        for mutation in &view.mutations {
            let identity =
                ResourceIdentity::try_from_wire(mutation.identity.as_ref().ok_or_else(|| {
                    Status::invalid_argument("resource mutation is missing identity")
                })?)
                .map_err(|error| Status::invalid_argument(error.to_string()))?;
            if identity.adapter_id() != authenticated_adapter || identity.resource_kind() != kind {
                return Err(Status::permission_denied(
                    "resource mutation identity does not match authenticated view",
                ));
            }
            let mutation = mutation.mutation.as_ref().ok_or_else(|| {
                Status::invalid_argument("resource mutation is missing mutation variant")
            })?;
            if mode == ResourceReportMode::Snapshot
                && reported == AdapterSnapshotSupport::Authoritative
                && matches!(mutation, resource_report_mutation::Mutation::Unknown(_))
            {
                return Err(Status::invalid_argument(
                    "authoritative snapshot cannot list an unknown resource",
                ));
            }
            if let resource_report_mutation::Mutation::Upsert(upsert) = mutation {
                let payload = upsert.resource_payload.as_ref().ok_or_else(|| {
                    Status::invalid_argument("resource upsert is missing resource payload")
                })?;
                let projection = upsert.projection_payload.as_ref().ok_or_else(|| {
                    Status::invalid_argument("resource upsert is missing projection payload")
                })?;
                adapters
                    .validate_resource_projection(&identity, payload, projection)
                    .map_err(|error| Status::invalid_argument(error.to_string()))?;
            }
        }
    }
    Ok(())
}

const fn snapshot_strength(tier: AdapterSnapshotSupport) -> u8 {
    match tier {
        AdapterSnapshotSupport::Authoritative => 3,
        AdapterSnapshotSupport::Partial => 2,
        AdapterSnapshotSupport::None => 1,
        AdapterSnapshotSupport::Unspecified => 0,
    }
}

fn require_same_adapter(actual: Option<&AdapterId>, expected: &AdapterId) -> Result<(), Status> {
    if actual != Some(expected) {
        return Err(Status::permission_denied(
            "authenticated adapter does not match request target",
        ));
    }
    Ok(())
}

fn canonicalize_observation_sender(
    observation: &mut Observation,
    canonical: ActorEndpointRef,
) -> Result<(), Status> {
    if observation.sender.as_ref().is_some_and(|claimed| {
        claim_conflicts(&claimed.actor_id, &canonical.actor_id)
            || claim_conflicts(&claimed.endpoint_id, &canonical.endpoint_id)
            || claim_conflicts(&claimed.device_id, &canonical.device_id)
            || claim_conflicts(&claimed.endpoint_generation, &canonical.endpoint_generation)
    }) {
        return Err(Status::permission_denied(
            "observation sender does not match authenticated adapter attachment",
        ));
    }
    observation.sender = Some(canonical);
    Ok(())
}

fn claim_conflicts<T: PartialEq>(claimed: &Option<T>, verified: &Option<T>) -> bool {
    claimed
        .as_ref()
        .is_some_and(|value| verified.as_ref() != Some(value))
}

fn session_result_event_id(
    result: session::IngestResult,
) -> Option<patchbay_contracts::patchbay::EventId> {
    let event_id = match result {
        session::IngestResult::Registered { event_id }
        | session::IngestResult::ReportApplied { event_id }
        | session::IngestResult::GenerationBumped { event_id, .. } => event_id,
    };
    Some(event_id)
}

async fn record_adapter_audit(
    audit: &dyn AuditSink,
    kind: patchbay_contracts::patchbay::AuditEventKind,
    adapter_id: &AdapterId,
    failure_code: Option<patchbay_contracts::patchbay::FailureCode>,
    reason: &str,
) -> Result<(), Status> {
    let mut draft = AuditRecordDraft::new(crate::identity::now_timestamp()?, kind);
    draft.actor_id = Some(patchbay_contracts::patchbay::ActorId {
        value: adapter_id.value.clone(),
    });
    draft.failure_code = failure_code;
    draft.reason_code = reason.to_owned();
    audit
        .record(draft)
        .await
        .map_err(|error| Status::unavailable(error.to_string()))?;
    Ok(())
}

fn map_adapter_error(error: adapter::AdapterError) -> Status {
    match error {
        adapter::AdapterError::InvalidRegistration(message) => Status::invalid_argument(message),
        adapter::AdapterError::StaleGeneration { .. } => {
            Status::failed_precondition(error.to_string())
        }
        adapter::AdapterError::InvalidDeliveryAcknowledgement(message) => {
            Status::failed_precondition(message)
        }
        adapter::AdapterError::CorruptRecord(message)
        | adapter::AdapterError::CorruptLog(message) => Status::internal(message),
        adapter::AdapterError::Resource(error) => map_resource_error(error),
        adapter::AdapterError::Storage(error) => map_storage_error_to_status(error),
    }
}

fn map_spawn_claim_error(error: session::SpawnClaimError) -> Status {
    match error {
        session::SpawnClaimError::UnknownClaim(_)
        | session::SpawnClaimError::GenerationAlreadyClaimed(_)
        | session::SpawnClaimError::ClaimRuntimeConflict { .. }
        | session::SpawnClaimError::ExternalRuntimeOwnershipConflict { .. }
        | session::SpawnClaimError::IllegalDispositionTransition { .. } => {
            Status::failed_precondition(error.to_string())
        }
        session::SpawnClaimError::CorruptRecord(message) => Status::invalid_argument(message),
        session::SpawnClaimError::EmptyAuthorityDomain
        | session::SpawnClaimError::CorruptLog(_)
        | session::SpawnClaimError::LogicalTarget(_)
        | session::SpawnClaimError::Storage(_) => Status::internal(error.to_string()),
    }
}

fn map_resource_error(error: resource::ResourceError) -> Status {
    match error {
        resource::ResourceError::InvalidReport(_)
        | resource::ResourceError::Identity(_)
        | resource::ResourceError::TerminalTombstone(_)
        | resource::ResourceError::StaleAdapterGeneration { .. } => {
            Status::invalid_argument(error.to_string())
        }
        resource::ResourceError::CorruptRecord(message) => Status::invalid_argument(message),
        resource::ResourceError::CorruptLog(message) => Status::internal(message),
        resource::ResourceError::Storage(error) => map_storage_error_to_status(error),
    }
}

fn map_session_error(error: session::SessionError) -> Status {
    match error {
        session::SessionError::InvalidTransition { .. }
        | session::SessionError::StaleGeneration { .. }
        | session::SessionError::StaleSourceCursor { .. } => {
            Status::failed_precondition(error.to_string())
        }
        error @ (session::SessionError::EmptyAuthorityDomain
        | session::SessionError::AuthorityDomainMismatch { .. }) => {
            Status::internal(error.to_string())
        }
        session::SessionError::CorruptRecord(message) => Status::invalid_argument(message),
        session::SessionError::LogicalTarget(error) => {
            Status::failed_precondition(error.to_string())
        }
        session::SessionError::CorruptLog(message) => Status::internal(message),
        session::SessionError::Storage(error) => map_storage_error_to_status(error),
    }
}
