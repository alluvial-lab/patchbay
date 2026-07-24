use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use patchbay_contracts::patchbay::{
    ActorEndpointRef, ActorId, AdapterId, AuthorityDomainId, BootstrapRequest, CommandId, DeviceId,
    EnrollControlSurfacePrincipalRequest, Generation, Lsn, Operation, OperationKind,
    PrincipalCredential, PrincipalEnrollment, RevokeOperatorSessionRequest, RuntimeSessionId,
    SessionActivityState, SessionConnectivityState, SessionRegistered, SessionState,
    StoredEventKind, SubmissionOutcome, SubmitRequest, SubscribeRequest, TargetScope,
    TargetScopeKind, VerifyOperatorPasswordRequest,
};
use patchbay_core::{
    session::events as session_events,
    storage::{RusqliteStorage, Storage},
};
use patchbay_core_server::{
    admin_service::{AdminServiceImpl, SetupSecret},
    issuer::{
        OPERATOR_ID_HEADER, OPERATOR_SESSION_HEADER, PRINCIPAL_ID_HEADER, PRINCIPAL_SECRET_HEADER,
    },
    login_security::{LoginAuditEvent, LoginAuditSink, LoginLimitConfig, LoginLimiter},
    rpc::{
        admin_service_client::AdminServiceClient, admin_service_server::AdminService,
        admin_service_server::AdminServiceServer, control_service_client::ControlServiceClient,
        control_service_server::ControlServiceServer,
    },
    service::{ControlServiceImpl, CoreSecretInterceptor, CORE_SECRET_HEADER},
};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Channel, Code, Request};

const CORE_SECRET: &str = "test-core-secret";
const SETUP_SECRET: &str = "one-time-setup-secret";
const OPERATOR_ID: &str = "operator-primary";
const PASSWORD: &str = "correct-password";
const PASSWORD_HASH: &str = "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g";

#[derive(Default)]
struct RecordingLoginAudit {
    events: Mutex<Vec<LoginAuditEvent>>,
}

impl LoginAuditSink for RecordingLoginAudit {
    fn record(&self, event: LoginAuditEvent) {
        self.events.lock().unwrap().push(event);
    }
}

struct DualServer {
    network: ControlServiceClient<Channel>,
    network_admin: AdminServiceClient<Channel>,
    admin: AdminServiceClient<Channel>,
    storage: RusqliteStorage,
    network_task: JoinHandle<()>,
    admin_task: JoinHandle<()>,
}

impl Drop for DualServer {
    fn drop(&mut self) {
        self.network_task.abort();
        self.admin_task.abort();
    }
}

