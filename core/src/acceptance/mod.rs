//! Pure operation-acceptance domain state.
//!
//! This module owns the canonical command lifecycle adjacency and the fold that
//! applies durable command-transition events. Both live acceptance and replay
//! use the same functions so recovery cannot acquire different state-machine
//! semantics from the write path.

pub mod elicitation;
pub mod elicitation_response;
pub mod index;
pub mod observation;
pub mod pipeline;
pub mod ports;
pub mod replay;
pub mod state;
pub mod transitions;

pub use elicitation::{rebuild_slots_from_log, ElicitationRecord, ElicitationSlotLayer};
pub use elicitation_response::validate_response_payload;
pub use index::CommandIndex;
pub use observation::{
    ingest_observation, CommandSnapshot, CommandStateLookup, IngestResult, TransitionCandidate,
};
pub use pipeline::{submit, submit_with_clock, target_key_for, COMMITTED_OPERATION_KINDS};
pub use ports::{
    ActiveElicitation, Authorized, Clock, ElicitationContractLookup, GrantCheck, GrantDenied,
    SystemClock, TargetBinding, TargetNotFound, TargetResolver,
};
pub use replay::rebuild_from_log;
pub use state::{is_terminal, CommandRecord, OperationStateExt};
pub use transitions::{allowed_transition, apply_grant_revocation_effect, apply_transition, AcceptanceError};
