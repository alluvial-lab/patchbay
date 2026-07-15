use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AdapterId, AuthorityDomainId, CommandId, EventId, Generation, Grant,
    GrantId, GrantProvenance, GrantRevocationPolicy, IdempotencyKey, LoadSnapshotRequest, Lsn,
    Operation, OperationKind, OperationState, RuntimeSessionId, SessionActivityState,
    SessionConnectivityState, SessionRegistered, SessionState, StoredEventKind, StoredEventPayload,
    SubmissionOutcome, SubmitRequest, SubscribeRequest, TargetScope, TargetScopeKind,
};
use patchbay_core::{
    authority::events as authority_events,
    session::events as session_events,
    storage::{
        DedupOutcome, RecordedEvent, RusqliteStorage, Storage, StorageError, StoredSnapshot,
        TargetKey,
    },
};
use patchbay_core_server::{
    issuer::{OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER},
    rpc::{
        control_service_client::ControlServiceClient, control_service_server::ControlServiceServer,
    },
    service::{
        map_storage_error_to_status, ControlServiceImpl, CoreSecretInterceptor, CORE_SECRET_HEADER,
    },
};
use tempfile::TempDir;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Channel, Code, Request};
use tonic_types::StatusExt;

const SECRET: &str = "test-core-secret";
const OPERATOR_SESSION: &str = "opaque-session-32fb8181";
const OPERATOR_ACTOR: &str = "operator-primary";
const CONCURRENT_SUBMISSIONS: usize = 16;

#[derive(Clone)]
struct FailPostAppendReadOnceStorage {
    inner: RusqliteStorage,
    fail_next_read: Arc<AtomicBool>,
}

impl FailPostAppendReadOnceStorage {
    fn new(inner: RusqliteStorage) -> Self {
        Self {
            inner,
            fail_next_read: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Storage for FailPostAppendReadOnceStorage {
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
        let outcome = self
            .inner
            .append_dedup(authority_domain_id, key, target, payload)
            .await?;
        if matches!(&outcome, DedupOutcome::Appended(_)) {
            self.fail_next_read.store(true, Ordering::SeqCst);
        }
        Ok(outcome)
    }

    async fn read_after(
        &self,
        authority_domain_id: &AuthorityDomainId,
        cursor: Lsn,
    ) -> Result<Vec<RecordedEvent>, StorageError> {
        if self.fail_next_read.swap(false, Ordering::SeqCst) {
            return Err(StorageError::ReadFailed {
                message: "injected catch-up read failure".to_owned(),
                retryable: true,
            });
        }
        self.inner.read_after(authority_domain_id, cursor).await
    }

    async fn write_snapshot(
        &self,
        authority_domain_id: &AuthorityDomainId,
        snapshot_lsn: Lsn,
        snapshot_payload: Vec<u8>,
    ) -> Result<(), StorageError> {
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

struct TestServer {
    client: ControlServiceClient<Channel>,
    storage: RusqliteStorage,
    task: JoinHandle<()>,
    _directory: TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn grpc_seam_submits_streams_and_loads_snapshots() {
    let mut server = start_server().await;

    let unauthorized = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("unauthorized", "unauthorized")),
            },
            "wrong-secret",
        ))
        .await
        .expect_err("an invalid core secret must fail closed");
    assert_eq!(unauthorized.code(), Code::Unauthenticated);

    let mut missing_actor = authenticated_request(
        SubmitRequest {
            operation: Some(operation("missing-actor", "missing-actor")),
        },
        SECRET,
    );
    missing_actor.metadata_mut().remove(OPERATOR_ID_HEADER);
    let missing_actor = server
        .client
        .submit(missing_actor)
        .await
        .expect_err("operator actor metadata must be present");
    assert_eq!(missing_actor.code(), Code::Unauthenticated);

    let accepted = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("command-1", "key-1")),
            },
            SECRET,
        ))
        .await
        .expect("authorized submit must complete")
        .into_inner();
    assert_eq!(
        SubmissionOutcome::try_from(accepted.outcome).expect("known outcome"),
        SubmissionOutcome::Accepted
    );
    assert_eq!(
        OperationState::try_from(accepted.operation_state).expect("known state"),
        OperationState::Accepted
    );

    let mut subscription = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: Some(Lsn { value: 0 }),
            },
            SECRET,
        ))
        .await
        .expect("subscribe must establish")
        .into_inner();
    let mut saw_operation = false;
    while let Some(event) = subscription
        .message()
        .await
        .expect("stream message must decode")
    {
        let kind = event.payload.map(|payload| payload.kind);
        saw_operation |= kind == Some(StoredEventKind::Operation as i32);
    }
    assert!(
        saw_operation,
        "cursor-zero replay must include the operation"
    );

    let mut resumed = server
        .client
        .subscribe(authenticated_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: accepted.accepted_lsn,
            },
            SECRET,
        ))
        .await
        .expect("cursor resume must establish")
        .into_inner();
    assert!(
        resumed
            .message()
            .await
            .expect("resumed stream must decode")
            .is_none(),
        "subscribe must return only events with LSN greater than the cursor and complete"
    );

    let empty_snapshot = server
        .client
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: None,
            },
            SECRET,
        ))
        .await
        .expect("snapshot lookup must complete")
        .into_inner();
    assert!(!empty_snapshot.present);
    assert!(empty_snapshot.event_id.is_none());
    assert!(empty_snapshot.snapshot_payload.is_empty());

    let accepted_lsn = accepted
        .accepted_lsn
        .expect("accepted result must carry LSN");
    server
        .storage
        .write_snapshot(&domain(), accepted_lsn, b"snapshot-v1".to_vec())
        .await
        .expect("snapshot fixture must write");
    let loaded_snapshot = server
        .client
        .load_snapshot(authenticated_request(
            LoadSnapshotRequest {
                authority_domain_id: Some(domain()),
                at_or_before: None,
            },
            SECRET,
        ))
        .await
        .expect("latest snapshot lookup must complete")
        .into_inner();
    assert!(loaded_snapshot.present);
    assert_eq!(loaded_snapshot.snapshot_payload, b"snapshot-v1");
}

