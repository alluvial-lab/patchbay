---
id: feature-research-web-control-security
kind: feature
stage: done
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

## Engagement record

Completed: 2026-06-28

Decision relevance: inform `docs/SECURITY.md` and Patchbay's v0 protocol/security posture for a single-operator, self-hosted web control plane operating remote/headless agent sessions.

Settled dials:

- `scope_authority`: `pre-registered`
- `verification_rigor`: `standard`
- `intent`: `inform-architecture-decision`
- `output_kind`: `synthesis-brief`

Decomposition used:

- Single-operator auth + device/session model.
- Browser session, CSRF, replay, and transport protection.
- Remote control safety: grants, revocation, audit logging, stale/wrong-target resistance.
- V0 essentials vs reserved-future security seams.

Outputs:

- Synthesis brief: `.research/analysis/briefs/web-control-security.md`
- Verification checklist: `.research/analysis/briefs/web-control-security-verification.md`
- Source attestations: `.research/attestation/{owasp-session-management,owasp-csrf,owasp-authentication,owasp-authorization,owasp-logging,mdn-set-cookie,nist-session-management}.md`

Gate outcomes:

- Citation lint: 67 resolved/non-broken citations, 0 broken, 0 thin, 0 pattern flags (`--no-url-check` used to avoid environment URL-probe noise after direct source fetches).
- Adversarial-read: first pass `NEEDS-REVISION`; findings fixed; second pass `APPROVED`.
- Spot-check: completed by lead; no remaining blockers after revised adversarial review.
- Acquisition candidates: none.

## Seed questions

- What is an appropriate v0 authentication and device/session binding model for a single-operator control plane?
- What should Patchbay use for browser session binding, CSRF protection, replay resistance, and local/container deployments?
- How should emergency revocation and audit logging work for control surfaces?
- Which patterns are overkill for v0 but important to reserve space for?

## Expected output

A `.research/analysis/briefs/` synthesis brief that informs `docs/SECURITY.md` and the v0 protocol/security posture. Follow-up work items may be emitted only after operator confirmation.
