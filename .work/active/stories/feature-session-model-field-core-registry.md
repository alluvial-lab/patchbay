---
id: feature-session-model-field-core-registry
kind: story
stage: done
parent: feature-session-model-field
depends_on: [feature-session-model-field-proto-contract]
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Fold and materialize mutable session model state

Extend the core session report writer, durable event helpers, session registry,
and snapshot materializer so the current opaque model is registered, carried
through a generation replacement, and updated only by a `SessionModelChanged`
delta.

## Acceptance evidence

- First registration and a generation bump preserve the reported model in the
  live record and materialized `Session` snapshot.
- An equal-generation model-only report appends one `SessionModelChanged`
  event containing the expected prior and new value, folds it, and rebuilds to
  the same current model without changing identity or session-state axes.
- A combined report deterministically appends the existing state-axis deltas,
  then model change, then relabel; retry after a partial append remains
  replayable.
- Existing identity/tombstone validation rejects a malformed or stale model
  mutation exactly as the other session mutations do.

## Ordering

Depends on the contract checkpoint. Adapter reporting and operator surfaces use
the resulting snapshot/event semantics.

## Implementation notes
- Execution capability: inline single-owner implementation; the registry is the durable source of truth and its replay guards are the high-risk boundary.
- Review weight: standard (default).
- Files changed: session ingest/event/registry code and Rust session fixtures; adapter report mapping and snapshot materialization; server session fixtures.
- Tests added/removed: model-only ingestion/rebuild and mismatch rejection tests; existing combined-delta and partial-append retry tests now include model changes. No tests removed.
- Simplification: no second model-observation path; the existing session report and durable registry fold carry the state.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `cargo test -p patchbay-core --test sessions_ingest --test sessions_registry --test sessions_replay_resolver` (30 passed) and `cargo test -p patchbay-core-server --lib adapter_service::tests` (9 passed).
