---
id: gate-security-upgrade-dompurify
kind: story
stage: drafting
tags: [security, dependencies]
parent: null
depends_on: []
release_binding: null
gate_origin: security
created: 2026-08-11
updated: 2026-08-11
---

# Upgrade DOMPurify past the detached-subtree XSS advisory

## Severity
Medium

## Domain
Dependencies & Supply Chain

## Location
`web-cockpit/package-lock.json:773`

## Evidence

`npm --prefix web-cockpit audit --omit=dev` reports `dompurify` 3.4.12 affected by `GHSA-55q2-fjhq-7xh7`: removing an `IN_PLACE` hook can leave a detached executable subtree. Patchbay does not configure this hook today, reducing immediate exploitability, but cockpit markdown sanitation should not remain on an affected release.

## Remediation direction

Update DOMPurify to a fixed release, regenerate the lockfile, and run markdown sanitization, conformance-mutation, and cockpit tests.

## Release disposition

Parked unbound under the operator's v0.2.0 gate policy (medium risk); it does not block shipment.
