---
id: research-handoff-web-control-security-1
kind: feature
stage: drafting
tags: [security, foundation, prose]
parent: null
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_origin: web-control-security
---

# Define v0 web control security posture in docs/SECURITY.md

Patchbay should add a foundation security document for the v0 web-first control plane, covering single-operator scope, session binding, CSRF, command authorization, emergency revocation, audit logging, deployment assumptions, and reserved future seams.

This is a prose/foundation item and should likely be scoped before implementation items so the security posture is explicit before code depends on it.

## Research grounding

**Source**: `.research/analysis/briefs/web-control-security.md` (slug: `web-control-security`)

The research recommends hardened server-side browser sessions, CSRF defenses, deny-by-default command authorization, emergency revocation, and audit logging as v0 essentials, with MFA/passkeys/OIDC/mTLS/multi-operator RBAC reserved as future seams.
