---
id: gate-security-upgrade-pi-sdk-vulnerable-transitives
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

## Implementation and verification

Upgraded both aligned Pi SDK packages to 0.84.1 and migrated the adapter from the retired `AuthStorage`/session `ModelRegistry` surface to `ModelRuntime` plus `createAgentSessionFromServices`. `npm audit --omit=dev --audit-level=high` reports zero vulnerabilities; all 29 Pi adapter tests and the separate-process walking-skeleton E2E pass. The E2E Pi fixture was migrated to the same `ModelRuntime` surface after the first E2E run exposed its retired `AuthStorage` setup.
