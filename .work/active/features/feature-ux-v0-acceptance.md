---
id: feature-ux-v0-acceptance
kind: feature
stage: drafting
tags: [prose, ux, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
---

# Feature: Define v0 web cockpit UX acceptance criteria

The docs name Claude-app-style continuity as a quality bar, but the first web cockpit needs actionable acceptance criteria for screens, states, and failure handling.

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
