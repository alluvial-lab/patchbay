---
id: research-handoff-web-control-security-2
kind: feature
stage: drafting
tags: [security]
parent: null
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_origin: web-control-security
---

# Implement v0 browser session auth and CSRF defenses

Patchbay should implement the v0 browser-session security baseline for the web cockpit: server-side sessions, hardened `__Host-` cookies, localhost vs non-localhost transport policy, login throttling, session revocation, CSRF tokens on state-changing requests, custom request headers, Origin / Fetch Metadata checks where available, and no GET mutation.

## Research grounding

**Source**: `.research/analysis/briefs/web-control-security.md` (slug: `web-control-security`)

The research grounds the browser control surface recommendation in OWASP, MDN, and NIST guidance on session secrets, hardened cookies, CSRF defenses, transport protection, login throttling, and avoiding JavaScript-readable session secrets.
