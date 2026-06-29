---
id: feature-session-identity-adapter-contract
kind: feature
stage: drafting
tags: [protocol, adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define session identity and adapter capability contract

Wrong-session prevention cannot rely on optional adapter metadata. Patchbay needs a normative session identity and adapter capability contract before Pi or other adapters are implemented.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes genuine design choices (adapter capability tier model, session generation semantics, capability manifest schema) that need a `feature-design` pass, not collapsed prose authoring. This is the misroute the prose-author black-box test should have caught originally.

A specific item carried over from `feature-persistence-snapshot-model`: the three-tier adapter snapshot model (authoritative / partial / none) currently committed in `docs/PROTOCOL.md` was invented during a prose feature without a design pass. It belongs in this feature's adapter-capability-tier design work and should be ratified or revised here.

## Scope

- Canonical session identity tuple and mandatory fields.
- Session generation/epoch semantics.
- Session replacement, tombstone, and reuse rules.
- Message/command/reply/event id spaces and correlation rules.
- Adapter registration/authentication to the core.
- Adapter capability manifest schema.
- Capability tiers for streaming, snapshots, cancellation, idempotency, and session replacement.

## Acceptance criteria

- `docs/PROTOCOL.md` defines session identity and correlation precisely.
- `docs/ARCHITECTURE.md` describes adapter registration and lifecycle.
- `docs/GLOSSARY.md` defines generation/epoch, endpoint, adapter capability, and correlation context.
- The Pi adapter can map its capabilities without redefining Patchbay core identity.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
