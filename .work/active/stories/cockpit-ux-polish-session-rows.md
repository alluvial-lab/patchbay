---
id: cockpit-ux-polish-session-rows
kind: story
stage: done
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

- [x] Existing session-list tests cover identity-first order, selected/needs-you styling, stale dominance, and cwd overflow behavior.
- [x] DOM/CSS structure checks keep identity/context nodes bounded and preserve full cwd text through accessible labeling; they do not claim measured mobile page geometry.
- [x] Re-labelling a session does not change its selection key or target identity.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-luna` high, direct implementation after the visual-contract checkpoint.
- Review weight: thorough (caller override), feature review remains pending after integrated implementation.
- Files changed: `web-cockpit/src/ui/session-list.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`.
- Tests added/removed: accessible full-context/title coverage for long cwd values, stable selection-key assertion, identity-first context assertion; shell tests pass after type build.
- Simplification: reused the existing identity formatter, status primitive, and row selection key; no new session state or label-derived identity was added.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Review pass 2 evidence clarification: session-row coverage proves identity-first DOM order, bounded one-line context styles, accessible full cwd text, and stable selection identity. It does not claim browser-measured overflow or geometry.
