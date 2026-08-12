---
id: gate-security-v0.2.1-loopback-http-proxy-bleed
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

## Implementation notes

- Execution capability: inline direct-read security repair; the transport and its regression test form one cohesive boundary.
- Review weight: standard (project default); standalone-story review remained bounded and inline.
- Root cause confirmed: the production default selected `globalThis.fetch` for every validated URL. A child Node process started with `--use-env-proxy` and `HTTP_PROXY`, with `NO_PROXY` intentionally empty, routed the allowed `http://127.0.0.1` bearer request to the proxy and failed the new regression test.
- Files changed: `token-commune-adapter/src/gateway_url.ts`, `token-commune-adapter/src/gateway_client.ts`, `token-commune-adapter/tests/gateway_client.test.ts`.
- Fix: exported the existing loopback-HTTP classification and selected a dedicated `node:http` transport with an explicit private `Agent` for default loopback-HTTP requests. The explicit agent bypasses Node's environment-proxy global behavior while preserving the existing HTTPS/global-fetch path and explicit test-fetch injection.
- Regression test: launches a real loopback gateway and configured proxy, runs the compiled client in Node environment-proxy mode, verifies the gateway receives the bearer header, and asserts the proxy receives no ordinary or CONNECT request. It skips only on Node releases that do not expose environment-proxy mode and therefore cannot reproduce this path.
- Four-step confirmation: the test failed before the fix with a proxied 502 transport result; it passes after the fix with zero proxy requests; all 63 token-commune-adapter tests pass; the original proxy traversal is absent.
- Simplification: reused the URL validator's loopback predicate rather than creating a second hostname list.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review record

- Verdict: approve; no blockers, important findings, or nits.
- Correctness/security: default loopback plaintext requests cannot inherit the process proxy agent because they always carry an explicit direct HTTP agent; the real-process regression verifies both the destination and the absence of proxy contact.
- Tests: the regression kills the original `globalThis.fetch` behavior and covers both ordinary proxy requests and CONNECT attempts. Existing timeout, bounded-body, redirect rejection, credential-reflection, and decoder tests remain green through the new response wrapper.
- Design/breakage: HTTPS continues to use `globalThis.fetch`; explicit injected fetches remain honored. Rejecting all proxy-configured loopback development was ruled out because direct transport removes the leak without disabling the supported local topology.
- Foundation docs: no assertion changed; the fix strengthens the documented loopback-only local deployment posture.
- Reviewer path: bounded inline standalone-story review; no independent, fresh-context, or cross-model reviewer ran.
