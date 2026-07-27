---
id: backlog-session-record-fields-gap
tags: [security, foundation]
created: 2026-07-27
---

# Browser session records lack the fields SECURITY.md commits

Docs-audit finding (2026-07-27): `docs/SECURITY.md:94` says session records
include operator id, endpoint id, created/last-used/expiration/revoked times,
and session generation. Reality: web records lack endpoint id, session
generation, and revoked time (`web-server/src/sessions.ts:11-20`); core
records carry only actor, expiry, and a revoked boolean
(`server/src/operator_session.rs:15-18`). Decision for v0.1+: implement the
promised fields (endpoint binding also matters for multi-endpoint futures) or
descope the prose.

## Disposition

Absorbed by `epic-revocation-lifecycle-session-principal-revocation`: browser session records now retain endpoint/device identity, endpoint generation, timestamps, and `revokedAt`; core operator sessions retain replayed generation fences and verified compound bindings. No follow-up item is required.
