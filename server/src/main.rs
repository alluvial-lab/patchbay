use std::{env, net::SocketAddr};

use patchbay_contracts::patchbay::AuthorityDomainId;
use patchbay_core::storage::RusqliteStorage;
use patchbay_core_server::{
    adapter_service::{AdapterControlServiceImpl, AdapterEvidenceVerifier},
    rpc::{
        adapter_control_service_server::AdapterControlServiceServer,
        control_service_server::ControlServiceServer,
    },
    service::{ControlServiceImpl, CoreSecretInterceptor},
};
use tonic::transport::Server;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:50051";
const DEFAULT_DB_PATH: &str = "patchbay.sqlite3";
const DEFAULT_AUTHORITY_DOMAIN_ID: &str = "default";

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
    let database_path = env::var("PATCHBAY_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_owned());
    let authority_domain_id = AuthorityDomainId {
        value: env::var("PATCHBAY_AUTHORITY_DOMAIN_ID")
            .unwrap_or_else(|_| DEFAULT_AUTHORITY_DOMAIN_ID.to_owned()),
    };

    let storage = RusqliteStorage::open(&database_path)?;
    let control_service =
        ControlServiceImpl::new(storage.clone(), authority_domain_id.clone()).await?;
    let control_service = ControlServiceServer::with_interceptor(control_service, interceptor);
    let adapter_service =
        AdapterControlServiceImpl::new(storage, authority_domain_id, adapter_evidence).await?;
    let adapter_service = AdapterControlServiceServer::new(adapter_service);

    println!("patchbay-core-server: h2c on {address}");
    Server::builder()
        .add_service(control_service)
        .add_service(adapter_service)
        .serve(address)
        .await?;
    Ok(())
}
