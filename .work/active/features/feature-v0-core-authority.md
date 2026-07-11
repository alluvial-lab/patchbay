---
id: feature-v0-core-authority
kind: feature
stage: drafting
tags: [security, protocol, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Authority, grants, and audit

## Brief

Build the authority layer: grants, revocation, spawn authority, descendant-grant creation, and audit. A grant authorizes a subject (an actor, optionally narrowed to an endpoint or endpoint class) to perform a set of OperationKinds against a target scope. Grants are explicit, revocable, and evaluated inside one authority domain. The authority feature implements the grant-check port that the acceptance pipeline calls before accepting an operation.

v0.1.0 is single-operator, so the authority model is simple in practice — the operator can do everything — but the model keeps actor, endpoint, grant, and audit concepts explicit so future multi-human coordination is possible without rework. Fleet-level spawn authority is in v0.1.0 scope (single-operator, single-core, not HA/multi-core). Non-cascading spawn-grant revocation and descendant-grant creation are stated-normative obligations.

This feature has the weakest formal backing: all `authority.qnt` properties are stated-normative (draft). The one promoted property that touches authority (`RevokedSessionCannotCommand`) lives in `csrf_browser.qnt` and models the browser/CSRF boundary — it is web-server-facing, not core-internal. The authority feature's obligations are real but not yet checked.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (grants and audit records are durable). Implements the grant-check port that acceptance calls; acceptance and authority can proceed in parallel after persistence lands because the port interface decouples them.

## Formal-model backing

- All `authority.qnt` properties are stated-normative (draft) — obligations the feature must satisfy but that do not yet have checked formulas. The v1 formal gate owns the real authority properties.
- `RevokedSessionCannotCommand` (promoted, `csrf_browser.qnt`) — models the browser/CSRF boundary, NOT core-internal authority. Listed here only to clarify the boundary; it belongs to `feature-v0-web-server`.

## Foundation references

- `docs/PROTOCOL.md` — Authority grants; Spawn authority; Security and trust boundary
- `docs/SECURITY.md` — threat model, grants, revocation, audit, descendant grants
- `docs/ARCHITECTURE.md` — Authority and identity plane
- `docs/VERIFICATION.md` — stated-normative authority obligations
- `contracts/proto/patchbay/authority.proto` — `Grant`, `GrantProvenance`, `GrantRevocationPolicy`, `DescendantGrant`, `Revocation`
- `contracts/proto/patchbay/common.proto` — `ActorId`, `EndpointId`, `AuthorityDomainId`, `GrantId`, `TargetScope`
- `specs/seed/authority.qnt` — stated-normative authority obligations
