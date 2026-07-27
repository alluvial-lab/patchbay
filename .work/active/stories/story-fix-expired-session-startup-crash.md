---
id: story-fix-expired-session-startup-crash
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Expired operator session crashes cockpit startup instead of showing login

## Symptom

Operator report (2026-07-27): after ~9h away, the cockpit showed
`cockpit_startup_failed` / "CSRF token request failed (403)" instead of the
login page.

## Root cause

Normal session expiry mishandled. Web operator sessions have an 8h TTL
(`web-server/src/sessions.ts` `DEFAULT_SESSION_TTL_MS`); the operator's
session expired while away (audit index shows `session_renewed` while active,
then nothing). The `/csrf-token` route correctly returns 403
(`session_expired`) for a known-but-inactive session — but
`startCockpit` (`web-cockpit/src/main.ts`) only treated **401** as
"needs login" and rethrew everything else into `renderStartupFailure`.
Server semantics: 401 = no/unknown session; 403 = known-but-expired/revoked.
Both mean "log in again".

## Fix approach

One-branch change in `startCockpit`: treat both 401 and 403 from the CSRF
token request as login-required and run the normal login flow. Server
responses unchanged (the 401/403 distinction carries useful audit meaning).

## Regression test

`web-cockpit/tests/main.test.ts` — startup with /csrf-token answering 403
`session_expired` renders the login form (no failure banner), and a
successful login proceeds to the composed cockpit (token re-fetched).

## Implementation notes (2026-07-27)

- **Files changed**: `web-cockpit/src/main.ts` (401+403 → login flow),
  `web-cockpit/tests/main.test.ts` (regression test).
- **Confirmation**: new test passes; full suite 67/67 green; fix verified
  present in the built browser bundle. Live confirmation pending operator
  reload (login page should appear instead of the failure banner; after
  login the cockpit works normally).
- **Bounded inline review verdict**: minimal single-branch change; preserves
  the server's 401/403 semantic distinction; no test weakened.
- **Note**: sessions expire after 8h of non-use — the operator will see the
  login page after long absences; that is the intended security posture, now
  with the intended UX.

## Operator confirmation (2026-07-27)

Confirmed: expired session now presents the login page; login succeeds and
the cockpit loads normally. Story closed to done.
