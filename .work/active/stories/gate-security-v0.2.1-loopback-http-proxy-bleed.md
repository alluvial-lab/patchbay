---
id: gate-security-v0.2.1-loopback-http-proxy-bleed
kind: story
stage: drafting
tags: [security]
parent: null
depends_on: []
release_binding: null
gate_origin: security
created: 2026-08-12
updated: 2026-08-12
---

# Loopback-HTTP token-commune requests can leak the bearer header under HTTP_PROXY

> Parked from the v0.2.1 gate-security scan (Medium). Niche: requires `HTTP_PROXY` set + loopback HTTP use.

## Severity
Medium

## Domain
Data Protection

## Location
- `token-commune-adapter/src/gateway_url.ts:7`
- `token-commune-adapter/src/gateway_client.ts:125`

## Evidence
Loopback HTTP is validated only by URL hostname, but production uses `globalThis.fetch`; under Node environment-proxy mode, an allowed `http://127.*` request traverses `HTTP_PROXY` and exposes the bearer header inside the plaintext CONNECT tunnel.

## Remediation direction
Force direct/no-proxy transport for loopback HTTP (e.g. a no-proxy dispatcher for loopback targets), or reject proxy-enabled use when the gateway URL is loopback HTTP.
