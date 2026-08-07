---
id: epic-token-commune-observer-polling-ingestion-disconnect-reconnect
kind: story
stage: done
tags: [adapter, protocol]
parent: epic-token-commune-observer-polling-ingestion
depends_on: [epic-token-commune-observer-polling-ingestion-dedup-gap]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune disconnect, stale, and reconnect composition

## Checkpoint

Supervise the existing held-open `ReceiveDeliveries` loop and the poller under
one process abort scope. External shutdown stops both; unexpected fatal child
exit aborts the sibling and rejects the process. Preserve the existing retryable
delivery-stream reconnect behavior and do not introduce a heartbeat or
poll-cadence liveness rule.

The core remains the stale-state authority: abnormal delivery-stream loss
stales owned resources. While core ingress is unavailable, no report/Observation
is accepted and event acknowledgement cannot advance. After reattachment, the
next accepted PARTIAL report restores only listed resources, then retained
latest-50 state reconciles overlap or emits a gap on rollover. A process restart
uses the fresh-baseline rule.

## Files

- `token-commune-adapter/src/main.ts`
- `token-commune-adapter/src/poller.ts`
- `token-commune-adapter/tests/main.test.ts`
- `token-commune-adapter/tests/poller.test.ts`

## Acceptance evidence

- Fake stream/core tests prove one attach, one delivery loop, one poller,
  coordinated abort/fatal propagation, and idempotent disposal without orphan
  waits/RPCs.
- Simulated outage emits no fabricated current/stale/liveness evidence and does
  not advance the latest-window tracker.
- Same-process overlap recovery emits only missed ids; rollover recovery emits
  gap then visible facts; restart suppresses pre-install replay.
- Existing core stream-drop degradation remains the authority; this adapter
  introduces no session connectivity/resource freshness state machine.

## Ordering constraint

Depends on tracker transaction semantics. Mutation evidence validates the
integrated honesty boundary after this composition exists.

## Implementation notes

`AdapterProcess` now supervises exactly one held-open delivery loop and one
poller under a shared child abort scope. External shutdown stops both; a fatal
child aborts its sibling and rejects. Retryable delivery reconnect remains the
core-owned stale signal—no adapter heartbeat, stale/current mutation, or
poll-cadence liveness state was added. Poll ingress failure accepts no event or
tracker acknowledgement; the next successful cycle sends a fresh PARTIAL report
before overlap/gap reconciliation. Fake process/core tests cover coordinated
abort, fatal propagation, outage, same-process recovery, and restart-baseline
behavior without wall-clock/network dependencies.
