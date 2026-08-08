---
id: epic-token-commune-observer-conformance-promotion-closeout
kind: story
stage: implementing
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: [epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Promote exact evidence and close through the verification deep lane

## Checkpoint

Promote the seven token-commune vectors only after phase-1 completeness,
real-core E2E, and phase-2 adversarial evidence all execute and every declared
mutation is exactly reported killed. Regenerate the existing conformance
traceability and implementation-evidence prose from the shared profile; record
paths/property ids/mutation ids without hand-maintained totals.

Run the project `[verification]` deep lane for every child and the integrated
feature at effective weight `thorough`: completeness convergence first,
adversarial convergence second. Reviewers attack vector field consumption,
reference-oracle independence, runner/count drift, key sink coverage, lost
terminalization, stale rendering, and surviving mutations. Findings are
proposals; the receiver verifies and dispositions each one.

## Primary files

- all seven `contracts/vectors/token-commune-*.json` files
- `contracts/scripts/check-vectors.mjs`
- `docs/VERIFICATION.md`
- all package runners/oracles/E2E tests from prior checkpoints

## Acceptance evidence

- The exact seven property/vector pairs have production scenario execution and
  exact mutation-kill reports; missing/unexpected evidence fails closed.
- Full adapter/core/server/operator-domain/web/contracts verification, clippy,
  generated drift/presentation/model checks, and `git diff --check` pass without
  skip, retry masking, weakened expectation, or hard-coded success.
- Assurance language is limited to promoted vector + implementation-checked;
  no model-checked, checked-normative, cross-adapter portability, or
  release-verified claim is made.
- Completeness and adversarial review phases each converge to no
  receiver-confirmed material current-cycle blocker before the feature advances.

## Ordering constraint

Final checkpoint. Depends on all completeness, E2E, security, failure, and
presentation evidence.
