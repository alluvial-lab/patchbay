//! Operational-resource identity and identity-only target registration.

pub mod identity;
pub mod registry;
pub(crate) mod resolver;

pub use identity::{ResourceIdentity, ResourceIdentityError};
pub use registry::ResourceRegistry;
