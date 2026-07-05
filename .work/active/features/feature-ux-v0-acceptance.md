---
id: feature-ux-v0-acceptance
kind: feature
stage: drafting
tags: [ux, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-05
gate_origin: null
release_binding: null
---

# Feature: Define v0 web cockpit UX acceptance criteria

The docs name Claude-app-style continuity as a quality bar, but the first web cockpit needs actionable acceptance criteria for screens, states, and failure handling.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes real UX design choices: required v0 screens and navigation, session detail / message timeline behavior, and composer requirements. These are UX architecture decisions (what screens, what navigation pattern, what timeline model), not just writing criteria. The `feature-design` lane can invoke the ux-ui-design skills (`screens`, `flows`) for the design pass. The prose-author black-box test should have caught this originally.

## Scope

- Required v0 screens and navigation.
- Session list fields and badges.
- Session detail / message timeline behavior.
- Composer requirements.
- Command delivery timeline and failure states.
- Reconnect/stale/offline banners.
- Multi-device continuity expectations.
- Empty/error/loading states.

## Acceptance criteria

- `docs/UX.md` separates session liveness states from command delivery states.
- `docs/UX.md` defines v0 required screens and visible fields.
- UX text references canonical protocol states rather than maintaining a divergent state list.
- The web cockpit can be designed without guessing what must be visible before sending a command.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
