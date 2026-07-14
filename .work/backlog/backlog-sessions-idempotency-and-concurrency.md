---
id: backlog-sessions-idempotency-and-concurrency
kind: feature
stage: backlog
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Backlog: Sessions idempotency soundness and warm-path concurrency

## Source
Found during deep review of `feature-v0-core-sessions` (Phase 2 adversarial, cross-model openai-codex/gpt-5.6-sol).

## Findings

### 1. Idempotency guards treat conflicting events as harmless redelivery
Idempotency is inferred from "key already exists" or `LSN <= last_lsn`, not from event identity + payload equality. Examples:
- A second `SessionRegistered` at a distinct later LSN with a different generation/state/labels is silently ignored rather than rejected as `CorruptLog`.
- After applying `Unknown → Live` at LSN 2, a different event at the same LSN claiming `Unknown → Offline` returns `Ok(())`.
- A generation bump with the same LSN and `from_generation` but a different `to_generation` is accepted as redelivery (the existing tombstone check doesn't compare `to_generation`).

Canonical storage should never issue two different events at one LSN, but if it happens, fail-fast should expose the corruption. The duplicate-registration case is reachable via the concurrency issue below.

**Direction**: compare payload/identity equality on redelivery, not just key existence + LSN.

### 2. Read-decide-append warm-path can create unreplayable logs under concurrency
`ingest_session_report` performs an unlocked read-decide-append: read current state via `SessionLookup`, decide the delta, append. The registry is warmed separately by the caller. Two concurrent reports can derive mutations from the same stale `SessionRecord` (e.g. both read generation 1, append `1→2` and `1→3`; replay applies the first bump, then the second conflicts with the tombstone → `CorruptLog`).

**Context**: acceptance's `ingest_observation` has the SAME read-decide-append shape (no lock) and is at `stage: done` — this is an established pattern in this codebase, not sessions-specific. v0.1.0 is single-writer (one core process). The risk is latent unless/until concurrent ingestion callers exist. The warm-path ordering (finding 3 in Phase 2) compounds it: if the caller feeds committed events to `observe` in completion order rather than LSN order, `last_authoritative_lsn` guards can silently drop events (out-of-order observation diverges from ordered replay).

**Direction**: document the single-writer/concurrent-caller assumption, OR add a serialization layer (mutex around the read-decide-append-warm sequence), OR make the warm path append-only-then-replay (read the just-written event back via `read_after` rather than trusting caller-supplied `observe` ordering). The acceptance feature should be evaluated for the same issue.

## Priority
Latent for v0.1.0 single-writer. Becomes real if concurrent ingestion callers are introduced or if the warm path is not strictly LSN-ordered. Worth documenting the assumption; worth fixing the warm-path ordering (finding 3) which is independent of concurrency.
