---
id: epic-token-commune-observer-conformance-phase-1-completeness-vectors
kind: story
stage: implementing
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
