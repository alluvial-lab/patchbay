use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use patchbay_contracts::patchbay::{
    observation_request, resource_report, resource_report_mutation, AcceptedOperation,
    AdapterDiagnosticReport, AdapterDiagnosticReportResult, AdapterId, AdapterSnapshotSupport,
    AttachRequest, AttachResult, AuthorityDomainId, Delivery, FailureCode, Generation,
    ObservationRequest, ObservationResult, OperationState, ReceiveRequest,
    SessionActivityState, SessionConnectivityState, StoredEventKind,
};
use patchbay_core::{
    acceptance::{self, CommandIndex},
    audit::{AuditSink, DurableAuditSink, RequiredAuditFanout, StderrAuditSink},
    diagnostics::{ingest_adapter_diagnostic, validate_adapter_diagnostic_report},
    adapter::{self, AdapterRegistry},
    authority::hash_principal_credential,
    resource::{self, ResourceIdentity, ResourceRegistry, ResourceReportMode, ValidatedResourceReport},
    session::{self, SessionRegistry, SessionReport},
    storage::{AuditRecordDraft, RecordedEvent, Storage},
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
    identity::random_token,
    rpc::adapter_control_service_server::AdapterControlService,
    service::{map_acceptance_error_to_status, map_storage_error_to_status},
};

pub const ADAPTER_ID_HEADER: &str = "x-patchbay-adapter-id";
pub const ADAPTER_EVIDENCE_HEADER: &str = "x-patchbay-adapter-evidence";
pub const ADAPTER_ATTACHMENT_TOKEN_HEADER: &str = "x-patchbay-adapter-attachment-token";

#[derive(Clone)]
pub struct AdapterEvidenceVerifier {
    expected: Arc<[u8]>,
}

impl AdapterEvidenceVerifier {
    pub fn new(evidence: impl Into<String>) -> Result<Self, String> {
        let evidence = evidence.into();
        if evidence.is_empty() {
            return Err(
                "PATCHBAY_ADAPTER_ATTACHMENT_SECRET must be configured and non-empty".into(),
            );
        }
        if !evidence.is_ascii() {
            return Err("PATCHBAY_ADAPTER_ATTACHMENT_SECRET must be ASCII".into());
        }
        Ok(Self {
            expected: evidence.into_bytes().into(),
        })
    }

    fn verify_attach(&self, evidence: &[u8]) -> Result<(), Status> {
        if constant_time_eq(evidence, &self.expected) {
            Ok(())
        } else {
            Err(Status::unauthenticated(
                "invalid adapter attachment evidence",
            ))
        }
    }

