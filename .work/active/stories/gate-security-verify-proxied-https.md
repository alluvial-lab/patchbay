---
id: gate-security-verify-proxied-https
kind: story
stage: drafting
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
