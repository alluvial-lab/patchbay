use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
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
    storage::{RecordedEvent, Storage},
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
    commands: Arc<Mutex<CommandProjection>>,
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
        let commands = rebuild_command_projection(&storage, &authority_domain_id)
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
                let mut sessions = self.sessions.lock().await;
                let result = session::ingest_session_report(&self.storage, &mut *sessions, report)
                    .await
                    .map_err(map_session_error)?;
                let rebuilt = session::rebuild_from_log(&self.storage, &domain)
                    .await
                    .map_err(map_session_error)?;
                *sessions = rebuilt;
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
        let sessions = Arc::clone(&self.sessions);
        let delivery_stream_epochs = Arc::clone(&self.delivery_stream_epochs);
        let stale_domain = domain.clone();
        let stale_adapter = authenticated_adapter.clone();
        let on_abnormal_disconnect: DisconnectCallback = Box::new(move || {
            let task = async move {
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

                let session_result = session::mark_adapter_sessions_stale(
                    &storage,
                    &mut *sessions.lock().await,
                    &stale_domain,
                    &stale_adapter,
                )
                .await;
                drop(epochs);

                if let Err(error) = command_result {
                    eprintln!(
                        "patchbay-core-server: failed to reconcile running commands after adapter disconnect: {error}"
                    );
                }
                if let Err(error) = session_result {
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
