---
id: epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution
kind: story
stage: done
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-typed-resource-identity]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Make target resolution target-kind-polymorphic

## Checkpoint

Land Unit 2 from the parent design: replace the session-shaped `TargetBinding`
with `RuntimeSession | Resource | AuthorityDomain`; add an identity-only
`ResourceRegistry`; compose it with the existing `SessionRegistry` behind the
single acceptance-owned `TargetResolver`; and update production adapter routing
to obtain the addressed adapter from either the session or nested resource
shape. The diagnostics-only resolver must return an honest authority-domain
binding rather than a fake session.

## Acceptance evidence

- Existing session replay/resolution behavior is unchanged for tombstones,
  exact/wildcard generation lookup, unknown generations, and offline/failed
  connectivity.
- A registered resource resolves only by its exact typed tuple; unknown,
  malformed, legacy-only, cross-adapter, and cross-kind targets fail closed.
- Ordinary production Submit dispatches runtime-session/resource kinds only;
  the diagnostics special resolver remains isolated and returns
  `AuthorityDomain` without synthetic runtime identity.
- Adapter delivery and authenticated Observation checks route a resource only
  to the adapter inside its nested identity.
- The new registry stores identity membership only and contains no health,
  snapshot, revision, generation, completeness, or payload state.

## Ordering constraints

Consumes the typed identity checkpoint. It may proceed independently of the
resource-authority checkpoint, but both must share the parent's canonical
parser. Do not invent an ephemeral production resource registration source;
the resource-state sibling will populate the seam from durable authenticated
reports.

## Implementation notes

- Replaced the session-shaped binding struct with `TargetBinding::{RuntimeSession, Resource, AuthorityDomain}`. Runtime-session bindings now include deployment scope; the diagnostics-only resolver returns the authority domain directly instead of synthetic session identity.
- Added identity-only `ResourceRegistry` membership and the composite `TargetRegistry`, which parses `TargetScopeKind` once and delegates only runtime-session and resource targets. The server projection lock now wraps this composite while continuing to fold session events.
- Centralized adapter extraction in `target_adapter_id`; resource routing requires the complete canonical tuple. Adapter delivery filtering and authenticated Observation checks now consume this helper, so a partial nested adapter id is inert.
- Kept the production resource registry empty with only the typed registration seam. No ephemeral registration source or resource state fields were introduced.

## Verification

- `cargo check --workspace --all-targets` — passed.
- `cargo test -p patchbay-core --test resource_resolver --test sessions_replay_resolver --test sessions_registry` — 21 passed.
- `cargo test -p patchbay-core-server adapter_service::tests::resource_delivery_routes_only_to_the_nested_owning_adapter` — passed, including cross-adapter and malformed routing denial.
