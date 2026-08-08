---
id: epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries
kind: story
stage: implementing
tags: [adapter, verification, ux]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-phase-2-security-adversaries]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Phase 2: failure-terminalization and presentation adversaries

## Checkpoint

Implement and execute
`token-commune-unsupported-operation-terminalization` and
`token-commune-cockpit-presentation-honesty`. Exercise both a retryable
unsupported terminal-report failure and hard adapter-process replacement after
durable delivery acknowledgement. Prove pending or core-redelivered nonterminal
work completes exactly once before later work. Bind the exact adapter projection
fixture to the real operator-domain decoder/verdict and option-7 DOM renderer.

Attack stale dominance, unknown anchoring, exact provider-local runnable
evidence, contributor/member/subkey exclusion, `gpt-5.6` rejection, Patchbay
verdict ownership, and the no-dynamic-renderer boundary with hostile projection
fields.

## Primary files

- `contracts/vectors/token-commune-unsupported-operation-terminalization.json`
- `contracts/vectors/token-commune-cockpit-presentation-honesty.json`
- `token-commune-adapter/tests/conformance-vectors.test.ts`
- `token-commune-adapter/tests/e2e.test.ts`
- `web-cockpit/tests/conformance-vectors.test.ts`
- focused operator-domain/panel tests

## Acceptance evidence

- For both the same-process retry and replacement-process path, core history has
  exactly one durable delivered transition before exactly one failed/unsupported
  terminal transition, with no completion, duplicate durable ack, or stranded
  accepted/delivered command after recovery.
- Current/stale/unknown/invalid rows render honestly from the shared promoted
  fixture; stale is not live/runnable and unknown does not disappear.
- Cross-provider models, contributor/member/subkey data, the removed alias,
  adapter-claimed verdicts, renderer URLs, HTML, and scripts cannot create a
  positive verdict or reach/execute in the DOM.
- Clearing pending terminalization, changing the failure/state, duplicating an
  ack, ignoring freshness, dropping unknown, weakening model join, or accepting
  hostile renderer fields kills a named mutation witness.

## Ordering constraint

Depends on the security adversary checkpoint. Promotion waits for this entire
adversarial phase to converge.
