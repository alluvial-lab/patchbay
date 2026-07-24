---
id: feature-cockpit-icon-set
kind: feature
stage: done
tags: [ux, design-system, ui, fast-follower]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-23
updated: 2026-07-24
research_origin: null
---

# Feature: adopt an icon set for the cockpit chrome

**Promoted 2026-07-24** into the pre-release fix wave. UI surface: design-system
icon primitive in `.mockups/design-system/` plus application across the cockpit
chrome; mocking happens at `feature-design` tier per the tier-ordering rule.

Surfaced in live use (2026-07-23): the operator noted the composer's action
buttons should be icons, not text — a paperclip for Attach, an arrow for Send
("the typical paperclip button, and Send as a typical arrow button"). The
project currently has **no icon set** — `components.css` uses text labels plus
a few ad-hoc unicode glyphs and one inline SVG (the paperclip added in
`4a903cd` as a stopgap).

## Shape

- **Pick an icon set.** RESOLVED (2026-07-23): the operator's parallel
  project `projects/SNC/platform` uses **Lucide** (`lucide-react`, 29 imports)
  — use Lucide for the cockpit too, for cross-project consistency. Patchbay's
  cockpit is vanilla TS with no build step, so use Lucide's SVG paths directly
  as inline SVG (no `lucide-react`, no icon font, no npm icon package) — the
  same shape as the paperclip stopgap in `4a903cd` (which is already the
  Lucide paperclip path).
- **Integrate it into the design system** (`.mockups/design-system/`), not ad
  hoc per-button: an icon primitive (`.icon`, size tokens, stroke conventions)
  in `components.css`, passing `check-presentation` (the conformance floor
  must bind any new primitive, not bypass it).
- **Apply it:** composer actions (Attach → paperclip, Send → arrow-up),
  sidebar header actions (spawn/attach → plus/link), back buttons, disclosure
  carets, the delivery-badge expand affordance, and the Cancel/Interrupt
  contextual actions.

## Notes

- The stopgap paperclip (`4a903cd`) is an inline SVG in `session-detail.ts`;
  replace it with the chosen set's primitive when this lands.
- Keep the single-file/no-build-step mockup convention in mind — an
  inline-SVG-sprite or inline-path approach fits better than an icon font or
  npm icon package with a build step.

## Simplification opportunity

Replaces the ad-hoc unicode glyphs and the one-off inline paperclip SVG with a
single icon primitive; text-label buttons in the composer collapse to icons.

## Design decisions

- **Icon source and delivery:** use the settled Lucide outline set as local TypeScript path data rendered into inline SVG — it keeps the browser bundle dependency-free and makes icon names/type coverage a compile-time concern without an icon font, package, or build transform.
- **Primitive ownership:** add `.icon` as a locked presentation primitive, with icon-size tokens in `.mockups/design-system/tokens.css` and CSS in `.mockups/design-system/components.css`; extend `check-presentation` so the primitive needs both a real CSS selector and showcase element. The conformance check remains registry-derived for state primitives; the icon is an independently checked design-system primitive, not a protocol state.
- **Accessible icon controls:** icons inside buttons are always `aria-hidden="true"`; every icon-only button supplies an action-specific `aria-label` and `title`. Text remains available where it communicates protocol state or failure, not merely an action already named by the accessible control.
- **Delivery disclosure boundary:** retain the committed v0.1.0 compact delivery badge (current `CommandState` plus last transition) and do **not** add a delivery expand button/caret. A working expand affordance would falsely imply extra content or silently promote the explicitly reserved full delivery-trace seam. The shared chevron icons are available for real, existing disclosures only.
- **Sidebar Spawn/Attach boundary:** render icon-only header controls as visibly disabled until a real spawn/attach flow and capability/authority input exists; do not synthesize Operations, targets, or optimistic availability merely to make an icon clickable. This preserves the conformance-floor distinction between UI availability and authority.

