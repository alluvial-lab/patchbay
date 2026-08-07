---
id: epic-token-commune-observer-polling-ingestion-poll-runtime
kind: story
stage: implementing
tags: [adapter, protocol]
parent: epic-token-commune-observer-polling-ingestion
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune non-overlapping poll runtime

## Checkpoint

Add `token-commune-adapter/src/poller.ts` with the injected `PollClock`,
`PollWaiter`, `PollerCoreSink`, `TokenCommunePollerOptions`, and
`TokenCommunePoller` interfaces specified in the parent design. Run one
immediate cycle, then schedule completion-to-start delays without overlap.
Concurrently settle the five snapshot endpoints and `/commune/events` under one
abort signal. Expected endpoint failures become explicit unavailable evidence;
never reuse a prior response.

Keep cadence named and implemented as polling. The configured interval is the
minimum delay; normalized upstream retry advice may extend it but never shorten
it. No stream/webhook abstraction, wall-clock sleep in tests, or adapter-side
source cache belongs in this checkpoint.

## Files

- `token-commune-adapter/src/poller.ts`
- `token-commune-adapter/tests/poller.test.ts`

## Acceptance evidence

- A fake clock/waiter proves immediate first poll, no concurrent cycle, and the
  configured delay measured after completion.
- All six gateway methods receive the cycle signal and settle independently;
  one failed endpoint cannot substitute its preceding value.
- Valid retry advice extends the delay; invalid advice cannot cause a hot loop.
- Abort stops fetch/wait promptly and leaves no timer or pending cycle.

## Ordering constraint

This establishes the time/network boundary consumed by every later checkpoint.
Report ingress, event mapping, and tracker state must not be folded into this
scheduler before their dependent checkpoints land.
