---
id: gate-security-rotate-committed-live-credentials
kind: story
stage: done
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

## Remediation record (2026-07-24) — done

- **Note scrubbed**: `.work/session-notes/2026-07-23-v0-1-0-live-on-vm-live-test-arc.md`
  now carries redacted placeholders plus the convention: tracked notes never carry
  live secrets — describe where they live, not their values.
- **Rotated for real**: the devup deployment was torn down and re-bootstrapped fresh
  (no password-rotation RPC exists — `backlog-revocation-lifecycle-surface` covers the
  lifecycle surface). New core secret, adapter attachment secret, operator password,
  and TLS cert generated; old db preserved at gitignored `tmp/devup-prerotate-20260724/`.
- **Verified**: old secret+password → `[unauthenticated] invalid core principal secret`;
  new credentials authenticate; full stack (core/web/adapter) back up; `session-health`
  shows the session live. New secrets live only in 0600 `tmp/devup/.secrets.env`
  (gitignored) — never committed.
- **History**: leaked values remain in git history per operator decision (dev-only
  credentials, private self-hosted remote, all values invalidated by rotation).