#[tokio::test]
async fn bootstrap_is_local_first_run_only_and_establishes_distinct_principals() {
    let mut server = serve_dual(SetupSecret::new(
        SETUP_SECRET.to_owned(),
        Duration::from_secs(60),
    ))
    .await;

    let network_attempt = server
        .network_admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .expect_err("AdminService must not be registered on the network listener");
    assert_eq!(network_attempt.code(), Code::Unimplemented);

    let wrong_setup = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request("wrong", "web-server")))
        .await
        .expect_err("the bootstrap secret must be checked");
    assert_eq!(wrong_setup.code(), Code::PermissionDenied);

    let bootstrap = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .expect("the local bootstrap request must succeed")
        .into_inner();
    let web_principal = bootstrap
        .principal
        .clone()
        .expect("bootstrap returns the initial transport principal");
    assert_eq!(
        web_principal.endpoint_id.as_ref().unwrap().value,
        "web-server"
    );
    let web_session = bootstrap
        .session_id
        .clone()
        .expect("bootstrap returns a core-owned operator session");
    assert!(!web_session.value.is_empty());

    let second = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .expect_err("the setup secret must expire after one use");
    assert_eq!(second.code(), Code::FailedPrecondition);

    let wrong_password = server
        .network
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(actor()),
            password: "wrong".to_owned(),
            principal: Some(enrollment("cli", "cli-device", 1)),
        }))
        .await
        .expect_err("wrong operator passwords must fail");
    assert_eq!(wrong_password.code(), Code::Unauthenticated);

    let cli_login = server
        .network
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(actor()),
            password: PASSWORD.to_owned(),
            principal: Some(enrollment("cli", "cli-device", 1)),
        }))
        .await
        .expect("the core-owned password record must verify")
        .into_inner();
    let cli_principal = cli_login
        .principal
        .expect("password verification enrolls the CLI principal");
    assert_eq!(cli_principal.endpoint_id.as_ref().unwrap().value, "cli");
    assert_ne!(cli_principal.principal_id, web_principal.principal_id);

    let missing_principal = server
        .network
        .submit(actor_session_request(SubmitRequest {
            operation: Some(operation("missing-principal", "missing-principal-key")),
        }))
        .await
        .expect_err("self-asserted actor/session metadata is not verification");
    assert_eq!(missing_principal.code(), Code::Unauthenticated);

    let forged_actor = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("forged-actor", "forged-actor-key")),
            },
            &web_principal,
            "another-operator",
            &web_session.value,
        ))
        .await
        .expect_err("the actor claim must match the verified principal binding");
    assert_eq!(forged_actor.code(), Code::Unauthenticated);

    let mut wrong_credential = web_principal.clone();
    wrong_credential.secret = "wrong-principal-secret".to_owned();
    let rejected_credential = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation(
                    "wrong-principal-secret",
                    "wrong-principal-secret-key",
                )),
            },
            &wrong_credential,
            OPERATOR_ID,
            &web_session.value,
        ))
        .await
        .expect_err("a principal id without its credential must fail closed");
    assert_eq!(rejected_credential.code(), Code::Unauthenticated);

    let invented_session = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("invented-session", "invented-session-key")),
            },
            &web_principal,
            OPERATOR_ID,
            "invented-session",
        ))
        .await
        .expect_err("an arbitrary forwarded session id is not core-verifiable evidence");
    assert_eq!(invented_session.code(), Code::Unauthenticated);

    let web_result = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("web-command", "web-command-key")),
            },
            &web_principal,
            OPERATOR_ID,
            &web_session.value,
        ))
        .await
        .expect("the verified web principal and core session must submit")
        .into_inner();
    assert_eq!(web_result.outcome, SubmissionOutcome::Accepted as i32);

    let cli_result = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("cli-command", "cli-command-key")),
            },
            &cli_principal,
            OPERATOR_ID,
            cli_login
                .operator_session_id
                .as_ref()
                .unwrap()
                .value
                .as_str(),
        ))
        .await
        .expect("the distinct verified CLI principal must submit")
        .into_inner();
    assert_eq!(cli_result.outcome, SubmissionOutcome::Accepted as i32);

    let kinds: Vec<_> = server
        .storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .into_iter()
        .map(|event| StoredEventKind::try_from(event.payload.kind).unwrap())
        .collect();
    assert!(kinds.contains(&StoredEventKind::Grant));
    assert!(kinds.contains(&StoredEventKind::OperatorRecord));
    assert!(kinds.contains(&StoredEventKind::ControlSurfacePrincipal));
}

#[tokio::test]
async fn revoked_core_session_cannot_be_reused() {
    let mut server = serve_dual(SetupSecret::new(
        SETUP_SECRET.to_owned(),
        Duration::from_secs(60),
    ))
    .await;
    let bootstrap = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .unwrap()
        .into_inner();
    let principal = bootstrap.principal.unwrap();
    let session = bootstrap.session_id.unwrap();

    let revoked = server
        .network
        .revoke_operator_session(principal_request(
            RevokeOperatorSessionRequest {},
            &principal,
            OPERATOR_ID,
            &session.value,
        ))
        .await
        .expect("the current core-owned session can revoke itself")
        .into_inner();
    assert!(revoked.revoked);

    let error = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("revoked-session", "revoked-session-key")),
            },
            &principal,
            OPERATOR_ID,
            &session.value,
        ))
        .await
        .expect_err("a revoked core session must fail compound-issuer verification");
    assert_eq!(error.code(), Code::Unauthenticated);
}

