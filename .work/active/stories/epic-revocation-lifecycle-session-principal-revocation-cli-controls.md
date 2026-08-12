---
id: epic-revocation-lifecycle-session-principal-revocation-cli-controls
kind: story
stage: done
tags: [security]
parent: epic-revocation-lifecycle-session-principal-revocation
depends_on: [epic-revocation-lifecycle-session-principal-revocation-core-state]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# CLI revocation and recovery controls

## Checkpoint

Expose the generated emergency controls as `revoke-all-sessions`,
`revoke-principal`, `revoke-endpoint`, and `revoke-device`, with bounded reason
codes, safe JSON, self-target credential cleanup, and truthful re-entry output.

This checkpoint owns Unit 4 in the parent feature. The parent design is
authoritative for command signatures, grammar, output safety, and the
self-lockout/reconciliation rules.

## Acceptance evidence

- CLI parsing rejects missing/duplicate targets, unknown flags, malformed reason
  codes, and extra positionals before network access.
- Confirmed revoke-all clears the caller's local credentials. Confirmed
  principal/endpoint/device revocation clears them exactly when the stored
  credential lies in the target scope; revoking another identity leaves them.
- Transport/unknown failure never claims success and explains that the action
  may have committed and `patchbay-cli login` is the reconciliation path.
- Human and JSON output contain only safe ids, counts, generation, and source
  event identity; secrets and raw operator-session ids never reach output or
  argv guidance.
- Tests prove trusted-host login after revoke-all receives fresh credentials and
  a higher core session generation; same-id endpoint/device enrollment remains
  refused after scope revocation.

## Ordering constraints

Consumes the stable core RPCs and may implement in parallel with the web
checkpoint. The consumed one-time `setup` secret is not a recovery mechanism;
do not advertise it as one.

## Implementation notes
- Execution capability: inline implementation; shared CLI parsing, credential storage, and generated Connect client paths were sufficient.
- Review weight: standard (project default).
- Files changed: `cli/src/commands/revocation.ts`, `cli/src/main.ts`, and `cli/tests/auth-commands.test.ts`.
- Tests added/removed: revoke-all safe generation/event output and credential clearing, selective self-target cleanup, and honest transport failure tests added.
- Simplification: endpoint and device commands share one target validator/request path while retaining distinct command grammar.
- Discrepancies from design: no interactive confirmation was added because the authoritative command signatures define direct scriptable controls; the owning lockdown feature remains responsible for cockpit confirmation UX.
- Adjacent issues parked: none.
- Verification: `cd cli && npm test` passed (33 tests).
