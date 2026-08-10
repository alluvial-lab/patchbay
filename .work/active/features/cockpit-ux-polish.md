---
id: cockpit-ux-polish
kind: feature
stage: review
tags: [ux]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-10
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

## Design decisions
- **Settings entry point**: use a reversible overlay/sheet composed by the existing cockpit shell rather than a new settings navigation model — it keeps the current desktop rail/mobile overflow topology intact and makes the first preference easy to extend.
- **Tool-call visibility semantics**: make visibility a presentation-only preference with a `true` default; filter/collapse at render time while retaining all observations in the folded model and reconnect path — this preserves transcript fidelity and avoids a second source of truth.
- **Session-row context**: identity tuple first, human label second, cwd/project third, with one-line truncation and an explicit activity signal — labels remain scan aids and never replace verified target identity.
- **Delivery composition**: place `CommandState`, race explanation, failure/retry context, and cancel/interrupt affordances inside one instruction card with a reserved action slot — this removes box noise and prevents state-transition layout shifts without changing protocol semantics.
- **Review posture**: direct source mapping only; no nested advisory agent was used because the delegated task forbids recursion. The prominent surface is risk-reduced through four committed option mocks, existing component reuse, and explicit acceptance evidence in child stories.

## Architectural choice

Three approaches were considered:

1. **Extend the current cockpit shell in place (chosen)** — add a settings overlay, refine the existing session-row DOM/CSS, and compose delivery inside the current instruction/message path. This optimizes for least irreversible change, preserves desktop two-pane/mobile drill-in behavior, and keeps the shared presentation layer as the source of truth.
2. **Create a parallel “polished cockpit” surface** — build a new shell and migrate the current session/detail views behind it. This could enable a cleaner visual reset, but duplicates navigation, preference persistence, accessibility fixes, and state-binding seams; it is rejected as unnecessary product risk.
3. **Promote settings and delivery into new domain-level models** — add protocol-facing visibility/delivery concepts. This would make UI composition look explicit but would violate surface-neutrality, transcript fidelity, and the existing registry-derived boundary; it is explicitly rejected.

The chosen option keeps behavior and contracts in place while changing only surface composition and local presentation preferences.

## Mockups

- Screens: `.mockups/screens/cockpit-ux-polish/index.html`
- Options: `option-1.html` (Command deck), `option-2.html` (Instrument console), `option-3.html` (Reading room), `option-4.html` (Mobile switchboard)
- Selected: **option-1 — Command deck**, with its responsive two-pane/drill-in behavior — 2026-08-10
- Rationale: it is the least-irreversible extension of the current shell, keeps identity and delivery visible in one scan, and uses a progressive-disclosure settings overlay without inventing another destination model.

## Implementation Units

### Unit 1: Presentation-only settings preference
**Files**: `web-cockpit/src/ui/settings-view.ts` (new), `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/session-detail.ts`
**Story**: `cockpit-ux-polish-settings`

```ts
export interface CockpitShellPreferences {
  sessionsPanelCollapsed: boolean;
  showToolCalls: boolean;
}

export interface SettingsViewOptions {
  showToolCalls: boolean;
  onShowToolCallsChange(next: boolean): void;
}

export function renderSettingsView(
  document: Document,
  options: SettingsViewOptions,
): HTMLElement;

export interface SessionDetailOptions {
  // existing fields remain; this is presentation-only
  showToolCalls?: boolean;
}
```

**Implementation Notes**:
- Extend the existing authority-domain preference key; malformed or absent values default to `showToolCalls: true`.
- Pass the preference into the detail renderer and suppress/collapse only tool-call presentation nodes. Never delete observations, alter command association, or alter reconnect/model folding.
- Keep the overlay keyboard reachable, labeled as a presentation preference, and composed through the existing `CockpitDestination`/shell controls rather than a second navigation registry.

**Acceptance Criteria**:
- [ ] Toggling the control immediately changes visible tool-call presentation and can restore it.
- [ ] Refresh/recreate of the shell retains the preference per authority domain; other domains remain unaffected.
- [ ] No protocol contract, model type, canonical state, or transcript ordering changes.

### Unit 2: Identity-first, mobile-safe session rows
**Files**: `web-cockpit/src/ui/session-list.ts`, `web-cockpit/src/ui/shell.css`
**Story**: `cockpit-ux-polish-session-rows`