    fn verify_request<T>(&self, request: &Request<T>) -> Result<AdapterId, Status> {
        let evidence = request
            .metadata()
            .get(ADAPTER_EVIDENCE_HEADER)
            .map(|value| value.as_encoded_bytes())
            .ok_or_else(|| Status::unauthenticated("missing adapter attachment evidence"))?;
        if !constant_time_eq(evidence, &self.expected) {
            return Err(Status::unauthenticated(
                "invalid adapter attachment evidence",
            ));
        }
        let adapter_id = request
            .metadata()
            .get(ADAPTER_ID_HEADER)
            .ok_or_else(|| Status::unauthenticated("missing adapter id"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid adapter id"))?;
        if adapter_id.is_empty() {
            return Err(Status::unauthenticated("adapter id must not be empty"));
        }
        Ok(AdapterId {
            value: adapter_id.to_owned(),
        })
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

#[derive(Clone)]
pub struct AdapterControlServiceImpl<S> {
    storage: S,
    authority_domain_id: AuthorityDomainId,
    evidence: AdapterEvidenceVerifier,
    audit: Arc<dyn AuditSink>,
    adapters: Arc<Mutex<AdapterRegistry>>,
    commands: Arc<Mutex<CommandProjection>>,
    sessions: Arc<Mutex<SessionRegistry>>,
    resources: Arc<Mutex<ResourceRegistry>>,
    attachment_tokens: Arc<Mutex<HashMap<AdapterId, Vec<u8>>>>,
    delivery_stream_epochs: Arc<Mutex<HashMap<AdapterId, u64>>>,
    decision_gate: CoreDecisionGate,
}

impl<S> AdapterControlServiceImpl<S>
where
    S: Storage + Clone + Send + Sync + 'static,
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
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".into());
        }
        let adapters = adapter::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let commands = rebuild_command_projection(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let sessions = session::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let resources = resource::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let audit: Arc<dyn AuditSink> = Arc::new(RequiredAuditFanout::new(
            Arc::new(DurableAuditSink::new(storage.clone(), authority_domain_id.clone())),
            vec![Arc::new(StderrAuditSink)],
        ));
        Ok(Self {
            storage,
            authority_domain_id,
            evidence,
            audit,
            adapters: Arc::new(Mutex::new(adapters)),
            commands: Arc::new(Mutex::new(commands)),
            sessions: Arc::new(Mutex::new(sessions)),
            resources: Arc::new(Mutex::new(resources)),
            attachment_tokens: Arc::new(Mutex::new(HashMap::new())),
            delivery_stream_epochs: Arc::new(Mutex::new(HashMap::new())),
            decision_gate,
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
}

const DELIVERY_SCAN_INTERVAL: Duration = Duration::from_millis(100);

type DeliveryStream = Pin<Box<dyn Stream<Item = Result<Delivery, Status>> + Send + 'static>>;
type DisconnectCallback = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug, Clone)]
struct CommandProjection {
    index: CommandIndex,
    cursor: u64,
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
    command_projection_from_events(&events)
}

fn command_projection_from_events(
    events: &[RecordedEvent],
) -> Result<CommandProjection, acceptance::AcceptanceError> {
    let mut index = CommandIndex::new();
    let mut cursor = 0;
    for event in events {
        index.apply(event)?;
        cursor = recorded_event_lsn(event)?;
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
    for event in &events {
        projection.index.apply(event)?;
        projection.cursor = recorded_event_lsn(event)?;
    }
    Ok(events)
}

fn recorded_event_lsn(event: &RecordedEvent) -> Result<u64, acceptance::AcceptanceError> {
    event
        .event_id
        .lsn
        .as_ref()
        .map(|lsn| lsn.value)
        .ok_or_else(|| acceptance::AcceptanceError::CorruptRecord("event has no LSN".into()))
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
        .filter_map(|event| {
            if event.payload.kind != StoredEventKind::Operation as i32 {
                return None;
            }
            let accepted = match AcceptedOperation::decode(event.payload.payload.as_slice()) {
                Ok(accepted) => accepted,
                Err(error) => {
                    return Some(Err(Status::internal(format!(
                        "cannot decode accepted operation: {error}"
                    ))))
                }
            };
            let operation = match accepted.operation {
                Some(operation) => operation,
                None => return Some(Err(Status::internal("accepted operation has no operation"))),
            };
            let targets_adapter = operation
                .target_scope
                .as_ref()
                .and_then(target_adapter_id)
                == Some(adapter_id);
            let remains_deliverable = operation
                .command_id
                .as_ref()
                .and_then(|command_id| commands.get_command(command_id))
                .is_some_and(|record| {
                    matches!(
                        record.state,
                        OperationState::Accepted | OperationState::Delivered
                    )
                });
            (targets_adapter && remains_deliverable).then_some(Ok(Delivery {
                operation: Some(operation),
                delivery_event_id: Some(event.event_id.clone()),
            }))
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
    } = context;
    let (sender, receiver) = mpsc::channel(16);
    tokio::spawn(async move {
        let mut delivery_cursor = initial_cursor.max(initial_projection.cursor);
        let mut scan_cursor = initial_projection.cursor;
        let mut subscription_commands = initial_projection.index;
        let initial = deliveries_for_events(
            &initial_events,
            &subscription_commands,
            &adapter_id,
            initial_cursor,
        );
        for delivery in initial {
            if sender.send(delivery).await.is_err() {
                return;
            }
        }

        loop {
            if sender.is_closed() {
                return;
            }
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
                        subscription_commands.apply(event)?;
                        scan_cursor = recorded_event_lsn(event)?;
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
                                    let deliveries = deliveries_for_events(
                                        &events,
                                        &subscription_commands,
                                        &adapter_id,
                                        delivery_cursor,
                                    );
                                    delivery_cursor = delivery_cursor.max(scan_cursor);
                                    Ok((events.is_empty(), deliveries))
                                }
                                Err(error) => Err(map_acceptance_error_to_status(error)),
                            }
                        }
                        Err(error) => Err(map_acceptance_error_to_status(error)),
                    }
                }
                Err(error) => Err(map_storage_error_to_status(error)),
            };
            drop(epochs);

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
    S: Storage + Clone + Send + Sync + 'static,
{
    async fn attach(
        &self,
        request: Request<AttachRequest>,
    ) -> Result<Response<AttachResult>, Status> {
        let request = request.into_inner();
        self.evidence.verify_attach(&request.attachment_evidence)?;
        let registration = request
            .registration
            .ok_or_else(|| Status::invalid_argument("attach request is missing registration"))?;
        let domain = registration.authority_domain_id.as_ref().ok_or_else(|| {
            Status::invalid_argument("registration is missing authority_domain_id")
        })?;
        self.require_domain(domain)?;
        let adapter_id = registration
            .adapter_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("registration is missing adapter_id"))?;
        let attachment_token = random_token();
        let attachment_token_hash = hash_principal_credential(&attachment_token);
        // Registration, token replacement, and every adapter decision are
        // ordered by the composition-root gate. A request that authenticated
        // against the old token before this point must not establish a
        // decision after this replacement commits.
        let _decision_guard = self.decision_gate.acquire().await;
        let mut adapters = self.adapters.lock().await;
        let event_id = match adapter::ingest_registration(&self.storage, &mut adapters, registration)
            .await
        {
            Ok(event_id) => event_id,
            Err(error) => {
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
                .ok_or_else(|| Status::unauthenticated("adapter attachment is not current; reattach required"))?
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
        let receipt = ingest_adapter_diagnostic(
            &self.storage,
            &self.authority_domain_id,
            validated,
        )
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
            Some(observation_request::Observation::SessionReport(report)) => {
                require_same_adapter(report.adapter_id.as_ref(), &authenticated_adapter)?;
                let report = SessionReport {
                    authority_domain_id: domain.clone(),
                    adapter_id: report.adapter_id.ok_or_else(|| {
                        Status::invalid_argument("session report is missing adapter_id")
                    })?,
                    deployment_scope: report.deployment_scope,
                    runtime_session_id: report.runtime_session_id.ok_or_else(|| {
                        Status::invalid_argument("session report is missing runtime_session_id")
                    })?,
                    session_generation: report
                        .session_generation
                        .unwrap_or(Generation { value: 0 }),
                    connectivity: SessionConnectivityState::try_from(report.connectivity)
                        .map_err(|_| Status::invalid_argument("unknown connectivity state"))?,
                    activity: SessionActivityState::try_from(report.activity)
                        .map_err(|_| Status::invalid_argument("unknown activity state"))?,
                    project: report.project,
                    cwd: report.cwd,
                    name: report.name,
                    model: report.model,
                    spawn_origin: report.spawn_origin,
                };
                // The adapter owns an independent session projection. Rebuild
                // it at the gate boundary before deriving the next report
                // delta; otherwise a lockdown (or any core-side append) can
                // leave this writer with a stale pre-event view and produce a
                // live registration/transition that replay correctly rejects.
                let rebuilt = session::rebuild_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_session_error)?;
                let mut sessions = self.sessions.lock().await;
                *sessions = rebuilt;
                let result = match session::ingest_session_report(&self.storage, &mut *sessions, report)
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        let kind = if matches!(error, session::SessionError::StaleGeneration { .. }) {
                            patchbay_contracts::patchbay::AuditEventKind::TargetGenerationMismatch
                        } else {
                            patchbay_contracts::patchbay::AuditEventKind::AdapterFailed
                        };
                        record_adapter_audit(
                            self.audit.as_ref(),
                            kind,
                            &authenticated_adapter,
                            None,
                            "session_report_rejected",
                        )
                        .await?;
                        return Err(map_session_error(error));
                    }
                };
                let rebuilt = session::rebuild_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_session_error)?;
                *sessions = rebuilt;
                session_result_event_id(result)
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
                {
                    let adapters = self.adapters.lock().await;
                    let record = adapters.get(&authenticated_adapter).ok_or_else(|| {
                        Status::unauthenticated(
                            "adapter attachment is not current; reattach required",
                        )
                    })?;
                    if record.registration.adapter_generation.as_ref() != Some(&generation) {
                        return Err(Status::failed_precondition(
                            "resource report adapter generation is stale",
                        ));
                    }
                    validate_resource_views(
                        &adapters,
                        &authenticated_adapter,
                        mode,
                        &views,
                    )?;
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
            Some(observation_request::Observation::Event(observation)) => {
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
                let mut commands = self.commands.lock().await;
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
            *live_commands =
                command_projection_from_events(&events).map_err(map_acceptance_error_to_status)?;
            (events, live_commands.clone())
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
                    let failed = match caught_up {
                        Ok(_) => adapter::fail_running_commands_for_adapter(
                            &storage,
                            &projection.index,
                            &stale_domain,
                            &stale_adapter,
                        )
                        .await
                        .map(|_| ()),
                        Err(error) => Err(adapter::AdapterError::CorruptRecord(error.to_string())),
                    };
                    let rebuilt =
                        catch_up_command_projection(&storage, &stale_domain, &mut projection).await;
                    failed.and_then(|()| {
                        rebuilt.map(|_| ()).map_err(|error| {
                            adapter::AdapterError::CorruptRecord(error.to_string())
                        })
                    })
                };

                let state_result: Result<(), String> = async {
                    let rebuilt_sessions = session::rebuild_from_log(&storage, &stale_domain)
                        .await
                        .map_err(|error| error.to_string())?;
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
                    *sessions.lock().await = session::rebuild_from_log(&storage, &stale_domain)
                        .await
                        .map_err(|error| error.to_string())?;
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
                    .await {
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
        let kind = view.resource_kind.as_ref().ok_or_else(|| {
            Status::invalid_argument("resource view is missing resource_kind")
        })?;
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
            let identity = ResourceIdentity::try_from_wire(
                mutation.identity.as_ref().ok_or_else(|| {
                    Status::invalid_argument("resource mutation is missing identity")
                })?,
            )
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

fn session_result_event_id(
    result: session::IngestResult,
) -> Option<patchbay_contracts::patchbay::EventId> {
    match result {
        session::IngestResult::Registered { event_id }
        | session::IngestResult::ConnectivityChanged { event_id, .. }
        | session::IngestResult::ActivityChanged { event_id, .. }
        | session::IngestResult::Relabeled { event_id }
        | session::IngestResult::ModelChanged { event_id, .. } => Some(event_id),
        session::IngestResult::GenerationBumped {
            new_generation_event_id,
            ..
        } => Some(new_generation_event_id),
        session::IngestResult::DeltasApplied { event_ids } => event_ids.last().cloned(),
        session::IngestResult::NoChange => None,
    }
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
        adapter::AdapterError::CorruptRecord(message) => Status::internal(message),
        adapter::AdapterError::Storage(error) => map_storage_error_to_status(error),
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
        | session::SessionError::StaleGeneration { .. } => {
            Status::failed_precondition(error.to_string())
        }
        session::SessionError::CorruptRecord(message) => Status::invalid_argument(message),
        session::SessionError::CorruptLog(message) => Status::internal(message),
        session::SessionError::Storage(error) => map_storage_error_to_status(error),
    }
}
