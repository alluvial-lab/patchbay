//! Pure operation-acceptance domain state.
//!
//! This module owns the canonical command lifecycle adjacency and the fold that
//! applies durable command-transition events. Both live acceptance and replay
//! use the same functions so recovery cannot acquire different state-machine
//! semantics from the write path.

pub mod index;
pub mod observation;
pub mod pipeline;
pub mod ports;
pub mod replay;
pub mod state;
pub mod transitions;

pub use index::CommandIndex;
pub use observation::{ingest_observation, CommandStateLookup, IngestResult, TransitionCandidate};
pub use pipeline::{submit, target_key_for};
pub use ports::{
    Authorized, GrantCheck, GrantDenied, TargetBinding, TargetNotFound, TargetResolver,
};
pub use replay::rebuild_from_log;
pub use state::{is_terminal, CommandRecord, OperationStateExt};
pub use transitions::{allowed_transition, apply_transition, AcceptanceError};
