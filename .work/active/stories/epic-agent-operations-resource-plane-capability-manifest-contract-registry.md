---
id: epic-agent-operations-resource-plane-capability-manifest-contract-registry
kind: story
stage: implementing
tags: [protocol, adapter]
parent: epic-agent-operations-resource-plane-capability-manifest
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Generate the target-category and resource projection contract

## Design checkpoint

Land Unit 1 from the parent design in `contracts/proto/patchbay/adapter.proto`:
the closed `AdapterTargetCategory` registry, reserved wire-present
`KNOWLEDGE_BUNDLE`/OKF-v0.2 seam, typed schema descriptors, per-resource
snapshot/projection declarations, and tag-preserving rename of adapter-wide
`snapshot_support` to `session_snapshot_support`. Regenerate the committed Rust
and TypeScript artifacts from the proto; do not hand-edit generated output.

`ResourceKind` remains the open adapter-owned identity component from
`resource-identity`. `provider_pool` and `usage_window` are declaration/test
examples beneath `OPERATIONAL_RESOURCE`, not core enum members. This checkpoint
does not validate registration, admit reports, render resources, or add a plugin
loader.

## Acceptance evidence

- Rust and TypeScript generated types agree on field identities, enum values,
  schema descriptors, projection contracts, and per-resource snapshot tiers.
- Existing tag 4 bytes remain decodable while repository callers migrate to the
  explicit session-scoped generated field name.
- Target-category values are exactly unspecified/runtime-session/operational-
  resource/knowledge-bundle; the knowledge-bundle comment names OKF v0.2 and
  states that admission remains reserved.
- Contract generation, Rust/TypeScript builds, Buf lint, and generated-drift
  checks pass.

## Ordering constraint

This is the contract source for the core admission checkpoint. Do not begin
manifest validation against handwritten stand-ins before this story lands.
