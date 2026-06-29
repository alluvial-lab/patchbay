---
id: research-handoff-protocol-contract-tooling-1
kind: feature
stage: drafting
tags: [protocol, verification]
parent: null
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_origin: protocol-contract-tooling
---

# Establish Protobuf+Buf contract generation pipeline

Patchbay should establish Protobuf schemas managed by Buf as the v0 boundary-contract source for durable protocol messages, command/event payloads, and shared enum vocabularies across the Rust coordination core and TypeScript operator domain.

This should include initial contract layout, Buf configuration, Rust generation via a prost-based path, TypeScript generation via Protobuf-ES, and generation drift checks.

## Research grounding

**Source**: `.research/analysis/briefs/protocol-contract-tooling.md` (slug: `protocol-contract-tooling`)

The research recommends Protobuf + Buf as Patchbay's first protocol/boundary contract source and describes the Rust/TypeScript generation path and CI checks needed to avoid hand-copied DTOs.
