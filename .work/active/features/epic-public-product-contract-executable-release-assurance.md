---
id: epic-public-product-contract-executable-release-assurance
kind: feature
stage: drafting
tags: [verification, protocol]
parent: epic-public-product-contract
depends_on: [epic-public-product-contract-public-compatibility, epic-public-product-contract-self-hosted-operations, epic-public-product-contract-adapter-portability-proof, epic-public-product-contract-verification-claim-correction]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Executable release assurance

## Brief

Turn the property-graded v1 assurance policy into an executable release decision. CI and the release process must run the real formal checker where a property is formally gated, execute conformance scenarios against running product code, and report whether each public safety claim is specified, model-checked, implementation-checked, or release-verified. Invocation strings, metadata envelopes, generated traceability tables, and fixture validation are useful inputs but never sufficient behavioral evidence.

The formal release gate covers exactly the four committed concurrency/recovery kernels: command terminal races, session-generation isolation, crash/replay/snapshot convergence, and multi-surface Elicitation races. Every other public safety claim still needs implementation-backed evidence at its appropriate grade. The feature consumes corrected property identities, the stable public contract, tested self-hosting/recovery behavior, and adapter conformance evidence. It may scaffold runners earlier, but it cannot honestly complete until executable core, control-surface, persistence, and adapter components exist.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: final assurance integrator after the public contract, operational path, adapter proof, and verification-claim correction.
- The dependency chain is intentionally honest: no metadata-only shortcut may advance this feature while the running implementation is absent.

## Foundation references

- `docs/VERIFICATION.md` — v1 release assurance policy and four formal gates
- `docs/SPEC.md` — public safety and compatibility commitments
- `contracts/vectors/` — current draft executable examples
- `contracts/scripts/check-models.mjs` — current model metadata/traceability machinery
- `contracts/scripts/check-vectors.mjs` — current vector metadata/traceability machinery
