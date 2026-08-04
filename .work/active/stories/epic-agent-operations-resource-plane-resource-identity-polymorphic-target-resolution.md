---
id: epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution
kind: story
stage: implementing
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-typed-resource-identity]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
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
