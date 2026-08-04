---
id: epic-agent-operations-resource-plane-resource-identity-typed-resource-identity
kind: story
stage: done
tags: [foundation, protocol]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
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

## Implementation notes

- Generated the Rust and TypeScript contracts from the canonical Protobuf schema after adding `ResourceId`, open `ResourceKind`, and nested `ResourceIdentity`. Protobuf tag 8 remains the same string wire field under the audit-only name `legacy_audit_resource_id`; operational identity uses tag 9.
- Added the core `ResourceIdentity` domain value with private fields, non-empty construction, canonical scope conversion, and fail-closed parsing for partial, mixed, legacy-only, and dual shapes. Repository-owned control-surface revocation audit producers now use the renamed legacy field.
- Kept this checkpoint state-agnostic: no generation, revision, snapshot, health, capability, or resource-report semantics were introduced.
- Execution capability: direct host implementation on `openai-codex/gpt-5.6-sol` at high reasoning because this delegated worker surface did not expose a nested subagent dispatch tool; one coherent feature owner preserves the same write boundary.

## Verification

- `cargo test -p patchbay-core --test resource_identity` — 3 passed.
- `cargo test -p patchbay-contracts` — passed.
- `cd contracts/ts && npm run build` — passed.
- `cd contracts && buf generate` followed by `git diff --check` — deterministic generation and clean patch formatting.
