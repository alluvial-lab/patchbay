---
id: epic-revocation-lifecycle-lockdown
kind: feature
stage: drafting
tags: [security, foundation, ui]
parent: epic-revocation-lifecycle
depends_on: [epic-revocation-lifecycle-grant-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Security lockdown & bootstrap-channel exit

## Brief

SECURITY.md commits a durable **security lockdown** posture: reject new
Operations, mark affected runtime sessions stale, require fresh login, and
record the reason — durable across core restart (crash recovery replays the
log and lockdown remains in effect). Exit requires re-establishing bootstrap
trust **via the bootstrap channel** (local CLI/console, distinct from routine
web login — the channel distinction is load-bearing). None of this exists:
there is no lockdown decision surface at all.

This feature delivers lockdown end to end: the operator-facing lockdown
trigger (control-surface action), the core posture (durable event, rejection
gate on new Operations, session staleness marking, login invalidation), the
bootstrap-channel exit (admin-service path, since bootstrap already lives on
the loopback admin listener), and the **lockdown entry/exit audit producers**
— discharging the deferred obligation recorded in
`epic-observability-dogfooding-core-diagnostics` (the `AuditEventKind`
vocabulary already names them; the feature review deferred producers because
no decision surface existed — now it does).

It also owns the **cockpit emergency-controls UX**: the lockdown trigger and
the lockdown-state banner are net-new, safety-critical surfaces. **Mockups
REQUIRED at feature design** (`/ux-ui-design:screens
epic-revocation-lifecycle-lockdown`) — deferred from epic tier to keep
decomposition moving; this is the epic's one net-new UI surface, everything
else composes into existing views.

Does NOT cover: the revocation actions themselves (sibling features);
multi-operator lockdown scope (reserved).

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: sequences after grant-lifecycle (shared submission-authorization
  touchpoints; grant-lifecycle establishes the current enforcement pattern).

## Simplification opportunity

- The lockdown rejection gate should ride the same submission-authorization
  path as grant checks, not a parallel gate.
- Lockdown exit reuses the existing loopback admin/bootstrap channel rather
  than a new channel.

## Foundation references

- `docs/SECURITY.md` — revocation model (#5), Lockdown exit (durability,
  bootstrap-channel requirement, channel-distinction rationale)
- `docs/PROTOCOL.md` — Snapshots and streams (staleness), failure vocabulary
- `docs/UX.md` — emergency-control presentation (to be extended via mockups)
