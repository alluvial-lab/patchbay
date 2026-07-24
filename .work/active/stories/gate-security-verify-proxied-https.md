---
id: gate-security-verify-proxied-https
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: security
created: 2026-07-24
updated: 2026-07-24
---

# Verify browser HTTPS across trusted reverse-proxy hops

## Severity
Medium

## Domain
Infrastructure & Deployment

## Location
`web-server/src/middleware/csrf-auth.ts:52`

## Evidence
```ts
if (socket.encrypted === true) return true;
const address = socket.remoteAddress;
return address === "127.0.0.1" || address === "::1" || address === "::ffff:127.0.0.1";
```

Any loopback peer is treated as a localhost browser. A local reverse proxy can therefore forward non-localhost plaintext HTTP and have login/session requests accepted as secure, with no explicit trusted-proxy mode or verified original scheme.

## Remediation direction
Separate the direct-localhost exception from reverse-proxy deployment. Require direct TLS for non-loopback browser origins, or add an explicit trusted-proxy configuration that restricts proxy addresses and validates a standardized forwarded scheme before accepting sessions. Add a regression test for a loopback proxy carrying an external HTTP request and document the fail-closed deployment shape.

## Completion

- Direct TLS remains accepted; the HTTP localhost exception is now limited to direct loopback requests with no `X-Forwarded-Proto` header.
- Added opt-in `PATCHBAY_TRUST_LOOPBACK_PROXY=true`. It trusts only a loopback proxy and only a single normalized `X-Forwarded-Proto: https` value. Any forwarded `http`, ambiguous forwarded value, untrusted proxy header, or non-loopback plaintext request fails closed.
- Threaded the transport policy through login, logout, CSRF-token, and browser-to-core RPC session guards.
- Documented direct-TLS and local-proxy deployment requirements in `docs/RUNBOOK.md` and updated the security posture to name the supported proxy boundary.

## Verification

- `cd web-server && npm test` — passed: 24 tests, 0 failures.
- `cd web-server && npm run build` — passed.
- The added regression covers an untrusted loopback proxy; an enabled loopback proxy reporting `http` or no forwarded scheme at login; and an existing browser session request forwarded as `http`. All return `400 {"error":"https_required"}`. It also verifies accepted `https` only with the explicit proxy opt-in.
