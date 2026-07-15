pub mod issuer;
pub mod service;
pub mod state;

pub mod rpc {
    tonic::include_proto!("patchbay");
}
