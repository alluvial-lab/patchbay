---
id: cockpit-ux-polish
kind: feature
stage: drafting
tags: [ux]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Cockpit UX polish

## Brief
Consolidate the three dogfooding UX ideas into a mockup-first cockpit polish feature. Absorbed findings:

- `idea-cockpit-settings-section`: add a settings area beginning with a tool-call visibility toggle while preserving transcript fidelity in the core.
- `idea-session-list-row-redesign`: establish explicit session-row hierarchy and stable, mobile-safe cwd presentation without hiding activity state.
- `idea-delivery-line-layout-stability`: fold delivery state into instruction cards and reserve stable space for interrupt affordances to prevent layout shifts and separate-box noise.

This supports the v1 mobile-responsive/switch-quality must.

## Simplification opportunity
Reuse the existing shared presentation primitives and state registry; improve hierarchy and stable dimensions without creating a parallel transcript, delivery-state, or settings model.