```ts
export function renderSessionRow(
  document: Document,
  session: SessionView,
  selected: boolean,
  onSelect: (session: SessionView) => void,
  adapter?: AdapterView,
): HTMLButtonElement;

export function renderSessionStatus(
  document: Document,
  session: SessionView,
): HTMLElement;
```

**Implementation Notes**:
- Preserve the existing verified identity formatter and session selection key. Make the visual order identity → label → cwd/project → composed status/attention explicit in the DOM.
- Keep `SessionConnectivityState` dominant over `SessionActivityState`; stale/unknown/offline/failed activity is visibly de-emphasized but not hidden. Keep `needsYou` as attention metadata.
- Use overflow-safe, one-line cwd/context treatment at narrow widths; do not shorten the identity tuple or let human labels replace it.

**Acceptance Criteria**:
- [ ] Desktop and mobile rows show the full identity tuple before intent-bearing labels/actions.
- [ ] Cwd/project context truncates without horizontal page overflow and remains available as accessible text.
- [ ] Selection, needs-you, adapter diagnostics, and canonical status bindings continue to render through the existing primitives.

### Unit 3: Stable instruction-card delivery composition
**Files**: `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/operation-delivery.ts`, `web-cockpit/src/ui/shell.css`
**Story**: `cockpit-ux-polish-delivery-cards`

```ts
export function renderOperationDelivery(
  document: Document,
  command: CommandView,
  actions?: OperationDeliveryActions,
  lockdownActive?: boolean,
): HTMLElement;

export function renderInstructionCard(
  document: Document,
  command: CommandView,
  actions?: OperationDeliveryActions,
  lockdownActive?: boolean,
): HTMLElement;
```

**Implementation Notes**:
- Refactor the existing operator command/message path to render instruction text, stable target identity, delivery state, terminal-race explanation, failure vocabulary, and contextual actions in one semantic card.
- Reserve a fixed/minimum-height action slot for cancel/interrupt controls on every card state. Empty slots are intentional; they prevent accepted/delivered/running transitions from moving surrounding transcript content.
- Reuse the canonical `renderOperationDelivery`, failure mapping, retry-safety indicator, and lockdown disabling. Never infer authority or retry safety from visual availability.

**Acceptance Criteria**:
- [ ] Accepted, delivered, running, completed, failed, cancelled, expired, and superseded command states remain registry-derived and visually distinct from liveness.
- [ ] The interrupt/cancel affordance appears in the reserved slot when supported and remains keyboard/focus accessible when present.
- [ ] State changes, failure details, and race explanations do not create a second floating delivery box or cause horizontal/vertical layout shifts at mobile widths.

## Implementation Order

1. `cockpit-ux-polish-visual-contract` — lock the selected mock's DOM seams and verify the shared presentation vocabulary.
2. `cockpit-ux-polish-settings` — add the presentation-only preference and thread it through the existing shell/detail composition.
3. `cockpit-ux-polish-session-rows` — establish identity-first, cwd-safe rows using the same shell and status primitives.
4. `cockpit-ux-polish-delivery-cards` — integrate delivery into instruction cards after the shared shell/detail seams are stable.

The parent feature remains the ownership and review bundle; child stories are checkpoints, not separate implementation agents.

## Simplification

- Reuse `session-list.ts`, `session-detail.ts`, `operation-delivery.ts`, `shell.ts`, `shell.css`, `tokens.css`, and `components.css`; no parallel cockpit, transcript, delivery state, or settings model.
- Keep `PresentationModel` and generated contracts unchanged; visibility is a local view preference, not a new protocol field.
- Retain the existing `renderOperationDelivery` and failure/retry components, moving composition rather than duplicating state styling.
- Do not edit the locked design system; use its existing tokens and state-binding primitives.

## Testing

