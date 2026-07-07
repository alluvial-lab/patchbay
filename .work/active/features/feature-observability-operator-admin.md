---
id: feature-observability-operator-admin
kind: feature
stage: drafting
tags: [foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-persistence-snapshot-model]
created: 2026-06-28
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Define operator/admin observability

## Misroute note (2026-07-07)

Stripped `[prose]` — this is a design feature, not prose authoring. The scope involves genuine design decisions: (1) whether observability is v0 or post-v0 is a scope/classification decision, not consolidation of an already-settled answer (the docs don't currently settle it); (2) "the v0 control surface or CLI has enough diagnostic expectations to debug failed delivery" requires designing what diagnostic surfaces exist (delivery trace, logs, metrics, event inspection) and what "enough" means; (3) "security docs cover what must not be logged" requires deciding redaction/sensitive-payload-handling rules with security consequences. These are choosing-between-approaches / architectural-commitment decisions, not collapsed prose authoring of settled material. Routed through `feature-design`; `prose` tag removed. Same misroute pattern documented in the epic's lane-routing discipline and the 2026-07-06 codification of the prose black-box test.

Review noted that Patchbay should help the operator answer why a command did not deliver. Observability is part of the human control plane, not just implementation plumbing.

## Scope

- Health and status of core, adapters, and control surfaces.
- Delivery trace for a command: accepted, routed, adapter response, execution result.
- Logs, metrics, and event inspection expectations.
- Safe redaction and sensitive payload handling.
- CLI/admin debugging requirements.

## Acceptance criteria

- Foundation docs identify observability as v0 or post-v0 with clear scope.
- The v0 control surface or CLI has enough diagnostic expectations to debug failed delivery.
- Security docs cover what must not be logged.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
