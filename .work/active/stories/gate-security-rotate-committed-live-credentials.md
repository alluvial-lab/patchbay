---
id: gate-security-rotate-committed-live-credentials
kind: story
stage: implementing
tags: [security, secrets]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: security
created: 2026-07-24
updated: 2026-07-24
---

# Rotate live deployment credentials committed in a tracked session note

## Severity
Critical

## Domain
Secrets & Configuration

## Location
`.work/session-notes/2026-07-23-v0-1-0-live-on-vm-live-test-arc.md:23`, `.work/session-notes/2026-07-23-v0-1-0-live-on-vm-live-test-arc.md:33`

## Evidence
```text
- Login (both cockpit form + CLI): operator-dev / <redacted live password>
- Web-server: ... PATCHBAY_CORE_SECRET=<redacted live core secret>
```

The tracked note describes these as credentials for the live, LAN-reachable VM stack. The raw values are intentionally not repeated in this finding.

## Remediation direction
Treat both values as compromised: rotate the operator password and core secret immediately, invalidate affected sessions/principals, verify whether the live stack or any reused deployment remains exposed, and replace the tracked note with redacted placeholders. Decide whether repository-history rewriting is warranted based on reuse and distribution, without relying on deletion from the current tree as revocation.
