---
id: epic-revocation-lifecycle-lockdown-cli-conformance
kind: story
stage: done
tags: [security, protocol]
parent: epic-revocation-lifecycle-lockdown
depends_on: [epic-revocation-lifecycle-lockdown-trigger-exit-rpcs]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-29
updated: 2026-07-30
---

# Ship CLI recovery, integrated conformance, and rolling foundation

## Checkpoint

Land Units 6–7 from the parent design: scriptable `lockdown-enter` through the
authenticated ControlService, literal `lockdown-exit` through the loopback
AdminService with no credential-store dependency, safe human/JSON output, and
the cross-boundary recovery/concurrency/atomicity evidence. Roll SECURITY,
PROTOCOL, VERIFICATION, UX, GLOSSARY, and RUNBOOK assertions forward with the
implemented behavior and honest assurance tier.

## Acceptance evidence

- `lockdown-enter --reason-code CODE --confirm LOCKDOWN` validates locally,
  clears credentials only after a well-formed committed/already-active result,
  and never prints raw session/credential material.
- `lockdown-exit [--reason-code CODE]` works with no credential file only through
  `makeAdminClient`; non-loopback admin configuration fails before network
  access. It does not accept setup-secret/password flags.
- Unknown outcomes and malformed/contradictory responses never claim success;
  exit failure remains visibly locked. Human/JSON output uses canonical safe
  codes, decimal-string LSN/generation values, and documented exit codes.
- Real-process, barrier-race, transaction-failure, audit-redaction, model,
  vector, Rust workspace, TypeScript workspace, presentation, and generated
  drift checks pass.
- Foundation docs state all-Operation rejection, named non-Operation read
  exceptions, fresh-login semantics, reason-code-only persistence, and
  bootstrap-channel exit without historical migration prose or setup-secret
  recovery guidance.

## Ordering constraints

Consumes the proved RPC/recovery boundary and may proceed in parallel with the
cockpit story. Do not weaken the admin listener or introduce a routine-web exit

## Implementation notes

- Added `lockdown-enter` and `lockdown-exit` command handlers with local
  confirmation/reason validation, strict response validation, safe human/JSON
  projections, decimal-string event/generation values, and honest failure
  exit codes.
- Entry uses the authenticated ControlService and clears the credential store
  only after a committed active posture is validated. Exit uses only the
  loopback-constrained AdminService client and never reads the credential file.
- Added conformance tests for literal confirmation, post-commit credential
  clearing, bootstrap-channel output, redaction, and no-success-on-malformed
  responses.

## Verification

- `cd cli && npm test` passed (36 tests).
to make CLI tests easier. The feature remains open for integrated review until
both final checkpoints are done.
