//! Core-owned time port and deterministic clock adapters.
//!
//! Time is an acceptance/authority dependency, not an implementation detail of
//! the server or a particular persistence backend. Callers sample the port at
//! a boundary and pass that value into pure domain predicates.

use std::{
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use prost_types::Timestamp;

/// The core time port.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Production wall-clock adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        timestamp_from_system_time(SystemTime::now())
    }
}

/// Deterministic mutable clock for tests and controlled compositions.
#[derive(Debug, Clone)]
pub struct TestClock {
    now: Arc<RwLock<Timestamp>>,
}

impl TestClock {
    #[must_use]
    pub fn new(now: Timestamp) -> Self {
        Self {
            now: Arc::new(RwLock::new(now)),
        }
    }

    pub fn set(&self, now: Timestamp) {
        *self.now.write().expect("test clock lock poisoned") = now;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.now.read().expect("test clock lock poisoned")
    }
}

/// Convert a system instant to a normalized protobuf timestamp.
#[must_use]
pub fn timestamp_from_system_time(time: SystemTime) -> Timestamp {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => Timestamp {
            seconds: i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
            nanos: duration.subsec_nanos() as i32,
        },
        Err(error) => {
            let duration = error.duration();
            let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
            if duration.subsec_nanos() == 0 {
                Timestamp {
                    seconds: -seconds,
                    nanos: 0,
                }
            } else {
                Timestamp {
                    seconds: -seconds - 1,
                    nanos: 1_000_000_000 - duration.subsec_nanos() as i32,
                }
            }
        }
    }
}
