---
id: epic-foundation-hardening
kind: epic
stage: implementing
tags: [foundation]
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Epic: Foundation hardening after adversarial review

Patchbay's initial docs establish the right product direction, but review found that implementation should not begin until the starting slice, protocol state machines, security model, persistence/snapshot semantics, and verification source-of-truth are sharpened.

This epic tracks the refinement program that converts the current foundation docs from a strong vision into an executable starting-state project.

## Review synthesis

Fresh-context top-down and bottom-up review converged on these concerns:

- Define a concrete v0 walking skeleton rather than an entire platform program.
- Consolidate command/session/failure state into one source of truth.
- Specify persistence, ordering, snapshots, and crash recovery before relying on durable acceptance.
- Define security principals, grants, device/browser identity, threat model, and revocation.
- Pin session identity/generation semantics so wrong-session prevention is implementable.
- Define adapter capability tiers and Pi parity without letting Pi become the core ontology.
- Decide how prose, formal models, generated contracts, and conformance vectors relate.
- Split UX presentation state into session liveness vs command delivery and make v0 screens actionable.
- Decide whether leases are v0 scope or deferred.

## Acceptance criteria

- Foundation docs define a buildable v0 slice with explicit exclusions.
- `docs/PROTOCOL.md` contains canonical state-machine and identity semantics rather than scattered enum-like lists.
- `docs/SECURITY.md` or equivalent defines v0 threat model and principal/grant posture.
- `docs/ARCHITECTURE.md` defines v0 persistence/topology/snapshot ordering assumptions.
- `docs/VERIFICATION.md` maps v0 models to normative artifacts and conformance checks.
- `docs/UX.md` has v0 cockpit acceptance criteria and separates session and command presentation states.
- Follow-on implementation can begin without inventing protocol semantics ad hoc.