#[tokio::test]
async fn grant_subject_uses_verified_actor_not_operator_session() {
    assert_ne!(OPERATOR_SESSION, OPERATOR_ACTOR);
    let mut server = start_server().await;

    let result = server
        .client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("actor-bound-command", "actor-bound-key")),
            },
            SECRET,
        ))
        .await
        .expect("a grant bound to the verified actor must authorize the opaque session")
        .into_inner();

    assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
    assert!(!result.deduplicated);
}

#[tokio::test]
async fn retry_reconciles_a_commit_after_post_append_catch_up_failure() {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let inner = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&inner).await;
    let storage = FailPostAppendReadOnceStorage::new(inner.clone());
    let (mut client, task) = serve(storage).await;

    let first = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("recoverable-command", "recoverable-key")),
            },
            SECRET,
        ))
        .await
        .expect_err("the injected post-append read failure must fail the first response");
    assert_eq!(first.code(), Code::Unavailable);
    assert!(first.get_error_details().retry_info().is_some());

    let retried = client
        .submit(authenticated_request(
            SubmitRequest {
                operation: Some(operation("recoverable-command", "recoverable-key")),
            },
            SECRET,
        ))
        .await
        .expect("retry must reconcile the durable append before dedup lookup")
        .into_inner();
    assert_eq!(retried.outcome, SubmissionOutcome::Accepted as i32);
    assert_eq!(retried.operation_state, OperationState::Accepted as i32);
    assert!(retried.deduplicated);

    let operation_count = inner
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .expect("durable log must remain readable")
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::Operation as i32)
        .count();
    assert_eq!(operation_count, 1, "retry must not append a second command");

    task.abort();
}

#[tokio::test]
async fn concurrent_submits_complete_without_deadlock() {
    let server = start_server().await;
    let mut tasks = Vec::with_capacity(CONCURRENT_SUBMISSIONS);
    for index in 0..CONCURRENT_SUBMISSIONS {
        let mut client = server.client.clone();
        tasks.push(tokio::spawn(async move {
            client
                .submit(authenticated_request(
                    SubmitRequest {
                        operation: Some(operation(
                            &format!("concurrent-command-{index}"),
                            &format!("concurrent-key-{index}"),
                        )),
                    },
                    SECRET,
                ))
                .await
                .map(|response| response.into_inner())
        }));
    }

    let results = tokio::time::timeout(Duration::from_secs(10), async {
        let mut results = Vec::with_capacity(CONCURRENT_SUBMISSIONS);
        for task in tasks {
            results.push(task.await.expect("submit task must not panic"));
        }
        results
    })
    .await
    .expect("parallel submissions must not deadlock");

    for result in results {
        let result = result.expect("parallel submit must complete");
        assert_eq!(result.outcome, SubmissionOutcome::Accepted as i32);
    }
}

#[test]
fn binary_refuses_to_start_without_a_secret() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_patchbay-core-server"))
        .env_remove("PATCHBAY_CORE_SECRET")
        .output()
        .expect("server binary must be executable");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to start without it"),
        "startup failure must explain the missing trust root"
    );
}

