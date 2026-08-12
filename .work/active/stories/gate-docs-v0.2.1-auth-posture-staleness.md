---
id: gate-docs-v0.2.1-auth-posture-staleness
kind: story
stage: done
tags: [documentation, security]
parent: null
depends_on: []
release_binding: null
gate_origin: docs
created: 2026-08-12
updated: 2026-08-12
---

# Roll auth-posture docs forward to the v0.2.1 per-adapter credential model

> Parked from the v0.2.1 gate-docs scan. Overlaps the existing v0.2.0 parked item `gate-docs-readme-v0.2-current-status`.

## Severity
Medium (docs drift)

## Locations / drift
- `docs/SECURITY.md:5,78-88,148-160` — auth posture still framed as v0.1.0/session-only; describes generic adapter trust material, not the per-adapter credential binding + authenticated attachment identity rule. Update enrollment + report-source sections.
- `docs/PROTOCOL.md:45,697` — attachment semantics underspecified relative to the implementation: credentials are selected by claimed adapter ID and must be unique per adapter; state the committed rule or mark it implementation-only.
- `README.md:11,48-51,71-83,132-146` — still presents v0.1.0/Pi-only status, omits the v0.2.0 token-commune resource adapter. Roll current-status + repository-layout forward (or mark historical).

## Remediation direction
Roll the auth/enrollment/attachment sections forward to the v0.2.1 model (per-adapter `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS`, authenticated identity → accepted adapter_id+generation, canonical audit sender). The RUNBOOK migration note + CHANGELOG v0.2.1 entry are already done; these are the deeper foundation-doc updates.

## Resolution (2026-08-12)

`docs/SECURITY.md` now describes the current v0.2.1 resource-aware posture, per-adapter credential selection/uniqueness, authenticated attachment identity and token fencing, canonical adapter report/audit identity, and current resource/session report boundaries. `docs/PROTOCOL.md` now makes the per-adapter credential map and exact claimed-adapter selection an explicit Attach rule and documents current attachment lifecycle semantics. README and RUNBOOK were coordinated in the sibling stories. Verified with `git diff --check` and bounded accuracy/drift review against `server/src/adapter_service.rs`, `core/src/adapter/mod.rs`, `docs/RUNBOOK.md`, `CHANGELOG.md`, and `docs/PROTOCOL.md`.
