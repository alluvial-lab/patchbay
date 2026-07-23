---
id: backlog-adapter-staleness-full-coverage
created: 2026-07-23
tags: [security, protocol, fast-follower]
research_origin: null
---

# Backlog: full-coverage adapter staleness (heartbeat / last-report-age, or long-poll delivery)

Surfaced by the epic-v0-1-0-implementation maximum review, pass 3 (convergence,
2026-07-23), as Important finding P3-I1 — parked, not a v0.1.0 blocker.

## The gap

The epic pass-2 B3b fix marks an adapter's sessions `stale` on an abnormal
delivery-stream drop (operator decision Q2a: connection-liveness signal). The
mechanism is genuine and tested — but the v0.1.0 delivery model is a polling
fallback: the stream drains the durable tail and completes in milliseconds per
~100ms poll, and command execution happens after stream completion. So the
staleness signal only fires for deaths during an active stream drain. An
adapter that dies **between polls or mid-execution** (the majority of real
deaths) leaves its sessions presented as `live/working` until the adapter
restarts. Demonstrated by the pass-3 reviewer's probe test.

## Why parked

- The mechanism implements exactly what operator decision Q2a scoped.
- Commands are never lost (epic pass-2 B3a redelivery, verified).
- A replacement adapter process cannot compound the confusion (epic pass-2 B2
  fencing token, verified).
- The residual is presentational honesty in a single-operator deployment;
  natural recovery (restart the adapter) restores truth via re-attach + live
  reports.

## Fast-follower shape (two options, pick at design time)

1. **Heartbeat / last-report-age staleness:** the core tracks each adapter's
   last report time; a background sweep marks sessions stale after a threshold
   with no report. Covers all death modes; requires a core-side timer.
2. **Long-poll the delivery stream:** hold `ReceiveDeliveries` open until new
   events or a timeout, so the existing (already-tested) disconnect hook spans
   the adapter's lifetime. Moderate change confined to `receive_deliveries` +
   the adapter poll loop; turns the polling fallback into a long-poll.

Also fold in P3-N1 (commands rot at `running` after mid-execution death — the
documented Q1a bound) and P3-N2 (per-poll full-log command rebuild — perf note
for when the log grows). Documented limitation currently lives in
`docs/RUNBOOK.md` § Known v0.1.0 limitations.
