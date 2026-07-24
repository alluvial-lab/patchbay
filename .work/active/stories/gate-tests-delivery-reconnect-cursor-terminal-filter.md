---
id: gate-tests-delivery-reconnect-cursor-terminal-filter
kind: story
stage: implementing
tags: [testing, adapter, protocol]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: tests
created: 2026-07-24
updated: 2026-07-24
---

# Test reconnect catch-up preserves delivery eligibility

## Priority
High

## Value evidence
Item: `feature-adapter-staleness-liveness`

Contract / risk / regression: the long-lived `ReceiveDeliveries` subscription must advance the adapter cursor only after the durable acknowledgement, and on a broken stream/reconnect it must catch up unseen eligible work without re-offering a command that became running or terminal. This protects the release's delivery exactly-once-at-the-boundary semantics: a terminal command must not execute again after reconnect, while a later accepted command must not be skipped.

The current tests separately prove idle-tail delivery (`server/src/adapter_service/tests.rs:529`), delivered-but-not-running redelivery (`server/src/adapter_service/tests.rs:402`), and running-loss reconciliation (`server/src/adapter_service/tests.rs:594`). The adapter updates `#cursor` only after `#beginDelivery` acknowledges (`pi-adapter/src/main.ts:179-182`). None drives a stream failure/reconnect through a non-zero cursor with both (a) a previously delivered command that reaches a terminal state and (b) a later accepted operation. The feature review explicitly records that the old integrated acknowledged-history assertion was dropped.

## Gap type
Missing reconnect e2e seam / cursor-and-ack state-transition coverage.

## Suggested test
```ts
// Start the real adapter stream and deliver command A; acknowledge and finish A.
// Break the stream, then append/accept command B before reconnecting.
// Reconnect using the adapter's advanced cursor and assert:
//   - A is never delivered or executed again because it is terminal;
//   - B is delivered exactly once and reaches its expected lifecycle;
//   - the reconnect cursor covers the acknowledged prefix without skipping B.
// Include the delivery-ack-before-cursor boundary (ack succeeds before the
// stream fails) so this is a regression for the long-lived-stream conversion.
```

## Test location (suggested)
`pi-adapter/tests/e2e.test.ts`, with a focused server-level counterpart in `server/src/adapter_service/tests.rs` if needed to isolate post-batch eligibility filtering.
