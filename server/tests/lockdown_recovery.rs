use patchbay_contracts::patchbay::{
    ActorId, AuditEventKind, AuthorityDomainId, BootstrapRequest, EnterSecurityLockdownRequest,
    ExitSecurityLockdownRequest, PrincipalEnrollment, TargetScopeKind,
};
use patchbay_core::storage::{AuditPageSpec, AuditedStorage, RusqliteStorage, Storage};
use patchbay_core_server::{
    admin_service::{AdminServiceImpl, SetupSecret},
    login_security::{LoginLimiter, StderrLoginAuditSink},
    operator_session::OperatorSessionBinding,
    rpc::{admin_service_server::AdminService, control_service_server::ControlService},
    service::ControlServiceImpl,
};
use std::sync::Arc;
use tonic::Request;

const DOMAIN: &str = "default";
const ACTOR: &str = "operator-primary";
const PASSWORD_HASH: &str = "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g";

#[tokio::test]
async fn file_backed_bootstrap_grant_survives_storage_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bootstrap-restart.sqlite3");
    let authority_domain_id = AuthorityDomainId {
        value: DOMAIN.to_owned(),
    };
    let storage = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    let control = ControlServiceImpl::new_with_security(
        storage.clone(),
        authority_domain_id.clone(),
        std::time::Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await
    .unwrap();
    let admin = AdminServiceImpl::new(
        control.clone(),
        SetupSecret::new(
            "setup-secret".to_owned(),
            std::time::Duration::from_secs(60),
        ),
    );
    let grant_id = admin
        .bootstrap_operator(Request::new(BootstrapRequest {
            setup_secret: "setup-secret".to_owned(),
            operator_actor_id: Some(ActorId {
                value: ACTOR.to_owned(),
            }),
            password_hash: PASSWORD_HASH.to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(patchbay_contracts::patchbay::EndpointId {
                    value: "cli".to_owned(),
                }),
                device_id: Some(patchbay_contracts::patchbay::DeviceId {
                    value: "console".to_owned(),
                }),
                endpoint_generation: Some(patchbay_contracts::patchbay::Generation { value: 1 }),
            }),
        }))
        .await
        .unwrap()
        .into_inner()
        .grant_id
        .expect("bootstrap returns its durable grant identity");

    drop(admin);
    drop(control);
    drop(storage);
    tokio::task::yield_now().await;

    let reopened = AuditedStorage::new(RusqliteStorage::open(path.to_str().unwrap()).unwrap());
    let restarted = ControlServiceImpl::new_with_security(
        reopened,
        authority_domain_id,
        std::time::Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await
    .unwrap();
    assert!(restarted.is_bootstrapped().await);
    assert!(restarted
        .projection_state()
        .grant(&grant_id)
        .await
        .is_some());
}

fn issuer_headers(request: &mut Request<impl Sized>, session_id: &str, principal_id: &str, principal_secret: &str) {
    request.metadata_mut().insert("x-patchbay-principal-id", principal_id.parse().unwrap());
    request.metadata_mut().insert("x-patchbay-principal-secret", principal_secret.parse().unwrap());
    request.metadata_mut().insert("x-patchbay-operator-id", ACTOR.parse().unwrap());
    request.metadata_mut().insert("x-patchbay-operator-session-id", session_id.parse().unwrap());
}

