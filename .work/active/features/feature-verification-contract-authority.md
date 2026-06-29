---
id: feature-verification-contract-authority
kind: feature
stage: drafting
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-persistence-snapshot-model, feature-security-threat-model, feature-research-contract-tooling]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define verification, contract, and authority order

The docs currently say prose, formal models, generated contracts, and conformance vectors all matter, but they do not define which artifact is authoritative when they disagree.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes architectural choices: the authority order among prose docs, formal models, IDL/schema, conformance vectors, and implementation (which artifact wins when they disagree) is a design decision. Generation targets and traceability rules are build-pipeline design. The v0 contract source is partially grounded by `feature-research-contract-tooling` but the authority-order question is not. The prose-author black-box test should have caught this originally.

## Scope

- Authority order among prose docs, TLA+/Quint models, Alloy models, IDL/schema, conformance vectors, and implementation.
- v0 contract source: Protobuf+Buf, JSON Schema, or explicit spike decision.
- Generation targets for Rust and TypeScript.
- Traceability from model properties to contract fields and conformance vectors.
- Model promotion criteria for v0.

## Acceptance criteria

- `docs/VERIFICATION.md` states artifact authority order and traceability rules.
- `docs/SPEC.md` states the v0 contract source or explicitly blocks durable protocol implementation until the spike resolves.
- `docs/PROTOCOL.md` distinguishes semantic authority from wire encoding authority.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
