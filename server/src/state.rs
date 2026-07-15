use std::sync::Arc;

use patchbay_contracts::patchbay::{AuthorityDomainId, CommandId, Lsn, OperationKind, TargetScope};
use patchbay_core::{
    acceptance::{
        Authorized, CommandIndex, CommandSnapshot, CommandStateLookup, GrantCheck, GrantDenied,
        TargetBinding, TargetNotFound, TargetResolver,
    },
    authority::{AuthorityRegistry, IssuerContext},
    session::SessionRegistry,
    storage::{RecordedEvent, Storage},
};
use tokio::sync::{Mutex, MutexGuard};

/// Server-owned concurrency boundary around core projections.
///
/// The canonical acquisition order is storage -> grant check -> target
/// resolver -> command-state lookup, matching the parameter order at the
/// acceptance boundary. Projection locks are short-lived and never nested in
/// this implementation: each port releases its lock before the next port is
/// called. `submit_guard` serializes submission plus projection catch-up so a
/// successful append is visible to an immediately following deduplicated
/// submission. This can be replaced by a server-local actor without changing
/// the core library or the wire contract.
#[derive(Clone)]
pub struct ProjectionState {
    grant_check: LockedGrantCheck,
    target_resolver: LockedTargetResolver,
    state_lookup: LockedCommandStateLookup,
    last_applied_lsn: Arc<Mutex<u64>>,
    submit_gate: Arc<Mutex<()>>,
}

impl ProjectionState {
    pub async fn rebuild<S: Storage>(
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<Self, String> {
        let events = storage
            .read_after(authority_domain_id, Lsn { value: 0 })
            .await
            .map_err(|error| error.to_string())?;

        let mut authority = AuthorityRegistry::new();
        let mut sessions = SessionRegistry::new();
        let mut commands = CommandIndex::new();
        let mut last_applied_lsn = 0;
        for event in &events {
            last_applied_lsn = validate_next_event(event, authority_domain_id, last_applied_lsn)?;
            authority
                .observe(event)
                .map_err(|error| error.to_string())?;
            sessions.observe(event).map_err(|error| error.to_string())?;
            commands.apply(event).map_err(|error| error.to_string())?;
        }

        Ok(Self {
            grant_check: LockedGrantCheck::new(authority),
            target_resolver: LockedTargetResolver::new(sessions),
            state_lookup: LockedCommandStateLookup::new(commands),
            last_applied_lsn: Arc::new(Mutex::new(last_applied_lsn)),
            submit_gate: Arc::new(Mutex::new(())),
        })
    }

    #[must_use]
    pub fn grant_check(&self) -> &LockedGrantCheck {
        &self.grant_check
    }

    #[must_use]
    pub fn target_resolver(&self) -> &LockedTargetResolver {
        &self.target_resolver
    }

    #[must_use]
    pub fn state_lookup(&self) -> &LockedCommandStateLookup {
        &self.state_lookup
    }

    pub async fn submit_guard(&self) -> MutexGuard<'_, ()> {
        self.submit_gate.lock().await
    }

    /// Fold newly committed events into every server-owned projection.
    pub async fn catch_up<S: Storage>(
        &self,
        storage: &S,
        authority_domain_id: &AuthorityDomainId,
    ) -> Result<(), String> {
        let mut cursor = self.last_applied_lsn.lock().await;
        let events = storage
            .read_after(authority_domain_id, Lsn { value: *cursor })
            .await
            .map_err(|error| error.to_string())?;

        for event in events {
            let next_lsn = validate_next_event(&event, authority_domain_id, *cursor)?;
            self.grant_check
                .observe(&event)
                .await
                .map_err(|error| error.to_string())?;
            self.target_resolver
                .observe(&event)
                .await
                .map_err(|error| error.to_string())?;
            self.state_lookup
                .apply(&event)
                .await
                .map_err(|error| error.to_string())?;
            *cursor = next_lsn;
        }
        Ok(())
    }
}

fn validate_next_event(
    event: &RecordedEvent,
    authority_domain_id: &AuthorityDomainId,
    previous_lsn: u64,
) -> Result<u64, String> {
    let domain = event
        .event_id
        .authority_domain_id
        .as_ref()
        .ok_or_else(|| "replay event has no authority domain".to_owned())?;
    if domain != authority_domain_id {
        return Err(format!(
            "replay event belongs to authority domain {:?}, expected {:?}",
            domain, authority_domain_id
        ));
    }
    let lsn = event
        .event_id
        .lsn
        .as_ref()
        .ok_or_else(|| "replay event has no LSN".to_owned())?
        .value;
    if lsn <= previous_lsn {
        return Err(format!(
            "replay event LSN {lsn} is not after previous LSN {previous_lsn}"
        ));
    }
    Ok(lsn)
}

#[derive(Clone)]
pub struct LockedGrantCheck {
    inner: Arc<Mutex<AuthorityRegistry>>,
}

impl LockedGrantCheck {
    fn new(registry: AuthorityRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
        }
    }

    async fn observe(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::authority::AuthorityError> {
        self.inner.lock().await.observe(event)
    }
}

impl GrantCheck for LockedGrantCheck {
    async fn check(
        &self,
        authority_domain_id: &AuthorityDomainId,
        issuer: &dyn IssuerContext,
        operation_kind: OperationKind,
        target_scope: &TargetScope,
    ) -> Result<Authorized, GrantDenied> {
        let registry = self.inner.lock().await;
        GrantCheck::check(
            &*registry,
            authority_domain_id,
            issuer,
            operation_kind,
            target_scope,
        )
        .await
    }
}

#[derive(Clone)]
pub struct LockedTargetResolver {
    inner: Arc<Mutex<SessionRegistry>>,
}

impl LockedTargetResolver {
    fn new(registry: SessionRegistry) -> Self {
        Self {
            inner: Arc::new(Mutex::new(registry)),
        }
    }

    async fn observe(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::session::SessionError> {
        self.inner.lock().await.observe(event)
    }
}

impl TargetResolver for LockedTargetResolver {
    async fn resolve(
        &self,
        authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        let registry = self.inner.lock().await;
        TargetResolver::resolve(&*registry, authority_domain_id, target_scope).await
    }
}

#[derive(Clone)]
pub struct LockedCommandStateLookup {
    inner: Arc<Mutex<CommandIndex>>,
}

impl LockedCommandStateLookup {
    fn new(index: CommandIndex) -> Self {
        Self {
            inner: Arc::new(Mutex::new(index)),
        }
    }

    async fn apply(
        &self,
        event: &RecordedEvent,
    ) -> Result<(), patchbay_core::acceptance::AcceptanceError> {
        self.inner.lock().await.apply(event)
    }
}

impl CommandStateLookup for LockedCommandStateLookup {
    async fn current_state(&self, command_id: &CommandId) -> Option<CommandSnapshot> {
        let index = self.inner.lock().await;
        CommandStateLookup::current_state(&*index, command_id).await
    }
}
