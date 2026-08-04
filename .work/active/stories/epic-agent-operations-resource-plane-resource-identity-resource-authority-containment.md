---
id: epic-agent-operations-resource-plane-resource-identity-resource-authority-containment
kind: story
stage: implementing
tags: [foundation, protocol, security]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-typed-resource-identity]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
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