#[test]
fn startup_secret_and_storage_error_mapping_fail_safe() {
    assert!(CoreSecretInterceptor::new("").is_err());

    let unavailable = map_storage_error_to_status(StorageError::Unavailable("offline".into()));
    assert_eq!(unavailable.code(), Code::Unavailable);
    assert!(unavailable.get_error_details().retry_info().is_some());

    assert_eq!(
        map_storage_error_to_status(StorageError::IdempotencyConflict).code(),
        Code::FailedPrecondition
    );
    assert_eq!(
        map_storage_error_to_status(StorageError::CorruptRecord("bad".into())).code(),
        Code::Internal
    );
}

async fn start_server() -> TestServer {
    let directory = tempfile::tempdir().expect("test directory must be created");
    let database_path = directory.path().join("patchbay.sqlite3");
    let storage = RusqliteStorage::open(database_path.to_str().expect("UTF-8 test path"))
        .expect("test storage must open");
    seed_authority_and_session(&storage).await;
    let (client, task) = serve(storage.clone()).await;

    TestServer {
        client,
        storage,
        task,
        _directory: directory,
    }
}

async fn serve<S>(storage: S) -> (ControlServiceClient<Channel>, JoinHandle<()>)
where
    S: Storage + Clone + Send + Sync + 'static,
{
    let service = ControlServiceImpl::new(storage, domain())
        .await
        .expect("service projections must rebuild");
    let interceptor = CoreSecretInterceptor::new(SECRET).expect("test secret is valid");
    let service = ControlServiceServer::with_interceptor(service, interceptor);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral listener must bind");
    let address = listener.local_addr().expect("listener has address");
    let incoming = TcpListenerStream::new(listener);
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(incoming)
            .await
            .expect("test server must run");
    });
    let client = ControlServiceClient::connect(format!("http://{address}"))
        .await
        .expect("test client must connect");
    (client, task)
}

async fn seed_authority_and_session(storage: &RusqliteStorage) {
    let target = target_scope();
    let grant = Grant {
        grant_id: Some(GrantId {
            value: "operator-grant".to_owned(),
        }),
        authority_domain_id: Some(domain()),
        subject_actor_id: Some(ActorId {
            value: OPERATOR_ACTOR.to_owned(),
        }),
        target_scope: Some(target.clone()),
        allowed_operation_kinds: vec![OperationKind::Instruct as i32],
        provenance: Some(GrantProvenance {
            reason: "gRPC smoke fixture".to_owned(),
            ..GrantProvenance::default()
        }),
        revocation_policy: GrantRevocationPolicy::Continue as i32,
        ..Grant::default()
    };
    storage
        .append(&domain(), authority_events::grant(domain(), grant))
        .await
        .expect("grant fixture must append");

    let registration = session_events::registered(
        domain(),
        SessionRegistered {
            adapter_id: target.adapter_id,
            deployment_scope: target.deployment_scope,
            runtime_session_id: target.runtime_session_id,
            session_generation: target.session_generation,
            initial_state: Some(SessionState {
                connectivity: SessionConnectivityState::Live as i32,
                activity: SessionActivityState::Idle as i32,
            }),
            project: "patchbay".to_owned(),
            cwd: "/work/patchbay".to_owned(),
            name: "smoke".to_owned(),
            spawn_origin: None,
        },
    );
    storage
        .append(&domain(), session_events::encode(&registration))
        .await
        .expect("session fixture must append");
}

fn authenticated_request<T>(message: T, secret: &str) -> Request<T> {
    let mut request = Request::new(message);
    request.metadata_mut().insert(
        CORE_SECRET_HEADER,
        secret.parse().expect("test secret is valid metadata"),
    );
    request.metadata_mut().insert(
        OPERATOR_SESSION_HEADER,
        OPERATOR_SESSION
            .parse()
            .expect("test operator session is valid metadata"),
    );
    request.metadata_mut().insert(
        OPERATOR_ID_HEADER,
        OPERATOR_ACTOR
            .parse()
            .expect("test operator actor is valid metadata"),
    );
    request
}

fn operation(command_id: &str, idempotency_key: &str) -> Operation {
    Operation {
        command_id: Some(CommandId {
            value: command_id.to_owned(),
        }),
        authority_domain_id: Some(domain()),
        sender: Some(ActorEndpointRef::default()),
        recipient: Some(ActorEndpointRef::default()),
        kind: OperationKind::Instruct as i32,
        target_scope: Some(target_scope()),
        idempotency_key: idempotency_key.to_owned(),
        ..Operation::default()
    }
}

fn target_scope() -> TargetScope {
    TargetScope {
        kind: TargetScopeKind::RuntimeSession as i32,
        adapter_id: Some(AdapterId {
            value: "pi".to_owned(),
        }),
        runtime_session_id: Some(RuntimeSessionId {
            value: "session-1".to_owned(),
        }),
        session_generation: Some(Generation { value: 1 }),
        deployment_scope: "machine-a".to_owned(),
        ..TargetScope::default()
    }
}

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}
