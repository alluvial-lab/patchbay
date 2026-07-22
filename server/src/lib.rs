pub mod adapter_service;
pub mod admin_service;
pub mod identity;
pub mod issuer;
pub mod login_security;
pub mod operator_session;
pub mod service;
pub mod state;

pub mod rpc {
    tonic::include_proto!("patchbay");
}
