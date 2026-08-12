---
id: gate-security-token-commune-http-credentials
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

# Token-commune bearer credentials may be transmitted over plaintext HTTP

> Surfaced by the retroactive deep security scan of `v0.2.0`. Release-relevant: the token-commune gateway integration and member-key bearer authentication are new in `v0.2.0`.

## Severity
Medium

## Domain
Data Protection

## Location
- `token-commune-adapter/src/config.ts:42`
- `token-commune-adapter/src/gateway_client.ts:130`
- `token-commune-adapter/src/gateway_client.ts:137`
- `token-commune-adapter/src/credential.ts:49`

## Evidence
Both configuration and client validation explicitly permit `http:`:
```typescript
!["http:", "https:"].includes(gatewayBaseUrl.protocol)
```
Every gateway request applies the credential:
```typescript
options.credential.apply(headers);
```
The credential is a reusable bearer key:
```typescript
headers.set("Authorization", `Bearer ${key}`);
```
A non-loopback HTTP gateway exposes the member key and returned operational telemetry to passive interception and active response manipulation.

## Remediation direction
Require HTTPS for non-loopback gateway URLs. If plaintext HTTP is necessary for local development, restrict it to verified loopback hosts and document the exception explicitly. Add configuration tests rejecting LAN, container-network, and remote `http:` endpoints.
