---
id: feature-research-web-control-security
kind: feature
stage: drafting
tags: [research, security]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_dials:
  scope_authority: pre-registered
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: Web-first control plane security patterns

Research security patterns for a self-hosted, web-first human control plane that can operate remote/headless agent sessions.

## Seed questions

- What is an appropriate v0 authentication and device/session binding model for a single-operator control plane?
- What should Patchbay use for browser session binding, CSRF protection, replay resistance, and local/container deployments?
- How should emergency revocation and audit logging work for control surfaces?
- Which patterns are overkill for v0 but important to reserve space for?

## Expected output

A `.research/analysis/briefs/` synthesis brief that informs `docs/SECURITY.md` and the v0 protocol/security posture. Follow-up work items may be emitted only after operator confirmation.
