---
id: story-v0-core-sessions-ingest
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-core-sessions
depends_on: [story-v0-core-sessions-state-machine, story-v0-core-sessions-registry]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-13
updated: 2026-07-16
---

# Story: Session report ingestion (the writer)

## Scope

Implement Unit 3 of `feature-v0-core-sessions`: the direct ingestion method — the direct analog of acceptance's `ingest_observation`. Receives an adapter-reported session observation, detects what changed, writes the appropriate `SessionState` delta event, and returns. Owns its event kind end-to-end.

## Units

- `core/src/session/ingest.rs` — `SessionReport`, `IngestResult`, `SessionLookup` port, `ingest_session_report`

## Implementation

See `feature-v0-core-sessions.md` Unit 3 for exact signatures. Key points:

- `ingest_session_report<S, L>(storage, session_lookup, report)` mirrors `ingest_observation<S, L>(storage, state_lookup, observation)`: receive evidence → read current state via `SessionLookup` → detect transition → write delta event → return.
- `SessionLookup` is the read port the writer uses (mirrors `CommandStateLookup`). `SessionRegistry` implements it (added in this story or in story 2 — coordinate so the impl lands once).
- Generation bump: `report.session_generation > current` → write a single `SessionGenerationBumped` event that both tombstones the prior generation AND establishes the new one. The tombstone fact is carried in the event; the LSN is the event's LSN.
- Equal generation: derive state-axis / metadata deltas. Validate state-axis transitions against the tables (from story 1) before writing. Disallowed → `InvalidTransition`.
- Lower generation: return `StaleGeneration` error, do NOT mutate state (the `GenerationMonotonic` action guard).
- No change: return `NoChange` (idempotent re-report).
- The durable write happens before the in-memory registry is updated (durability first). The warm-path mechanism (replay-after-write vs in-process channel notify) is pinned here: prefer replay-after-write for simplicity in v0.1.0 (re-read the just-written event and observe it), matching how acceptance keeps `CommandIndex` warm.

## Acceptance Criteria

- [ ] First registration writes a `SessionRegistered` event and returns `Registered`
- [ ] Generation bump writes a `SessionGenerationBumped` event that tombstones the prior generation
- [ ] Equal-generation report with changed connectivity writes a `SessionConnectivityChanged` event
- [ ] Equal-generation report with changed activity writes a `SessionActivityChanged` event
- [ ] Equal-generation report with changed metadata writes a `SessionRelabeled` event
- [ ] Equal-generation report with no changes returns `NoChange` (idempotent)
- [ ] Lower-generation report returns `StaleGeneration` error and does NOT mutate state
- [ ] Disallowed state-axis transition returns `InvalidTransition` error before writing
- [ ] The durable write happens before the in-memory registry is updated

## Notes

- Depends on stories 1 (state machine) and 2 (registry + events).
- This is the writer pattern (Q4=b). The decisive precedent is `ingest_observation` in `core/src/acceptance/observation.rs` — read it before implementing.
- `SessionReport` is the sessions analog of `Observation`. The adapter reports raw state; the core derives the transition. This keeps "the core tombstones the prior generation" honest.

## Implementation notes

- Added `core/src/session/ingest.rs` with the report/result types, the static-dispatch `SessionLookup` read port, its `SessionRegistry` implementation, and the validate → lookup → derive → append → return writer flow.
- Generation supersession appends one `SessionGenerationBumped` event; both result identifiers intentionally name that same committed event. Equal-generation reports emit at most one prioritized axis/metadata delta, and lower generations or invalid transitions append nothing.
- Mirrored acceptance's actual warm-path boundary: `ingest_observation` does not mutate `CommandIndex`, so session ingestion likewise returns after the durable append. The caller re-reads/observes the committed event to keep `SessionRegistry` warm, preserving durability-first ordering.
- Added `core/tests/sessions_ingest.rs` covering registration, one-event generation bump plus tombstone folding, connectivity/activity/relabel deltas, idempotent no-change, stale-generation rejection, and pre-write invalid-transition rejection.
- Verification passed: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core` and `CARGO_HOME=/tmp/cargo-home cargo test -p patchbay-core`.
