---
id: gate-security-upgrade-pi-sdk-vulnerable-transitives
kind: story
stage: implementing
tags: [security, dependencies]
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: security
created: 2026-08-11
updated: 2026-08-11
---

# Upgrade the Pi SDK dependency chain past high-severity advisories

## Severity
High

## Domain
Dependencies & Supply Chain

## Location
`pi-adapter/package-lock.json:581`

## Evidence

`npm --prefix pi-adapter audit --omit=dev` reports two high-severity vulnerabilities in the production dependency chain, including `undici` 8.5.0 (`GHSA-4cwx-7wf7-3272`) and vulnerable `brace-expansion` releases. npm identifies `@earendil-works/pi-coding-agent` 0.84.1 as the available remediation.

## Remediation direction

Upgrade the aligned Pi SDK packages to 0.84.1, regenerate the lockfile, verify the audit is clean of high/critical findings, and run the Pi adapter build/test suite.