## Mockups

- Design-system refinement: `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, and `.mockups/design-system/tokens.css`. The components showcase gains an icon section covering size, button variants, semantics, and the Lucide shapes used by the cockpit. There is no parent epic mockup and no new screen or flow; this is the required component-library refinement tier.

## Architectural choice

Use a small local `icons.ts` catalog plus one DOM factory. The catalog is the only application source that owns Lucide path data; the factory applies the shared 24×24 view box and outline stroke contract; consumers only select a typed icon name. CSS, tokens, and the standalone showcase define the skin-able primitive independently of the TypeScript factory.

Alternatives considered:

1. **Inline SVG at each call site.** Zero module surface, but repeats the same accessibility/stroke setup and returns to one-off paths; rejected.
2. **A document-level SVG `<symbol>` sprite.** Avoids repeated path nodes, but requires lifecycle-managed global DOM injection and makes isolated component tests/showcase examples less direct; rejected for this small fixed set.
3. **Local typed path catalog and factory (chosen).** One small dependency-free module gives a single source for names/path geometry, while ordinary inline SVG remains compatible with the no-build-step cockpit.

The trickiest unit is the catalog/factory boundary: it must preserve Lucide stroke geometry while preventing unlabeled icon-only controls and avoid turning visual names into protocol state. It is designed before the call-site conversion.

## Implementation Units

### Unit 1: Design-system icon primitive and conformance binding

**Files**: `.mockups/design-system/tokens.css`, `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, `contracts/scripts/check-presentation.mjs`, `contracts/scripts/test-presentation-check.mjs`

**Story**: `feature-cockpit-icon-set-design-system-conformance`

Add tokenized icon geometry and an independently checkable component primitive:

```css
/* tokens.css */
:root {
  --icon-size-sm: 14px;
  --icon-size-md: 16px;
  --icon-size-lg: 20px;
}

/* components.css */
.icon {
  display: block;
  width: var(--icon-size-md);
  height: var(--icon-size-md);
  flex: 0 0 auto;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  pointer-events: none;
}
.icon--sm { width: var(--icon-size-sm); height: var(--icon-size-sm); }
.icon--lg { width: var(--icon-size-lg); height: var(--icon-size-lg); }
```

`components.html` gains an `#icons` navigation/section with static inline SVG examples for `arrow-left`, `arrow-up`, `paperclip`, `plus`, `link`, `chevron-down`, `chevron-right`, `x`, and `square`, including icon-only button semantics (`aria-label`, tooltip/title) and small/default/large sizing. It remains standalone HTML/CSS/JS and does not import application TypeScript.

`check-presentation.mjs` adds `icon` to `LOCKED_PRIMITIVES` and strengthens locked-primitive validation to require both an uncommented CSS class selector and a real DOM showcase element. `test-presentation-check.mjs` gains a broken fixture that removes the `.icon` CSS selector or showcase usage and proves the command fails non-zero. This keeps the new primitive inside the same conformance floor rather than merely documenting it.

**Acceptance criteria**:
- [ ] `tokens.css` defines small, medium, and large icon-size variables; `.icon` inherits `currentColor` and enforces the Lucide 2px rounded outline convention.
- [ ] The showcase visibly exercises every icon needed by cockpit chrome, icon-only labels/tooltips, and size variants in both theme modes.
- [ ] `check-presentation` and its meta-test fail if the icon CSS or its showcase element is removed, and pass on the real artifacts.

### Unit 2: Typed Lucide inline-SVG factory

**File**: `web-cockpit/src/ui/icons.ts`

**Story**: `feature-cockpit-icon-set-cockpit-chrome`

Define the fixed icon vocabulary and one DOM factory. Store the Lucide geometries as readonly SVG `path` `d` strings rather than raw HTML or an npm dependency.

