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
updated: 2026-07-11
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

## Design input from `verification-claim-correction` review

The deep-review convergence loop of `epic-public-product-contract-verification-claim-correction` (rounds 1–3) surfaced a systematic model-architecture question this feature must resolve when it builds the real formal checker:

- **Trace-fidelity / independent-oracle question.** Several seed-model properties have invariants that inspect state written by the *same action* that decides acceptance, rather than immutable attempted-evidence state. A mutation that accepts arbitrary inputs while recording the expected values can pass such an invariant. The CSRF models were fixed to inspect attempted evidence (`story-fix-csrf-trace-and-ssot-drift`); the authority/subscription models were not, and 4 such properties were demoted in `verification-claim-correction` Unit 7. The round-3 review claimed 9 *surviving* promoted properties (6 Elicitation, 2 subscription, `TypedCorrelation`) share the defect; the host partially disputed this (an independent mutation test of `ElicitationPendingFinality` was caught by the property, contradicting the reviewer's "passed" claim), so the question is open and per-property.
- **What this feature owns.** When this feature reaches design, it must: mutation-test each surviving promoted property individually; demote, narrow, or re-architect per the result; and apply the attempted-evidence discipline uniformly across all server-side-acceptance models. This is the "run the real formal checker" work the brief already names — the independent-oracle question is part of what makes a checker "real" rather than metadata.
- **Parked context.** `idea-csrf-trace-fidelity` (backlog) carries the full pattern description and the round-3 dispute record. The 15 properties already demoted by `verification-claim-correction` carry `<TBD>` invocations naming this feature as the owner of their real formulas.

This is recorded as a design input, not a dependency blocker: `verification-claim-correction` closes with the 15 confirmed demotions and the systematic question routed here.
