---
id: adapter-report-source-ordering-core-fence
kind: story
stage: implementing
tags: [protocol, storage]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-contract-foundation]
release_binding: null
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