```ts
export const ICON_NAMES = [
  "arrow-left", "arrow-up", "paperclip", "plus", "link",
  "chevron-down", "chevron-right", "x", "square",
] as const;
export type IconName = (typeof ICON_NAMES)[number];

export interface IconDefinition {
  readonly paths: readonly string[];
}
export const LUCIDE_ICONS: Readonly<Record<IconName, IconDefinition>>;

export type IconSize = "sm" | "md" | "lg";
export interface IconOptions { readonly size?: IconSize; }

export function renderIcon(
  document: Document,
  name: IconName,
  options?: IconOptions,
): SVGSVGElement;
```

`renderIcon` creates `<svg class="icon [icon--sm|icon--lg]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">`, appends paths from `LUCIDE_ICONS[name]`, and never accepts untyped/raw path input from a consumer. The medium size is the base `.icon` class. The existing `paperclip` `d` value is retained verbatim.

**Acceptance criteria**:
- [ ] All requested cockpit meanings map to a named Lucide geometry in one typed catalog; no consumer carries an SVG path string.
- [ ] Factory output has the shared view box, stroke semantics, `.icon` class, and is hidden from the accessibility tree.
- [ ] A unit/interface test verifies representative factory output and rejects unknown icon names at TypeScript compile time.

### Unit 3: Cockpit-chrome conversion without semantic widening

**Files**: `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`

**Story**: `feature-cockpit-icon-set-cockpit-chrome`

Import `renderIcon` and replace the existing ad-hoc chrome while retaining the existing action callbacks and submission gates:

```ts
// session-detail.ts call-site shape
const send = iconButton(document, "arrow-up", "Send instruction", "btn btn-primary btn--sm");
const attach = iconButton(document, "paperclip", "Attach file or image", "btn btn-secondary");
const back = iconButton(document, "arrow-left", "Back to sessions", "btn btn-ghost btn--sm");
const cancel = iconButton(document, "x", "Cancel running operation", "btn btn-secondary btn--sm");
const interrupt = iconButton(document, "square", "Interrupt running operation", "btn btn-danger btn--sm");
```

`iconButton` is a private helper that creates a `type="button"` or supplied submit button with `btn--icon-only`, sets `aria-label` and `title`, and appends `renderIcon`. Composer submit remains `type="submit"`; its disabled/identity-before-intent behavior is unchanged. Delete the one-off `createElementNS` paperclip block and the Unicode-arrow back label.

`renderSidebar` in `shell.ts` adds disabled header icon controls for Spawn (`plus`) and Attach (`link`) with clear labels/titles until an actual entry flow/capability report is provided. `shell.css` gets only structural header-action layout (for example `.sidebar__actions`); it must not add protocol-state class bindings. The catalog offers `chevron-down`/`chevron-right` for existing or future genuine disclosures, but this change deliberately does not add a delivery expand affordance or a trace UI.

**Acceptance criteria**:
- [ ] Composer uses Lucide paperclip/arrow-up, mobile back uses arrow-left, running actions use x/square, and sidebar header controls use plus/link; all icon-only buttons have distinct accessible names.
- [ ] No inline `d` string, raw SVG setup, Unicode navigation arrow, or text-only action control remains in the converted chrome.
- [ ] The compact delivery line still exposes only current state plus last transition and has no disclosure/trace control.
- [ ] Existing send/cancel/interrupt behavior and stable-target gating stay unchanged; disabled sidebar affordances do not emit an Operation.

## Simplification

- Delete the one-off paperclip SVG construction in `web-cockpit/src/ui/session-detail.ts` and the Unicode back-arrow label.
- Consolidate all cockpit icon geometry, view-box, stroke, and `aria-hidden` handling into `web-cockpit/src/ui/icons.ts`; no sprite lifecycle, icon package, or build-plugin abstraction is introduced.
- Retain textual command states, failure terms, and elicitation controls: they carry semantic information rather than duplicate an icon label.

## Testing

