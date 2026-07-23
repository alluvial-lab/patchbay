---
id: backlog-revocation-lifecycle-surface
created: 2026-07-23
tags: [security, fast-follower]
research_origin: null
---

# Backlog: full revocation/authorization lifecycle surface

Surfaced by the epic-v0-1-0-implementation maximum review, pass 2 (adversarial,
2026-07-23), as an Important finding — parked, not a v0.1.0 blocker.

## The gap

The trust-boundary feature shipped current-session revocation
(`RevokeOperatorSession`), but the stated v0.1.0 revocation contract
(`docs/SECURITY.md` § Lockdown / revocation) is only partially implemented:

- No revoke-all-sessions (only the current session).
- No endpoint/device/principal revocation (a compromised principal credential
  cannot be revoked short of rotating the core secret).
- No grant revocation path (grants are durable; no public grant-admin RPC).
- No lockdown/lockdown-exit surface (SECURITY:203-208 commits a durable
  lockdown posture + bootstrap-channel exit; not implemented).
- Grant expiration is explicitly ignored (`core/src/authority/state.rs:45`).
- `Subscribe` authenticates the compound issuer but performs no grant check.

## Why parked (not a v0.1.0 blocker)

Exploitability is limited in the single-operator v0.1.0 topology: bootstrap
creates one permanent authority-domain grant, there is no public grant-admin
path, and a compromised principal is bounded by the operator's own actor. The
reviewer rated it Important, not Blocker. It becomes load-bearing for
multi-operator, split-deploy, or a real incident-response need.

## Fast-follower shape

A `feature-revocation-lifecycle` (or folded into the core-diagnostics/admin
arc): revoke-all, endpoint/principal revocation, grant revocation + expiry
enforcement, lockdown/exit per SECURITY:203-208, Subscribe grant check. Each
piece has an existing contract anchor (GrantRevocationPolicy, Revocation
events, the lockdown section).
