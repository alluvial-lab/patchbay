---
id: epic-token-commune-observer-polling-ingestion-honesty-mutation-evidence
kind: story
stage: done
tags: [adapter, protocol, testing]
parent: epic-token-commune-observer-polling-ingestion
depends_on: [epic-token-commune-observer-polling-ingestion-disconnect-reconnect]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
---

# token-commune polling honesty mutation evidence

## Checkpoint

Complete the time/network-mocked interface and regression suite for every
honesty invariant in the parent design. Use fake gateway/core/clock/waiter ports;
expected kinds, schema refs, gap reasons, target identities, timestamps, and
traces must be independent fixtures rather than production registries reflected
back as their own oracle.

Execute, observe failing, and revert named production mutants for scheduling,
PARTIAL/source omission, timestamps, event coverage/scope, latest-50 baseline and
gap handling, acknowledgement-aware dedup, disconnect behavior, and
polling/no-cursor labels. Record commands/results in the parent implementation
summary. This checkpoint does not promote conformance vectors or duplicate the
downstream real-core conformance feature.

## Files

- `token-commune-adapter/tests/poller.test.ts`
- `token-commune-adapter/tests/event_observation.test.ts`
- `token-commune-adapter/tests/event_window.test.ts`
- `token-commune-adapter/tests/main.test.ts`
- `token-commune-adapter/tests/gateway_client.test.ts`

## Acceptance evidence

- Overlap/backoff/cache/PARTIAL mutants fail.
- Initial replay/gap suppression/pre-ack dedup/reconnect-reset/missed-count
  mutants fail.
- Poll-time substitution, declared-only-kind mapping, wrong target scope, EVENT
  instead of STATUS, and honesty-label removal mutants fail.
- Fabricated liveness/stale/current evidence or tracker advancement during
  failed ingress mutants fail.
- Strict build, full package tests, and `git diff --check` pass after every
  mutant is restored.

## Ordering constraint

Final checkpoint: it depends on the complete supervised runtime. Child stories
advance directly to done only with green evidence; thorough review occurs at the
integrated parent feature boundary.

## Implementation notes

The deterministic suite now has 55 package tests. Independent fixtures protect
polling/non-overlap and Retry-After, PARTIAL/source omission, refresh versus
source timestamps, exact five-kind STATUS/resource mapping, declared-only
handling, initial/no-overlap gap honesty, acknowledgement-aware dedup, and
no-liveness reconnect behavior.

Executed and reverted three production mutants:

- `PARTIAL -> AUTHORITATIVE` in snapshot projection: focused poller tests exited
  1 with two PARTIAL-honesty failures.
- event-id acknowledgement moved before core ingress: focused poller tests exited
  1 at the pre-ack retry assertion.
- `calibration` removed from the declared-only branch: focused mapper tests
  exited 1 at declared-only coverage.

After restoration, `npm run build`, all 55 `npm test` cases,
`cargo test -p patchbay-core --test acceptance_observation`, and
`git diff --check` pass. `cargo test -p patchbay-core --tests` also passes; the
repository-wide doctest lane remains separately broken by its pre-existing
rustdoc dependency-resolution failure and is not part of the package gate.
