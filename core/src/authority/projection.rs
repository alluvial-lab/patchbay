//! Read-and-warm port for the durable authority projection.

use patchbay_contracts::patchbay::GrantId;

use crate::storage::RecordedEvent;

use super::{AuthorityError, AuthorityRegistry, GrantRecord};

/// Read access to current grant state used by authority ingestion.
pub trait GrantLookup: Send + Sync {
    fn current_grant(
        &self,
        grant_id: &GrantId,
    ) -> impl std::future::Future<Output = Option<GrantRecord>> + Send;
}

/// A grant projection that can fold a committed authority event.
///
/// Ingestion appends before observing, so storage remains authoritative and a
/// fold failure forces callers to rebuild this hot projection from the log.
pub trait GrantProjection: GrantLookup {
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError>;
}

impl GrantLookup for AuthorityRegistry {
    async fn current_grant(&self, grant_id: &GrantId) -> Option<GrantRecord> {
        self.get_grant(grant_id).cloned()
    }
}

impl GrantProjection for AuthorityRegistry {
    fn observe(&mut self, event: &RecordedEvent) -> Result<(), AuthorityError> {
        AuthorityRegistry::observe(self, event)
    }
}
