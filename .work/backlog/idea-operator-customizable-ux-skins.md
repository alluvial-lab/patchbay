---
id: idea-operator-customizable-ux-skins
created: 2026-07-06
updated: 2026-07-07
gate_origin: null
release_binding: null
tags: [ux, surface, extensibility]
---

# Parked idea: operator-customizable UX skins/layouts

An operator may want a Patchbay control surface to *look and feel* like a different harness or tool — a "Codex-style" cockpit, a "Claude-style" cockpit, a CLI-flavored layout, an Antigravity-style surface, etc. — rather than a single fixed Patchbay visual language. The UX layer should be customizable above the conformance floor, the way adapters are pluggable above the protocol.

## Context

- `feature-ux-v0-acceptance` (done 2026-07-06) established **surface-neutrality** as a principle symmetric to adapter-neutrality: surface-specific presentation is a surface-declared feature, not a core UX primitive. The v0 web cockpit is the *first conformant instance* of a surface-neutral conformance floor, not a pinned visual design.
- That feature named a **shared presentation-component layer** as a reserved architectural seam — the layer that binds canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`) and is skin-able via design tokens. That seam is what makes this idea possible: swap tokens + override presentational primitives, keep the state-binding.
- The conformance floor is behavioral + state-binding only; it deliberately does not mandate design tokens or a visual language, so skins are a reserved seam above the floor.
- `docs/UX.md` "Reserved seams" already lists "Operator-customizable skins/layouts" and "Design tokens / visual language" as reserved.

## Why parked

- v0 ships one conformant surface (the web cockpit) with one visual language. Customizable skins are breadth, not a v0 requirement.
- The enabling seam (the shared presentation-component layer) is itself deferred — skins can't be meaningfully customizable until that layer exists and exposes a token surface. Parking this idea until the seam is built avoids speculating about a skinning API before the substrate it would skin exists.
- The extensibility discipline (`feature-extension-seams-non-foreclosure`) is the right place to ensure skins stay a reserved seam, not a v0 obligation.

## What this idea should influence

- The **shared presentation-component layer implementation** (a reserved follow-up of `feature-ux-v0-acceptance`) should expose a design-token surface and keep presentational primitives overridable, so a skin is "tokens + primitive overrides," not a fork of protocol semantics.
- The v0 web cockpit mockup pass (v0 surface-design follow-up) should not accidentally bake a single visual language into the floor; it designs one *instance*, and the floor stays skin-agnostic.
- `feature-extension-seams-non-foreclosure` should treat "operator-customizable presentation above the conformance floor" as a reserved extension seam.
- A skin must never fork protocol semantics from its surface — the same constraint already applied to the Expo app (`docs/UX.md`).

## Keep parked until

The operator identifies a concrete need for a non-default skin (e.g. an operator wants a CLI-flavored web layout, or a Codex-style presentation), AND the shared presentation-component layer exists with a token surface to skin. At that point, scope it as a feature and route through design. Promotion is a registry/classification update, not a reversal — the floor was always skin-agnostic.
