use std::{pin::Pin, time::Duration};

use patchbay_contracts::patchbay::{
    AuthorityDomainId, LoadSnapshotRequest, LoadSnapshotResponse, Lsn, SubmissionResult,
    SubmitRequest, SubscribeEvent, SubscribeRequest,
};
use patchbay_core::{
    acceptance::{self, AcceptanceError},
    storage::{Storage, StorageError},
};
use tokio_stream::{self as stream, Stream};
use tonic::{service::Interceptor, Code, Request, Response, Status};
use tonic_types::{ErrorDetails, StatusExt};

use crate::{
    issuer::MetadataIssuerContext, rpc::control_service_server::ControlService,
    state::ProjectionState,
};

pub const CORE_SECRET_HEADER: &str = "x-patchbay-core-secret";

#[derive(Clone)]
pub struct CoreSecretInterceptor {
    expected: Vec<u8>,
}

impl CoreSecretInterceptor {
    pub fn new(secret: impl Into<String>) -> Result<Self, String> {
        let secret = secret.into();
        if secret.is_empty() {
            return Err("PATCHBAY_CORE_SECRET must be configured and non-empty".to_owned());
        }
        if !secret.is_ascii() {
            return Err("PATCHBAY_CORE_SECRET must contain ASCII metadata characters".to_owned());
        }
        Ok(Self {
            expected: secret.into_bytes(),
        })
    }
}

impl Interceptor for CoreSecretInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let supplied = request
            .metadata()
            .get(CORE_SECRET_HEADER)
            .map(|value| value.as_encoded_bytes());
        if !supplied.is_some_and(|value| constant_time_eq(value, &self.expected)) {
            return Err(Status::unauthenticated("invalid core principal secret"));
        }
        Ok(request)
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
pub struct ControlServiceImpl<S> {
    storage: S,
    state: ProjectionState,
    authority_domain_id: AuthorityDomainId,
}

impl<S: Storage> ControlServiceImpl<S> {
    pub async fn new(storage: S, authority_domain_id: AuthorityDomainId) -> Result<Self, String> {
        if authority_domain_id.value.is_empty() {
            return Err("authority domain id must not be empty".to_owned());
        }
        let state = ProjectionState::rebuild(&storage, &authority_domain_id).await?;
        Ok(Self {
            storage,
            state,
            authority_domain_id,
        })
    }
}

type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send + 'static>>;

#[tonic::async_trait]
impl<S> ControlService for ControlServiceImpl<S>
where
    S: Storage + Clone + Send + Sync + 'static,
{
    async fn submit(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmissionResult>, Status> {
        let operation = request
            .get_ref()
            .operation
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("submit request is missing operation"))?;
        let authority_domain_id = operation
            .authority_domain_id
            .clone()
            .ok_or_else(|| Status::invalid_argument("operation is missing authority_domain_id"))?;
        self.require_configured_domain(&authority_domain_id)?;
        let issuer = MetadataIssuerContext::from_request(&request, authority_domain_id.clone())?;
        let operation = request.into_inner().operation.ok_or_else(|| {
            Status::invalid_argument("submit request lost its validated operation")
        })?;

        // Keep acceptance and projection catch-up atomic from the server's
        // perspective. This makes an immediate retry observe the just-appended
        // command while still allowing all RPC handlers to run concurrently.
        let _submit_guard = self.state.submit_guard().await;
        let result = acceptance::submit(
            &self.storage,
            self.state.grant_check(),
            self.state.target_resolver(),
            self.state.state_lookup(),
            &issuer,
            operation,
        )
        .await
        .map_err(map_acceptance_error_to_status)?;
        self.state
            .catch_up(&self.storage, &authority_domain_id)
            .await
            .map_err(|error| Status::internal(format!("projection catch-up failed: {error}")))?;

        Ok(Response::new(result))
    }

    type SubscribeStream = SubscribeStream;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let request = request.into_inner();
        let authority_domain_id = required_domain(request.authority_domain_id)?;
        self.require_configured_domain(&authority_domain_id)?;
        let cursor = request.cursor.unwrap_or(Lsn { value: 0 });
        let events = self
            .storage
            .read_after(&authority_domain_id, cursor)
            .await
            .map_err(map_storage_error_to_status)?;
        let events = events.into_iter().map(|event| {
            Ok(SubscribeEvent {
                event_id: Some(event.event_id),
                payload: Some(event.payload),
            })
        });
        Ok(Response::new(Box::pin(stream::iter(events))))
    }

    async fn load_snapshot(
        &self,
        request: Request<LoadSnapshotRequest>,
    ) -> Result<Response<LoadSnapshotResponse>, Status> {
        let request = request.into_inner();
        let authority_domain_id = required_domain(request.authority_domain_id)?;
        self.require_configured_domain(&authority_domain_id)?;
        let snapshot = self
            .storage
            .load_latest_snapshot(&authority_domain_id, request.at_or_before)
            .await
            .map_err(map_storage_error_to_status)?;
        let response = snapshot.map_or(
            LoadSnapshotResponse {
                present: false,
                event_id: None,
                snapshot_payload: Vec::new(),
            },
            |snapshot| LoadSnapshotResponse {
                present: true,
                event_id: Some(snapshot.event_id),
                snapshot_payload: snapshot.payload,
            },
        );
        Ok(Response::new(response))
    }
}

