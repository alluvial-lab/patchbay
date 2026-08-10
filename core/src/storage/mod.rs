//! Storage plane.
//!
//! The storage port (`Storage` trait) is the Ports & Adapters boundary for
//! durability: domain logic depends on the trait, not on any specific backend.
//! The first backend is rusqlite (SQLite in WAL mode with `synchronous=FULL`).
//!
//! See `docs/PROTOCOL.md` § "Persistence and recovery" and § "Snapshots and
//! streams" for the semantics this module implements.

pub mod audited;
pub mod port;
pub mod recovery;
pub mod rusqlite;

pub use audited::{audit_draft_for_source, AuditedStorage};
pub use port::{
    event_id, AuditPageSpec, AuditRecordDraft, AuditedAppend, AuditedBatchAppend,
    AuditedDecisionAppend,
    AuditedDedupOutcome, CoreGenerationStore, DedupOutcome,
    RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey,
};
pub use recovery::{recover, RecoveryState, ValidatedSnapshot};
pub use rusqlite::{RusqliteStorage, LATEST_SCHEMA_VERSION};
