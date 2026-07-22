use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use patchbay_contracts::patchbay::AuthorityDomainId;
use patchbay_core::storage::RusqliteStorage;
use patchbay_core_server::{
    adapter_service::{AdapterControlServiceImpl, AdapterEvidenceVerifier},
    admin_service::{AdminServiceImpl, SetupSecret},
    login_security::{LoginLimiter, StderrLoginAuditSink},
    operator_session::DEFAULT_OPERATOR_SESSION_TTL,
    rpc::{
        adapter_control_service_server::AdapterControlServiceServer,
        admin_service_server::AdminServiceServer, control_service_server::ControlServiceServer,
    },
    service::{ControlServiceImpl, CoreSecretInterceptor},
};
use tonic::transport::Server;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:50051";
const DEFAULT_ADMIN_BIND_ADDR: &str = "127.0.0.1:50052";
const DEFAULT_DB_PATH: &str = "patchbay.sqlite3";
const DEFAULT_AUTHORITY_DOMAIN_ID: &str = "default";
const DEFAULT_SETUP_SECRET_TTL_SECS: u64 = 600;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read and validate the trust root before opening storage or binding a
    // listener. There is deliberately no default and no open mode.
    let secret = env::var("PATCHBAY_CORE_SECRET")
        .map_err(|_| "PATCHBAY_CORE_SECRET is required; refusing to start without it")?;
    let interceptor = CoreSecretInterceptor::new(secret)?;
    let adapter_evidence = AdapterEvidenceVerifier::new(
        env::var("PATCHBAY_ADAPTER_ATTACHMENT_SECRET").map_err(|_| {
            "PATCHBAY_ADAPTER_ATTACHMENT_SECRET is required; refusing to start without an adapter trust root"
        })?,
    )?;

    let address: SocketAddr = env::var("PATCHBAY_BIND_ADDR")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned())
        .parse()?;
    let admin_address = local_admin_address(
        &env::var("PATCHBAY_ADMIN_BIND_ADDR")
            .unwrap_or_else(|_| DEFAULT_ADMIN_BIND_ADDR.to_owned()),
    )?;
    if address == admin_address {
        return Err("PATCHBAY_ADMIN_BIND_ADDR must be distinct from PATCHBAY_BIND_ADDR".into());
    }
    let setup_ttl = setup_secret_ttl(env::var("PATCHBAY_SETUP_SECRET_TTL_SECS").ok())?;
    let database_path = env::var("PATCHBAY_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_owned());
    let authority_domain_id = AuthorityDomainId {
        value: env::var("PATCHBAY_AUTHORITY_DOMAIN_ID")
            .unwrap_or_else(|_| DEFAULT_AUTHORITY_DOMAIN_ID.to_owned()),
    };

    let storage = RusqliteStorage::open(&database_path)?;
    let control_service = ControlServiceImpl::new_with_security(
        storage.clone(),
        authority_domain_id.clone(),
        DEFAULT_OPERATOR_SESSION_TTL,
        LoginLimiter::default(),
        Arc::new(StderrLoginAuditSink),
    )
    .await?;
    let bootstrapped = control_service.is_bootstrapped().await;
    let (setup_secret, setup_secret_value) = SetupSecret::generate(setup_ttl);
    let admin_service = AdminServiceImpl::new(control_service.clone(), setup_secret);
    let control_service = ControlServiceServer::with_interceptor(control_service, interceptor);
    let adapter_service =
        AdapterControlServiceImpl::new(storage, authority_domain_id, adapter_evidence).await?;
    let adapter_service = AdapterControlServiceServer::new(adapter_service);

    println!("patchbay-core-server: h2c on {address}");
    println!("patchbay-core-server: local admin h2c on {admin_address}");
    if !bootstrapped {
        println!(
            "patchbay-core-server: one-time setup secret (expires in {}s): {setup_secret_value}",
            setup_ttl.as_secs()
        );
    }

    let network = Server::builder()
        .add_service(control_service)
        .add_service(adapter_service)
        .serve(address);
    let local_admin = Server::builder()
        .add_service(AdminServiceServer::new(admin_service))
        .serve(admin_address);
    tokio::try_join!(network, local_admin)?;
    Ok(())
}

fn local_admin_address(value: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let address: SocketAddr = value.parse()?;
    if !address.ip().is_loopback() {
        return Err("PATCHBAY_ADMIN_BIND_ADDR must use a loopback address".into());
    }
    Ok(address)
}

fn setup_secret_ttl(configured: Option<String>) -> Result<Duration, Box<dyn std::error::Error>> {
    let seconds = configured
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(DEFAULT_SETUP_SECRET_TTL_SECS);
    if seconds == 0 {
        return Err("PATCHBAY_SETUP_SECRET_TTL_SECS must be positive".into());
    }
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_listener_is_loopback_only() {
        assert!(local_admin_address("127.0.0.1:50052").is_ok());
        assert!(local_admin_address("[::1]:50052").is_ok());
        assert!(local_admin_address("0.0.0.0:50052").is_err());
        assert!(local_admin_address("192.168.1.10:50052").is_err());
    }

    #[test]
    fn setup_secret_ttl_must_be_positive() {
        assert!(setup_secret_ttl(Some("0".to_owned())).is_err());
        assert_eq!(
            setup_secret_ttl(Some("12".to_owned())).unwrap(),
            Duration::from_secs(12)
        );
    }
}
