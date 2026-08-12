---
id: epic-token-commune-observer-conformance-harness-registry-guards
kind: story
stage: done
tags: [adapter, verification]
parent: epic-token-commune-observer-conformance
depends_on: []
release_binding: v0.2.0
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, high reasoning, explicitly selected by the autopilot caller for the cross-package verification boundary. One owning worker retained the full six-story chain; no sub-worker fan-out was used because the profile, mutation ledger, and generated traceability share one write boundary.
- Added the exact seven-entry token-commune profile and `token-commune-adapter` runner to the existing checker. The profile fails closed on missing/extra vector-property pairs, exact scenario-registration drift, and partial promotion.
- Added additive `mutation_witnesses` envelope validation and exact per-runner `PATCHBAY_CONFORMANCE_MUTATION_KILLED` accounting. Missing, duplicate, unexpected, or unreported kills block generated-doc writes.
- Added property-specific static expected-outcome guards for every `TokenCommune*` property and kept the existing resource/session runner accounting intact.
- Added the shared independent oracle module, package runner scaffold, and seven draft profile vectors so later checkpoints fill one corpus rather than introducing a parallel harness.
- Verification: token adapter TypeScript build passed; checker syntax passed; `check:vectors` passed with the existing 8 promoted vectors / 11 implementation checks and no token execution while the exact profile remains draft; `check:models` passed with 68 registered property ids and generated traceability current; `git diff --check` passed.
