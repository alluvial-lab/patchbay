---
id: epic-token-commune-observer-polling-ingestion-dedup-gap
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-polling-ingestion
depends_on: [epic-token-commune-observer-polling-ingestion-event-observation-map]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune latest-50 dedup and gap reconciliation

## Checkpoint

Implement `LatestEventWindowTracker` from the parent design as deterministic,
bounded, in-memory state. A first successful page becomes a non-replayed
baseline and emits an initial boundary status per current provider-pool target.
Consecutive overlapping pages emit only newly visible acknowledged ids.
Empty-to-short is continuous; empty-to-50 and every other non-overlap transition
emit an explicit gap before currently visible facts.

Tracker transitions are acknowledgement-aware: acknowledge a gap/event only
after core acceptance and commit the page only after all planned outputs finish.
A partial RPC failure retries only unfinished output. State covers repeated
polls and same-process reconnect; it does not claim process-restart exactly-once
or history older than the source window.

## Files

- `token-commune-adapter/src/event_window.ts`
- `token-commune-adapter/src/poller.ts`
- `token-commune-adapter/tests/event_window.test.ts`
- `token-commune-adapter/tests/poller.test.ts`

## Acceptance evidence

- Initial pages emit no historical PoolEvent Observations and exactly one
  deduplicated baseline gap for each current target.
- Overlap, repeated pages, same-timestamp ids, empty/short/full transitions,
  rollover, and history-becomes-empty discontinuity have independent expected
  traces.
- Failed event/gap ingress does not advance acknowledgement; retry does not
  duplicate already accepted output.
- Tracker storage remains bounded by latest-window evidence and emits no missed
  count, cursor, replay, or source-order claim.

## Ordering constraint

Depends on the stable event mapper. Process supervision must preserve this
tracker across same-process core reconnect and reset it only on process start.
