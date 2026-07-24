use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use patchbay_contracts::patchbay::{
    observation_request, AdapterId, AttachRequest, AttachResult, AuthorityDomainId, Delivery,
    Generation, ObservationRequest, ObservationResult, Operation, OperationState, ReceiveRequest,
    SessionActivityState, SessionConnectivityState, StoredEventKind,
};
use patchbay_core::{
    acceptance::{self, CommandIndex},
    adapter::{self, AdapterRegistry},
    authority::hash_principal_credential,
    session::{self, SessionRegistry, SessionReport},
    storage::Storage,
};
use prost::Message;
use tokio::sync::Mutex;
use tokio_stream::{self as stream, Stream};
use tonic::{Request, Response, Status};

#[cfg(test)]
mod tests;

use crate::{
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
    adapters: Arc<Mutex<AdapterRegistry>>,
    commands: Arc<Mutex<CommandIndex>>,
    sessions: Arc<Mutex<SessionRegistry>>,
    attachment_tokens: Arc<Mutex<HashMap<AdapterId, Vec<u8>>>>,
    delivery_stream_epochs: Arc<Mutex<HashMap<AdapterId, u64>>>,
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
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".into());
        }
        let adapters = adapter::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let commands = acceptance::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        let sessions = session::rebuild_from_log(&storage, &authority_domain_id)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self {
            storage,
            authority_domain_id,
            evidence,
            adapters: Arc::new(Mutex::new(adapters)),
            commands: Arc::new(Mutex::new(commands)),
            sessions: Arc::new(Mutex::new(sessions)),
            attachment_tokens: Arc::new(Mutex::new(HashMap::new())),
            delivery_stream_epochs: Arc::new(Mutex::new(HashMap::new())),
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

type DeliveryStream = Pin<Box<dyn Stream<Item = Result<Delivery, Status>> + Send + 'static>>;
type DisconnectCallback = Box<dyn FnOnce() + Send + 'static>;

/// A finite delivery tail is healthy only once the transport polls it to
/// completion. Dropping it early (or producing an error item) is the server's
/// connection-liveness signal for an abnormal adapter disconnect.
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
                // The adapter consumed the complete durable tail. This is the
                // normal v0.1.0 polling completion, not a disconnect signal.
                self.on_abnormal_disconnect.take();
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
        let mut adapters = self.adapters.lock().await;
        let event_id = adapter::ingest_registration(&self.storage, &mut adapters, registration)
            .await
            .map_err(map_adapter_error)?;
        // Keep registration acceptance and token replacement in one critical
        // section so a slower older attach cannot overwrite a newer fence.
        self.attachment_tokens
            .lock()
            .await
            .insert(adapter_id, attachment_token_hash);
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

    async fn ingest_observation(
        &self,
        request: Request<ObservationRequest>,
    ) -> Result<Response<ObservationResult>, Status> {
        let authenticated_adapter = self.authenticate_request(&request).await?;
        let request = request.into_inner();
        let domain = request
            .authority_domain_id
            .ok_or_else(|| Status::invalid_argument("missing authority_domain_id"))?;
        self.require_domain(&domain)?;
        self.require_attached(&authenticated_adapter).await?;

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
                let result = session::ingest_session_report(
                    &self.storage,
                    &mut *self.sessions.lock().await,
                    report,
                )
                .await
                .map_err(map_session_error)?;
                let rebuilt = session::rebuild_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_session_error)?;
                *self.sessions.lock().await = rebuilt;
                session_result_event_id(result)
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
                        .and_then(|target| target.adapter_id.as_ref()),
                    &authenticated_adapter,
                )?;
                let mut commands = self.commands.lock().await;
                let event_id = if adapter::is_delivery_acknowledgement(&observation) {
                    adapter::ingest_delivery_acknowledgement(&self.storage, &commands, observation)
                        .await
                        .map_err(map_adapter_error)?
                        .observation_event_id
                } else {
                    match acceptance::ingest_observation(&self.storage, &*commands, observation)
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
                *commands = acceptance::rebuild_from_log(&self.storage, &domain)
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
        let cursor = request
            .cursor
            .unwrap_or(patchbay_contracts::patchbay::Lsn { value: 0 });
        let events = self
            .storage
            .read_after(&domain, cursor)
            .await
            .map_err(map_storage_error_to_status)?;
        let mut live_commands = self.commands.lock().await;
        *live_commands = acceptance::rebuild_from_log(&self.storage, &domain)
            .await
            .map_err(map_acceptance_error_to_status)?;
        let commands = live_commands.clone();
        drop(live_commands);
        let delivery_adapter = authenticated_adapter.clone();
        let deliveries = events.into_iter().filter_map(move |event| {
            if event.payload.kind != StoredEventKind::Operation as i32 {
                return None;
            }
            let operation = match Operation::decode(event.payload.payload.as_slice()) {
                Ok(operation) => operation,
                Err(error) => {
                    return Some(Err(Status::internal(format!(
                        "cannot decode accepted operation: {error}"
                    ))))
                }
            };
            let targets_adapter = operation
                .target_scope
                .as_ref()
                .and_then(|target| target.adapter_id.as_ref())
                == Some(&delivery_adapter);
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
                delivery_event_id: Some(event.event_id),
            }))
        });
        let storage = self.storage.clone();
        let sessions = Arc::clone(&self.sessions);
        let delivery_stream_epochs = Arc::clone(&self.delivery_stream_epochs);
        let stale_domain = domain.clone();
        let stale_adapter = authenticated_adapter;
        let on_abnormal_disconnect: DisconnectCallback = Box::new(move || {
            let task = async move {
                // A newer poll supersedes an older stream's delayed drop task.
                // Holding the epoch guard through the state append establishes
                // a total order: either this disconnect marks stale first and
                // the newer adapter report restores live, or the newer stream
                // wins and this obsolete marker is inert.
                let epochs = delivery_stream_epochs.lock().await;
                if epochs.get(&stale_adapter) != Some(&stream_epoch) {
                    return;
                }
                let result = session::mark_adapter_sessions_stale(
                    &storage,
                    &mut *sessions.lock().await,
                    &stale_domain,
                    &stale_adapter,
                )
                .await;
                drop(epochs);
                if let Err(error) = result {
                    eprintln!(
                        "patchbay-core-server: failed to mark sessions stale after adapter disconnect: {error}"
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

        // v0.1.0 polling fallback: this server-stream returns the durable tail
        // currently available. The adapter immediately resumes from its cursor.
        // Polling the tail through `None` is clean completion; transport drop
        // before that point marks this adapter's sessions stale.
        let tail = DeliveryTail::new(Box::pin(stream::iter(deliveries)), on_abnormal_disconnect);
        Ok(Response::new(Box::pin(tail)))
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
