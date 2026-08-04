use patchbay_contracts::patchbay::{
    AdapterId, AdapterSnapshotSupport, Generation, PayloadEnvelope, ResourceFreshnessState,
    ResourceKind,
};
use prost_types::Timestamp;

use super::ResourceIdentity;

/// Durable projected state for one exact operational-resource identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceRecord {
    pub identity: ResourceIdentity,
    pub resource_payload: Option<PayloadEnvelope>,
    pub projection_payload: Option<PayloadEnvelope>,
    pub freshness: ResourceFreshnessState,
    pub source_adapter_generation: Generation,
    pub revision_lsn: u64,
    pub observed_at: Timestamp,
    pub tombstoned_at_lsn: Option<u64>,
    pub replaced_by: Option<ResourceIdentity>,
}

impl ResourceRecord {
    #[must_use]
    pub fn tombstoned(&self) -> bool {
        self.tombstoned_at_lsn.is_some()
    }
}

/// One adapter-owned resource collection view.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceViewKey {
    pub adapter_id: AdapterId,
    pub resource_kind: ResourceKind,
}

/// Reconciliation metadata for one adapter-owned resource collection view.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceViewRecord {
    pub key: ResourceViewKey,
    pub completeness: AdapterSnapshotSupport,
    pub source_adapter_generation: Generation,
    pub revision_lsn: u64,
    pub observed_at: Timestamp,
}
