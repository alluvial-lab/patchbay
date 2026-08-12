---
id: gate-security-upgrade-web-server-router-deps
kind: story
stage: done
tags: [security, dependencies]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: security
created: 2026-08-11
updated: 2026-08-11
---

# Upgrade vulnerable web-server routing dependencies

## Severity
High

## Domain
Dependencies & Supply Chain

## Location
`web-server/package-lock.json:392`

## Evidence

`npm --prefix web-server audit --omit=dev` reports high-severity host-confusion advisories for `fast-uri` 3.1.3 (`GHSA-v2hh-gcrm-f6hx`, `GHSA-7p8r-x3mc-p8w7`) and an HTTP/2 denial-of-service advisory for `find-my-way` 9.6.0 (`GHSA-c96f-x56v-gq3h`). npm reports a non-breaking lockfile remediation is available.

## Remediation direction

Apply the supported npm audit update, verify the production audit is clean of high/critical findings, and run the web-server build/test suite.

## Implementation and verification

Applied the supported lockfile update. `npm audit --omit=dev --audit-level=high` reports zero vulnerabilities and all 31 web-server tests pass.
