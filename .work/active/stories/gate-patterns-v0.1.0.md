---
id: gate-patterns-v0.1.0
kind: story
stage: done
tags: [patterns]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: patterns
created: 2026-07-24
updated: 2026-07-24
---

# Patterns extracted for v0.1.0

## New patterns codified
- `domain-owned-ports` — Narrow consumer-owned interfaces separate core behavior from time, storage, verified ingress, and sibling projections. (7 port interfaces across acceptance, storage, and authority.)
- `generated-protobuf-contracts` — Protobuf schema generates checked-in Rust and TypeScript boundary artifacts through Buf and language build workflows. (3 generation/drift workflow occurrences.)
- `registry-derived-protocol-boundaries` — Generated operation and state enums are parsed, constrained, and dispatched at acceptance, adapter, and server boundaries. (3+ receiving-boundary occurrences.)
- `fail-fast-boundary-validation` — RPC, adapter, and acceptance ingress reject malformed framing and invalid/missing values before stateful work. (4 validated ingress boundaries.)
- `durable-log-projections` — Command, authority, and session views reconstruct by validated LSN-ordered folds of the authority-domain log. (3 projection replay folds.)
- `presentation-registry-conformance` — Operation, connectivity, activity, and Elicitation registries bind to CSS and showcase primitives through a schema-parity checker. (4 registry binding entries.)

## Inconsistencies flagged

None. No existing pattern catalog was present, and the audited bundle contains no divergence against a prior documented pattern.

## Pattern files written
- `.agents/skills/patterns/domain-owned-ports.md`
- `.agents/skills/patterns/generated-protobuf-contracts.md`
- `.agents/skills/patterns/registry-derived-protocol-boundaries.md`
- `.agents/skills/patterns/fail-fast-boundary-validation.md`
- `.agents/skills/patterns/durable-log-projections.md`
- `.agents/skills/patterns/presentation-registry-conformance.md`
- `.agents/skills/patterns/SKILL.md` (new index)
- `.agents/rules/patterns.md` (generated hook-loaded digest)

## Gate notes

The host did not expose a generic scanner-subagent tool, so discovery used an inline source-read-only pass. No release-orchestration item was included in the bundle scope.