#[tokio::test]
async fn subscribe_excludes_authentication_and_authority_records() {
    let mut server = serve_dual(SetupSecret::new(
        SETUP_SECRET.to_owned(),
        Duration::from_secs(60),
    ))
    .await;
    let bootstrap = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .unwrap()
        .into_inner();
    let principal = bootstrap.principal.unwrap();
    let session = bootstrap.session_id.unwrap();
    let accepted = server
        .network
        .submit(principal_request(
            SubmitRequest {
                operation: Some(operation("subscribed-command", "subscribed-command-key")),
            },
            &principal,
            OPERATOR_ID,
            &session.value,
        ))
        .await
        .unwrap()
        .into_inner();
    let accepted_lsn = accepted.accepted_lsn.unwrap().value;

    let mut stream = server
        .network
        .subscribe(principal_request(
            SubscribeRequest {
                authority_domain_id: Some(domain()),
                cursor: Some(Lsn { value: 1 }),
            },
            &principal,
            OPERATOR_ID,
            &session.value,
        ))
        .await
        .unwrap()
        .into_inner();
    let mut events = Vec::new();
    while let Some(event) = stream.message().await.unwrap() {
        events.push(event);
    }

    assert!(
        !events.is_empty(),
        "operator-facing command state remains visible"
    );
    assert_eq!(
        events[0]
            .event_id
            .as_ref()
            .unwrap()
            .lsn
            .as_ref()
            .unwrap()
            .value,
        accepted_lsn,
        "filtered security records keep their durable LSNs as cursor gaps"
    );
    assert!(
        accepted_lsn > 2,
        "security records precede the visible command"
    );
    for event in events {
        let payload = event.payload.expect("subscription event has payload");
        let kind = StoredEventKind::try_from(payload.kind).unwrap();
        assert!(!matches!(
            kind,
            StoredEventKind::OperatorRecord
                | StoredEventKind::ControlSurfacePrincipal
                | StoredEventKind::Grant
                | StoredEventKind::DescendantGrant
                | StoredEventKind::Revocation
        ));
        assert!(
            !payload
                .payload
                .windows(PASSWORD_HASH.len())
                .any(|window| window == PASSWORD_HASH.as_bytes()),
            "subscription payload must not contain the stored password verifier"
        );
    }
}

#[tokio::test]
async fn password_rpc_throttles_before_a_correct_password_and_recovers_after_decay() {
    let audit = Arc::new(RecordingLoginAudit::default());
    let clock = Arc::new(Mutex::new(Instant::now()));
    let limiter_clock = clock.clone();
    let limiter = LoginLimiter::new_with_clock(
        LoginLimitConfig {
            window: Duration::from_secs(60),
            account_max_failures: 2,
            network_max_failures: 2,
            max_concurrent_verifications: 2,
            max_tracked_accounts: 16,
            max_tracked_networks: 16,
        },
        move || *limiter_clock.lock().unwrap(),
    )
    .unwrap();
    let mut server = serve_dual_with_security(
        SetupSecret::new(SETUP_SECRET.to_owned(), Duration::from_secs(60)),
        Duration::from_secs(60),
        limiter,
        audit.clone(),
    )
    .await;
    server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .unwrap();

    for attempt in 0..2 {
        let error = server
            .network
            .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
                operator_actor_id: Some(actor()),
                password: format!("wrong-{attempt}"),
                principal: Some(enrollment("cli-throttled", "cli-device", 1)),
            }))
            .await
            .expect_err("failed passwords must accumulate against account and network");
        assert_eq!(error.code(), Code::Unauthenticated);
    }

    let throttled = server
        .network
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(actor()),
            password: PASSWORD.to_owned(),
            principal: Some(enrollment("cli-throttled", "cli-device", 1)),
        }))
        .await
        .expect_err("the limiter must reject before checking even a correct password");
    assert_eq!(throttled.code(), Code::ResourceExhausted);

    *clock.lock().unwrap() += Duration::from_secs(61);
    let recovered = server
        .network
        .verify_operator_password(core_request(VerifyOperatorPasswordRequest {
            operator_actor_id: Some(actor()),
            password: PASSWORD.to_owned(),
            principal: Some(enrollment("cli-after-decay", "cli-device", 1)),
        }))
        .await
        .expect("decayed account and network windows permit a successful login")
        .into_inner();
    assert!(recovered.operator_session_id.is_some());

    let events = audit.events.lock().unwrap();
    assert!(events
        .iter()
        .any(|event| event.reason == "invalid_credentials"));
    assert!(events.iter().any(|event| event.reason == "login_throttled"));
    assert!(events.iter().any(|event| event.reason == "authenticated"));
    for event in events.iter() {
        let line = event.redacted_line();
        assert!(!line.contains(PASSWORD));
        assert!(!line.contains(PASSWORD_HASH));
    }
}

