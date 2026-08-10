---
id: cockpit-ux-polish-settings
kind: story
stage: done
parent: cockpit-ux-polish
depends_on: [cockpit-ux-polish-visual-contract]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Cockpit settings visibility preference

Add the first settings surface to the existing cockpit shell. The initial preference is tool-call visibility; it changes only presentation and never removes or rewrites transcript observations in the authoritative model.

## Checkpoint

- Add a settings view/overlay in `web-cockpit/src/ui/settings-view.ts` and compose it from `web-cockpit/src/ui/shell.ts`.
- Extend the existing per-authority-domain `CockpitShellPreferences` store with `showToolCalls: boolean`, defaulting to `true` when absent or malformed.
- Thread `showToolCalls` into `renderSessionDetail` without changing `PresentationModel`, protocol contracts, or transcript folding.
- Hide/collapse tool-call observations at render time only, with an explicit way to restore them; message and lifecycle observations remain visible.

## Acceptance evidence

- [x] Settings is reachable from the existing shell navigation without creating a parallel destination model.
- [x] Preference persistence is scoped to the existing authority-domain key and safely falls back to the default.
- [x] Toggling visibility does not alter the folded model, command delivery states, observation ordering, or reconnect behavior.
- [x] Keyboard and screen-reader users can identify the setting, its scope, and its current value.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-luna` high, direct implementation; settings is a bounded shell/detail presentation change.
- Review weight: thorough (caller override), feature review remains pending after integrated implementation.
- Files changed: `web-cockpit/src/ui/settings-view.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`.
- Tests added/removed: shell coverage for domain-scoped persistence, malformed/default-safe visibility, dialog semantics, toggle round-trip, and unchanged folded observations; `npm --prefix web-cockpit run build:types` and shell tests pass.
- Simplification: settings is an overlay over the existing destination shell; no new destination registry, protocol field, transcript model, or command state was introduced.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Review pass 2: modal teardown now restores every reusable background element's prior `inert` value across full shell renders; focus restoration resolves against the currently visible responsive opener. The Settings-close → mobile-Elicitation regression proves the reused sheet loses Settings' temporary inertness, opens, and receives focus after a desktop-to-mobile change.
- Review pass 2 accessibility evidence: axe runs against the actual `index.html` production mount before Settings, during the modal, and with the mobile Elicitation sheet open.
