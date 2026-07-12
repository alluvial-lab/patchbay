//! Storage plane.
//!
//! The storage port (`Storage` trait) is the Ports & Adapters boundary for
//! durability: domain logic depends on the trait, not on any specific backend.
//! The first backend is rusqlite (SQLite in WAL mode with `synchronous=FULL`).
//!
//! See `docs/PROTOCOL.md` § "Persistence and recovery" and § "Snapshots and
//! streams" for the semantics this module implements.

pub mod port;
pub mod recovery;
pub mod rusqlite;

pub use port::{event_id, DedupOutcome, RecordedEvent, Storage, StorageError, StoredSnapshot, TargetKey};
pub use recovery::{recover, RecoveryState};
pub use rusqlite::RusqliteStorage;
