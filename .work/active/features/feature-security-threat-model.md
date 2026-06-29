---
id: feature-security-threat-model
kind: feature
stage: done
tags: [prose, security, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-research-web-control-security]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
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

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Implementation notes

- Files changed: `docs/SECURITY.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`, `.work/active/features/feature-security-threat-model.md`.
- Tests added: none; prose/foundation documentation change.
- Verification performed: proofread changed docs in context; checked dependencies are done with `work-view`; verified security terminology is reflected in protocol, verification, and glossary docs.
- Review fixes: added endpoint/device enrollment posture, login throttling/authenticator setup, AGENTS orientation entry for `docs/SECURITY.md`, audit-record terminology, device/principal glossary terms, actor/endpoint alignment, `Secure` cookie requirement wording, adapter-core trust-boundary language, deployment HTTPS clarification, and delegation parent-grant seam.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-06-28)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: addressed inline where useful: actor/device/endpoint terminology, audit-record wording, `Secure` cookie wording, and AGENTS orientation discoverability.

**Notes**: Deep feature review. Fresh-context completeness and adversarial passes found missing endpoint/device enrollment, login throttling/authenticator setup, AGENTS orientation discoverability for `docs/SECURITY.md`, audit/event terminology drift, and actor/endpoint wording issues. These were fixed in `d4b4e3a`; final inline verification found no remaining blockers or important findings. Parent epic remains implementing because sibling features are still active.
