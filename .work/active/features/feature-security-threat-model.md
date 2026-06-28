---
id: feature-security-threat-model
kind: feature
stage: drafting
tags: [prose, security, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-research-web-control-security]
---

# Feature: Define v0 security, principal, and threat model

Patchbay controls remote/headless agents and potentially shell/job adapters. The docs need a concrete first security posture before a web-first control plane is implemented.

## Scope

- Threat model and explicit out-of-scope adversaries.
- Principal model: operator, device, browser session, endpoint, adapter, actor.
- Device/control-surface enrollment and revocation posture.
- Grant shape and authorization algorithm.
- Replay protection and command issuer binding.
- Emergency revocation and audit events.
- Forbidden v0 deployments if any, such as internet-exposed unauthenticated core.

## Acceptance criteria

- Add `docs/SECURITY.md` or equivalent.
- `docs/PROTOCOL.md` defines grants and revocation using the same terminology.
- `docs/VERIFICATION.md` can map authority safety to concrete variables.
- Browser/web cockpit security expectations are stated for v0.
