---
id: feature-formal-model-seed
kind: feature
stage: drafting
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-verification-contract-authority]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Author seed formal models

Patchbay's verification posture requires checked models before implementation treats coordination semantics as product behavior. This feature creates the first normative model artifacts after the prose state machines and verification authority order are defined.

## Scope

- Author the first TLA+/Quint model for operator-intent delivery.
- Model accepted command durability, visible terminal/continuing states, timeout semantics, and retry/deduplication at the Patchbay boundary.
- Author initial Alloy relational invariants for identity uniqueness, authority graph constraints, and any lease properties that remain in v0 scope.
- Record model-promotion metadata: property checked, finite bounds/constants, tool invocation, expected pass/fail status, and product-semantics note.
- Document how model variables trace to protocol state-machine terms and future contract fields.

## Acceptance criteria

- `specs/` contains the seed TLA+/Quint model and Alloy model, or docs explicitly record why one of the two is deferred from v0.
- `docs/VERIFICATION.md` references the seed models and their promotion status.
- The models check the v0 command/session semantics defined by `feature-command-state-ssot` rather than inventing new terminology.
- A future implementation item can derive property/conformance-test obligations from the model artifacts.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
