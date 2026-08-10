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
/// Ingestion appends before observing, so storage remains authoritative. The
/// clone bound lets ingestion stage a complete fold and publish it only after
/// every committed event succeeds.
pub trait GrantProjection: GrantLookup + Clone {
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
