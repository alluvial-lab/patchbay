use std::{pin::Pin, sync::Arc};

use patchbay_contracts::patchbay::{
    observation_request, AdapterId, AttachRequest, AttachResult, AuthorityDomainId, Delivery,
    Generation, ObservationRequest, ObservationResult, Operation, OperationState, ReceiveRequest,
    SessionActivityState, SessionConnectivityState, StoredEventKind,
};
use patchbay_core::{
    acceptance::{self, CommandIndex},
    adapter::{self, AdapterRegistry},
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
    rpc::adapter_control_service_server::AdapterControlService,
    service::{map_acceptance_error_to_status, map_storage_error_to_status},
};

pub const ADAPTER_ID_HEADER: &str = "x-patchbay-adapter-id";
pub const ADAPTER_EVIDENCE_HEADER: &str = "x-patchbay-adapter-evidence";

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
        })
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
        let event_id = adapter::ingest_registration(
            &self.storage,
            &mut *self.adapters.lock().await,
            registration,
        )
        .await
        .map_err(map_adapter_error)?;
        Ok(Response::new(AttachResult {
            accepted: true,
            attach_event_id: Some(event_id),
            failure_code: String::new(),
        }))
    }

    async fn ingest_observation(
        &self,
        request: Request<ObservationRequest>,
    ) -> Result<Response<ObservationResult>, Status> {
        let authenticated_adapter = self.evidence.verify_request(&request)?;
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
        let authenticated_adapter = self.evidence.verify_request(&request)?;
        let request = request.into_inner();
        require_same_adapter(request.adapter_id.as_ref(), &authenticated_adapter)?;
        let domain = self.require_attached(&authenticated_adapter).await?;
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
                == Some(&authenticated_adapter);
            let remains_accepted = operation
                .command_id
                .as_ref()
                .and_then(|command_id| commands.get_command(command_id))
                .is_some_and(|record| record.state == OperationState::Accepted);
            (targets_adapter && remains_accepted).then_some(Ok(Delivery {
                operation: Some(operation),
                delivery_event_id: Some(event.event_id),
            }))
        });
        // v0.1.0 polling fallback: this server-stream returns the durable tail
        // currently available. The adapter immediately resumes from its cursor.
        Ok(Response::new(Box::pin(stream::iter(deliveries))))
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
        | session::IngestResult::Relabeled { event_id } => Some(event_id),
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
