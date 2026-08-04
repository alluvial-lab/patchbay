use std::collections::HashSet;

use super::ResourceIdentity;

/// Identity-only membership projection for operational resources.
///
/// Resource state, health, revisions, snapshots, and payloads deliberately do
/// not belong here. A later authenticated durable resource projection owns
/// population through [`register`](Self::register).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceRegistry {
    identities: HashSet<ResourceIdentity>,
}

impl ResourceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, identity: ResourceIdentity) -> bool {
        self.identities.insert(identity)
    }

    #[must_use]
    pub fn contains(&self, identity: &ResourceIdentity) -> bool {
        self.identities.contains(identity)
    }

    pub fn resources(&self) -> impl Iterator<Item = &ResourceIdentity> {
        self.identities.iter()
    }
}
