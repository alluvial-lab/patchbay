---
id: epic-revocation-lifecycle-lockdown-trigger-exit-rpcs
kind: story
stage: implementing
tags: [security, protocol]
parent: epic-revocation-lifecycle-lockdown
depends_on: [epic-revocation-lifecycle-lockdown-core-posture]
release_binding: null
gate_origin: null
created: 2026-07-29
updated: 2026-07-29
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