#[tokio::test]
async fn setup_secret_expires_by_timeout_without_bootstrapping() {
    let storage = seeded_storage().await;
    let control = ControlServiceImpl::new(storage.clone(), domain())
        .await
        .unwrap();
    let service = AdminServiceImpl::new(
        control,
        SetupSecret::new(SETUP_SECRET.to_owned(), Duration::ZERO),
    );
    let error = service
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .expect_err("a timed-out setup secret must fail closed");
    assert_eq!(error.code(), Code::FailedPrecondition);
    let operator_events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.payload.kind == StoredEventKind::OperatorRecord as i32)
        .count();
    assert_eq!(operator_events, 0);
}

#[tokio::test]
async fn malformed_bootstrap_input_is_rejected_before_any_security_record_is_written() {
    let storage = seeded_storage().await;
    let control = ControlServiceImpl::new(storage.clone(), domain())
        .await
        .unwrap();
    let service = AdminServiceImpl::new(
        control,
        SetupSecret::new(SETUP_SECRET.to_owned(), Duration::from_secs(60)),
    );
    let mut request = bootstrap_request(SETUP_SECRET, "web-server");
    request.password_hash = "not-a-scrypt-hash".to_owned();
    let error = service
        .bootstrap_operator(Request::new(request))
        .await
        .expect_err("malformed operator records must fail before bootstrap writes");
    assert_eq!(error.code(), Code::InvalidArgument);

    let security_events = storage
        .read_after(&domain(), Lsn { value: 0 })
        .await
        .unwrap()
        .into_iter()
        .filter(|event| {
            matches!(
                StoredEventKind::try_from(event.payload.kind).unwrap(),
                StoredEventKind::Grant
                    | StoredEventKind::OperatorRecord
                    | StoredEventKind::ControlSurfacePrincipal
            )
        })
        .count();
    assert_eq!(security_events, 0);
}

#[tokio::test]
async fn authenticated_principal_can_enroll_another_endpoint() {
    let mut server = serve_dual(SetupSecret::new(
        SETUP_SECRET.to_owned(),
        Duration::from_secs(60),
    ))
    .await;
    let bootstrap = server
        .admin
        .bootstrap_operator(Request::new(bootstrap_request(SETUP_SECRET, "web-server")))
        .await
        .unwrap()
        .into_inner();
    let web = bootstrap.principal.unwrap();
    let web_session = bootstrap.session_id.unwrap();
    let enrolled = server
        .network
        .enroll_control_surface_principal(principal_request(
            EnrollControlSurfacePrincipalRequest {
                principal: Some(enrollment("cli-second", "cli-device", 1)),
            },
            &web,
            OPERATOR_ID,
            &web_session.value,
        ))
        .await
        .expect("an existing compound issuer may enroll another endpoint")
        .into_inner()
        .principal
        .unwrap();
    assert_eq!(enrolled.endpoint_id.unwrap().value, "cli-second");
    assert_ne!(enrolled.principal_id, web.principal_id);
}

async fn serve_dual(setup_secret: SetupSecret) -> DualServer {
    serve_dual_with_security(
        setup_secret,
        Duration::from_secs(8 * 60 * 60),
        LoginLimiter::default(),
        Arc::new(RecordingLoginAudit::default()),
    )
    .await
}

