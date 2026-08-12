---
id: epic-revocation-lifecycle-lockdown-cockpit-shell-ui
kind: story
stage: done
tags: [security, ux]
parent: epic-revocation-lifecycle-lockdown
depends_on: [epic-revocation-lifecycle-lockdown-trigger-exit-rpcs]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-29
updated: 2026-07-30
---

# Realize the signed-off cockpit shell and lockdown Security view

## Checkpoint

Land Unit 5 from the parent design using
`.mockups/screens/epic-revocation-lifecycle-lockdown/option-hybrid.html` as UI
authority. Replace the sidebar-only cockpit topology with the desktop icon rail
and destination punch-out model, mobile bottom tabs/More and drill-in behavior,
and the single-column Security destination. Add the inline lockdown banner,
read-only presentation, and exact two-step Arm → type `LOCKDOWN` trigger ritual.
Do not re-mock or replace the production session-detail component.

## Acceptance evidence

- Desktop rail, active left accent, Sessions panel punch-out, Security view,
  planned destinations, and persisted panel state match the signed-off hybrid;
  there is no second panel chevron.
- Mobile uses equal-width Sessions/Security/More tabs, top active accent,
  safe-area spacing, and list→detail back navigation around the same detail
  component.
- Snapshot/event folds render the persistent inline banner with safe reason,
  timestamp, domain, and literal `patchbay-cli lockdown-exit` guidance.
- During lockdown every composer, delivery action, Elicitation answer,
  diagnostics refresh, and security mutation is disabled with an explicit lock
  reason; a client-side dispatch guard backs up the UI while server rejection
  remains authoritative.
- Entry makes no request until both dialogs complete and the confirmation field
  equals `LOCKDOWN`; cancelling either dialog makes no request. No exit affordance
  or transport method exists in browser code.
- DOM/property/accessibility tests cover destination semantics, narrow/wide
  topology, alert announcement, focus/labels, reduced motion, and
  stale-never-live presentation.

## Ordering constraints

Consumes the stable trigger/security-snapshot bridge. Keep protocol-state CSS in
the registry-derived design-system layer and topology CSS in `shell.css`; do not
fork state names or hand-copy generated DTOs.

## Implementation notes

- Ported the signed-off hybrid into the production shell: icon-only desktop
  rail with left-accent active state, Sessions panel punch-out, persisted
  namespaced collapse preference, mobile bottom tabs/More, and list-to-detail
  drill-in around the existing `session-detail` component.
- Added `PresentationModel.lockdown` folding from snapshot/source events and the
  persistent inline danger banner. Session and Elicitation controls become
  explicitly read-only during lockdown; stale sessions cannot render live.
- Added the Security destination and Arm → safe reason → exact `LOCKDOWN`
  confirmation ritual. Browser code has no exit method or admin route; the
  recovery instruction is presentation-only.
- Promoted stale/overlay/motion values into the design-system token/component
  sources and added CSRF interception for security mutations.

## Verification

- `cd web-cockpit && npm test` passed (72 tests).
- Bundle verification passed after the final UI change: `npm run build`, then
  the relevant lockdown/banner/CSRF/navigation strings were present in
  `dist/assets/cockpit.js`.
