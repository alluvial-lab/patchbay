---
id: story-protocol-idl-traceability-script
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-protocol-idl-and-conformance
depends_on: [story-protocol-idl-conformance-vectors, story-protocol-idl-proto-package]
created: 2026-07-06
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Story: CI traceability script + VERIFICATION.md reference

Implements Unit 4 of `feature-protocol-idl-and-conformance`.

## Scope

Author the CI script (`contracts/scripts/check-vectors.ts` or `.rs`/`.py`) that reads all `contracts/vectors/*.json` and: (a) fails if a checked-model property lacks a promoted vector; (b) fails if a vector references a missing/misspelled property; (c) fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (surfaced contradiction per Q3 of `feature-verification-contract-authority`); (d) generates the `docs/VERIFICATION.md` traceability table as a checked-in artifact. Wire it as a CI-runnable script. Update `docs/VERIFICATION.md` to reference the vectors location and the generated traceability table.

See the feature body's Unit 4 for the file list and acceptance criteria.

## Acceptance criteria

- [ ] Script runs and validates all vectors against the model property list.
- [ ] Script generates a traceability table artifact.
- [ ] `docs/VERIFICATION.md` references `contracts/vectors/` and the traceability table.
- [ ] Script fails on a deliberately-broken vector (negative test).

## Review (2026-07-06)

**Verdict**: Approve (fast-lane via feature review)

**Notes**: Reviewed as part of the feature-protocol-idl-and-conformance deep-lane review (gpt-5.5 fresh context). Initial review returned Request changes (3 important findings: failure-vector operation_state contradiction, reply-correlation mis-typing, missing drift check); all fixed in commit 9a2854f; targeted re-review returned READY. Builds pass (cargo build, npm run build); check-vectors.mjs passes (12 vectors); check:drift detects generated-code modifications. Story advanced implementing → review; rolled up to feature.