#[tokio::test]
async fn entry_restart_and_credential_independent_admin_exit_recover_posture() {
    let authority_domain_id = AuthorityDomainId { value: DOMAIN.to_owned() };
    let storage = AuditedStorage::new(RusqliteStorage::open_in_memory().unwrap());
    let control = ControlServiceImpl::new_with_security(
        storage.clone(),
        authority_domain_id.clone(),
        std::time::Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await
    .unwrap();
    let admin = AdminServiceImpl::new(control.clone(), SetupSecret::new("setup-secret".to_owned(), std::time::Duration::from_secs(60)));
    let bootstrap = admin
        .bootstrap_operator(Request::new(BootstrapRequest {
            setup_secret: "setup-secret".to_owned(),
            operator_actor_id: Some(ActorId { value: ACTOR.to_owned() }),
            password_hash: PASSWORD_HASH.to_owned(),
            principal: Some(PrincipalEnrollment {
                endpoint_id: Some(patchbay_contracts::patchbay::EndpointId { value: "cli".to_owned() }),
                device_id: Some(patchbay_contracts::patchbay::DeviceId { value: "console".to_owned() }),
                endpoint_generation: Some(patchbay_contracts::patchbay::Generation { value: 1 }),
            }),
        }))
        .await
        .unwrap()
        .into_inner();
    let session_id = bootstrap.session_id.unwrap();
    let principal = bootstrap.principal.unwrap();

    let mut enter = Request::new(EnterSecurityLockdownRequest {
        authority_domain_id: Some(authority_domain_id.clone()),
        reason_code: "suspected_endpoint_compromise".to_owned(),
    });
    issuer_headers(&mut enter, &session_id.value, &principal.principal_id, &principal.secret);
    let entered = control.enter_security_lockdown(enter).await.unwrap().into_inner();
    assert!(entered.lockdown.unwrap().active);
    assert!(!entered.already_active);

    // A fresh process rebuilds the posture and generation floor from the log.
    let restarted = ControlServiceImpl::new_with_security(
        storage.clone(),
        authority_domain_id.clone(),
        std::time::Duration::from_secs(3600),
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await
    .unwrap();
    assert!(restarted.projection_state().lockdown_state().await.active);
    assert!(!restarted.projection_state().verify_operator_session(&session_id, &OperatorSessionBinding {
        actor_id: ActorId { value: ACTOR.to_owned() },
        endpoint_id: principal.endpoint_id.clone().unwrap(),
        device_id: principal.device_id.clone().unwrap(),
        endpoint_generation: principal.endpoint_generation.unwrap(),
    }).await);

    // Admin exit has no operator metadata and therefore remains callable after
    // entry/restart has invalidated the routine session.
    let exit = AdminServiceImpl::new(restarted.clone(), SetupSecret::new("unused".to_owned(), std::time::Duration::from_secs(60)))
        .exit_security_lockdown(Request::new(ExitSecurityLockdownRequest {
            authority_domain_id: Some(authority_domain_id.clone()),
            reason_code: "operator_recovery".to_owned(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!exit.lockdown.unwrap().active);
    assert!(!restarted.projection_state().lockdown_state().await.active);

    // A newly issued routine session is above the replayed generation floor.
    let next = restarted
        .projection_state()
        .issue_operator_session(OperatorSessionBinding {
            actor_id: ActorId { value: ACTOR.to_owned() },
            endpoint_id: patchbay_contracts::patchbay::EndpointId { value: "cli-2".to_owned() },
            device_id: patchbay_contracts::patchbay::DeviceId { value: "console-2".to_owned() },
            endpoint_generation: patchbay_contracts::patchbay::Generation { value: 2 },
        })
        .await;
    assert!(next.session_generation.value > 1);

    // The durable audit index preserves the verified entry attribution and the
    // bootstrap-channel exit record without exposing session credentials.
    let page = storage
        .query_audit(
            &authority_domain_id,
            AuditPageSpec {
                kinds: vec![AuditEventKind::LockdownEntered, AuditEventKind::LockdownExited],
                actor_id: None,
                endpoint_id: None,
                command_id: None,
                grant_id: None,
                target: None,
                failure_codes: Vec::new(),
                reason_codes: Vec::new(),
                occurred_from: None,
                occurred_before: None,
                before_lsn: None,
                limit: 100,
            },
        )
        .await
        .expect("lockdown audit query must remain available");
    let entered_audit = page
        .records
        .iter()
        .find(|record| record.kind == AuditEventKind::LockdownEntered as i32)
        .expect("entry audit record is indexed");
    assert_eq!(entered_audit.actor_id.as_ref().map(|id| id.value.as_str()), Some(ACTOR));
    assert_eq!(entered_audit.endpoint_id.as_ref().map(|id| id.value.as_str()), Some("cli"));
    assert_eq!(entered_audit.device_id.as_ref().map(|id| id.value.as_str()), Some("console"));
    assert_eq!(entered_audit.reason_code, "suspected_endpoint_compromise");
    assert_eq!(entered_audit.target_scope.as_ref().map(|scope| scope.kind), Some(TargetScopeKind::AuthorityDomain as i32));
    let exited_audit = page
        .records
        .iter()
        .find(|record| record.kind == AuditEventKind::LockdownExited as i32)
        .expect("exit audit record is indexed");
    assert_eq!(exited_audit.reason_code, "operator_recovery");
    assert_eq!(exited_audit.target_scope.as_ref().map(|scope| scope.kind), Some(TargetScopeKind::AuthorityDomain as i32));
}
