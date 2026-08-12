---
id: gate-security-token-commune-http-credentials
kind: story
stage: done
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

## Symptom
Configuration and direct gateway-client construction both accepted non-loopback `http:` base URLs, after which every request attached the reusable member-key bearer credential.

## Root cause
The two gateway URL boundaries validated only URL shape and the broad `http:`/`https:` scheme set; neither coupled plaintext transport permission to a verified loopback hostname.

## Fix approach
Use one shared gateway URL guard at both configuration ingress and direct client construction. Keep HTTPS valid for any credential-free host, and permit HTTP only for the exact local-development loopback forms `localhost`, IPv4 `127.0.0.0/8`, and IPv6 `[::1]`.

## Regression test
`token-commune-adapter/tests/resource_contract.test.ts` rejects LAN, container-network, and remote HTTP configuration while retaining loopback HTTP. `token-commune-adapter/tests/gateway_client.test.ts` proves direct client callers cannot bypass the same transport policy.

## Implementation notes

- **Execution capability:** direct inline implementation; the bug was confined to two URL-ingress boundaries with one shared policy and did not need delegated exploration.
- **Files changed:** added `token-commune-adapter/src/gateway_url.ts`; applied it in `src/config.ts` and `src/gateway_client.ts`; added regression coverage in both relevant test suites; documented the local-development exception in `docs/RUNBOOK.md`.
- **Reproduction:** the two new negative tests initially failed because `http://192.168.1.20` was accepted by both configuration and direct client construction.
- **Focused confirmation:** `npm run build && node --test dist/tests/resource_contract.test.js dist/tests/gateway_client.test.js` passed 14/14 after the fix.
- **Full confirmation:** `npm test` passed 62/62, including the real gateway/core flow. This command rebuilt the operator domain, web cockpit types, and token-commune adapter.
- **Original symptom:** LAN, container-network, and remote plaintext gateway URLs now fail before credential application; the existing loopback HTTP development path remains valid.
- **Adjacent issues parked:** none.

## Review (2026-08-12)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none
**Rejected**: none

**Notes**: Bounded inline standalone-story review; no independent, fresh-context, or cross-model reviewer ran. Correctness review confirmed the shared guard covers both configuration and direct client construction, HTTPS remains valid, and the loopback-only HTTP exception matches the documented development path. Security review found no hostname-prefix bypass or credential-bearing error path. Regression coverage exercises LAN, container-network, remote, IPv4 loopback, IPv6 loopback, and localhost cases. Tests, design alignment, intentional config hardening, foundation-doc alignment, naming, and comments were reviewed; no material findings remain. Per operator instruction, this done release-gate story remains in `.work/active/stories/` for release-deploy rather than being archived.
