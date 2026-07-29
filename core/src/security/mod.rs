//! Durable, authority-domain keyed security posture.
//!
//! Lockdown is a source event, not a process flag. The projection is folded
//! from the same durable log as authority and session state and is consumed by
//! the acceptance-owned `OperationPosture` port.

pub mod events;
pub mod projection;
pub mod replay;

pub use events::{entered, exited, encode, SecurityLockdownEvent};
pub use projection::{ActiveSecurityLockdown, SecurityError, SecurityPostureProjection};
pub use replay::rebuild_from_log;