impl<S> ControlServiceImpl<S> {
    fn require_configured_domain(&self, actual: &AuthorityDomainId) -> Result<(), Status> {
        if actual != &self.authority_domain_id {
            return Err(Status::invalid_argument(
                "request authority domain does not match this core",
            ));
        }
        Ok(())
    }
}

fn required_domain(domain: Option<AuthorityDomainId>) -> Result<AuthorityDomainId, Status> {
    let domain = domain.ok_or_else(|| Status::invalid_argument("missing authority_domain_id"))?;
    if domain.value.is_empty() {
        return Err(Status::invalid_argument(
            "authority_domain_id must not be empty",
        ));
    }
    Ok(domain)
}

pub fn map_acceptance_error_to_status(error: AcceptanceError) -> Status {
    match error {
        AcceptanceError::Storage(error) => map_storage_error_to_status(error),
        AcceptanceError::InvalidTargetScope(message) => Status::invalid_argument(message),
        AcceptanceError::AlreadyTerminal(message) => Status::failed_precondition(message),
        AcceptanceError::CorruptRecord(message) | AcceptanceError::CorruptLog(message) => {
            Status::internal(message)
        }
    }
}

pub fn map_storage_error_to_status(error: StorageError) -> Status {
    match error {
        StorageError::Unavailable(message) => retryable_unavailable(message),
        StorageError::WriteFailed {
            message,
            retryable: true,
        }
        | StorageError::ReadFailed {
            message,
            retryable: true,
        } => retryable_unavailable(message),
        StorageError::IdempotencyConflict => {
            Status::failed_precondition("idempotency key conflicts with the existing operation")
        }
        StorageError::CorruptRecord(message) => Status::internal(message),
        StorageError::WriteFailed { message, .. } | StorageError::ReadFailed { message, .. } => {
            Status::internal(message)
        }
        StorageError::SnapshotStale(lsn) => {
            Status::failed_precondition(format!("snapshot LSN {lsn} is stale"))
        }
        StorageError::SnapshotWrongDomain => {
            Status::failed_precondition("snapshot belongs to another authority domain")
        }
        StorageError::InvalidSnapshotLsn(lsn) => {
            Status::invalid_argument(format!("snapshot LSN {lsn} is not committed"))
        }
        StorageError::InvalidEventKind => Status::internal("stored event kind is invalid"),
    }
}

fn retryable_unavailable(message: String) -> Status {
    Status::with_error_details(
        Code::Unavailable,
        message,
        ErrorDetails::with_retry_info(Some(Duration::from_secs(1))),
    )
}
