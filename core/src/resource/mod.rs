//! Operational-resource identity, durable state, and target registration.

pub mod events;
pub mod identity;
pub mod ingest;
pub mod registry;
pub mod replay;
pub(crate) mod resolver;
pub mod state;

pub use identity::{ResourceIdentity, ResourceIdentityError};
pub use ingest::{
    adapter_redeclaration_event, adapter_stale_event, ingest_resource_report,
    AdapterResourceRedeclaration, ResourceIngestResult, ResourceReportMode, ValidatedResourceReport,
};
pub use registry::ResourceRegistry;
pub use replay::rebuild_from_log;
pub use state::{ResourceRecord, ResourceViewKey, ResourceViewRecord};

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("corrupt resource record: {0}")]
    CorruptRecord(String),
    #[error("corrupt resource log: {0}")]
    CorruptLog(String),
    #[error("invalid resource report: {0}")]
    InvalidReport(String),
    #[error("stale adapter generation: live={live}, reported={reported}")]
    StaleAdapterGeneration { live: u64, reported: u64 },
    #[error("resource identity is terminally tombstoned: {0:?}")]
    TerminalTombstone(ResourceIdentity),
    #[error(transparent)]
    Identity(#[from] ResourceIdentityError),
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
