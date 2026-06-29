---
id: feature-research-contract-tooling
kind: feature
stage: done
tags: [research, protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_dials:
  scope_authority: pre-registered
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: Protocol contract and schema source of truth

Research the best v0 contract source for Patchbay's Rust core and shared TypeScript operator domain.

## Engagement record

Completed: 2026-06-28

Decision relevance: choose Patchbay's v0 contract/schema source of truth and generation strategy before implementing protocol/domain boundaries.

Settled dials:

- `scope_authority`: `pre-registered`
- `verification_rigor`: `standard`
- `intent`: `inform-architecture-decision`
- `output_kind`: `synthesis-brief`

Decomposition used:

- Protobuf + Buf ecosystem for Rust/TypeScript contracts and breaking-change checks.
- JSON Schema / OpenAPI / TypeSpec-style contract sources for JSON-native APIs.
- TypeScript-first schema tools such as Zod/TypeBox and Rust/TypeScript generation bridges.
- Conformance-vector and versioning strategy across Rust core + TypeScript operator domain.

Outputs:

- Synthesis brief: `.research/analysis/briefs/protocol-contract-tooling.md`
- Verification checklist: `.research/analysis/briefs/protocol-contract-tooling-verification.md`
- Source attestations: `.research/attestation/{buf-generate,buf-breaking,protobuf-es,prost,protojson,json-schema-core,typebox,zod-json-schema,typespec,typespec-protobuf,openapi-generator-rust,openapi-generator-typescript,connectrpc}.md`

Gate outcomes:

- Citation lint: 59 resolved/non-broken citations, 0 broken, 0 thin, 0 pattern flags (`--no-url-check` used to avoid environment URL-probe noise after direct source fetches).
- Adversarial-read: `APPROVED`.
- Spot-check: completed by lead; no required revisions after adversarial review.
- Acquisition candidates: none.

## Seed questions

- Should Patchbay use Protobuf+Buf, JSON Schema, TypeBox/Zod, or another contract source for v0?
- How should semantic models, wire contracts, generated Rust/TypeScript types, and conformance vectors relate?
- What tooling best supports generated contracts without hand-copied DTOs?
- What is the migration cost if future Expo/mobile or additional adapters need generated clients?

## Expected output

A `.research/analysis/briefs/` synthesis brief with source-grounded recommendation and implementation notes. Follow-up work items may be emitted only after operator confirmation.
