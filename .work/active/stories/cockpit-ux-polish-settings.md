---
id: cockpit-ux-polish-settings
kind: story
stage: implementing
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

- [ ] Settings is reachable from the existing shell navigation without creating a parallel destination model.
- [ ] Preference persistence is scoped to the existing authority-domain key and safely falls back to the default.
- [ ] Toggling visibility does not alter the folded model, command delivery states, observation ordering, or reconnect behavior.
- [ ] Keyboard and screen-reader users can identify the setting, its scope, and its current value.
