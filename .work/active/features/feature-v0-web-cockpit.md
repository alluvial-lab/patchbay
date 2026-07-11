---
id: feature-v0-web-cockpit
kind: feature
stage: drafting
tags: [ux, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-web-server]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Responsive web cockpit

## Brief

Build the responsive web cockpit — the operator's primary control surface and the v0.1.0 product center. This is the "better than terminal" phone experience: session list with liveness/delivery badges, composer for sending prompts and instructions, command delivery timeline with failure states, and reconnect/stale/offline banners. The quality benchmark is Claude-app-style remote control continuity.

The cockpit runs the shared TypeScript operator domain in the browser (protocol client, delivery/reconnect state machines, presentation model) as a client of the web server. It must meet the surface-neutral UX conformance floor defined in `docs/UX.md`: present every canonical protocol state honestly, separate session liveness from command delivery, show stable target identity before allowing submission, and distinguish denial from unsupported from revoked.

The cockpit is the first conformant instance of the conformance floor. Mockups are deferred to a named follow-on per the UX design decision (mocking inside the criteria feature would silently privilege one visual instance and work against surface-neutrality); this feature's `feature-design` pass should pick up the mockup work.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: the end of the phone-usable critical path (core → protocol-seam → web-server → web-cockpit). This is the layer that makes the operator's phone piloting real.

## Foundation references

- `docs/UX.md` — surface-neutral conformance floor, v0 web cockpit instance, required screens and fields, delivery-state separation, reconnect/stale/offline banners
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain, presentation model
- `docs/PROTOCOL.md` — CommandState, SessionConnectivityState, SessionActivityState, ElicitationState, failure vocabulary
- `docs/SPEC.md` — v0.1.0 performance posture (qualitative responsiveness floor: "feels responsive under normal single-operator use")
- `feature-ux-v0-acceptance` (done) — the UX conformance floor design this feature implements
