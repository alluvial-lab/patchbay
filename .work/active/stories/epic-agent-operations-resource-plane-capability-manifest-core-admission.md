---
id: epic-agent-operations-resource-plane-capability-manifest-core-admission
kind: story
stage: done
tags: [protocol, adapter]
parent: epic-agent-operations-resource-plane-capability-manifest
depends_on: [epic-agent-operations-resource-plane-capability-manifest-contract-registry]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Validate manifests and expose one resource admission boundary

## Design checkpoint

Land Unit 2 from the parent design. Add the generated-contract-derived
`ValidatedAdapterCapability` projection in `core/src/adapter/capability.rs`,
validate it on both live registration and replay, store it beside each durable
adapter record, and expose exact `(adapter_id, resource_kind)` lookup plus exact
payload/projection envelope descriptor matching. The validator receives an
explicit attach/replay context: replay alone may normalize a pre-category,
resource-empty durable record to session-only; fresh attach requires explicit
categories, and the compatibility path can never admit a resource.

The committed allowlist admits runtime sessions and operational resources only.
Unknown, unspecified, and reserved knowledge-bundle categories fail closed.
Resource declarations require the operational-resource category, unique open
`ResourceKind`s, per-kind authoritative/partial/none tier, and a projection
contract tied to the operational-resource conformance target. Exact schema-ref
and content-type matching is declaration binding, not semantic byte validation.
Capabilities remain advisory and must not become grant authority, resolution,
or a delivery gate.

## Acceptance evidence

- Session-only, resource-only, and mixed manifests validate only when category,
  session tier, resource declarations, and projection descriptors are
  internally consistent.
- One adapter can declare `provider_pool` and `usage_window` with different
  tiers/schemas; exact `ResourceIdentity` lookup selects each declaration while
  cross-adapter and undeclared-kind lookups deny.
- Invalid-manifest tests cover duplicate/unknown/reserved categories, duplicate
  kinds, missing categories on fresh attach, unspecified tiers/content types,
  incomplete descriptors, and projection category mismatch before durable
  append.
- Replay applies the same validator; a legacy resource-empty record may become
  session-only, while a corrupt or resource-bearing category-less manifest
  cannot create a weaker in-memory projection.
- An OKF-v0.2-shaped declaration under `KNOWLEDGE_BUNDLE` is rejected, and no
  session/resource state changes on any rejection.

## Ordering constraint

Depends on generated contract types. Resource-report ingress is sibling
`resource-state` scope; this checkpoint supplies the exact admission/schema
binding API that sibling must call without inventing report or snapshot state.

## Implementation notes

- Added a generated-contract-derived validated capability projection with a
  committed target-category allowlist, exact per-kind resource declarations,
  bounded schema descriptors, and attach/replay validation contexts.
- Fresh attach now requires explicit internally consistent categories. Replay
  alone normalizes a category-less, resource-empty legacy registration to
  session-only; resource-bearing or otherwise malformed records still abort
  replay.
- Stored the validated projection beside each adapter registration and exposed
  exact `ResourceIdentity` lookup plus payload/projection descriptor matching.
  These APIs neither register a resource nor consult grants or delivery support.
- Added focused tests for session-only/resource-only/mixed declarations,
  two-kind lookup, cross-adapter denial, schema mismatch, reserved OKF/category
  rejection, invalid enum/tier/descriptor relations, no-append fresh rejection,
  and replay compatibility/corruption.
- Verified the focused capability suite, full Rust workspace tests, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
