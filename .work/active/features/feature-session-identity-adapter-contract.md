---
id: feature-session-identity-adapter-contract
kind: feature
stage: drafting
tags: [prose, protocol, adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
---

# Feature: Define session identity and adapter capability contract

Wrong-session prevention cannot rely on optional adapter metadata. Patchbay needs a normative session identity and adapter capability contract before Pi or other adapters are implemented.

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
