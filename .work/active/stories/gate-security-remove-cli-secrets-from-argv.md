---
id: gate-security-remove-cli-secrets-from-argv
kind: story
stage: done
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

## Completion

- Removed `--password` and `--setup-secret` from CLI parsing and help. Both are now rejected before command dispatch, without echoing their values.
- `setup` and `login` read the documented `PATCHBAY_SETUP_SECRET` / `PATCHBAY_OPERATOR_PASSWORD` variables first, otherwise use a non-echoing interactive TTY prompt; non-interactive callers without the required environment variable fail closed.
- Updated the CLI core-smoke invocation and `docs/RUNBOOK.md` to use/document the secret environment variables and prohibit secret-bearing argv.

## Verification

- `cd cli && npm test` — passed: 17 tests, 0 failures.
- The added CLI regression test passes an inline secret argument, confirms rejection and that normal output excludes the supplied value; it also confirms help contains neither secret option.
