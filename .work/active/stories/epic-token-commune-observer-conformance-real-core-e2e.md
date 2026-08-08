---
id: epic-token-commune-observer-conformance-real-core-e2e
kind: story
stage: done
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-phase-1-completeness-vectors]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Bind completeness evidence to the real gateway, adapter, and core process

## Checkpoint

Expand the serial token-commune E2E to start a scripted local HTTP gateway, load
a real `0600` member-key file, run the real adapter and Rust core against
SQLite, and use generated attach/report/subscribe/load-snapshot APIs. Execute the
full attach → PARTIAL report → event baseline/overlap → missed poll → abnormal
disconnect → stale snapshot → generation-2 reconnect/gap/listed recovery flow.

Attempt stale-token/generation-1 and cross-owner ingress after generation 2 wins.
Recursively scan resource reports/snapshots, Observations, subscriptions,
diagnostics/audit queries, local logs, and raw durable blobs for the member-key
sentinel and its common encodings.

## Primary files

- `token-commune-adapter/tests/e2e.test.ts`
- `token-commune-adapter/tests/fixtures/conformance-gateway.ts`

## Acceptance evidence

- The actual core snapshot carries both PARTIAL view revisions and the expected
  current → stale/unknown → listed-current transitions.
- Latest-50 rollover is an explicit gap with no fabricated repair or count.
- Old token/generation/cross-owner evidence appends no resource or Observation
  state after the newer attachment is current.
- The gateway member key is absent from every listed external/durable sink; test
  failures identify the sink without printing the key.
- Synchronization uses committed LSN/state, fixed fixtures, and bounded waits;
  retries never mask a failed assertion.

## Ordering constraint

Depends on green phase-1 vectors. Phase-2 adversaries use this real process
fixture rather than inventing a parallel E2E harness.

## Implementation notes

- Added a reusable scripted local HTTP gateway fixture with exact endpoint/Authorization assertions and drove it through the real `0600` credential loader and HTTP gateway decoder.
- Expanded the serial E2E across the real Node adapter, generated RPC clients, Rust core, and SQLite: generation-1 mixed-success PARTIAL report; non-replayed baseline and one overlap event; all-endpoint missed poll; stale preservation; abnormal delivery-stream stop; generation-2 attach; 50-event unknown-continuity gap; listed-only current recovery; old-generation/token rejection; and cross-owner Observation rejection.
- The actual resource snapshot proves two PARTIAL view revisions, current→stale transitions, generation-2 listed-current/omitted-stale behavior, and no fabricated missed count. Failed old/cross-owner attempts append no ResourceState or non-registration Observation.
- Added a fail-closed gateway response-reflection boundary: a successful body containing any raw, bearer-derived, URL, base64, or JSON credential form is treated as `invalid-response` before decoding, so hostile upstream fields cannot enter a resource envelope.
- The E2E scans subscription/event output, materialized ResourceSnapshot bytes, diagnostic query output, local diagnostics, and raw SQLite bytes with the independent secret oracle. Only outbound HTTP Authorization contains the sentinel.
- Verification: focused real gateway/core E2E passed in 2.4s; full token-commune adapter suite passed 60/60, including both real-core tests.