- **Settings/presentation tests**: extend `web-cockpit/tests/shell.test.ts` and `web-cockpit/tests/model.test.ts` to prove default/persisted visibility, authority-domain scoping, and unchanged folded observations/ordering.
- **Session-row interface tests**: extend `web-cockpit/tests/shell.test.ts` for identity-first DOM order, cwd truncation/no overflow at mobile width, selected/needs-you states, and stale connectivity dominance.
- **Delivery regression tests**: extend `web-cockpit/tests/shell.test.ts` and delivery-focused UI tests for every representative canonical state, race/failure labels, lockdown disabling, stable reserved action space, and no duplicate delivery box.
- **Conformance check**: run the existing presentation registry check and the web-cockpit type/test suite; no new protocol vector is needed because this feature changes no protocol semantics.

## Risks

- **Riskiest assumption**: tool-call visibility can be implemented as a render preference without breaking observation-to-command association. Mitigation: filter only final DOM presentation and assert the folded model/ordering is unchanged.
- **Layout regression risk**: fixed action space may feel too sparse on very small screens. Mitigation: use a minimum slot that collapses to a full-width action row below the mobile breakpoint, while preserving the reserved vertical rhythm.
- **Preference drift risk**: adding a field to existing local storage can encounter malformed prior values. Mitigation: parse defensively and default to visible, never block cockpit startup.
- **Aesthetic overreach risk**: four options could imply a new visual language. Mitigation: selected option uses existing tokens/components; this feature does not alter `.mockups/design-system/`.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-luna` high, one feature-owning direct implementation with ordered child checkpoints; no nested delegation per caller posture.
- Review weight: thorough (explicit caller override); feature is intentionally left at `stage: review` for fresh review.
- Child commits: `5294dfa` visual contract, `23531a3` settings, `59dbadd` session rows, `16d0439` delivery cards.
- Files changed: `web-cockpit/src/ui/settings-view.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/operation-delivery.ts`, `web-cockpit/src/ui/session-list.ts`, `web-cockpit/src/ui/shell.css`, and `web-cockpit/tests/shell.test.ts`.
- Integrated verification: `npm --prefix web-cockpit test` passed (120 tests); `node contracts/scripts/check-presentation.mjs` passed (5 registries, axe-core accessibility); type build and browser bundle passed.
- Acceptance: selected Option 1 topology remains the current shell; settings is presentation-only and authority-domain scoped; session rows preserve identity/status/needs-you; instruction cards compose canonical delivery state with a reserved action slot; transcript folding and protocol contracts are unchanged.
- Simplification: reused existing shell, session/detail, delivery, status, failure, and registry-derived presentation primitives; no parallel transcript, delivery, settings, protocol state, or model was added.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Extension pressure classification

- **Committed v0.1.0 surface behavior**: clearer identity, delivery, and responsive presentation in the existing web cockpit, with canonical registry bindings unchanged.
- **Reserved seam preserved**: settings preference storage remains local to the control surface and can grow without protocol fields; skins/layouts remain surface-declared per `docs/UX.md`.
- **Explicitly rejected**: protocol-level transcript filtering, a second transcript/delivery/settings model, and adapter- or core-owned UI variants. These would require a separate scope act and are not hidden gaps.

## Review (2026-08-10) — thorough pass 1 fixes

**Verdict**: Request changes addressed; retained at `stage: review` for the required thorough convergence pass.

**Accepted findings fixed**:
- instruction cards now merge only on exact typed command correlation and retain the source Observation text; uncorrelated/reconnect transcript entries remain separate;
- the tool-call preference suppresses provenance-tagged tool activity detail across rows, header, banners, and timeline while preserving canonical/runtime activity and the folded transcript;
- correlated cancel/interrupt and late result ordering now yields stable presentation-only race explanations, with state-valid capability-gated actions and inert terminal slots;
- Settings now has modal focus containment, Escape and visible-opener restoration, inert background, semantic toggle/list/session-row controls, a conventional icon, and production-shell axe coverage;
- mobile tabs reserve safe-area-aware list/detail/composer space, expose expanded-state semantics, clear More correctly, and keep mobile action targets at least 44px.

**Verification**:
- `npm --prefix web-cockpit test` — passed: build/typecheck/browser bundle plus 127 tests;
- `npm --prefix contracts/ts run check:presentation` — passed: 5 registries, contrast, showcase bindings, and axe-core scan;
- `git diff --check` — passed.

**Notes**: review weight `thorough`, pass 1; direct-read/fix only per caller prohibition on nested reviewers. No protocol, generated contract, foundation assertion, selected mock, or feature scope changed.
