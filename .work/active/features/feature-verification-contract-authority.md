---
id: feature-verification-contract-authority
kind: feature
stage: drafting
tags: [prose, verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-persistence-snapshot-model, feature-security-threat-model, feature-research-contract-tooling]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define verification, contract, and authority order

The docs currently say prose, formal models, generated contracts, and conformance vectors all matter, but they do not define which artifact is authoritative when they disagree.

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
