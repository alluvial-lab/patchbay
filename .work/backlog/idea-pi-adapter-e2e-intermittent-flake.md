---
id: idea-pi-adapter-e2e-intermittent-flake
tags: [testing, flake]
created: 2026-07-26
---

# pi-adapter e2e intermittent cancellation/timing flake

`pi-adapter/tests/e2e.test.ts` ("core → adapter → real AgentSession →
observation loop, generation bump, reconnect, and core restart") fails
intermittently under the full parallel `npm test` run — observed by three
independent workers and the orchestrator during
`epic-observability-dogfooding` (failure shapes: assertion on
cancellation/timing; once `acceptedLsn` timing-adjacent). Isolated/serial
runs pass consistently, as do back-to-back full runs most of the time.

Likely a real timing assumption (fixed sleeps/polling windows) rather than
product breakage, but unproven. Fix: make the e2e's waits condition-based
(eventually-consistent polling with deadline) instead of fixed delays, or
serialize the e2e from the unit tests. Related: the v0.1.0 incident that
leaked 201K SQLite temp files into /tmp (backlog-test-tempfile-hygiene) —
check whether temp-file contention correlates with the flake.

## Resolution

The focused failure was test-side: the intentional abort of the replacement
`receiveDeliveries` stream correctly produced Connect's `Code.Canceled`, but
the test awaited that rejected promise as though normal completion were
expected. The replacement-stream checkpoint is synchronized through the existing
condition-based durable command-state waits rather than fixed sleeps, and the
test now asserts the expected cancellation explicitly with `assert.rejects`.
No production abort propagation bug was found; diagnostics RPC cancellation
remains scoped to the forwarder's own request signal.
