---
id: epic-observability-dogfooding-cockpit-diagnostics
kind: feature
stage: drafting
tags: [observability, dogfooding, ui]
parent: epic-observability-dogfooding
depends_on: [epic-observability-dogfooding-core-diagnostics]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Adapter diagnostics forwarding + cockpit surfacing

## Brief

The cockpit is the surface the operator actually has open while dogfooding,
but it shows nothing about adapter health: no connection state, no diagnostic
events, no adapter-side errors. Adapter failures are invisible until a session
silently goes stale.

This feature closes that gap end to end. The pi-adapter reports diagnostics
to the core as payload — promoting the reserved adapter-specific diagnostics
seam — the core records them (extending the core-diagnostics surface), and the
cockpit presents adapter health, connection state, and recent diagnostic
events within its existing views. One capability, three layers; kept as a
single feature because the contract shape, core recording, and presentation
must agree on the same diagnostic vocabulary.

The presentation composes into existing cockpit surfaces and reuses the
existing CommandState/status presentation patterns — no net-new screen, so no
mockups at epic tier per the mockup-first convention's skip rule. If feature
design finds the diagnostics presentation wants a dedicated view rather than
composition, it falls back to `/ux-ui-design:screens` at that point.

It does NOT cover: the adapter-local durable log file
(`epic-observability-dogfooding-adapter-log-sink`), the base core-diagnostics
query surface (`epic-observability-dogfooding-core-diagnostics`), a dedicated
health/status dashboard, or the delivery-trace timeline UI (both reserved).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: consumer of `epic-observability-dogfooding-core-diagnostics`
  (its recording/query substrate) and producer of the adapter-diagnostics
  contract addition. Parallel with the CLI consumer. Priority 4 in the epic's
  seed order, but the highest dogfooding value — the cockpit is the primary
  inspection surface.

## Simplification opportunity

- Gives the cockpit an honest adapter-health signal, replacing the current
  implicit "silence until stale" behavior — sessions going `stale` with no
  visible cause is a presentation-honesty gap this feature closes at the
  source.
- Adapter diagnostic codes map onto the PROTOCOL failure vocabulary at the
  Patchbay boundary; design should reuse that mapping rather than inventing a
  parallel cockpit-only vocabulary.

## Foundation references

- `docs/PROTOCOL.md` — Payload; failure vocabulary (adapter diagnostic codes
  extension seam); extension seams registry
- `docs/UX.md` — delivery-state floor, presentation honesty
- `docs/SPEC.md` — post-v0.1.0 observability scope
- `docs/ADAPTER-PI.md` — adapter capability declarations

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will
fill in the diagnostics payload shape, recording path, and presentation
units. -->
