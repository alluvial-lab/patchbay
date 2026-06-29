---
id: research-handoff-protocol-contract-tooling-2
kind: feature
stage: drafting
tags: [protocol, verification, testing]
parent: null
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_origin: protocol-contract-tooling
---

# Add protocol conformance vector corpus and cross-language tests

Patchbay should define a transport-neutral protocol conformance vector corpus and require both Rust core code and TypeScript operator-domain code to consume the same examples.

The vectors should cover command lifecycle, snapshots, authority outcomes, failure vocabulary, idempotent retry, target generation, and replay/reconnect semantics.

## Research grounding

**Source**: `.research/analysis/briefs/protocol-contract-tooling.md` (slug: `protocol-contract-tooling`)

The research separates schema shape from product semantics and recommends shared conformance vectors as the executable bridge between generated Rust/TypeScript contracts and Patchbay's protocol rules.