async fn serve_dual_with_security(
    setup_secret: SetupSecret,
    operator_session_ttl: Duration,
    login_limiter: LoginLimiter,
    login_audit: Arc<dyn LoginAuditSink>,
) -> DualServer {
    let storage = seeded_storage().await;
    let control = ControlServiceImpl::new_with_security(
        storage.clone(),
        domain(),
        operator_session_ttl,
        login_limiter,
        login_audit,
    )
    .await
    .unwrap();
    let admin = AdminServiceImpl::new(control.clone(), setup_secret);

    let network_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let network_address = network_listener.local_addr().unwrap();
    let network_service = ControlServiceServer::with_interceptor(
        control,
        CoreSecretInterceptor::new(CORE_SECRET).unwrap(),
    );
    let network_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(network_service)
            .serve_with_incoming(TcpListenerStream::new(network_listener))
            .await
            .unwrap();
    });

    let admin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let admin_address = admin_listener.local_addr().unwrap();
    let admin_task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(AdminServiceServer::new(admin))
            .serve_with_incoming(TcpListenerStream::new(admin_listener))
            .await
            .unwrap();
    });

    let network_uri = format!("http://{network_address}");
    DualServer {
        network: ControlServiceClient::connect(network_uri.clone())
            .await
            .unwrap(),
        network_admin: AdminServiceClient::connect(network_uri).await.unwrap(),
        admin: AdminServiceClient::connect(format!("http://{admin_address}"))
            .await
            .unwrap(),
        storage,
        network_task,
        admin_task,
    }
}

async fn seeded_storage() -> RusqliteStorage {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let target = target_scope();
    storage
        .append(
            &domain(),
            session_events::encode(&session_events::registered(
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
                    name: "trust-boundary-test".to_owned(),
                    model: "provider/model".to_owned(),
                    spawn_origin: None,
                },
            )),
        )
        .await
        .unwrap();
    storage
}

fn bootstrap_request(secret: &str, endpoint: &str) -> BootstrapRequest {
    BootstrapRequest {
        setup_secret: secret.to_owned(),
        operator_actor_id: Some(actor()),
        password_hash: PASSWORD_HASH.to_owned(),
        principal: Some(enrollment(endpoint, "web-device", 1)),
    }
}

fn enrollment(endpoint: &str, device: &str, generation: u64) -> PrincipalEnrollment {
    PrincipalEnrollment {
        endpoint_id: Some(patchbay_contracts::patchbay::EndpointId {
            value: endpoint.to_owned(),
        }),
        device_id: Some(DeviceId {
            value: device.to_owned(),
        }),
        endpoint_generation: Some(Generation { value: generation }),
    }
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

fn actor() -> ActorId {
    ActorId {
        value: OPERATOR_ID.to_owned(),
    }
}

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn core_request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert(CORE_SECRET_HEADER, CORE_SECRET.parse().unwrap());
    request
}

fn actor_session_request<T>(message: T) -> Request<T> {
    let mut request = core_request(message);
    request
        .metadata_mut()
        .insert(OPERATOR_ID_HEADER, OPERATOR_ID.parse().unwrap());
    request.metadata_mut().insert(
        OPERATOR_SESSION_HEADER,
        "self-asserted-session".parse().unwrap(),
    );
    request
}

fn principal_request<T>(
    message: T,
    principal: &PrincipalCredential,
    actor_id: &str,
    session_id: &str,
) -> Request<T> {
    let mut request = core_request(message);
    request
        .metadata_mut()
        .insert(OPERATOR_ID_HEADER, actor_id.parse().unwrap());
    request
        .metadata_mut()
        .insert(OPERATOR_SESSION_HEADER, session_id.parse().unwrap());
    request
        .metadata_mut()
        .insert(PRINCIPAL_ID_HEADER, principal.principal_id.parse().unwrap());
    request
        .metadata_mut()
        .insert(PRINCIPAL_SECRET_HEADER, principal.secret.parse().unwrap());
    request
}
