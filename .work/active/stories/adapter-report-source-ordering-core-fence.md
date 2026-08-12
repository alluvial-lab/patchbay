---
id: adapter-report-source-ordering-core-fence
kind: story
stage: done
tags: [protocol, storage]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-contract-foundation]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Fence session ingestion by durable source order

## Checkpoint

Bind authenticated session reports to the current adapter producer epoch,
reject stale source cursors before deriving field transitions, append one
atomic report event, and rebuild the source watermark through the canonical
session projection and snapshot path.

## Acceptance evidence

- Same runtime/adapter generation requires a strictly greater revision; lower
  runtime generation, old adapter generation, equal revision, and lower
  revision append no session-state mutation and record stale audit evidence.
- A newer runtime-session or authenticated adapter generation can establish a
  fresh positive local revision without admitting evidence from the old
  producer.
- One accepted report changing connectivity, activity, labels, and model writes
  exactly one event; an unchanged newer report still durably advances its
  watermark.
- Append failure leaves both projection and cursor unchanged. Hot replay,
  restart replay, and `SessionSnapshot` agree on visible values and the last
  source cursor.
- Legacy session deltas still replay, while disconnect/lockdown staleness does
  not consume adapter source order.
- The obsolete multi-delta append/warm/result machinery and its
  implementation-bound partial-prefix tests are removed.

## Ordering constraints

Consumes `adapter-report-source-ordering-contract-foundation`. It can proceed in
the same feature-owned wave as the Pi sequencer, but the integrated
conformance checkpoint waits for both producer and consumer.

## Current-HEAD reconciliation (2026-08-10)

- The session projection is now authority-domain-bound and fallible, with an exact raw owned-event replay ledger. `SessionReportApplied` must enter that ledger only after its full pre-state/cursor/axis validation and atomic record replacement succeeds; failed folds must remain exactly non-mutating and leave the LSN available to a corrected envelope.
- Shared full-log replay already rejects missing/zero/gapped/duplicate/`UNSPECIFIED` records, while the registry intentionally ignores unowned sibling events. The report fold extends this division rather than adding a competing prefix cursor.
- Server ingress currently rebuilds inside `CoreDecisionGate`, append-then-folds legacy report deltas, then rebuilds. The implementation retains the gate and rebuild boundaries and changes only adapter reports to one append-and-folded full-report event. Legacy deltas and core-authored disconnect/lockdown degradation remain and preserve the source cursor.
- Aggregate `ProjectionState::catch_up` now stages all projections before publication; snapshot carriage must integrate with that cancellation-safe path rather than publishing a private session registry early.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, maximum reasoning (caller-selected for cross-file durability, replay, snapshot, and authenticated-ingress semantics); direct-read single-owner delivery, no nested agents or peeragent.
- Review weight: `thorough` (explicit caller override); child checkpoint review is not applicable.
- Files changed: `core/src/session/{mod,events,ingest,registry}.rs`, focused session/acceptance/conformance tests, `server/src/{adapter_service,state}.rs`, and compatibility fixtures in server integration tests.
- Tests added/removed: replaced append-then-fold delta tests with atomic full-report, unchanged-watermark, stale-order, malformed-boundary, append-failure, replay, and exact-prestate tests; added authenticated stale-audit and snapshot-cursor tests. Legacy delta replay tests remain.
- Simplification: removed the handwritten core `SessionReport` DTO and all multi-delta report composition. Equal-generation ingress now performs one comparison, one append, and one fold; core-authored disconnect still uses its legacy delta without manufacturing adapter order.
- Discrepancies from design: no material semantic deviation. `IngestResult` exposes one `event_id` for generation bumps instead of retaining the obsolete duplicate tombstone/new-generation aliases; server keeps its established optional response helper although every accepted report now has an event id.
- Adjacent issues parked: none.

## Verification evidence

- `cargo test -p patchbay-core --test sessions_ingest --test sessions_registry --test sessions_replay_resolver --test sessions_proptest` — passed (54 focused tests).
- `cargo test -p patchbay-core-server --all-features --lib adapter_service::tests` — passed (27 authenticated adapter-ingress tests).
- `cargo test -p patchbay-core-server --lib state::tests::session_snapshot_publishes_the_last_source_cursor` — passed.
- `cargo test -p patchbay-core -p patchbay-core-server --all-features` — passed, including full core/server unit, integration, conformance-vector, property, replay-prefix, gRPC, spawn-completion, and trust-boundary suites.
- `git diff --check` — passed after scoped formatting; global rustfmt baseline remains excluded by item constraint.
