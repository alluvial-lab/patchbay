use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use patchbay_contracts::patchbay::{ActorId, OperatorSessionId};
use tokio::sync::Mutex;

use crate::identity::random_token;

pub const DEFAULT_OPERATOR_SESSION_TTL: Duration = Duration::from_secs(8 * 60 * 60);

#[derive(Debug, Clone)]
struct OperatorSessionRecord {
    actor_id: ActorId,
    expires_at: Instant,
    revoked: bool,
}

/// Core-owned, process-local operator sessions. Restart invalidates every
/// token, which fails closed; callers must authenticate again.
#[derive(Debug, Clone)]
pub struct OperatorSessionRegistry {
    sessions: Arc<Mutex<HashMap<String, OperatorSessionRecord>>>,
    ttl: Duration,
}

impl OperatorSessionRegistry {
    pub fn new(ttl: Duration) -> Result<Self, String> {
        if ttl.is_zero() {
            return Err("operator session TTL must be positive".to_owned());
        }
        Ok(Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        })
    }

    pub async fn issue(&self, actor_id: ActorId) -> OperatorSessionId {
        let mut sessions = self.sessions.lock().await;
        let mut value = format!("operator-session-{}", random_token());
        while sessions.contains_key(&value) {
            value = format!("operator-session-{}", random_token());
        }
        sessions.insert(
            value.clone(),
            OperatorSessionRecord {
                actor_id,
                expires_at: Instant::now() + self.ttl,
                revoked: false,
            },
        );
        OperatorSessionId { value }
    }

    pub async fn verify(&self, session_id: &OperatorSessionId, actor_id: &ActorId) -> bool {
        let sessions = self.sessions.lock().await;
        sessions.get(&session_id.value).is_some_and(|session| {
            !session.revoked && Instant::now() < session.expires_at && &session.actor_id == actor_id
        })
    }

    pub async fn revoke(&self, session_id: &OperatorSessionId, actor_id: &ActorId) -> bool {
        let mut sessions = self.sessions.lock().await;
        let Some(session) = sessions.get_mut(&session_id.value) else {
            return false;
        };
        if &session.actor_id != actor_id || session.revoked || Instant::now() >= session.expires_at
        {
            return false;
        }
        session.revoked = true;
        true
    }
}
