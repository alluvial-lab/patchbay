use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, ControlSurfacePrincipalRecord, DeviceId, EndpointId, Generation,
    OperatorRecord,
};
use patchbay_core::authority::{
    hash_principal_credential, ingest_control_surface_principal, ingest_operator_record,
    rebuild_operator_registry, OperatorError, OperatorRegistry,
};
use patchbay_core::storage::RusqliteStorage;
use prost_types::Timestamp;
use scrypt::{scrypt, Params};

fn domain() -> AuthorityDomainId {
    AuthorityDomainId {
        value: "authority-main".to_owned(),
    }
}

fn actor() -> ActorId {
    ActorId {
        value: "operator-primary".to_owned(),
    }
}

fn password_hash(password: &str) -> String {
    let salt = [7_u8; 16];
    let mut hash = [0_u8; 64];
    scrypt(
        password.as_bytes(),
        &salt,
        &Params::new(14, 8, 1, 64).unwrap(),
        &mut hash,
    )
    .unwrap();
    let salt = URL_SAFE_NO_PAD.encode(salt);
    let hash = URL_SAFE_NO_PAD.encode(hash);
    format!("scrypt${salt}${hash}")
}

fn operator_record(password: &str) -> OperatorRecord {
    OperatorRecord {
        actor_id: Some(actor()),
        password_hash: password_hash(password),
        created_at: Some(Timestamp {
            seconds: 1,
            nanos: 0,
        }),
        authority_domain_id: None,
    }
}

fn principal(id: &str, secret: &str, endpoint: &str) -> ControlSurfacePrincipalRecord {
    ControlSurfacePrincipalRecord {
        principal_id: id.to_owned(),
        operator_actor_id: Some(actor()),
        endpoint_id: Some(EndpointId {
            value: endpoint.to_owned(),
        }),
        device_id: Some(DeviceId {
            value: "device-a".to_owned(),
        }),
        endpoint_generation: Some(Generation { value: 1 }),
        credential_hash: hash_principal_credential(secret),
        created_at: Some(Timestamp {
            seconds: 2,
            nanos: 0,
        }),
        authority_domain_id: None,
    }
}

#[tokio::test]
async fn operator_and_principal_survive_projection_rebuild() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = OperatorRegistry::new();
    ingest_operator_record(
        &storage,
        &mut registry,
        &domain(),
        operator_record("correct horse battery staple"),
    )
    .await
    .unwrap();
    ingest_control_surface_principal(
        &storage,
        &mut registry,
        &domain(),
        principal("principal-web", "web-secret", "web-server"),
    )
    .await
    .unwrap();

    assert!(registry
        .verify_password(&actor(), "correct horse battery staple")
        .unwrap());
    assert!(!registry.verify_password(&actor(), "wrong").unwrap());
    assert_eq!(
        registry
            .verify_principal("principal-web", "web-secret")
            .unwrap()
            .endpoint_id
            .unwrap()
            .value,
        "web-server"
    );

    let rebuilt = rebuild_operator_registry(&storage, &domain())
        .await
        .unwrap();
    assert_eq!(rebuilt, registry);
    assert!(rebuilt
        .verify_password(&actor(), "correct horse battery staple")
        .unwrap());
    assert!(rebuilt
        .verify_principal("principal-web", "web-secret")
        .is_some());
}

#[tokio::test]
async fn bootstrap_is_first_run_only() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = OperatorRegistry::new();
    ingest_operator_record(
        &storage,
        &mut registry,
        &domain(),
        operator_record("password-one"),
    )
    .await
    .unwrap();

    let error = ingest_operator_record(
        &storage,
        &mut registry,
        &domain(),
        operator_record("password-two"),
    )
    .await
    .expect_err("a second operator record must be rejected");
    assert!(matches!(error, OperatorError::AlreadyBootstrapped));
}

#[tokio::test]
async fn latest_enrollment_for_an_endpoint_invalidates_the_prior_credential() {
    let storage = RusqliteStorage::open_in_memory().unwrap();
    let mut registry = OperatorRegistry::new();
    ingest_operator_record(
        &storage,
        &mut registry,
        &domain(),
        operator_record("password"),
    )
    .await
    .unwrap();
    ingest_control_surface_principal(
        &storage,
        &mut registry,
        &domain(),
        principal("principal-old", "old-secret", "cli-endpoint"),
    )
    .await
    .unwrap();
    ingest_control_surface_principal(
        &storage,
        &mut registry,
        &domain(),
        principal("principal-new", "new-secret", "cli-endpoint"),
    )
    .await
    .unwrap();

    assert!(registry
        .verify_principal("principal-old", "old-secret")
        .is_none());
    assert!(registry
        .verify_principal("principal-new", "new-secret")
        .is_some());
}
