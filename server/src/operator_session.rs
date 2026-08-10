use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use patchbay_contracts::patchbay::{
    security_lockdown_event, ActorId, DeviceId, EndpointId, Generation, OperatorSessionId,
    OperatorSessionRevocation, SecurityLockdownEvent, StoredEventKind,
};
use prost::Message;
use tokio::sync::{Mutex, MutexGuard};

use crate::identity::random_token;
use patchbay_core::storage::RecordedEvent;

pub const DEFAULT_OPERATOR_SESSION_TTL: Duration = Duration::from_secs(8 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSessionBinding {
    pub actor_id: ActorId,
    pub endpoint_id: EndpointId,
    pub device_id: DeviceId,
    pub endpoint_generation: Generation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedOperatorSession {
    pub id: OperatorSessionId,
    pub session_generation: Generation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatorSessionRecord {
    binding: OperatorSessionBinding,
    session_generation: Generation,
    created_at: Instant,
    last_used_at: Instant,
    expires_at: Instant,
    revoked_at: Option<Instant>,
}

pub(crate) struct PreparedOperatorSessionInstall {
    sessions: HashMap<String, OperatorSessionRecord>,
    next_generation: HashMap<String, u64>,
    invalidated_through_generation: HashMap<String, u64>,
}

pub(crate) struct OperatorSessionInstallGuards<'a> {
    sessions: MutexGuard<'a, HashMap<String, OperatorSessionRecord>>,
    next_generation: MutexGuard<'a, HashMap<String, u64>>,
    invalidated_through_generation: MutexGuard<'a, HashMap<String, u64>>,
}

impl OperatorSessionInstallGuards<'_> {
    pub(crate) fn install(&mut self, prepared: PreparedOperatorSessionInstall) {
        *self.sessions = prepared.sessions;
        *self.next_generation = prepared.next_generation;
        *self.invalidated_through_generation = prepared.invalidated_through_generation;
    }
}

/// Core-owned operator sessions. Opaque session ids remain process-local and
/// are invalid after restart; only the durable generation fence is replayed.
#[derive(Debug, Clone)]
pub struct OperatorSessionRegistry {
    sessions: Arc<Mutex<HashMap<String, OperatorSessionRecord>>>,
    next_generation: Arc<Mutex<HashMap<String, u64>>>,
    invalidated_through_generation: Arc<Mutex<HashMap<String, u64>>>,
    /// Serializes process-local mutation with aggregate replay staging so a
    /// concurrent login/session refresh cannot be overwritten at install.
    mutation_guard: Arc<Mutex<()>>,
    ttl: Duration,
}

impl OperatorSessionRegistry {
    pub fn new(ttl: Duration) -> Result<Self, String> {
        if ttl.is_zero() {
            return Err("operator session TTL must be positive".to_owned());
        }
        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_generation: Arc::new(Mutex::new(HashMap::new())),
            invalidated_through_generation: Arc::new(Mutex::new(HashMap::new())),
            mutation_guard: Arc::new(Mutex::new(())),
            ttl,
        })
    }

    /// Deep-clone the process-local session state for an aggregate replay
    /// transaction. The ordinary `Clone` implementation intentionally shares
    /// live state; catch-up needs an isolated candidate instead.
    #[cfg(test)]
    pub(crate) async fn staged_clone(&self) -> Self {
        let _mutation_guard = self.mutation_guard.lock().await;
        self.staged_clone_unlocked().await
    }

    pub(crate) async fn replay_guard(&self) -> MutexGuard<'_, ()> {
        self.mutation_guard.lock().await
    }

    pub(crate) async fn staged_clone_unlocked(&self) -> Self {
        let sessions = self.sessions.lock().await;
        let next_generation = self.next_generation.lock().await;
        let invalidated_through_generation = self.invalidated_through_generation.lock().await;
        Self {
            sessions: Arc::new(Mutex::new(sessions.clone())),
            next_generation: Arc::new(Mutex::new(next_generation.clone())),
            invalidated_through_generation: Arc::new(Mutex::new(
                invalidated_through_generation.clone(),
            )),
            mutation_guard: Arc::new(Mutex::new(())),
            ttl: self.ttl,
        }
    }

    /// Copy a fully validated staged replay state before live publication
    /// guards are acquired.
    pub(crate) async fn prepare_install_unlocked(&self) -> PreparedOperatorSessionInstall {
        PreparedOperatorSessionInstall {
            sessions: self.sessions.lock().await.clone(),
            next_generation: self.next_generation.lock().await.clone(),
            invalidated_through_generation: self
                .invalidated_through_generation
                .lock()
                .await
                .clone(),
        }
    }

    /// Acquire every live operator-session map while the caller holds
    /// `replay_guard`. Calling `install` on the returned guards is synchronous,
    /// so aggregate publication has no cancellation point after its first
    /// assignment.
    pub(crate) async fn install_guards_unlocked(&self) -> OperatorSessionInstallGuards<'_> {
        OperatorSessionInstallGuards {
            sessions: self.sessions.lock().await,
            next_generation: self.next_generation.lock().await,
            invalidated_through_generation: self.invalidated_through_generation.lock().await,
        }
    }

    #[cfg(test)]
    pub(crate) async fn equivalent_to(&self, other: &Self) -> bool {
        let left = self.staged_clone().await;
        let right = other.staged_clone().await;
        *left.sessions.lock().await == *right.sessions.lock().await
            && *left.next_generation.lock().await == *right.next_generation.lock().await
            && *left.invalidated_through_generation.lock().await
                == *right.invalidated_through_generation.lock().await
            && left.ttl == right.ttl
    }

    pub async fn issue(&self, binding: OperatorSessionBinding) -> IssuedOperatorSession {
        let _mutation_guard = self.mutation_guard.lock().await;
        let mut next_generation = self.next_generation.lock().await;
        let floor = self
            .invalidated_through_generation
            .lock()
            .await
            .get(&binding.actor_id.value)
            .copied()
            .unwrap_or(0);
        let entry = next_generation
            .entry(binding.actor_id.value.clone())
            .or_insert(floor);
        *entry = (*entry).max(floor);
        let generation = entry.saturating_add(1);
        *entry = generation;
        drop(next_generation);

        let now = Instant::now();
        let mut sessions = self.sessions.lock().await;
        let mut value = format!("operator-session-{}", random_token());
        while sessions.contains_key(&value) {
            value = format!("operator-session-{}", random_token());
        }
        let id = OperatorSessionId { value: value.clone() };
        sessions.insert(
            value,
            OperatorSessionRecord {
                binding,
                session_generation: Generation { value: generation },
                created_at: now,
                last_used_at: now,
                expires_at: now + self.ttl,
                revoked_at: None,
            },
        );
        IssuedOperatorSession {
            id,
            session_generation: Generation { value: generation },
        }
    }

    pub async fn verify(
        &self,
        session_id: &OperatorSessionId,
        binding: &OperatorSessionBinding,
    ) -> bool {
        let _mutation_guard = self.mutation_guard.lock().await;
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&session_id.value) else {
            return false;
        };
        let now = Instant::now();
        if session.revoked_at.is_some()
            || now >= session.expires_at
            || session.binding != *binding
        {
            return false;
        }
        session.last_used_at = now;
        true
    }

    pub async fn revoke_current(
        &self,
        session_id: &OperatorSessionId,
        binding: &OperatorSessionBinding,
    ) -> bool {
        let _mutation_guard = self.mutation_guard.lock().await;
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&session_id.value) else {
            return false;
        };
        if session.binding != *binding || session.revoked_at.is_some() || Instant::now() >= session.expires_at {
            return false;
        }
        session.revoked_at = Some(Instant::now());
        true
    }

    pub async fn current_generation(&self, actor_id: &ActorId) -> Generation {
        let _mutation_guard = self.mutation_guard.lock().await;
        let next = self
            .next_generation
            .lock()
            .await
            .get(&actor_id.value)
            .copied()
            .unwrap_or(0);
        let floor = self
            .invalidated_through_generation
            .lock()
            .await
            .get(&actor_id.value)
            .copied()
            .unwrap_or(0);
        Generation {
            value: next.max(floor).max(1),
        }
    }

    pub async fn revoke_all_for_actor(
        &self,
        actor_id: &ActorId,
        through: &Generation,
    ) -> u32 {
        let _mutation_guard = self.mutation_guard.lock().await;
        let now = Instant::now();
        let mut sessions = self.sessions.lock().await;
        let mut revoked = 0;
        for session in sessions.values_mut() {
            if session.binding.actor_id == *actor_id
                && session.session_generation.value <= through.value
                && session.revoked_at.is_none()
                && now < session.expires_at
            {
                session.revoked_at = Some(now);
                revoked += 1;
            }
        }
        revoked
    }

    pub async fn revoke_matching_principal(
        &self,
        binding: impl Fn(&OperatorSessionBinding) -> bool,
    ) -> u32 {
        let _mutation_guard = self.mutation_guard.lock().await;
        let now = Instant::now();
        let mut sessions = self.sessions.lock().await;
        let mut revoked = 0;
        for session in sessions.values_mut() {
            if binding(&session.binding)
                && session.revoked_at.is_none()
                && now < session.expires_at
            {
                session.revoked_at = Some(now);
                revoked += 1;
            }
        }
        revoked
    }

    /// Fold durable generation-fence events into the process-local session
    /// registry. Lockdown entry uses the same monotonic fence as revoke-all.
    pub async fn observe(&self, event: &RecordedEvent) -> Result<(), String> {
        let _mutation_guard = self.mutation_guard.lock().await;
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            format!(
                "corrupt replay record: unknown stored event kind {}",
                event.payload.kind
            )
        })?;
        if kind == StoredEventKind::Unspecified {
            return Err(
                "corrupt replay log: operator-session event kind is unspecified".to_owned(),
            );
        }
        let (actor, generation) = if kind == StoredEventKind::OperatorSessionRevocation {
            let revocation = OperatorSessionRevocation::decode(event.payload.payload.as_slice())
                .map_err(|error| format!("cannot decode operator session revocation: {error}"))?;
            (
                revocation
                    .operator_actor_id
                    .ok_or_else(|| "operator session revocation has no actor".to_owned())?,
                revocation
                    .invalidated_through_generation
                    .ok_or_else(|| "operator session revocation has no generation".to_owned())?,
            )
        } else if kind == StoredEventKind::SecurityLockdown {
            let source = SecurityLockdownEvent::decode(event.payload.payload.as_slice())
                .map_err(|error| format!("cannot decode security event: {error}"))?;
            let entered = match source.transition {
                Some(security_lockdown_event::Transition::Entered(entered)) => entered,
                _ => return Ok(()),
            };
            let actor = entered
                .entered_by
                .and_then(|value| value.actor_id)
                .ok_or_else(|| "lockdown entry has no verified actor".to_owned())?;
            let generation = entered
                .invalidated_through_operator_session_generation
                .ok_or_else(|| "lockdown entry has no session generation".to_owned())?;
            (actor, generation)
        } else {
            return Ok(());
        };
        let mut floors = self.invalidated_through_generation.lock().await;
        let floor = floors.entry(actor.value.clone()).or_insert(0);
        if generation.value > *floor {
            *floor = generation.value;
        }
        drop(floors);
        let mut next = self.next_generation.lock().await;
        let value = next.entry(actor.value.clone()).or_insert(0);
        *value = (*value).max(generation.value);
        drop(next);
        let now = Instant::now();
        let mut sessions = self.sessions.lock().await;
        for session in sessions.values_mut() {
            if session.binding.actor_id == actor
                && session.session_generation.value <= generation.value
                && session.revoked_at.is_none()
                && now < session.expires_at
            {
                session.revoked_at = Some(now);
            }
        }
        Ok(())
    }

    pub async fn summaries(&self) -> Vec<patchbay_contracts::patchbay::OperatorSessionSummary> {
        let _mutation_guard = self.mutation_guard.lock().await;
        let now = Instant::now();
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .map(|session| patchbay_contracts::patchbay::OperatorSessionSummary {
                actor_id: Some(session.binding.actor_id.clone()),
                endpoint_id: Some(session.binding.endpoint_id.clone()),
                device_id: Some(session.binding.device_id.clone()),
                operator_session_generation: Some(session.session_generation),
                active: session.revoked_at.is_none() && now < session.expires_at,
                revoked: session.revoked_at.is_some(),
                expired: now >= session.expires_at,
            })
            .collect()
    }

    #[cfg(test)]
    pub async fn session_count(&self) -> usize {
        let _mutation_guard = self.mutation_guard.lock().await;
        self.sessions.lock().await.len()
    }
}

#[allow(dead_code)]
fn _record_fields_are_intentional(record: &OperatorSessionRecord) -> (&Instant, &Instant) {
    (&record.created_at, &record.last_used_at)
}
