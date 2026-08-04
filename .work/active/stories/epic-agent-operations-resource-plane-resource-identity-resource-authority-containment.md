---
id: epic-agent-operations-resource-plane-resource-identity-resource-authority-containment
kind: story
stage: done
tags: [foundation, protocol, security]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-typed-resource-identity]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Fence resource grant containment by full identity

## Checkpoint

Land Unit 3 from the parent design: validate resource Grant scopes through the
canonical typed parser and make `TargetScopeKind::Resource` containment exact
on requested target kind, adapter id, resource kind, and local resource id.
Keep adapter/fleet/authority-domain grants as the existing explicit wider
scopes; add no implicit kind wildcard.

## Acceptance evidence

- A live exact resource grant authorizes only its allowed OperationKinds on its
  exact typed tuple.
- Same local id under another adapter, same adapter/id under another kind,
  another local id, a partial or legacy scalar, and a non-resource request all
  deny before acceptance.
- Property strategies vary adapter/kind/id independently, and mutation checks
  fail if any tuple comparison or the requested-kind fence is removed.
- Adapter-scope containment recognizes the nested resource adapter id while
  endpoint, expiry, revocation, OperationKind, session, fleet, and domain rules
  retain their current behavior.
- Malformed durable Grant/DescendantGrant resource scopes fail replay rather
  than becoming partially authorized.

## Ordering constraints

Consumes the typed identity checkpoint and shares its parser with acceptance
and resolution. Do not add resource-kind-wide grants or capability declarations;
those require a later explicit authority/manifest design.

## Implementation notes

- Grant and descendant-grant replay now validate resource scopes through the canonical `ResourceIdentity` parser, rejecting partial, mixed, legacy-only, and empty tuples before projection.
- Resource-scope containment parses both sides and compares the exact adapter/kind/local-id tuple. A resource grant cannot match a non-resource request, and malformed requested resource scopes deny before fleet/domain/adapter wildcard evaluation.
- Adapter grants use the shared `target_adapter_id` helper, so a canonical nested resource is contained only by its owning adapter; endpoint, expiry, revocation, OperationKind, session, fleet, and domain checks remain unchanged.
- Added randomized independent adapter/kind/id containment evidence and explicit requested-kind mutation coverage; no kind-wide wildcard or capability coupling was introduced.

## Verification

- `cargo test -p patchbay-core --test authority_registry --test authority_grant_check --test authority_ingest --test authority_proptest` — 35 passed, including 100-case tuple-dimension property runs.
- `cargo check --workspace --all-targets` — passed.
