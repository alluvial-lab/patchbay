---
id: feature-v0-web-cockpit
kind: feature
stage: drafting
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-web-server, feature-v0-presentation-component-layer]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-17
---

# Feature: Responsive web cockpit

## Brief

Build the responsive web cockpit — the operator's primary control surface and the v0.1.0 product center. This is the "better than terminal" phone experience: session list with liveness/delivery badges, composer for sending prompts and instructions, command delivery timeline with failure states, and reconnect/stale/offline banners. The quality benchmark is Claude-app-style remote control continuity.

The cockpit runs the shared TypeScript operator domain in the browser (protocol client, delivery/reconnect state machines, presentation model) as a client of the web server. It must meet the surface-neutral UX conformance floor defined in `docs/UX.md`: present every canonical protocol state honestly, separate session liveness from command delivery, show stable target identity before allowing submission, and distinguish denial from unsupported from revoked.

The cockpit is the first conformant instance of the conformance floor. Mockups are deferred to a named follow-on per the UX design decision (mocking inside the criteria feature would silently privilege one visual instance and work against surface-neutrality); this feature's `feature-design` pass picks up the mockup work here, inheriting the design-system pipeline (`palette` → `components`) from `feature-v0-presentation-component-layer`.

## Mockups

- Design system: `.mockups/design-system/` (see `feature-v0-presentation-component-layer`)
- Shell screen: `.mockups/screens/feature-v0-web-cockpit/`
  - **Selected: option-2 (Identity-forward)** — locked 2026-07-16. Generous session rows (label + project dominant, identity + status as metadata), sidebar header actions for spawn/attach, filter search.
  - **Responsive IA (committed A / reserved B):** desktop is two-pane (list + live session-detail side-by-side); mobile is drill-in (list is the home, tap a session → full-screen detail with back button). B (drill-in) is both the reserved seam AND the natural mobile mode — promotion to desktop drill-in is additive (container change), not a rebuild.
  - Detail-pane header hidden on desktop (redundant with the active sidebar row); kept on mobile (drill-in needs back button + which-session context).
  - option-2.html is self-contained (inlined tokens+components) and interactive (mobile drill-in works via tap/back).
  - Session detail screen: `.mockups/screens/feature-v0-web-cockpit/detail/session-detail.html` — locked 2026-07-17. Chat-aligned timeline (operator right / agent left, capped 560px left-side content width), markdown rendering in agent bubbles (the mobile-readability differentiator), delivery state as a compact badge below each message (tap to expand full state history + LSNs as debug detail), binary approval = direct buttons (no option-list), multi-option question = select-one radio + free-text option + answer-and clarification, grouped multi-question card (N independent single-answer Elicitations as one visual card — v0.1.0-compatible; multi-answer contract is a reserved seam). Mobile: bottom-sheet for elicitations, fixed composer, page scroll.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: the end of the phone-usable critical path (core → protocol-seam → web-server → web-cockpit). This is the layer that makes the operator's phone piloting real.

## Foundation references

- `docs/UX.md` — surface-neutral conformance floor, v0 web cockpit instance, required screens and fields, delivery-state separation, reconnect/stale/offline banners
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain, presentation model
- `docs/PROTOCOL.md` — CommandState, SessionConnectivityState, SessionActivityState, ElicitationState, failure vocabulary
- `docs/SPEC.md` — v0.1.0 performance posture (qualitative responsiveness floor: "feels responsive under normal single-operator use")
- `feature-ux-v0-acceptance` (done) — the UX conformance floor design this feature implements
