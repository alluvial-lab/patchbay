---
id: epic-agent-operations-resource-plane-resource-identity
kind: feature
stage: drafting
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-07-30
---

# Resource identity, resolution & authority

## Brief

Promote the existing generic `TargetScopeKind = resource` from an untyped
`string resource_id` into a designed resource identity with target-resolution
semantics distinct from runtime-session identity. This is the foundation
feature for the resource plane: every other child feature depends on a
resource having a stable, typed, resolvable identity.

Today a resource-target Operation passes envelope and grant validation but
fails production target resolution because `TargetResolver` is hard-coded to
session fields (`core/src/session/resolver.rs` requires `runtime_session_id`).
This feature refactors the `TargetResolver` port to be target-kind-polymorphic
and adds a resource registry/resolver branch so resource targets resolve
without fabricating session identity. It also refines grant containment to
match on the full resource identity tuple (adapter_id, resource_id, kind)
rather than only `resource_id`, fencing cross-adapter resource-ID collision.

It does not define resource snapshot/revision state (that is `resource-state`),
the adapter capability manifest for resources (`capability-manifest`), or any
cockpit rendering (`cockpit-composition`).

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: foundation feature — others depend on its typed identity and resolver.

## Simplification opportunity

- Reuse the existing `TargetScope` envelope, `TargetScopeKind::Resource`, and the `TargetResolver` port (already generically named) rather than creating a parallel resolution subsystem. The polymorphism is the intended shape, not a retrofit.
- Eliminate the temptation to synthesize fake runtime-session identity for non-session targets.

## Foundation references

- `docs/ARCHITECTURE.md` — adapter plane; resource plane
- `docs/PROTOCOL.md` — target scopes, grants, `TargetScopeKind`
- `contracts/proto/patchbay/common.proto:80-99` — `TargetScope`, `TargetScopeKind`
- `contracts/proto/patchbay/authority.proto` — grant target scopes
- `core/src/acceptance/ports.rs:91-96` — `TargetResolver` result hard-coded to session fields
- `core/src/session/resolver.rs` — production resolver requires session identity
- `core/src/authority/state.rs:283-285` — resource containment ignores adapter_id/subtype

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`
- No direct UI in this feature; it is the identity/resolution foundation the cockpit feature renders.

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will fill in interfaces, signatures, and implementation units. -->
