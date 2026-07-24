---
id: feature-adapter-staleness-liveness-pi-delivery-loop
kind: story
stage: done
tags: [adapter]
parent: feature-adapter-staleness-liveness
depends_on: [feature-adapter-staleness-liveness-core-delivery-subscription]
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: consume the Pi delivery stream continuously

After the core long-lived stream exists, replace the production
`pollOnce()`/100ms-delay loop in `pi-adapter/src/main.ts` with a
cancellation-aware continuous stream consumer. Extend
`pi-adapter/src/core_client.ts` so `ReceiveDeliveries` accepts an optional
`AbortSignal` and retain the existing one-time unauthenticated re-attach retry.

Keep acknowledgement before execution, instruction concurrency (so a later
cancel can arrive), transcript observation ordering, and the cursor update after
each received delivery. Convert batch-oriented integration tests in
`pi-adapter/tests/e2e.test.ts` to bounded run-loop fixtures. Add the
restart-mid-turn regression and roll `docs/RUNBOOK.md` forward: the stale/live
and permanent-running limitations are resolved, while
`execution_outcome_unknown` remains an honest retry-safety warning.

## Acceptance evidence

- The live Pi adapter holds one authenticated delivery stream while idle and
  no longer issues an external 100ms polling loop.
- Operations accepted after stream establishment execute through the existing
  acknowledgement/running/terminal lifecycle with ordered observations.
- Stopping/restarting the adapter during a delayed instruction produces stale
  session state and a terminal failed command carrying
  `execution_outcome_unknown`, never permanent running state.
- Abort/dispose and fenced re-attach leave no unhandled async failure; focused
  Pi adapter tests pass.

## Completion evidence

- Replaced the external 100ms `pollOnce()` loop with one cancellation-aware continuous delivery consumer while preserving acknowledgement-before-execution, instruction concurrency, cursor advancement, and ordered observation flushing.
- Added transport retry and one-time unauthenticated attachment refresh behavior around the long-lived stream; abort and dispose share the stream cancellation path.
- Converted the Pi integration suite to durable-state waits and added the restart-mid-turn regression proving stale presentation plus `failed(execution_outcome_unknown)`.
- Updated `docs/RUNBOOK.md` to remove the resolved polling/running-rot limitations while retaining the transport-black-hole and retry-ambiguity warnings.
- Verified with `cd pi-adapter && npm test` (10 passed); the package defines build and test scripts but no separate lint script.
