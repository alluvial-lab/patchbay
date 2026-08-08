---
id: epic-token-commune-observer-conformance-phase-1-completeness-vectors
kind: story
stage: done
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-harness-registry-guards]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Phase 1: completeness vectors for honest adapter behavior

## Checkpoint

Implement and execute the completeness phase:
`token-commune-partial-snapshot-honesty`,
`token-commune-bounded-reconnect-honesty`, and
`token-commune-degradation-honesty`. Drive the real projector, poller, latest-50
tracker, and core-sink seams from vector fields. Judge them with raw-input
reference oracles that do not import the production classifier/projector helpers.

Each vector must execute every declared mutation and prove the same oracle that
accepts production rejects PARTIAL overclaim, cached-source reuse, initial
history replay, pre-ack dedup, hidden no-anchor gaps, missing empty reports,
disappearing unknown pools, and fabricated liveness/current state.

## Primary files

- `contracts/vectors/token-commune-partial-snapshot-honesty.json`
- `contracts/vectors/token-commune-bounded-reconnect-honesty.json`
- `contracts/vectors/token-commune-degradation-honesty.json`
- `token-commune-adapter/tests/conformance-vectors.test.ts`
- `token-commune-adapter/tests/conformance-oracles.ts`
- focused projector/poller/window tests as needed

## Acceptance evidence

- Both exact resource views remain snapshot-mode PARTIAL; failed source slices
  are unavailable/not-reported, omissions are non-terminal, and no aggregate is
  synthesized.
- Initial baseline, overlap, acknowledgement retry, saturated rollover, missed
  poll, disconnect, and listed-only reconnect traces equal the independent
  sequence/set model.
- Report-before-event ordering, gap reason, unknown continuity, and absence of a
  missed count/liveness assertion are observed from production.
- Every declared mutant is killed and exactly reported to the umbrella checker.

## Ordering constraint

Depends on the shared harness/registry guard checkpoint. The real-core E2E binds
these deterministic scenarios to the process boundary next.

## Implementation notes

- Implemented the three raw-input completeness vectors in the shared corpus and executed them through the real snapshot projector, `LatestEventWindowTracker`, and `TokenCommunePoller` seams.
- The independent oracle module owns literal two-view/PARTIAL rules, a set/sequence latest-50 model, and the reported→missed→disconnected→reconnected confidence truth table. It imports no projector, event-window classifier, poller, decoder, or presentation helper.
- Production observations are derived from vector fields. The reconnect witness covers initial non-replay, unacknowledged retry, post-ack dedup, a saturated no-anchor gap, no missed-count fabrication, and report-before-event ordering. The failed-poll witness executes all six failed gateway endpoints and still observes two empty PARTIAL views.
- Exact mutation execution killed all 15 declared phase-1 witnesses: PARTIAL overclaim, dropped view, prior-source reuse, zero coercion, aggregate synthesis, initial replay, pre-ack dedup, hidden gap, fabricated missed count, event-before-report, skipped empty report, prior endpoint carry, current-on-disconnect, polling liveness, and omitted-identity promotion.
- Verification: token adapter build passed; the package runner reported the three exact scenario ids and 15 exact mutation ids; the full token-adapter suite was green at 58 tests (including the existing real-core smoke path).
- **Pass-2 correction (2026-08-08, `b0605a9`):** the 15-witness completion claim above is superseded. The current vector declarations retain 12 genuine kills: 5 partial-snapshot, 5 bounded-reconnect, and 2 degradation witnesses.
