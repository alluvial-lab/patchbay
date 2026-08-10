---
id: cockpit-ux-polish-session-rows
kind: story
stage: implementing
parent: cockpit-ux-polish
depends_on: [cockpit-ux-polish-visual-contract]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Cockpit session-row hierarchy

Refine the existing session list row so an operator can switch quickly without losing identity, cwd context, or activity state on narrow screens.

## Checkpoint

- Update `web-cockpit/src/ui/session-list.ts` and the session-row rules in `web-cockpit/src/ui/shell.css`.
- Render the verified identity tuple first (`adapter · deployment scope · runtime id · generation`), then the human label, then a single-line cwd/project context with safe truncation.
- Keep connectivity dominant over activity: stale, offline, unknown, and failed rows must never look live, while working/idle remains visible as secondary information.
- Keep `needs you` attention separate from connectivity and activity; do not encode it as a new protocol state.

## Acceptance evidence

- [ ] Existing session-list tests cover identity-first order, selected/needs-you styling, stale dominance, and cwd overflow behavior.
- [ ] A mobile-width DOM/CSS check shows the identity and activity remain readable without horizontal page overflow.
- [ ] Re-labelling a session does not change its selection key or target identity.
