---
id: epic-agent-operations-resource-plane-resource-identity-typed-resource-identity
kind: story
stage: implementing
tags: [foundation, protocol]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
---

# Define typed operational-resource identity

## Checkpoint

Land Unit 1 from the parent design: generated `ResourceId`, open typed
`ResourceKind`, and full `ResourceIdentity = (adapter_id, resource_kind,
resource_id)`; carry the tuple in `TargetScope.resource`; and provide one core
parser/canonical constructor used by later resolver and authority checkpoints.
Preserve Protobuf tag 8 as `legacy_audit_resource_id` only so existing durable
control-surface revocation audit records remain decodable.

## Acceptance evidence

- Generated Rust and TypeScript distinguish local resource id, adapter-owned
  kind, and full routable identity without a hand-written boundary DTO.
- Empty, partial, mixed, legacy-only, and dual resource scopes fail the core
  identity parser; exact canonical scopes round-trip.
- Equality, hashing, Protobuf bytes, and target-key bytes distinguish changes
  in adapter, kind, or local id.
- Existing tag-8 audit bytes decode with their audit target intact; the legacy
  scalar cannot parse as an operational resource.
- Contract builds and generated-drift checks pass.

## Ordering constraints

This is the root checkpoint. Do not add snapshot/revision fields, a resource
kind enum, capability-manifest validation, or resource report ingress. The
resource-state and capability-manifest sibling features own those semantics.
