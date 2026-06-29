---
id: epic-foundation-hardening
kind: epic
stage: implementing
tags: [foundation]
depends_on: []
parent: null
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

## Lane routing discipline (2026-06-28)

A retrospective pass found that several `[prose]` features in this epic were misrouted — they discharged scope items involving genuine architectural/semantic choices through the collapsed prose-author lane, which skips the design gate, pre-mortem, and alternatives evaluation. The following have been retagged from `[prose]` to design features:

- `feature-session-identity-adapter-contract` — session generation semantics, adapter capability tiers.
- `feature-idempotency-ambiguous-execution` — `maybe_executed` state, idempotency-key semantics.
- `feature-lease-scope-decision` — leases in/out of v0, fencing design if in.
- `feature-ux-v0-acceptance` — screen inventory, navigation, timeline behavior.
- `feature-verification-contract-authority` — artifact authority order, generation targets.

Three reopened semantic decisions from already-done prose features were filed as explicit design features:

- `feature-design-terminal-commit-race` — the first-durable-terminal-commit-wins race rule.
- `feature-design-grant-shape` — grant field list and delegation seam.
- `feature-session-identity-adapter-contract` also carries the three-tier adapter snapshot model reopened from `feature-persistence-snapshot-model`.

Remaining prose features (`extension-seams-non-foreclosure`, `observability-operator-admin`, `pi-parity-checklist`) are genuine prose — authoring checklists, classification rules, inventories, and mappings — and stay in the prose lane.

**Going forward:** apply the prose-author black-box test honestly to every `[prose]` item. If the scope item involves choosing between approaches, pinning a semantic model, or making an architectural commitment, route to `feature-design` instead. When in doubt, prefer design — the design gate's cost is low; the cost of a semantic commitment made silently through prose is high.
