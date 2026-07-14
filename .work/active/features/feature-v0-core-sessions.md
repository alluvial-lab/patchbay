---
id: feature-v0-core-sessions
kind: feature
stage: review
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-13
---

# Feature: Session registry and generation

## Brief

Build the session registry: session state (connectivity × activity axes), session generation, session replacement, and stale/offline/unknown presentation. The core tracks which sessions exist, which machine/project/adapter/runtime each belongs to, and each session's authoritative connectivity and activity status. Session identity is stable enough that late replies cannot affect the wrong session.

Session state composes two axes: `SessionConnectivityState` (`live`, `stale`, `offline`, `unknown`, `failed`) and `SessionActivityState` (idle, working, etc.). A stale/unknown connectivity value dominates presentation. Session generation never decreases; a session replacement (e.g. Pi's `session_new`) bumps the generation and tombstones the prior generation, so late events binding to the pre-replacement context become `stale_event` audit records rather than polluting the new conversation.

This feature implements the session-registry port that the acceptance pipeline calls for target-identity binding. It can proceed in parallel with acceptance and authority after persistence lands.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (session state is durable). Implements the target-identity port that acceptance calls; acceptance and sessions can proceed in parallel after persistence lands because the port interface decouples them.

## Formal-model backing

- `GenerationMonotonic` (promoted, `session_generation.qnt`) — the live session generation never decreases. Strict-supersession (equal/lower reports are no-ops) is additionally enforced by the action guard but is NOT a checked temporal property (exceeded Apalache's experimental temporal support; see `idea-tlc-temporal-workaround`).
- Session-identity tuple stability, late-generation inertness, labels-cannot-override-identity — stated-normative obligations (v1 formal gate owns the real properties).

## Foundation references

- `docs/PROTOCOL.md` — Sessions; Session state axes; Id spaces; Snapshots and streams (session snapshots)
- `docs/ARCHITECTURE.md` — Runtime/session plane; State and snapshot plane
- `docs/VERIFICATION.md` — `GenerationMonotonic` promoted property; stated-normative session-identity obligations
- `docs/ADAPTER-PI.md` — `session_new` = generation bump + tombstone; `session_compact` does not bump; snapshot tier = partial
- `contracts/proto/patchbay/sessions.proto` — `Session`, `SessionState`, `SessionConnectivityState`, `SessionActivityState`, `SessionSnapshot`, `ViewRevision`
- `contracts/proto/patchbay/common.proto` — `RuntimeSessionId`, `Generation`, `AdapterId`
- `specs/seed/session_generation.qnt` — generation state and `GenerationMonotonic` property

## Design decisions (feature-design, 2026-07-13)

Resolved interactively with the operator after unpacking each option's trade-offs.

- **Q1 — Durable event shape: delta events.** Chosen over full-state events (verbose, obscures transition semantics — a generation bump looks identical to a relabel once applied) and observation-derived (ruled out: the tombstone is a core-owned action, not an adapter observation; a purely derived layer cannot durably record a core-decided tombstone). One `SessionState` event per mutation kind (register, generation-bump+tombstone, connectivity-change, activity-change, relabel), each consuming an LSN. Mirrors acceptance's choice of uniform `CommandTransition` delta events. Maps 1:1 to the formal model's LSN-consuming events (`advance`/`commitTerminal`-style). Replay is a clean fold that applies each delta in order.
- **Q2 — Snapshot checkpointing: defer (replay from LSN 0).** Chosen over fixing the projection-discriminator gap now. The snapshot table is keyed `(authority_domain_id, snapshot_lsn)` with no projection discriminator, so a session-only snapshot could cause command/elicitation projections to skip earlier events they need. Acceptance (`snapshot_checkpoint` is a no-op) and elicitation (`rebuild_slots_from_log` replays from LSN 0) already follow this pattern for the same reason. The cross-cutting storage-port + rusqlite change to add a projection discriminator belongs in its own refactor story, not bolted onto the sessions feature. Noted as a risk; tracked separately.
- **Q3 — `TargetResolver` validation depth: existence + tombstone-only.** Chosen over existence+liveness (conflates "can't reach now" with "doesn't exist" — would reject commands the operator may want to queue for a temporarily-offline session) and existence-only (lets you target a dead generation, which is targeting a session that no longer exists as a live target). Tombstone is an identity/safety fact (wrong generation = wrong target = `target_not_found`); connectivity is a delivery concern (acceptance is deliberately separate from delivery — the operator may legitimately accept a command for an offline session to queue/retry on reconnect). Keeps acceptance and delivery separated, which is a load-bearing principle. Aligns with the failure vocabulary: `target_not_found` = doesn't exist in the session context; offline sessions *do* exist.
- **Q4 — Ingress mechanism: direct ingestion writer.** Chosen over log-tailing projection (ruled out twice over: (1) a pure tail can only react to events someone else wrote, but the only candidate writer of `SessionState` events would be acceptance, and making acceptance derive generation-bump/tombstone events would leak session semantics into the acceptance pipeline — the exact boundary violation Ports & Adapters exists to prevent; (2) the protocol says the tombstone is "an audit record retained indefinitely" that survives log compaction — a stored fact, not a derived view; a pure tail can derive but can't durably store). Sessions owns its own state transitions (generation bump, tombstone), so it writes them — mirroring acceptance's `ingest_observation` (writer) vs the elicitation layer's pure-tail (which tails only because its triggering events belong to acceptance). The decisive precedent from the acceptance implementation: `ingest_observation` writes both the evidence and the derived transition; `ElicitationSlotLayer` owns no storage and has no write method. The rule the codebase already encodes: *if a feature owns its own state transitions, it writes them; if it reacts to another feature's events, it tails.* Sessions owns its transitions → it writes.
- **Q5 — Scope: full feature with child stories.** Chosen over splitting (the state axes are tightly coupled to the same registry and the same `SessionState` events; splitting them into a separate feature would fragment the projection). Implement everything: registry, connectivity×activity axes, generation monotonicity, replacement, stale/offline/unknown, `TargetResolver`. Spawn ~4 child stories.

## Architectural choice

A hybrid event-sourced session registry, mirroring the acceptance feature's established shape: the event log (owned by `feature-v0-core-persistence`) is the single source of truth for session state. Sessions writes `SessionState` delta events through the `Storage::append` port. An in-memory `SessionRegistry` is the hot lookup path, rebuilt from replay on startup. Snapshot checkpointing is deferred (replay from LSN 0), matching acceptance and elicitation today.

The sessions feature owns its event kind end-to-end. It exposes a direct ingestion method (`ingest_session_report`) — the direct analog of acceptance's `ingest_observation` — that receives an adapter-reported session observation, detects a generation bump, writes the tombstone + new-generation `SessionState` event, and returns. A separate `SessionRegistry` projection (the analog of `CommandIndex` / `ElicitationSlotLayer`) handles replay and read access via `observe(event)`. This is the writer pattern, not the pure-tail pattern, because sessions owns its own state transitions.

The `TargetResolver` port (already declared in `core/src/acceptance/ports.rs`) is implemented by the `SessionRegistry`: `resolve` binds a target scope to a concrete `TargetBinding` if the session identity tuple exists and is not tombstoned, returning `TargetNotFound` otherwise. Tombstone is treated as an identity/existence fact (reject); connectivity is a delivery concern (allow — the operator may queue commands for offline sessions).

This shape honors Ports & Adapters (sessions depends on `Storage` and implements `TargetResolver`; acceptance depends on the `TargetResolver` trait, not on sessions), Single Source of Truth (the event log is the only source of session state; the in-memory registry is a pure fold), Generated Contracts (`Session`, `SessionState`, `SessionSnapshot` are generated proto messages; `StoredEventKind::SessionState` is the schema-owned discriminator), and Fail Fast (invalid generation reports, disallowed state-axis transitions, and log corruption are rejected at the boundary).

## Implementation Units

### Unit 1: Session identity, state axes, and transition validation

**File**: `core/src/session/mod.rs`, `core/src/session/state.rs`

**Story**: `story-v0-core-sessions-state-machine`

The single source of truth for session identity, the connectivity×activity state axes, and the allowed transitions — derived from `docs/PROTOCOL.md`, not invented. Mirrors `command_lifecycle.qnt`'s `allowedTransition` and acceptance's `allowed_transition`.

```rust
// core/src/session/state.rs
use patchbay_contracts::patchbay::{
    SessionConnectivityState, SessionActivityState, SessionState,
};

/// The canonical session identity tuple from docs/PROTOCOL.md "Sessions".
/// adapter_id + deployment_scope + runtime_session_id + session_generation.
/// Project/cwd/name are metadata, NOT identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionIdentity {
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
}

/// The canonical SessionConnectivityState transition adjacency from
/// docs/PROTOCOL.md "Session state axes". Mirrors the protocol table exactly.
#[must_use]
pub fn allowed_connectivity_transition(
    from: SessionConnectivityState,
    to: SessionConnectivityState,
) -> bool {
    use SessionConnectivityState::*;
    match from {
        Unspecified => matches!(to, Live | Stale | Offline | Unknown | Failed),
        Unknown => matches!(to, Live | Stale | Offline | Failed),
        Live => matches!(to, Stale | Offline | Failed),
        Stale => matches!(to, Live | Offline | Unknown | Failed),
        Offline => matches!(to, Live | Stale | Unknown | Failed),
        Failed => matches!(to, Live | Stale | Offline | Unknown),
    }
}

/// The canonical SessionActivityState transition adjacency from
/// docs/PROTOCOL.md "Session state axes".
#[must_use]
pub fn allowed_activity_transition(
    from: SessionActivityState,
    to: SessionActivityState,
) -> bool {
    use SessionActivityState::*;
    match from {
        Unspecified => matches!(to, Idle | Working | Unknown),
        Unknown => matches!(to, Idle | Working),
        Idle => matches!(to, Working | Unknown),
        Working => matches!(to, Idle | Unknown),
    }
}

/// A stale or unknown connectivity value dominates presentation:
/// stale working is not live working. Returns the effective connectivity
/// for presentation given a raw state.
#[must_use]
pub fn effective_connectivity(state: SessionState) -> SessionConnectivityState {
    // Stale/unknown dominate: if connectivity is stale or unknown, that is
    // what the UI must render, regardless of activity.
    match state.connectivity() {
        SessionConnectivityState::Stale | SessionConnectivityState::Unknown => {
            state.connectivity()
        }
        other => other,
    }
}
```

**Implementation Notes**:
- `SessionIdentity` is a Rust newtype over the four identity fields, NOT the full `Session` proto. The proto `Session` carries metadata (project/cwd/name) and state; identity is the tuple alone. This enforces "labels cannot override identity" at the type level.
- The transition adjacency tables are copied verbatim from `docs/PROTOCOL.md` "Session state axes". They are the SSOT for allowed transitions, derived from the protocol.
- `Unspecified` is the initial state for both axes (first registration). The protocol's connectivity table starts from `unknown` (the first observation moves it to live/stale/offline/failed); `Unspecified` is the pre-observation state. Treat `Unspecified → Unknown` as the implicit first step, then apply the protocol table. (This matches how acceptance treats `OperationState::Unspecified` as the pre-`accepted` state.)
- `effective_connectivity` encodes the "stale/unknown dominates" rule as a pure function, testable in isolation.

**Acceptance Criteria**:
- [ ] `allowed_connectivity_transition` matches the protocol table exactly (exhaustive table test)
- [ ] `allowed_activity_transition` matches the protocol table exactly (exhaustive table test)
- [ ] `SessionIdentity` equality ignores project/cwd/name (two sessions with same tuple but different labels are equal)
- [ ] `effective_connectivity` returns `Stale`/`Unknown` when connectivity is stale/unknown, regardless of activity
- [ ] `Unspecified` is the only initial state for both axes

---

### Unit 2: Session delta events and the `SessionRegistry` projection

**File**: `core/src/session/events.rs`, `core/src/session/registry.rs`

**Story**: `story-v0-core-sessions-registry`

The durable event shape (delta events) and the in-memory projection that folds them. Mirrors acceptance's `CommandTransition` + `CommandIndex`.

```rust
// core/src/session/events.rs
// The delta event payload for StoredEventKind::SessionState. One event per
// mutation kind. This is the wire shape stored in the event log; the
// SessionRegistry projection folds these into in-memory SessionRecord state.
//
// NOTE: This requires adding a `SessionStateEvent` proto message to
// contracts/proto/patchbay/sessions.proto and regenerating. The discriminator
// StoredEventKind::SessionState = 7 already exists; this message is its payload.
// (Generated Contracts: the schema owns the variant set and the payload shape.)

// In sessions.proto, add:
//   message SessionStateEvent {
//     AuthorityDomainId authority_domain_id = 1;
//     oneof mutation {
//       SessionRegistered registered = 2;
//       SessionGenerationBumped generation_bumped = 3;
//       SessionConnectivityChanged connectivity_changed = 4;
//       SessionActivityChanged activity_changed = 5;
//       SessionRelabeled relabeled = 6;
//     }
//   }
//   message SessionRegistered {
//     AdapterId adapter_id = 1;
//     string deployment_scope = 2;
//     RuntimeSessionId runtime_session_id = 3;
//     Generation session_generation = 4;
//     SessionState initial_state = 5;  // typically Unspecified/Unknown
//     string project = 6;
//     string cwd = 7;
//     string name = 8;
//   }
//   message SessionGenerationBumped {
//     AdapterId adapter_id = 1;
//     string deployment_scope = 2;
//     RuntimeSessionId runtime_session_id = 3;
//     Generation from_generation = 4;
//     Generation to_generation = 5;
//     // The prior generation is tombstoned at this event's LSN. The tombstone
//     // fact (generation N existed, superseded at LSN X) is retained
//     // indefinitely as an audit record (docs/PROTOCOL.md "Sessions").
//   }
//   message SessionConnectivityChanged {
//     AdapterId adapter_id = 1;
//     string deployment_scope = 2;
//     RuntimeSessionId runtime_session_id = 3;
//     Generation session_generation = 4;
//     SessionConnectivityState from = 5;
//     SessionConnectivityState to = 6;
//   }
//   message SessionActivityChanged { /* same identity fields + from/to */ }
//   message SessionRelabeled {
//     AdapterId adapter_id = 1;
//     string deployment_scope = 2;
//     RuntimeSessionId runtime_session_id = 3;
//     Generation session_generation = 4;
//     string project = 5;
//     string cwd = 6;
//     string name = 7;
//   }
```

```rust
// core/src/session/registry.rs
use std::collections::HashMap;
use patchbay_contracts::patchbay::{Session, SessionState, SessionConnectivityState,
    SessionActivityState, AdapterId, RuntimeSessionId, Generation, AuthorityDomainId, Lsn};
use crate::storage::{RecordedEvent, Storage};
use super::{SessionIdentity, allowed_connectivity_transition, allowed_activity_transition};
use super::events::SessionStateEvent;

/// The in-memory session record, derived from the event log.
/// Mirrors acceptance's CommandRecord: a pure fold of SessionState events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub identity: SessionIdentity,
    pub state: SessionState,
    pub project: String,
    pub cwd: String,
    pub name: String,
    pub last_authoritative_lsn: Option<u64>,
    pub tombstoned: bool,
    pub superseded_at_lsn: Option<u64>,
}

/// A tombstone for a prior generation. Retained indefinitely as an audit
/// record (docs/PROTOCOL.md "Sessions"). Keyed by (runtime_session_id, generation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTombstone {
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub superseded_generation: Generation,
    pub superseded_at_lsn: u64,
}

/// The in-memory session registry projection. Rebuilt from replay on startup.
/// Mirrors acceptance's CommandIndex / ElicitationSlotLayer: a pure fold over
/// the event log, updated by observe(event).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionRegistry {
    /// Live sessions keyed by (adapter_id, deployment_scope, runtime_session_id).
    /// One live generation per runtime_session_id.
    sessions: HashMap<SessionLiveKey, SessionRecord>,
    /// Tombstones for superseded generations, retained indefinitely.
    /// Keyed by (runtime_session_id, generation).
    tombstones: HashMap<SessionTombstoneKey, SessionTombstone>,
}

/// The key for a live session: identity minus generation (one live gen per slot).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionLiveKey {
    adapter_id: AdapterId,
    deployment_scope: String,
    runtime_session_id: RuntimeSessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SessionTombstoneKey {
    runtime_session_id: RuntimeSessionId,
    generation: Generation,
}

impl SessionRegistry {
    pub fn new() -> Self { Self::default() }

    /// Fold one committed event into the session projection.
    /// Idempotent for re-delivered events. Mirrors ElicitationSlotLayer::observe.
    pub fn observe(&mut self, event: &RecordedEvent) -> Result<(), SessionError> {
        let kind = StoredEventKind::try_from(event.payload.kind).map_err(|_| {
            SessionError::CorruptRecord(format!("unknown stored event kind {}", event.payload.kind))
        })?;
        if kind != StoredEventKind::SessionState {
            return Ok(());  // sessions only consumes SessionState events
        }
        let (event_domain, event_lsn) = event_identity(event)?;
        let state_event = SessionStateEvent::decode(event.payload.payload.as_slice())
            .map_err(|e| SessionError::CorruptRecord(format!(
                "cannot decode session state event at LSN {event_lsn}: {e}")))?;
        // validate domain matches
        // dispatch on mutation oneof:
        //   registered -> insert new live session (first-write-wins on duplicate)
        //   generation_bumped -> tombstone prior gen, insert new live gen
        //   connectivity_changed -> validate transition, update state
        //   activity_changed -> validate transition, update state
        //   relabeled -> update metadata only (identity unchanged)
        Ok(())
    }

    /// Look up the live session for a target scope. Returns the binding if
    /// the session exists and is not tombstoned; None if not found or
    /// the targeted generation is tombstoned (stale target).
    pub fn resolve(&self, target_scope: &TargetScope) -> Option<TargetBinding> {
        // Extract identity fields from target_scope. If session_generation is
        // specified and that generation is tombstoned, return None (stale target).
        // If session_generation is unspecified, bind the live generation.
        // Return TargetBinding { runtime_session_id, session_generation, adapter_id }.
    }

    /// Look up a live session record by identity.
    pub fn get_session(&self, identity: &SessionIdentity) -> Option<&SessionRecord> { ... }

    /// Look up a tombstone for a superseded generation.
    pub fn get_tombstone(
        &self,
        runtime_session_id: &RuntimeSessionId,
        generation: &Generation,
    ) -> Option<&SessionTombstone> { ... }
}
```

**Implementation Notes**:
- The `SessionStateEvent` proto message is NEW — it must be added to `contracts/proto/patchbay/sessions.proto` and regenerated. The `StoredEventKind::SessionState = 7` discriminator already exists; this message is its payload. This is the Generated Contracts approach: the schema owns the payload shape.
- `SessionRegistry` mirrors `ElicitationSlotLayer` structurally: a `HashMap`-backed projection with an `observe(&mut self, event)` fold. The difference is sessions also has a *writer* (Unit 3); the elicitation layer is pure-tail.
- `SessionLiveKey` is identity-minus-generation: one live generation per `runtime_session_id`. A generation bump replaces the live entry and adds a tombstone for the prior generation.
- Tombstones are retained indefinitely (never evicted in v0.1.0). The protocol says "the tombstone fact is an audit record retained indefinitely." Log compaction reclaims per-generation detail but not the tombstone fact; v0.1.0 has no compaction, so tombstones accumulate. Noted as a risk.
- `observe` validates state-axis transitions via `allowed_connectivity_transition` / `allowed_activity_transition` and returns `SessionError::CorruptLog` on violation (Fail Fast, mirroring acceptance's `apply_transition`).
- First-write-wins on duplicate `registered` events (idempotent replay, mirroring elicitation's `observe_elicitation`).

**Acceptance Criteria**:
- [ ] `SessionStateEvent` proto added and generated bindings compile
- [ ] `SessionRegistry::observe` folds each mutation kind correctly
- [ ] A generation bump tombstones the prior generation and inserts the new live generation
- [ ] Tombstones are retained and queryable by `(runtime_session_id, generation)`
- [ ] `observe` rejects disallowed connectivity/activity transitions as `CorruptLog`
- [ ] `observe` is idempotent for re-delivered events
- [ ] `resolve` returns `None` for a tombstoned generation (stale target)

---

### Unit 3: Session report ingestion (the writer)

**File**: `core/src/session/ingest.rs`

**Story**: `story-v0-core-sessions-ingest`

The direct ingestion method — the direct analog of acceptance's `ingest_observation`. Receives an adapter-reported session observation, detects a generation bump, writes the `SessionState` event, and returns. Owns its event kind end-to-end.

```rust
// core/src/session/ingest.rs
use patchbay_contracts::patchbay::{
    AuthorityDomainId, AdapterId, RuntimeSessionId, Generation, EventId,
    SessionConnectivityState, SessionActivityState, SessionState,
    StoredEventKind, StoredEventPayload,
};
use prost::Message;
use crate::storage::Storage;
use super::registry::{SessionRegistry, SessionLookup};
use super::events::SessionStateEvent;
use super::SessionError;

/// An adapter-reported session observation. The adapter reports the current
/// identity tuple, state axes, and metadata for a session. The core decides
/// what (if anything) changed and writes the appropriate delta event.
///
/// This is the sessions analog of acceptance's Observation. The adapter
/// reports raw state; the core derives the transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReport {
    pub authority_domain_id: AuthorityDomainId,
    pub adapter_id: AdapterId,
    pub deployment_scope: String,
    pub runtime_session_id: RuntimeSessionId,
    pub session_generation: Generation,
    pub connectivity: SessionConnectivityState,
    pub activity: SessionActivityState,
    pub project: String,
    pub cwd: String,
    pub name: String,
}

/// The durable outcome of ingesting one session report.
/// Mirrors acceptance's IngestResult.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestResult {
    /// First registration of this session identity.
    Registered { event_id: EventId },
    /// Generation bumped: prior generation tombstoned, new generation live.
    GenerationBumped {
        tombstone_event_id: EventId,
        new_generation_event_id: EventId,
        from_generation: Generation,
        to_generation: Generation,
    },
    /// Connectivity axis changed.
    ConnectivityChanged { event_id: EventId, from: SessionConnectivityState, to: SessionConnectivityState },
    /// Activity axis changed.
    ActivityChanged { event_id: EventId, from: SessionActivityState, to: SessionActivityState },
    /// Metadata (project/cwd/name) changed; identity unchanged.
    Relabeled { event_id: EventId },
    /// No change: the report matches the current registry state (idempotent re-report).
    NoChange,
}

/// Read access to the live session projection. The ingestion path uses this
/// to detect what changed before writing a delta event. Mirrors acceptance's
/// CommandStateLookup.
pub trait SessionLookup: Send + Sync {
    fn current_session(
        &self,
        adapter_id: &AdapterId,
        deployment_scope: &str,
        runtime_session_id: &RuntimeSessionId,
    ) -> impl std::future::Future<Output = Option<SessionRecord>> + Send;
}

/// Ingest an adapter-reported session observation.
///
/// The core compares the report to the current registry state and writes the
/// appropriate delta event(s). A generation bump writes a single
/// SessionGenerationBumped event (which tombstones the prior generation and
/// establishes the new one). State-axis changes are validated against the
/// protocol transition tables before writing.
///
/// Mirrors acceptance::ingest_observation: receive evidence -> read current
/// state -> detect transition -> write delta event -> return.
pub async fn ingest_session_report<S, L>(
    storage: &S,
    session_lookup: &L,
    report: SessionReport,
) -> Result<IngestResult, SessionError>
where
    S: Storage,
    L: SessionLookup,
{
    let authority_domain_id = validate_report_domain(&report)?;
    let live = session_lookup
        .current_session(&report.adapter_id, &report.deployment_scope, &report.runtime_session_id)
        .await;

    match live {
        None => {
            // First registration. Write a SessionRegistered event.
            let event = SessionStateEvent { mutation: registered(report) };
            let event_id = storage.append(authority_domain_id, encode(event)).await?;
            Ok(IngestResult::Registered { event_id })
        }
        Some(current) => {
            // Compare report to current state, derive the delta.
            if report.session_generation > current.identity.session_generation {
                // Generation bump: tombstone prior, establish new.
                // Validate strict-supersession (GenerationMonotonic action guard).
                let event = SessionStateEvent { mutation: generation_bumped(&current, &report) };
                let event_id = storage.append(authority_domain_id, encode(event)).await?;
                Ok(IngestResult::GenerationBumped {
                    tombstone_event_id: event_id.clone(),  // same event carries the tombstone
                    new_generation_event_id: event_id,
                    from_generation: current.identity.session_generation,
                    to_generation: report.session_generation,
                })
            } else if report.session_generation == current.identity.session_generation {
                // Same generation: derive state-axis / metadata deltas.
                if report.connectivity != current.state.connectivity() {
                    validate_connectivity_transition(current.state.connectivity(), report.connectivity)?;
                    let event = SessionStateEvent { mutation: connectivity_changed(&current, &report) };
                    let event_id = storage.append(authority_domain_id, encode(event)).await?;
                    Ok(IngestResult::ConnectivityChanged { event_id, from: current.state.connectivity(), to: report.connectivity })
                } else if report.activity != current.state.activity() {
                    validate_activity_transition(current.state.activity(), report.activity)?;
                    let event = SessionStateEvent { mutation: activity_changed(&current, &report) };
                    let event_id = storage.append(authority_domain_id, encode(event)).await?;
                    Ok(IngestResult::ActivityChanged { event_id, from: current.state.activity(), to: report.activity })
                } else if metadata_changed(&current, &report) {
                    let event = SessionStateEvent { mutation: relabeled(&current, &report) };
                    let event_id = storage.append(authority_domain_id, encode(event)).await?;
                    Ok(IngestResult::Relabeled { event_id })
                } else {
                    Ok(IngestResult::NoChange)
                }
            } else {
                // Lower generation report: reject as stale_event audit record.
                // The live generation is left unchanged (GenerationMonotonic).
                // v0.1.0: return an error / audit result; do NOT mutate state.
                Err(SessionError::StaleGeneration {
                    live: current.identity.session_generation,
                    reported: report.session_generation,
                })
            }
        }
    }
}
```

**Implementation Notes**:
- This is the writer pattern, mirroring `ingest_observation` exactly: receive evidence → read current state via `SessionLookup` → detect transition → write delta event → return. The in-memory `SessionRegistry` is updated separately by replay/observe (or by an in-process notify after the durable write, mirroring how acceptance keeps `CommandIndex` warm).
- `SessionReport` is the sessions analog of `Observation`. The adapter reports raw state; the core derives the transition. This keeps "the core tombstones the prior generation" honest — the tombstone is a core action triggered by the adapter's generation report, not an adapter-declared fact.
- Generation bump writes a SINGLE `SessionGenerationBumped` event that both tombstones the prior generation AND establishes the new one. The tombstone fact (superseded_generation, superseded_at_lsn) is carried in the event; the LSN is the event's LSN. This matches the formal model's `tombstoneLsn` = the generation-report's LSN.
- Strict-supersession is enforced here (the action guard): `report.session_generation > current` supersedes; `==` is a no-op/state-update; `<` is rejected as `StaleGeneration` (a `stale_event` audit record, not a state mutation). This is the `GenerationMonotonic` action guard, NOT the checked temporal property.
- Equal-generation reports with changed state axes are validated against the transition tables before writing. A disallowed transition (e.g. `Live → Working` on the activity axis — wait, that's connectivity vs activity; `Live → Idle` is fine) returns `SessionError::InvalidTransition`.
- `SessionLookup` is the read port the writer uses, mirroring `CommandStateLookup`. The `SessionRegistry` implements it.
- The `SessionRegistry` projection is kept warm by replaying the event the writer just wrote (or by an in-process channel). This mirrors how acceptance keeps `CommandIndex` warm after `ingest_observation` writes. The exact warm-path mechanism (replay-after-write vs channel notify) is an implementation detail pinned in the story; the durable write is the source of truth either way.

**Acceptance Criteria**:
- [ ] First registration writes a `SessionRegistered` event and returns `Registered`
- [ ] Generation bump writes a `SessionGenerationBumped` event that tombstones the prior generation
- [ ] Equal-generation report with changed connectivity writes a `SessionConnectivityChanged` event
- [ ] Equal-generation report with changed activity writes an `SessionActivityChanged` event
- [ ] Equal-generation report with changed metadata writes a `SessionRelabeled` event
- [ ] Equal-generation report with no changes returns `NoChange` (idempotent)
- [ ] Lower-generation report returns `StaleGeneration` error and does NOT mutate state
- [ ] Disallowed state-axis transition returns `InvalidTransition` error before writing
- [ ] The durable write happens before the in-memory registry is updated (durability first)

---

### Unit 4: Replay, `TargetResolver` impl, and module wiring

**File**: `core/src/session/replay.rs`, `core/src/session/resolver.rs`, `core/src/session/mod.rs`, `core/src/lib.rs`

**Story**: `story-v0-core-sessions-replay-resolver`

Rebuild the registry from the log (mirroring `rebuild_from_log` / `rebuild_slots_from_log`), implement `TargetResolver` on the registry, and wire the module into the crate.

```rust
// core/src/session/replay.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, Lsn};
use crate::storage::Storage;
use super::registry::SessionRegistry;
use super::SessionError;

/// Rebuild a session registry by replaying one authority domain.
///
/// v0.1.0 replays from LSN 0 because the shared snapshot slot has no
/// projection discriminator. This matches command-index recovery and
/// elicitation-slot recovery. (See feature Risks.)
pub async fn rebuild_from_log<S: Storage>(
    storage: &S,
    authority_domain_id: &AuthorityDomainId,
) -> Result<SessionRegistry, SessionError> {
    let events = storage
        .read_after(authority_domain_id, Lsn { value: 0 })
        .await?;
    let mut registry = SessionRegistry::new();
    let mut previous_lsn = 0u64;

    for event in events {
        let (event_domain, event_lsn) = event_identity(&event)?;
        if event_domain != authority_domain_id {
            return Err(SessionError::CorruptLog(format!(
                "recovery event belongs to authority domain {:?}, expected {:?}",
                event_domain, authority_domain_id
            )));
        }
        if event_lsn <= previous_lsn {
            return Err(SessionError::CorruptLog(format!(
                "recovery event LSN {event_lsn} is not after previous LSN {previous_lsn}"
            )));
        }
        registry.observe(&event)?;
        previous_lsn = event_lsn;
    }

    Ok(registry)
}
```

```rust
// core/src/session/resolver.rs
use patchbay_contracts::patchbay::{AuthorityDomainId, TargetScope, RuntimeSessionId,
    Generation, AdapterId};
use crate::acceptance::ports::{TargetResolver, TargetBinding, TargetNotFound};
use super::registry::SessionRegistry;

/// Implement the TargetResolver port (declared in acceptance/ports.rs) on
/// the session registry. Acceptance calls resolve() to bind a target scope
/// to a concrete delivery identity before accepting an operation.
///
/// Validation depth (design decision Q3): existence + tombstone-only.
/// - Tombstoned generation -> TargetNotFound (stale target; wrong generation
///   = wrong target). This is an identity/existence failure.
/// - Offline/failed connectivity -> ALLOWED. Connectivity is a delivery
///   concern; the operator may queue commands for an offline session.
impl TargetResolver for SessionRegistry {
    async fn resolve(
        &self,
        _authority_domain_id: &AuthorityDomainId,
        target_scope: &TargetScope,
    ) -> Result<TargetBinding, TargetNotFound> {
        // Extract identity fields from target_scope.
        let adapter_id = target_scope.adapter_id.as_ref()
            .ok_or_else(|| TargetNotFound::NotFound { target: format!("{:?}", target_scope) })?;
        let runtime_session_id = target_scope.runtime_session_id.as_ref()
            .ok_or_else(|| TargetNotFound::NotFound { target: format!("{:?}", target_scope) })?;
        let deployment_scope = &target_scope.deployment_scope;

        // If the caller specified a generation, check it's not tombstoned.
        if let Some(requested_gen) = target_scope.session_generation.as_ref() {
            if self.is_tombstoned(runtime_session_id, requested_gen) {
                return Err(TargetNotFound::NotFound {
                    target: format!("tombstoned generation {:?}", requested_gen),
                });
            }
        }

        // Look up the live session. If the caller specified a generation,
        // it must match the live generation (else stale). If unspecified,
        // bind the live generation.
        match self.get_live_session(adapter_id, deployment_scope, runtime_session_id) {
            Some(record) => {
                // If a generation was requested, it must equal the live gen
                // (tombstone check above already rejected tombstoned gens).
                if let Some(requested_gen) = target_scope.session_generation.as_ref() {
                    if &record.identity.session_generation != requested_gen {
                        return Err(TargetNotFound::NotFound {
                            target: format!("generation {:?} is not live", requested_gen),
                        });
                    }
                }
                Ok(TargetBinding {
                    runtime_session_id: record.identity.runtime_session_id.clone(),
                    session_generation: record.identity.session_generation,
                    adapter_id: record.identity.adapter_id.clone(),
                })
            }
            None => Err(TargetNotFound::NotFound {
                target: format!("session not found: {:?}", runtime_session_id),
            }),
        }
    }
}
```

```rust
// core/src/session/mod.rs
pub mod state;
pub mod events;
pub mod registry;
pub mod ingest;
pub mod replay;
pub mod resolver;

pub use state::{SessionIdentity, allowed_connectivity_transition, allowed_activity_transition};
pub use events::SessionStateEvent;
pub use registry::{SessionRegistry, SessionRecord, SessionTombstone};
pub use ingest::{SessionReport, IngestResult, ingest_session_report, SessionLookup};
pub use replay::rebuild_from_log;

/// Sessions-specific errors. Typed thiserror enum, mirroring StorageError
/// and AcceptanceError.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("corrupt session record: {0}")]
    CorruptRecord(String),
    #[error("corrupt session log: {0}")]
    CorruptLog(String),
    #[error("invalid state-axis transition: {from:?} -> {to:?}")]
    InvalidTransition { from: String, to: String },
    #[error("stale generation report: live={live}, reported={reported}")]
    StaleGeneration { live: Generation, reported: Generation },
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),
}
```

```rust
// core/src/lib.rs (add the session module alongside acceptance and storage)
pub mod acceptance;
pub mod session;   // NEW
pub mod storage;
```

**Implementation Notes**:
- `rebuild_from_log` is a near-exact copy of `rebuild_slots_from_log` (elicitation) and `rebuild_from_log` (acceptance): read from LSN 0, fold via `observe`, validate LSN monotonicity and domain match. The shared shape is intentional — three projections with identical recovery structure.
- `TargetResolver` is implemented ON the `SessionRegistry`, not on a separate adapter struct. The registry holds the state; the port is a read interface over it. This matches how `CommandIndex` implements `CommandStateLookup`.
- `resolve` treats a tombstoned generation as `TargetNotFound` (stale target). A requested generation that is neither tombstoned nor live (e.g. a future generation the core hasn't seen) is also `TargetNotFound`. Connectivity is NOT checked — offline/failed sessions resolve successfully (Q3).
- The `SessionError` enum mirrors `StorageError`/`AcceptanceError`: `CorruptRecord`, `CorruptLog`, plus sessions-specific `InvalidTransition` and `StaleGeneration`. `Storage` errors bubble via `#[from]`.
- Module wiring: `core/src/session/` is a new module directory alongside `acceptance/` and `storage/`. `lib.rs` exports it.

**Acceptance Criteria**:
- [ ] `rebuild_from_log` reconstructs the registry identically to a live registry that observed the same events
- [ ] `rebuild_from_log` rejects out-of-order LSNs and cross-domain events as `CorruptLog`
- [ ] `TargetResolver::resolve` returns `Ok(TargetBinding)` for a live session
- [ ] `TargetResolver::resolve` returns `TargetNotFound` for a tombstoned generation
- [ ] `TargetResolver::resolve` returns `TargetNotFound` for an unknown session
- [ ] `TargetResolver::resolve` returns `Ok` for an offline/failed session (connectivity not checked)
- [ ] `TargetResolver::resolve` binds the live generation when `session_generation` is unspecified
- [ ] `core/src/session/` module compiles and is exported from `core/src/lib.rs`

---

### Unit 5: Property tests for session invariants

**File**: `core/tests/sessions_proptest.rs`

**Story**: `story-v0-core-sessions-proptests`

Property tests for the promoted `GenerationMonotonic` property and the stated-normative obligations. Mirrors `acceptance_proptest.rs` and `storage_proptest.rs`.

```rust
// core/tests/sessions_proptest.rs
// Property tests for the session registry. Mirrors acceptance_proptest.rs:
// - proptest strategies for session reports, generations, state axes
// - property oracles for GenerationMonotonic and stated-normative obligations
// - mutation tests to demonstrate properties are non-vacuous

proptest! {
    /// GenerationMonotonic (promoted): the live session generation never
    /// decreases across any sequence of session reports.
    #[test]
    fn generation_never_decreases(reports in any_session_report_sequence()) {
        let mut registry = SessionRegistry::new();
        let mut live_gen = 0u64;
        for report in reports {
            // apply the report (via observe on the encoded event, or via
            // ingest_session_report against an in-memory storage)
            // assert: after each report, the live generation >= previous live gen
        }
    }

    /// Strict-supersession (stated-normative, action guard): an equal report
    /// is a no-op on generation; a lower report is rejected and leaves the
    /// live generation unchanged.
    #[test]
    fn equal_generation_is_noop_lower_is_rejected(/* ... */) { ... }

    /// LateGenerationInert (stated-normative): a report binding to a
    /// tombstoned generation does not mutate the live generation.
    #[test]
    fn late_generation_is_inert(/* ... */) { ... }

    /// LabelsCannotOverrideIdentity (stated-normative): changing project/cwd/name
    /// does not change session identity or create a new session.
    #[test]
    fn relabel_preserves_identity(/* ... */) { ... }

    /// Tombstone retention: a tombstoned generation is retained and queryable
    /// after subsequent generation bumps.
    #[test]
    fn tombstones_retained(/* ... */) { ... }

    /// Replay determinism: rebuilding from the log produces identical state
    /// to a live registry that observed the same events.
    #[test]
    fn replay_matches_live(/* ... */) { ... }
}

// Mutation tests (non-vacuity): run the same properties against a buggy
// registry that allows generation decrease, and assert the property FAILS.
// Mirrors acceptance_proptest.rs mutation adapters.
```

**Implementation Notes**:
- Proptest strategies: `any_session_report()`, `any_generation()`, `any_connectivity_state()`, `any_activity_state()`, `any_session_report_sequence()` (a sequence of reports against one or more sessions, including generation bumps, state-axis changes, and stale-generation reports).
- The mutation tests are essential for non-vacuity (the acceptance proptests established this discipline). A buggy registry that allows generation decrease MUST fail `generation_never_decreases`.
- `GenerationMonotonic` is the only PROMOTED property — it must pass against the real implementation. The others are stated-normative obligations tested as properties but not backed by checked formulas; they document and enforce the intended behavior.
- Test against `RusqliteStorage::open_in_memory()` for the full write→replay round-trip, and against `NoopStorage`/fault-injecting wrappers for targeted property checks (mirroring acceptance test doubles).

**Acceptance Criteria**:
- [ ] `generation_never_decreases` passes against the real registry
- [ ] `generation_never_decreases` FAILS against a mutation that allows decrease (non-vacuous)
- [ ] `equal_generation_is_noop_lower_is_rejected` passes
- [ ] `late_generation_is_inert` passes
- [ ] `relabel_preserves_identity` passes
- [ ] `tombstones_retained` passes
- [ ] `replay_matches_live` passes (replay determinism)

---

## Implementation Order

1. `story-v0-core-sessions-state-machine` — identity tuple, state axes, transition adjacency (no deps; the SSOT for allowed transitions)
2. `story-v0-core-sessions-registry` — `SessionStateEvent` proto + `SessionRegistry` projection (depends on 1)
3. `story-v0-core-sessions-ingest` — `ingest_session_report` writer + `SessionLookup` port (depends on 1, 2)
4. `story-v0-core-sessions-replay-resolver` — `rebuild_from_log` + `TargetResolver` impl + module wiring (depends on 1, 2, 3)
5. `story-v0-core-sessions-proptests` — property tests for GenerationMonotonic + stated-normative obligations (depends on 1-4)

Stories 1-2 are sequential (the registry needs the state machine). Story 3 depends on 2 (the writer writes events the registry folds). Story 4 depends on 3 (replay + resolver need the full registry + writer). Story 5 depends on all.

## Testing

### Unit Tests: `core/tests/sessions_*.rs`
- `sessions_state_machine.rs` — exhaustive table tests for connectivity/activity transition adjacency, identity equality, effective_connectivity
- `sessions_registry.rs` — fold correctness, tombstone retention, idempotent observe
- `sessions_ingest.rs` — first registration, generation bump, state-axis changes, metadata changes, no-change, stale generation rejection, invalid transition rejection
- `sessions_replay.rs` — replay determinism, LSN monotonicity, cross-domain rejection
- `sessions_resolver.rs` — live session binds, tombstoned generation rejected, unknown session rejected, offline session allowed, unspecified-generation binds live
- `sessions_proptest.rs` — property oracles + mutation tests

### Integration Points
- **Acceptance ↔ Sessions**: acceptance calls `TargetResolver::resolve` (implemented by `SessionRegistry`) before accepting an operation. The existing `TestTargetResolver` in `core/tests/acceptance_pipeline.rs` is replaced by a real `SessionRegistry` in integration tests.
- **Sessions ↔ Storage**: sessions writes `SessionState` events via `Storage::append` and reads via `Storage::read_after` for replay. Same `Storage` port as acceptance.
- **Sessions ↔ Elicitation**: no direct coupling. Both are independent log consumers (sessions writes its own events; elicitation tails command transitions). They share the replay-from-LSN-0 pattern and the snapshot-discriminator gap.

## Risks

- **Snapshot checkpointing deferred (Q2).** The sessions projection replays from LSN 0 on recovery, like acceptance and elicitation. As the event log grows, recovery cost grows linearly. The cross-cutting fix (projection discriminator on the snapshot key) is tracked as a separate refactor story; it should land before v0.1.0 ships if log sizes warrant, but is not on the sessions critical path.
- **Tombstone accumulation.** Tombstones are retained indefinitely (protocol requirement). v0.1.0 has no log compaction, so tombstones accumulate in memory and in the log. For a single-operator deployment with modest session-replacement frequency, this is bounded and acceptable; noted as a future scaling concern.
- **Staleness is not time-driven in v0.1.0.** The protocol describes staleness as "lacks a sufficiently fresh authoritative signal," implying a time/heartbeat policy. v0.1.0 records connectivity as adapter-reported; it does not run a background staleness timer that flips `live → stale` on heartbeat timeout. A staleness/timer policy is a reserved seam (the protocol's "timeout/staleness policy" driver). The state-axis transitions are enforced, but the transition *triggers* are adapter-driven, not core-timer-driven. Noted as a v0.1.0 scope boundary.
- **`SessionStateEvent` proto is new.** Adding it requires regenerating contracts (`contracts/rust`, `contracts/ts`). The `StoredEventKind::SessionState = 7` discriminator already exists, so no enum change — just a new message. Low risk, but the regeneration must land before the registry story compiles.
- **Warm-path after write.** The writer (`ingest_session_report`) writes to storage; the in-memory `SessionRegistry` must be kept warm. The exact mechanism (replay-after-write vs in-process channel notify) is pinned in the ingest story. The durable write is the source of truth either way; the warm path is a performance concern, not a correctness one.

## Extension pressure classification

- **Committed v0.1.0**: session identity tuple `(adapter_id, deployment_scope, runtime_session_id, session_generation)`; `SessionConnectivityState` and `SessionActivityState` registries and transition tables; generation monotonicity (strict-supersession action guard); tombstone-on-replacement; `TargetResolver` port (existence + tombstone-only validation); `SessionState` delta events; replay-from-LSN-0 recovery.
- **Reserved seam**: snapshot checkpointing with projection discriminator (deferred; cross-cutting storage concern); time-driven staleness/heartbeat policy (the protocol's "timeout/staleness policy" driver); log compaction reclaiming per-generation detail (keeps the tombstone fact); `SessionSnapshot` materialization (the proto exists; core does not serve snapshots in v0.1.0 beyond the in-memory registry).
- **Explicitly rejected**: deriving session state purely from the Observation stream without dedicated `SessionState` events (the tombstone is core-owned authoritative state, not a derived view); making acceptance write `SessionState` events (leaks session semantics into the acceptance pipeline — Ports & Adapters violation); checking connectivity in `TargetResolver` (conflates delivery with existence — offline sessions may be legitimately targeted for queued commands).

## Implementation summary (implement-orchestrator, 2026-07-13)

All 5 child stories implemented and advanced to `stage: review`:
1. `story-v0-core-sessions-state-machine` (commit `e874bf2`) — identity tuple, state axes, transition tables, `SessionError`
2. `story-v0-core-sessions-registry` (commit `ba63380`) — `SessionStateEvent` proto + `SessionRegistry` projection (fold, tombstones, idempotent observe)
3. `story-v0-core-sessions-ingest` (commit `c68a48b`) — `ingest_session_report` writer (mirrors `ingest_observation`), `SessionLookup` port, strict-supersession action guard
4. `story-v0-core-sessions-replay-resolver` (commit `3a624d6`) — `rebuild_from_log` + `impl TargetResolver for SessionRegistry` (existence + tombstone-only, Q3)
5. `story-v0-core-sessions-proptests` (commit `b65d7b5`) — `GenerationMonotonic` + stated-normative properties + non-vacuous mutation test

Cross-cutting notes:
- The `SessionStateEvent` proto was added to `sessions.proto` and regenerated (Rust via `cargo build`/prost-build, TS via `buf generate`). The known `buf generate` formatting drift on Rust gen was handled by regenerating Rust via `cargo build` after `buf generate`.
- Wave 4 subagent was interrupted mid-work; orchestrator finished the story inline (test file + commit + stage advance).
- The `TargetResolver` port (declared in `core/src/acceptance/ports.rs` by the acceptance feature) is now implemented by `SessionRegistry` — the sessions↔acceptance seam is connected.

Verification: `cargo build` clean, `cargo test -p patchbay-core` 152 tests pass, `cargo clippy --all-targets` clean.

## Review (2026-07-13)

**Verdict**: Request changes (bounce to implementing)

**Depth**: Deep lane, two-phase (completeness → adversarial), cross-model fresh-context. Orchestrator (umans) is a different model class from the reviewers (openai-codex/gpt-5.6-sol), satisfying the cross-model advisory-review requirement. Both reviewers ran convergence loops.

**Blockers** (4 correctness bugs, verified against code):
- **B1 — Ingestion writes events that replay cannot rebuild** (`ingest.rs`): `ingest_session_report` validates only `authority_domain_id`; empty `adapter_id`/`deployment_scope`/`runtime_session_id` are accepted at write but rejected at replay (`mutation_identity`). Unreplayable log. -> `story-fix-sessions-ingest-correctness`
- **B2 — Generation bump discards new generation's state/metadata** (`ingest.rs` + `sessions.proto` + `registry.rs`): `SessionGenerationBumped` carries only `from`/`to_generation`; `observe_generation_bumped` clones the prior generation's stale state/metadata into the new generation. A session replacement (e.g. `session_new`) reports the new generation's state, which is silently lost. -> `story-fix-sessions-ingest-correctness`
- **B3 — Multi-field report truncated** (`ingest.rs`): the equal-generation branch returns after the first changed delta (connectivity→activity→metadata), dropping simultaneous changes to the other fields. -> `story-fix-sessions-ingest-correctness`
- **B4 — Tombstone keys omit adapter_id + deployment_scope** (`registry.rs`): `SessionTombstoneKey` is `(runtime_session_id, generation)` only; runtime session IDs are adapter-reported and not globally unique. Cross-adapter collision falsely rejects an unrelated live session as stale. -> `story-fix-sessions-tombstone-key`

**Important** (parked in backlog):
- Authority-domain isolation absent from lookup/resolution (`SessionLookup` takes no domain, `resolve` ignores `_authority_domain_id`). Latent in single-domain v0.1.0. -> `backlog-sessions-authority-domain-isolation`
- Test coverage gaps: replay corruption cases untested, acceptance↔sessions integration test missing, malformed-event replay tests missing, resolver boundary under-tested, proptest identity isolation absent. -> `backlog-sessions-test-coverage-gaps`
- Idempotency guards infer redelivery from key-existence/LSN not payload equality; read-decide-append warm-path can create unreplayable logs under concurrency. Established pattern (acceptance's `ingest_observation` is identical) — latent in single-writer v0.1.0. -> `backlog-sessions-idempotency-and-concurrency`

**Nits** (not filed):
- Resolver `not_found` helper stringifies the whole `TargetScope` in the error; acceptable for v0.1.0 diagnostics.
- `effective_connectivity` takes `SessionState` (the proto) but only uses connectivity; could take `SessionConnectivityState` directly. Minor, leave.

**Notes**: The implementation correctly mirrors acceptance's established patterns (writer + pure-tail projection, RPITIT ports, typed thiserror enums) and the state-axis transition tables are exactly right. The blockers are genuine correctness bugs in the sessions-specific logic (state inheritance on bump, multi-field truncation, tombstone key completeness, write/replay validation parity) — exactly the kind of issue the two-phase deep review exists to catch. Both reviewers independently flagged B2/B3/B4, raising confidence. The feature bounces to `implementing` until the two fix stories land.

## Re-review (2026-07-13, post-fix)

Both blocker fix stories landed and are at `stage: review`:
- `story-fix-sessions-tombstone-key` (B4, commit `da64160`) — tombstone key extended to full identity `(adapter_id, deployment_scope, runtime_session_id, generation)`; cross-adapter collision regression test added.
- `story-fix-sessions-ingest-correctness` (B1+B2+B3, commit `06e6251`) — B1: `validate_report` checks all identity fields before append; B2: `SessionGenerationBumped` proto carries `initial_state`+metadata, `observe_generation_bumped` applies reported state (not clone); B3: equal-gen branch appends all deltas (new `DeltasApplied` variant). Regression tests for all three.

Orchestrator verified the fixes directly against code (not just test-pass):
- B1: `validate_report` at ingest.rs:121, checks all four fields.
- B2: proto fields 6-9 added; registry applies `next.state = initial_state` + reported metadata (line 74-77), no longer blindly clones.
- B3: early returns removed; `DeltasApplied { event_ids }` combined variant.
- B4: `SessionTombstoneKey` includes `adapter_id` + `deployment_scope`.

Verification: `cargo build` clean, `cargo test -p patchbay-core` 161 tests pass (was 139 pre-fix), `cargo clippy --all-targets` clean. Gen diff additions-only (8 Rust + 22 TS lines, no reformatting drift).

**Verdict**: Approve with comments. All 4 blockers resolved. Important findings remain parked in backlog (authority-domain isolation, test coverage gaps, idempotency/concurrency — all latent in single-domain/single-writer v0.1.0). Feature re-advanced to `stage: review` with 7 child stories (5 original + 2 fix) all at `review`.
