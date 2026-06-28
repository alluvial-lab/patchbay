---
id: feature-research-contract-tooling
kind: feature
stage: drafting
tags: [research, protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
research_dials:
  scope_authority: pre-registered
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: Protocol contract and schema source of truth

Research the best v0 contract source for Patchbay's Rust core and shared TypeScript operator domain.

## Seed questions

- Should Patchbay use Protobuf+Buf, JSON Schema, TypeBox/Zod, or another contract source for v0?
- How should semantic models, wire contracts, generated Rust/TypeScript types, and conformance vectors relate?
- What tooling best supports generated contracts without hand-copied DTOs?
- What is the migration cost if future Expo/mobile or additional adapters need generated clients?

## Expected output

A `.research/analysis/briefs/` synthesis brief with source-grounded recommendation and implementation notes. Follow-up work items may be emitted only after operator confirmation.
