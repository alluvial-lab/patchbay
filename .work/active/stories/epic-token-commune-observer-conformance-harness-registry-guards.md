---
id: epic-token-commune-observer-conformance-harness-registry-guards
kind: story
stage: implementing
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Extend the shared conformance profile with exact mutation accounting

## Checkpoint

Extend the existing `contracts/vectors/` checker and exact package-runner
protocol with the token-commune adapter profile. Register the seven exact
`TokenCommune*` property/vector pairs, add the token adapter runner, and make
`mutation_witnesses` a mandatory non-empty promotion field for this profile.
Require exact scenario execution ids and exact killed-mutation ids before the
existing generated traceability block can change.

This is execution plumbing and self-validation only. It does not promote the
vectors, add a token-only corpus, or claim model evidence.

## Primary files

- `contracts/vectors/README.md`
- `contracts/scripts/check-vectors.mjs`
- `docs/VERIFICATION.md`
- `token-commune-adapter/package.json`
- `token-commune-adapter/tests/conformance-vectors.test.ts`
- `token-commune-adapter/tests/conformance-oracles.ts`

## Acceptance evidence

- Missing/renamed/extra token vectors or properties, duplicate cases/mutations,
  unknown runners, and incomplete execution reports fail closed.
- The checker compares exact `PATCHBAY_CONFORMANCE_EXECUTED` and
  `PATCHBAY_CONFORMANCE_MUTATION_KILLED` sets and does not regenerate docs on any
  failure.
- Static expected-outcome checkers are property-specific; no generic truthy or
  exit-zero promotion path exists.
- Profile counts and generated evidence derive from the registry/vector data,
  not hand-maintained prose numbers.
- Existing promoted resource vectors, dual session/resource cases, and package
  runners remain green.

## Ordering constraint

Root checkpoint. Phase-1 vectors register only through this bridge.
