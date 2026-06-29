---
id: research-handoff-web-control-security-3
kind: feature
stage: drafting
tags: [security, protocol]
parent: null
depends_on: []
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_origin: web-control-security
---

# Implement command grants, revocation, and audit log security baseline

Patchbay should implement the v0 remote-control security baseline for commands: deny-by-default grants, per-request command authorization, target/session generation checks, emergency revocation flows, security lockdown posture, and durable audit events for authentication, authorization, session management, command lifecycle, grant changes, adapter changes, and stale/wrong-target events.

## Research grounding

**Source**: `.research/analysis/briefs/web-control-security.md` (slug: `web-control-security`)

The research recommends treating single-operator v0 as authorization-bearing rather than authorization-free, and grounding revocation/audit behavior in least-privilege, deny-by-default, per-request authorization, session invalidation, and security logging guidance.
