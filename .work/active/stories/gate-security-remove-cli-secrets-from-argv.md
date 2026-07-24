---
id: gate-security-remove-cli-secrets-from-argv
kind: story
stage: drafting
tags: [security, secrets]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: security
created: 2026-07-24
updated: 2026-07-24
---

# Remove operator and setup secrets from CLI arguments

## Severity
Medium

## Domain
Secrets & Configuration

## Location
`cli/src/main.ts:39`

## Evidence
```ts
const VALUE_OPTIONS = new Set([
  "setup-secret",
  "operator-id",
  "password",
```

`setup --setup-secret ... --password ...` and `login --password ...` place reusable secrets in the process argument vector and commonly in shell history.

## Remediation direction
Read passwords and one-time setup secrets from a non-echoing TTY prompt or an explicit protected input channel (for automation, stdin/credential file or a documented secret manager path). Remove secret-bearing argv forms from usage, reject them rather than silently retaining compatibility, and add tests that process arguments and normal output never contain the supplied secret.
