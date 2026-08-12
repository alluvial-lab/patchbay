---
id: epic-revocation-lifecycle-lockdown-trigger-exit-rpcs
kind: story
stage: done
tags: [security, protocol]
parent: epic-revocation-lifecycle-lockdown
depends_on: [epic-revocation-lifecycle-lockdown-core-posture]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-29
updated: 2026-07-30
---

# Expose authorized lockdown entry and bootstrap-only exit

## Checkpoint

Land Units 3–4 from the parent design: authorize and atomically audit the
operator-facing entry RPC, expose redacted security snapshots, enforce lockdown
on non-Operation mutations, and implement the credential-independent exit only
on the loopback `AdminService`. Bridge entry/security reads and the existing
grant-revocation control into the web server, but never bridge exit.

## Acceptance evidence

- Entry requires a reverified compound issuer and live
  `session-management`/authority-domain grant under the shared
  `CoreDecisionGate`; a session-scoped grant denies and audits without a posture
  event.
- A committed entry source and `LockdownEntered` audit are atomic, invalidate all
  existing operator sessions, and report the replayed stale-session count.
- Routine web login creates a higher-generation read-only session but cannot
  clear posture. Submit/QueryDiagnostics and all security mutations remain
  server-rejected; Subscribe and snapshots remain readable.
- Exit is registered only on the separately bound loopback admin listener,
  needs no operator credential, commits source plus `LockdownExited` audit
  atomically, and remains locked on storage failure.
- The real listener test enters, restarts with persistent storage, exits through
  admin with no credential file, logs in at a higher generation, obtains a new
  authoritative adapter signal, and accepts a new Operation.
- Web route inventory and integration tests prove no browser-reachable exit or
  generic admin proxy exists.

## Ordering constraints

Depends on the generated event and core posture checkpoint. Land and prove the
self-lockout/re-entry path before cockpit or CLI polish. Hold the composition-root
`CoreDecisionGate` across catch-up, issuer re-verification, authorization,
append, and result construction; a service-local mutex is not sufficient.

## Implementation notes

- `EnterSecurityLockdown` now re-verifies the compound issuer under the shared
  gate, requires the authority-domain `session-management` grant, and atomically
  appends the posture source plus typed `LockdownEntered` audit.
- `ExitSecurityLockdown` is implemented only by `AdminService`; it requires no
  operator credential, uses the loopback-admin channel enum, and atomically pairs
  the exit source with `LockdownExited`. Password login and browser ControlService
  expose no exit path.
- Security snapshots materialize redacted operator-session, control-surface, and
  grant summaries. Lockdown blocks security mutations with `FAILED_PRECONDITION`
  while preserving snapshot/subscription/password-read paths.
- The web server bridges entry, security snapshots, and grant revocation with the
  existing CSRF/session guards. Entry encodes the committed response before
  revoking all local browser sessions; unknown transport outcomes fail closed.

## Verification

- `cargo test -p patchbay-core-server --test lockdown_recovery` passed, including
  entry, restart replay, credential-independent admin exit, and higher-generation
  issuance.
- `cd web-server && npm test` passed (31 tests), including CSRF, local session
  invalidation, security snapshot read, grant bridge, and no browser admin route.
