---
id: feature-session-model-field-core-registry
kind: story
stage: implementing
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