- Run `node contracts/scripts/check-presentation.mjs` and `node contracts/scripts/test-presentation-check.mjs`; they protect the new design-system primitive and its mockup coverage.
- Extend `web-cockpit/tests/shell.test.ts` to assert icon-only controls have the intended accessible names, contain `.icon`, retain disabled/send/cancel/interrupt behavior, and do not introduce a delivery `<details>`/trace disclosure.
- Run `cd web-cockpit && npm test`; this protects generated-contract integration and the existing identity-before-intent/stale-never-live interfaces. No tests are needed for static Lucide geometry beyond factory shape and representative path presence.

## Implementation Order

1. `feature-cockpit-icon-set-design-system-conformance`: add tokens, `.icon`, showcase examples, and the conformance/meta-test binding; run the presentation checks.
2. `feature-cockpit-icon-set-cockpit-chrome`: add the typed catalog/factory and migrate cockpit controls after the primitive exists; update shell tests.
3. Run the presentation check, its meta-test, and the cockpit test suite together; review the mockup showcase in both themes.

## Risks

- **Lucide path transcription:** an incorrect local path is easy to miss in a code review. Mitigate with one catalog, preserving the known-good paperclip exactly, and visually checking all requested shapes in the committed showcase.
- **Icon-only accessibility:** a lost label would make an action invisible to assistive technology. Mitigate with factory-hidden SVGs, required labels at each control, showcase examples, and DOM assertions.
- **Scope pressure from Spawn/Attach and delivery affordances:** the cockpit lacks a real spawn/attach entry flow and full delivery expansion is explicitly reserved. Disabled, honestly unavailable header controls and no delivery disclosure prevent a visual polish item from inventing authority or widening observability scope.
- **Presentation-check drift:** adding only markup or only CSS could leave a nominal primitive unverified. The check/meta-test extension makes both sides required.

## Implementation notes
- Execution capability: direct-read only; one feature owner implemented the two dependent checkpoints against the named design-system, cockpit, and test interfaces.
- Review weight: standard (default).
- Files changed: `.mockups/design-system/tokens.css`, `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, `contracts/scripts/check-presentation.mjs`, `contracts/scripts/test-presentation-check.mjs`, `web-cockpit/src/ui/icons.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/shell.css`, `web-cockpit/tests/shell.test.ts`.
- Tests added/removed: presentation meta-test coverage for missing icon CSS/showcase artifacts; factory-shape and compile-time icon vocabulary tests; cockpit DOM assertions for icon accessibility, disabled sidebar controls, and unchanged compact delivery rendering.
- Simplification: replaced the hand-built paperclip and repeated icon-control setup with a typed local Lucide catalog and one inline-SVG factory.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Integrated verification: `node contracts/scripts/check-presentation.mjs`, `node contracts/scripts/test-presentation-check.mjs`, and `cd web-cockpit && npm test` all passed (43 cockpit tests).

## Review record (2026-07-24, standard weight, cross-model)

Reviewer: fresh-context `openai-codex/gpt-5.6-sol` (implementer was `gpt-5.6-terra` — different model class). One balanced pass per standard weight.

**Verdict:** APPROVE-WITH-IMPORTANT-FINDINGS — no blockers. Reserved seams intact (no delivery caret; Spawn/Attach visibly disabled), static-SVG construction safe (catalog-only path data, `createElementNS`, no interpolation), accessibility sound, all checks green.

**Findings adjudicated:**
- Important — `square` icon used sharp `M3 3h18v18H3z` instead of Lucide's rounded-corner square (acceptance-criterion fidelity + visible family inconsistency on Interrupt). **Fixed in-wave** (receiver-confirmed, trivial): path replaced with the rounded-rect path equivalent of Lucide's `<rect x="3" y="3" width="18" height="18" rx="2"/>` in `web-cockpit/src/ui/icons.ts` and the design-system showcase. Re-verified: cockpit 43/43, check-presentation + meta-tests green.
- Blockers: none. Nits: none.
