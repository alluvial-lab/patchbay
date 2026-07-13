//! Pure operation-acceptance domain state.
//!
//! This module owns the canonical command lifecycle adjacency and the fold that
//! applies durable command-transition events. Both live acceptance and replay
//! use the same functions so recovery cannot acquire different state-machine
//! semantics from the write path.

pub mod state;
pub mod transitions;

pub use state::{is_terminal, CommandRecord, OperationStateExt};
pub use transitions::{allowed_transition, apply_transition, AcceptanceError};
