---
id: feature-observability-operator-admin
kind: feature
stage: drafting
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-persistence-snapshot-model]
---

# Feature: Define operator/admin observability

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
