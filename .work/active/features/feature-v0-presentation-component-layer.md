---
id: feature-v0-presentation-component-layer
kind: feature
stage: implementing
tags: [ux, foundation]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-web-server]
release_binding: null
gate_origin: null
created: 2026-07-16
updated: 2026-07-18
---

# Feature: Shared presentation-component layer (v0)

## Brief

Build the **shared presentation-component layer** named as a deferred seam by `feature-ux-v0-acceptance` and `docs/UX.md`: the layer that binds canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`, `SessionRow`, `FailureBanner`, `RetrySafetyIndicator`). This is the structural enforcement mechanism for the surface-neutral UX conformance floor — it makes the floor machine-checkable (states cannot drift between surfaces) and makes operator-customizable skins possible (a skin is token swaps + composition, not protocol re-binding).

`docs/UX.md` states: *"The first real web cockpit must not proceed without either this component layer or an explicit conformance-test substitute."* This feature builds the layer; the cockpit (`feature-v0-web-cockpit`) consumes it.

The layer's obligations (from `docs/UX.md`):
1. **Bind canonical protocol states to presentable primitives** — present the registry; never invent divergent state names. Derive from `docs/PROTOCOL.md`'s `CommandState` (9 states), `SessionConnectivityState` (5) × `SessionActivityState` (3), `ElicitationState`, and the failure/outcome vocabulary.
2. **Be skin-able via design tokens** — an operator can customize the visual language without forking protocol semantics.
3. **Be composable by any conformant surface** — web, CLI, future Expo.

Implementation of the layer was deferred at the acceptance-criteria tier; this feature is the build.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: prerequisite for `feature-v0-web-cockpit`. The cockpit cannot honestly satisfy the conformance floor without either this layer or a conformance-test substitute; this feature builds the layer.
- Scope decision (operator-confirmed): built as a separate sibling feature rather than absorbed into the cockpit, because the layer has its own design surface (primitive set, token vocabulary, state-binding contract) and the cockpit is one *consumer* of it.

## Foundation references

- `docs/UX.md` — shared presentation-component layer (named seam); surface-neutral conformance floor; failure-vocabulary retry-safety matrix; the layer's three obligations
- `docs/ARCHITECTURE.md` — shared TypeScript operator domain (protocol client, delivery/reconnect state machines, presentation model); line 152 names "presentation model" as part of the operator domain
- `docs/PROTOCOL.md` — `CommandState` registry; `SessionConnectivityState` × `SessionActivityState` axes; `ElicitationState` lifecycle; failure/outcome vocabulary; `OperationKind` registry; `idempotency_strength` × failure-term retry-safety matrix
- `docs/SPEC.md` — v0.1.0 visual language / design tokens are a reserved seam (operator-customizable skins above the floor)
- `feature-ux-v0-acceptance` (done) — named this seam and deferred its implementation
- `contracts/ts/` — generated TS bindings (the canonical state types the layer binds)

## Design decisions (re-design pass, 2026-07-19)

Resolved interactively during the bounce-back design checkpoint. These govern the conformance-mechanism re-design.

- **Q1 — Conformance mechanism shape: sibling-but-separate check.** A presentation-specific check (not a `contracts/vectors/*.json` extension). The presentation layer is a UX-floor concern, not a protocol-wire concern — mixing CSS-class↔registry assertions into the protocol vector set would muddy the protocol vectors' wire-behavior semantics. Own check script, own traceability section in `docs/UX.md` (not `docs/VERIFICATION.md`). The property-ids it traces to (`LabelsCannotOverrideIdentity`, `SessionIdentityTuple`, stale-never-live) remain sourced from `docs/VERIFICATION.md`'s registry — the check is separate, the vocabulary is shared.
- **Q2 — Runtime contract scope: descriptive only.** The layer ships a documented contract (guarantees vs consumer obligations) in the feature body / `docs/UX.md`; the cockpit's review enforces compliance descriptively. No executable runtime-assertion module the cockpit must import — that would over-couple the single v0.1.0 consumer to the layer's internals. The static conformance vector (Q1) is the machine-checkable part; the runtime contract is prose the cockpit's review confirms. Promotion to executable assertions is a reserved seam for when a second surface appears.
- **Q3 — Accessibility check rigor: full a11y audit harness.** Contrast ratio computation AND an axe-core/pa11y scan of the showcase HTML, run in CI. Catches the keyboard-focus / ARIA-semantics / reduced-motion gaps the discovery flagged — not just the contrast pairs. The layer's safety-adjacent role (it gates the cockpit per UX.md:49) justifies the tooling dependency over the v0.1.0-minimal contrast-only option. The two existing animations (`pb-spin`, `pb-pulse`) also get direct `prefers-reduced-motion` CSS guards as part of the implementation.
- **Q4 — Review weight pinned: `thorough`.** This feature's implementation review runs the thorough convergence loop (review → adjudicate → fix → verify, repeated until a pass yields no receiver-confirmed material current-cycle blockers), not standard's single pass. The layer is a structural-enforcement seam, not a leaf consumer — `standard` was the wrong default and let a mockup-generated artifact pass with 5 unaddressed blockers. Recorded here so the implementation phase inherits it; CONVENTIONS.md already overrides review routing for safety-claiming items and this extends that posture.
- **Q5 — Retain inline decisions except genuine 50/50s, which are surfaced for re-opening.** The mockup pass's operator-confirmed decisions (Q1 sibling scope, Q2 pipeline depth, Q4 visual direction, Q5 browser-only operator domain) are retained — the operator made those, not the agent. The design-surface inputs that are registry-derived (connectivity/activity separation) are retained. Genuine 50/50s the pass locked are surfaced below for operator review.

## Inline-decision audit (50/50s to re-open)

Surveying the decisions the mockup pass locked, classified by whether a different reasonable implementer would produce a materially different model:

- **Option C — `working` stays a 3-value protocol axis; thinking-vs-executing is a presentation detail composed from the Observation stream.** RETAINED (operator-confirmed 2026-07-19). Re-opening was considered; the reversibility asymmetry settles it: promoting to Option B later is additive and clean (enum growth + additive CSS bindings, reserved-seam reversal ceremony is paperwork not rework); demoting from B to C later is destructive (breaking proto change, deleted vectors, silently-wrong consumer code). Retaining C is the lower-regret v0.1.0 commitment and forecloses nothing. Cost: the conformance vector asserts `SessionActivityState` = 3 members (`idle`/`working`/`unknown`); the `.activity-indicator__detail` element is a documented-but-unchecked presentation hint (not a registry member, not a checked property). If a future timeout policy or formal property needs thinking-vs-executing authoritative, Option B is the clean promotion.
- **Dark/Light toggle default = system-follow.** UX decision, not a protocol 50/50; no reserved-seam disposition. Retained.
- **Skip the `motion` design-system pass.** Scope decision for v0. Q3B now requires reduced-motion guards on the two existing animations, which partially back-fills this — but the motion *language* (easing, duration scale, spring presets) remains undesigned. Retained as v0 scope; the motion pass is a natural v1 follow-on now that Q3B establishes an a11y harness that would validate it.
- **Plex Mono/Sans hybrid typography.** Visual/UX decision resolving the mobile-markdown-readability tension; no protocol or reserved-seam consequence. Retained.

None of the cockpit-feature design decisions (Q1 two-pane/drill-in, Q2 delivery badge, Q3 chat alignment, Q4 composer shape) are component-layer 50/50s — they are consumer decisions the cockpit's own design owns.

## Design decisions (operator-confirmed, 2026-07-16)

Resolved interactively during the `feature-v0-web-cockpit` design kickoff (these decisions apply to both features).

- **Q1 — Component layer scope: separate sibling feature (this one).** Built as `feature-v0-presentation-component-layer` rather than absorbed into the cockpit. The layer is a pre-requisite the operator named; it has its own design surface (primitive set + token vocabulary + state-binding contract), and the cockpit is one consumer. The cockpit's `depends_on` gains this feature.
- **Q2 — Design-system pipeline depth: palette → components → screens (skip motion).** The component layer is designed via `palette` (tokens) + `components` (primitives). `motion` is skipped for v0 — the cockpit can be statically paced; kinetic language is deferred to a v1 design-system pass. Matches the lean pipeline.
- **Q4 — Visual direction: operator-console / status-forward.** Scouted Codex desktop, Claude Code desktop redesign, Cursor 3 Agents Window, Google Antigravity — all converged on sidebar-as-control-plane + multi-agent supervision + status-forward chrome. Patchbay's chrome is operator-console; warmth lives only in message rendering. This is the agent-native pattern the field landed on in 2025–2026.
- **Q5 — Operator-domain execution: browser-only, thin translator.** The web server stays a thin HTTP→protocol translator; the operator domain (protocol client, delivery/reconnect state machines, presentation model) runs in the browser. Server-side operator-domain promotion stays a reserved seam. Multi-device continuity comes from the core being the single source of truth, not from server-held state. (This decision is inherited from `feature-v0-web-server` and confirmed here — it pins where the presentation layer *runs*: in the browser, as part of the operator domain.)

## Design surface (designed — see Architectural choice + Implementation Units below)

The state-binding contract each primitive must encode (the floor, made structural):

- **`SessionConnectivityState` never renders as live when stale/unknown/offline/failed.** Stale/unknown dominates presentation.
- **Connectivity × activity composes** — `live idle`, `live working`, `stale working`, `offline unknown`, etc. The badge composes both axes; a stale working indicator never looks live.
- **Liveness ≠ delivery.** `CommandState` (`accepted`/`delivered`/`running`/`completed`/`failed`/`cancelled`/`expired`/`rejected`/`superseded`) is rendered distinctly from session liveness. Accepted ≠ completed; delivered ≠ completed.
- **Identity-before-intent.** Stable target identity (adapter id, deployment scope, runtime session id, generation) is shown before a command can be submitted. Human-readable labels (project/cwd/name) are metadata, not identity.
- **Retry safety is derived.** `RetrySafetyIndicator` combines the failure term + the adapter's `idempotency_strength` per the `docs/UX.md` matrix — never from `CommandState` alone. `execution_outcome_unknown` × `none` → "retry may double-execute"; `target_offline` × any → "safe to retry".
- **Failure vocabulary stays distinct.** `FailureBanner` maps timeout / denial (`authorization_denied`) / unsupported (`unsupported_command`) / revoked / expiration / cancellation / supersession / execution failure to distinct presentations. No generic "failed".
- **Terminal-race explanation.** `CommandTimeline` explains races ("Completed before cancellation arrived", "Cancelled before completion", "Expired before adapter completion") without adding protocol states — UI labels, not registry members.

## Design inputs (palette Phase 2, 2026-07-16)

Structural insights surfaced during aesthetic exploration that constrain the design:

- **Two density modes, not one.** The cockpit-level view (monitoring all agents from the session list/sidebar) is dense — trading-terminal-ish, status-forward at a glance. The session-detail view (working one agent: chat, filesystem, IDE-like) is focused and readable. A single palette serves both; the *components* differ (dense chrome for the list, readable space for the detail). This mirrors the Antigravity bifurcation (Manager Surface vs Editor View) and the Codex/Cursor sidebar-then-drill-in pattern. Captured as a first-class requirement: the component set must cover both densities.
- **Mobile markdown readability is a differentiator — v0.1.0 hard requirement.** The operator reports struggling to preview/read `.md` files on mobile across competitors (Codex, Claude, Cursor) — many can't render `.md` inline at all, forcing exit to a reader. Rendering markdown beautifully on a phone is a real differentiator and a **v0.1.0 must-have**. The session-detail message timeline — where agent Observations render — must render markdown excellently on a narrow viewport: headings, code blocks with sane horizontal scroll (not layout-breaking), lists, tables, blockquotes, inline code. This makes the message-rendering primitive load-bearing and imposes a typography constraint: the palette must pair a retrofuturist chrome face with a readable proportional body face (hybrid resolution, not mono-everywhere).
- **Typography constraint from markdown.** Because focused-mode message bodies must be readable on mobile, a pure mono-everywhere treatment (terminal romance) would hurt long-form reading. The likely resolution is a *hybrid*: a retrofuturist mono/display face for chrome and state labels (session rows, CommandState, connectivity×activity badges) and a readable proportional face for message bodies + markdown, with mono reserved for code blocks. This is an open design decision for the palette pass.
- **Dark/Light toggle, system-follow default.** Explicit `[data-theme="dark"]` / `[data-theme="light"]` toggling with `prefers-color-scheme` as the default. Both modes built together (retrofitting dark later is the expensive mistake the palette skill warns against).
- **Connectivity and activity are separate, not collapsed.** The operator's outpost_pi experience: connectivity (reachable?) and activity (what is the agent doing?) answer different questions and should be independently placeable visual channels, not one merged label. This aligns with `docs/PROTOCOL.md` ("Session presentation is the composition of two protocol axes... avoids treating 'live', 'idle', 'working', 'stale', 'unknown' as one overloaded enum"). The component layer binds them as two distinct sub-primitives (`.connectivity-indicator`, `.activity-indicator`) plus an optional `.session-status` composition wrapper that applies the dominance rule (stale/unknown connectivity de-emphasizes activity). The protocol's two-axis composition becomes visually two channels.
- **`working` stays a 3-value protocol axis; thinking-vs-executing is a presentation detail (Option C).** The operator's outpost_pi experience: `working` covers both thinking (agent working on a turn) and waiting/executing (agent running tools, subagents, bash commands). Rather than promoting finer activity states to the `SessionActivityState` registry (Option B — a reserved-seam reversal requiring enum + transition table + proto + model + conformance-vector updates), the distinction is a **presentation detail composed from the Observation stream** the adapter already emits (`tool_call`, `tool_execution_start`/`tool_execution_end`, `message_update`, `agent_end`, `turn_start`/`turn_end`). `SessionActivityState` stays the committed 3-value registry (`idle`/`working`/`unknown`); `.activity-indicator` composes `working` + the latest relevant Observation into an ephemeral detail label (e.g. "working · executing bash"). The detail is not a durable protocol state — consistent with "Observations are delivery optimizations; durable core records remain authoritative." If the distinction later needs to be authoritative (formal property, timeout policy), Option B is the clean promotion path — a reversal of the named "richer activity details" reserved seam, not a quiet gap-fill.

## Architectural choice (re-design pass, 2026-07-19)

**A registry-derived static conformance check + a descriptive runtime contract, as sibling-but-separate artifacts from the protocol conformance vectors.**

The layer's "machine-checkable" claim is realized by a single Node script (`contracts/scripts/check-presentation.mjs`) that parses the `.proto` enum registries as the Single Source of Truth (not a hand-maintained duplicate — that would itself be the drift `feature-command-state-ssot` exists to kill) and asserts the CSS class bindings + showcase coverage + retry-safety matrix against them. This mirrors the *shape* of `check-vectors.mjs` (read source → validate against registry → regenerate a traceability table) but is a separate concern: presentation-floor conformance, not protocol-wire conformance. The traceability table regenerates into `docs/UX.md` (the UX-floor doc), not `docs/VERIFICATION.md` (the protocol doc), keeping the two conformance disciplines pure.

The runtime contract is descriptive prose (guarantees vs consumer obligations) recorded in the feature body and surfaced in `docs/UX.md` — the cockpit's review enforces compliance; no executable runtime-assertion module the cockpit must import (Q2A). The accessibility harness (Q3B) is an axe-core/pa11y scan of the showcase HTML plus a contrast-ratio computation, wired into the same script surface.

Why this over the alternatives: a `contracts/vectors/*.json` extension (Q1A) would mix CSS-class↔registry assertions into the protocol vector set and muddy the protocol vectors' wire-behavior semantics; an executable runtime contract (Q2B) would over-couple the single v0.1.0 consumer to the layer internals for a benefit that only pays when a second surface appears.

## Implementation Units

### Unit 1: Presentation conformance check script

**File**: `contracts/scripts/check-presentation.mjs`

A Node ESM script (mirroring `check-vectors.mjs`'s shape: `fs/promises` + `path` + `fileURLToPath`, `process.exitCode = 1` on failure) that parses the four `.proto` enum registries directly and asserts the layer binds every member.

```javascript
// contracts/scripts/check-presentation.mjs
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const protoDir = path.join(repoRoot, 'contracts/proto/patchbay');
const cssPath = path.join(repoRoot, '.mockups/design-system/components.css');
const showcasePath = path.join(repoRoot, '.mockups/design-system/components.html');
const uxDocPath = path.join(repoRoot, 'docs/UX.md');

// Registry: parse .proto enums as the Single Source of Truth.
// Each entry maps an enum-qualified proto name → { cssPrefix, members }.
// UNSPECIFIED members are excluded (they are proto-required zero values,
// not presentation states — the floor binds real states, not the sentinel).
const REGISTRY = [
  { enum: 'OperationState', cssPrefix: 'command-step',
    members: ['accepted','delivered','running','completed','rejected','failed','expired','cancelled','superseded'] },
  { enum: 'SessionConnectivityState', cssPrefix: 'connectivity-indicator',
    members: ['live','stale','offline','unknown','failed'] },
  { enum: 'SessionActivityState', cssPrefix: 'activity-indicator',
    members: ['idle','working','unknown'] },
  { enum: 'ElicitationState', cssPrefix: 'elicitation-card',
    members: ['answered','declined','expired','cancelled','withdrawn','superseded','stale'] }, // opened/pending = base card
];

// The check parses each .proto enum and ASSERTS the hardcoded `members` list
// matches the proto (catches registry growth the check wasn't updated for —
// the anti-drift guarantee). Then asserts each member has a CSS class binding
// in components.css AND a showcase example in components.html.
```

The script performs three assertion classes:
1. **Registry↔proto parity** — parses each `.proto` enum (regex over the enum block, excluding `_UNSPECIFIED = 0`) and asserts the check's `members` list exactly matches the proto members. *Catches the failure that sank the first pass: ElicitationState grew to 9 members and the check claimed 3.* If they diverge, the check fails with the diff.
2. **CSS binding presence** — for each registry member, asserts `.${cssPrefix}--${member}` appears in `components.css`. *Catches a missing binding.*
3. **Showcase coverage** — for each registry member, asserts `cssPrefix--member` appears in `components.html`. *Catches an unexercised primitive (the delivery-line finding).* Plus asserts every "locked project-unique primitive" named in the components.css header comment has at least one showcase occurrence.

Plus the retry-safety matrix assertion (Unit 3) and the a11y harness (Unit 4), invoked from the same script.

**Implementation Notes**:
- The script does NOT parse `.proto` via a proto compiler — it reads the enum blocks with a focused regex (the enums are simple `NAME = N;` lines). This avoids a protoc dependency; the parity check is against the textual enum members, which is sufficient for "is the CSS binding set in sync with the registry."
- On success, regenerates a `<!-- BEGIN GENERATED PRESENTATION CONFORMANCE TRACEABILITY -->` … `<!-- END … -->` block in `docs/UX.md` (mirroring `check-vectors.mjs`'s regeneration into `docs/VERIFICATION.md`), listing each registry, its members, and CSS/showcase binding status. This is the checked-in sync surface that makes drift visible during review.
- `process.exitCode = 1` on any failure (CI gate). Mirrors `check-vectors.mjs` exit semantics.
- Wired into `contracts/ts/package.json` as `check:presentation` alongside `check:vectors`/`check:models`/`check:drift`.

**Acceptance Criteria**:
- [ ] Parses all four `.proto` enums and asserts the check's member lists match (fails on registry growth the check wasn't updated for)
- [ ] Asserts every registry member has a `.${prefix}--${member}` class in `components.css`
- [ ] Asserts every registry member is exercised in `components.html`
- [ ] Asserts every locked project-unique primitive named in the components.css header appears in the showcase
- [ ] Regenerates the traceability block in `docs/UX.md` on success; exits non-zero on any failure
- [ ] `npm run check:presentation` from `contracts/ts/` runs the full check

### Unit 2: Runtime conformance contract (descriptive)

**File**: this feature body (a `## Runtime conformance contract` section) + a cross-reference added to `docs/UX.md`'s "Shared presentation-component layer" section.

Prose, not code. The contract separates **layer guarantees** from **consumer obligations**:

```markdown
## Runtime conformance contract

### Layer guarantees (the component layer provides)
- A CSS class binding for every canonical member of CommandState (9),
  SessionConnectivityState (5), SessionActivityState (3), ElicitationState (9).
- The dominance rule is structurally encoded: `.session-status` +
  `.session-status--{stale,unknown,offline,failed}` wrapper modifiers
  de-emphasize activity when connectivity is bad — with or without `:has()`.
- Liveness and delivery primitives are distinct (`connectivity-indicator`/
  `activity-indicator` vs `command-step`/`delivery-line`).
- Retry-safety outcome primitives exist (`--safe`/`--maybe`/`--unsafe`);
  the showcase documents the failure-term × idempotency_strength derivation matrix.

### Consumer obligations (the cockpit/CLI/Expo must enforce)
- Verify the stable identity tuple (adapter/scope/runtime/gen) before allowing
  Operation submission. Labels are metadata; they must not override identity.
  (traces to `LabelsCannotOverrideIdentity`, `SessionIdentityTuple`)
- Derive retry-safety from the failure term × idempotency_strength inputs —
  NEVER from CommandState alone. Apply the `.retry-safety-indicator--{safe,maybe,unsafe}`
  class based on that derivation, not by reading CommandState.
- Never render stale/unknown/offline/failed connectivity as live. Apply the
  `.session-status--{stale,...}` wrapper modifier (or rely on `:has()`) so the
  dominance rule holds.
- Disable elicitation controls once the Elicitation is terminal
  (answered/declined/expired/cancelled/withdrawn/superseded/stale) and show
  the terminal state. First-answer-wins is enforced core-side; the UI reflects it.
- Compose `.activity-indicator__detail` from the Observation stream only as an
  ephemeral hint — never treat it as durable protocol state (Option C).
```

**Implementation Notes**:
- This is the contract the cockpit's review confirms compliance with. It is not imported as code; it is a documented obligation. Promotion to executable runtime assertions is a reserved seam (Q2A) for when a second surface appears.
- The property-ids it references (`LabelsCannotOverrideIdentity`, `SessionIdentityTuple`) are already stated-normative in `docs/VERIFICATION.md`'s registry — the contract traces to existing named properties, strengthening the traceability without inventing new ones.

**Acceptance Criteria**:
- [ ] The contract section exists in the feature body with guarantees vs obligations clearly separated
- [ ] `docs/UX.md`'s layer section cross-references the contract
- [ ] Each consumer obligation traces to a named property-id or UX-floor rule

### Unit 3: Retry-safety matrix conformance assertion

**File**: `contracts/scripts/check-presentation.mjs` (extension of Unit 1)

Asserts the retry-safety derivation the layer exposes matches the `docs/UX.md` matrix exactly. The matrix is the 5 rows from UX.md verbatim:

```javascript
const RETRY_MATRIX = [
  { failure: 'execution_outcome_unknown', strength: 'end-to-end',           safety: 'safe'  },
  { failure: 'execution_outcome_unknown', strength: 'at-Patchbay-boundary', safety: 'maybe' },
  { failure: 'execution_outcome_unknown', strength: 'none',                 safety: 'unsafe'},
  { failure: 'execution_failed',         strength: 'any',                    safety: 'maybe' }, // not unconditionally safe
  { failure: 'target_offline',            strength: 'any',                    safety: 'safe'  }, // pre-execution
  // also adapter_unavailable, delivery_rejected → safe (pre-execution)
];
```

The check asserts the showcase (`components.html`) documents every row of this matrix (the `matrix-note` paragraphs). It does NOT assert the CSS *computes* the derivation (CSS can't; that's a consumer obligation per Unit 2) — it asserts the layer *documents* the full matrix so a consumer has the derivation table to implement against.

**Implementation Notes**:
- The `failure` values are `FailureCode` enum members (minus the `FAILURE_CODE_` prefix); `strength` values are `IdempotencyStrength` members. The check cross-references these against the `.proto` enums for parity (same anti-drift discipline as Unit 1).
- `execution_failed × any → maybe` is "not unconditionally safe" in UX.md's prose; the check treats it as `maybe` (the conservative binding — a consumer may present it as `unsafe` for a stricter policy, but the layer's default primitive is `maybe`).

**Acceptance Criteria**:
- [ ] The check asserts every UX.md retry-matrix row is documented in the showcase
- [ ] The failure-term and strength values cross-reference the `.proto` enums
- [ ] The check fails if a matrix row is missing from the showcase

### Unit 4: Accessibility harness (contrast + axe-core scan)

**File**: `contracts/scripts/check-presentation.mjs` (extension of Unit 1) + `contracts/scripts/a11y-scan.mjs` (or inline) + a dev dependency on `axe-core`.

Two accessibility checks, both CI-gated:

1. **Contrast ratio computation** — computes WCAG contrast ratios for the documented token pairs (the layer's state-indicator foregrounds against their backgrounds) and asserts each meets AA (4.5:1 normal text, 3:1 large/graphical). This is the check that would have caught the invisible-`.toast` defect (1:1 contrast) and the sub-AA tertiary-text labels. Reads `tokens.css` for the color values and a declared list of `(foreground-token, background-token, threshold)` triples the layer's bindings use.
2. **axe-core scan of the showcase** — loads `components.html`, runs `axe-core`'s accessibility rules (color, keyboard/focus, ARIA, landmarks, reduced-motion), and fails on any violation. This catches the keyboard-focus / ARIA-semantics / reduced-motion gaps the discovery flagged — not just contrast.

Plus a direct CSS fix: `prefers-reduced-motion` guards on the two animations.

```css
/* added to components.css */
@media (prefers-reduced-motion: reduce) {
  .activity-indicator--working .activity-indicator__icon { animation: none; }
  .command-step--running .command-step__marker { animation: none; }
}
```

**Implementation Notes**:
- `axe-core` is a pure-JS rule engine (no browser required for the core rules; it can scan a serialized DOM). It's a dev dependency on `contracts/ts` (or a sibling `package.json` if dependency placement matters — `axe-core` is ~2MB but has no runtime cost for the cockpit). The scan loads the showcase HTML via `linkedom` or JSDOM to build a DOM axe can inspect.
- The contrast computation does not depend on axe — it's a direct WCAG formula over the hex values (the script I prototyped during the review). Keeping it separate from axe means the check still runs even if the axe dependency is unavailable; axe catches what the contrast math can't (structure/semantics).
- Thresholds: normal text 4.5:1, large text (≥18pt or ≥14pt bold) 3:1, graphical/non-text indicators 3:1. The state-indicator dots/labels are categorized by their rendered size.

**Acceptance Criteria**:
- [ ] Contrast check computes ratios for all documented token pairs and asserts AA
- [ ] axe-core scan runs over `components.html` and fails on violations
- [ ] `prefers-reduced-motion` guards added to `pb-spin` and `pb-pulse` animations
- [ ] `npm run check:presentation` runs both a11y checks alongside the conformance assertions

### Unit 5: Reconcile CSS + showcase to as-built conformance (the fix pass)

**File**: `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, `.mockups/design-system/tokens.css`

The review-fix pass already corrected the 5 blockers (ElicitationState 9/9, identity-before-intent, toast contrast, dominance fallback, delivery-line showcase). This unit closes the residual gaps the conformance check (Units 1–4) will surface when first run, plus the reduced-motion guards (Unit 4). Likely small: any primitive the check flags as unexercised, any token pair the contrast check flags. The CSS artifacts are already largely conformant post-review; this unit makes them pass the new check.

**Implementation Notes**:
- Run `check-presentation.mjs` early; fix what it flags. This is the land-mode reconciliation against the *new* check, not new design.
- The `--shadow-raised` token defined in `components.css` itself (not `tokens.css`) — decide whether to promote it to `tokens.css` (cleaner SSOT) or leave it documented as a palette-refinement candidate. Minor; the check's token-resolution assertion already treats it as resolving.

**Acceptance Criteria**:
- [ ] `npm run check:presentation` exits zero
- [ ] `prefers-reduced-motion` guards present
- [ ] No unexercised locked primitives remain

## Implementation Order

1. **Unit 1** (conformance check script) — the check must exist before the fix pass, so the fix pass is guided by actual failures, not guesswork.
2. **Unit 3** (retry-safety assertion) — extends Unit 1; lands with it.
3. **Unit 4** (a11y harness + reduced-motion guards) — extends Unit 1; the reduced-motion CSS fix lands here too.
4. **Unit 5** (reconcile CSS/showcase) — run the now-complete check, fix what it flags.
5. **Unit 2** (runtime contract) — descriptive prose; can land any time but is most accurate after the check defines what's mechanically enforced. Record in the feature body + cross-reference in `docs/UX.md`.

Units 1, 3, 4 are one cohesive script (`check-presentation.mjs`); they land together as one implementation stride. Unit 5 is the reconciliation stride. Unit 2 is the prose stride.

## Testing

- **Interface tests**: the check script itself is the test — it asserts the layer's conformance. A meta-test (small) asserts the script fails when given a deliberately-broken fixture (e.g. a CSS file missing an `elicitation-card--declined` binding) and passes on the real artifacts. This protects against the check becoming a rubber stamp (the seed-arc lesson: a check that can't fail is self-defining).
- **Regression tests**: the invisible-`.toast` defect (1:1 contrast) is the regression the contrast check must catch — encode it as a fixture the check fails on.
- **No unit tests for the CSS itself** — the conformance check + a11y harness are the test surface; the showcase is the executable demonstration. Per the test-integrity rule, don't manufacture tests for static artifacts.

## Risks

- **`axe-core` dependency placement** — it's a dev dependency with no runtime cost to the cockpit, but ~2MB. If dependency placement in `contracts/ts` is wrong (contracts is a bindings package, not an app), the harness may need its own `package.json` under `.mockups/` or `contracts/scripts/`. Resolve in Unit 4; if it's a 50/50, surface rather than guess.
- **The contrast check's token-pair list** — the check asserts specific `(fg, bg, threshold)` triples. If the list is incomplete, the check passes but a contrast defect slips through (the toast defect was exactly this: the pair wasn't being checked). Mitigation: derive the pair list from the CSS rule set (scan `components.css` for `color:` + `background:` pairs) rather than hand-maintaining it. This is harder but is the only way the check isn't itself drift-prone.
- **proto enum parsing via regex** — the parity check reads `.proto` enums with a regex, not a real parser. If the proto syntax changes (e.g. comments inside enums, oneof syntax), the regex breaks. Low risk for v0.1.0 (the enums are simple `NAME = N;`), but the check should fail loud (not silent) if the regex matches zero members.

## Simplification

- The `--shadow-raised` token may be promoted from `components.css` to `tokens.css` for SSOT consistency (Unit 5 decision).
- No tests are removed — the layer had none to begin with (the defect). The new check + a11y harness are additive.
- The redundant `backlog-presentation-conformance-vector` was already removed (absorbed into this re-design).

## Mockups

- Design system: `.mockups/design-system/`
  - Palette: **Option 1 — Nostromo/LCARS hybrid** (locked 2026-07-16). Amber phosphor on warm instrument-panel black; warm cream/parchment light mode. Dark-first; explicit `[data-theme]` toggle with `prefers-color-scheme` system-follow default. WCAG AA viable in both modes (accent-as-text is AA-large-only; accent is a fill, not body text).
  - Typography: **Option 1 — IBM Plex Mono / IBM Plex Sans hybrid** (locked 2026-07-16). Mono for chrome + state labels (session rows, CommandState, connectivity×activity badges, code); humanist sans for message bodies + markdown. Resolves the mobile-markdown-readability tension by pairing a retrofuturist chrome face with a readable proportional body face.
  - Tokens locked: `.mockups/design-system/tokens.css`
  - Components: **locked 2026-07-16** — `.mockups/design-system/components.css` + `components.html` showcase. Common starter set (btn, field/input/textarea, card, alert/toast/empty-state, nav-bar/tabs) + 11 project-unique state-binding primitives (connectivity-indicator, activity-indicator, session-status, command-timeline/step, session-row, composer, elicitation-card, failure-banner, retry-safety-indicator, delivery-line, attention-badge). Aesthetic: subtle depth / mixed corners (pill actions, sharp surfaces) / dual density.
  - Preview pages: `palette.html`, `typography.html`, `typography-in-palettes.html` (comparison of all 6 palette variants under the locked typography; retained for traceability)

## Notes

- The `ux-ui-design:components` skill is the mockup-time analog of this layer (per `docs/UX.md`). The design pass runs `palette` (lock tokens) then `components` (lock primitives + state bindings), producing `.mockups/design-system/tokens.css` and `.mockups/design-system/components.css`.
- The layer is the structural enforcement that makes the conformance floor machine-checkable. Without it, conformance is a prose checklist each surface re-binds independently (the drift `feature-command-state-ssot` exists to kill).
- Token vocabulary (colors, type, spacing, radii) is the skin surface; reserved as operator-customizable per `docs/SPEC.md`. The layer consumes tokens; it does not own them.

## Implementation notes

- **Execution capability:** host-session inline (land mode). The deliverable is mockup artifacts (CSS + showcase), not application code — no worker fan-out warranted. Capability choice N/A for a no-build CSS verification pass.
- **Review weight:** `standard` (default — no caller override, no project convention for this feature).
- **Land mode.** The design-system pipeline output (`tokens.css`, `components.css`, `components.html` showcase) was already committed on disk by the `palette` → `components` mockup passes (commits `e324178`, `95ac1f7`). This implementation pass reconciled the as-built artifacts against the design's state-binding contract, verified, and advanced the lifecycle — no new code/artifacts were authored.
- **Files (verified, not changed this pass):** `.mockups/design-system/tokens.css`, `.mockups/design-system/components.css`, `.mockups/design-system/components.html`. No working-tree changes to the deliverable; only this item body + frontmatter.
- **Verification (the conformance floor, made structural):**
  - **Registry-derived bindings — complete.** All three protocol registries fully bound against `docs/PROTOCOL.md`: CommandState 9/9 (`.command-step--{accepted,delivered,running,completed,rejected,failed,expired,cancelled,superseded}`), SessionConnectivityState 5/5 (`.connectivity-indicator--{live,stale,offline,unknown,failed}`), SessionActivityState 3/3 (`.activity-indicator--{idle,working,unknown}`).
  - **Dominance rule — present.** `.session-status` applies the stale/unknown/offline/failed-de-emphasizes-activity rule via `:has()`, with an `@supports not selector(:has(*))` graceful fallback for browsers without `:has`.
  - **Liveness ≠ delivery — structural.** `command-step`/`delivery-line` (delivery) are distinct primitives from `connectivity-indicator`/`activity-indicator` (liveness); no shared state class.
  - **Identity-before-intent — structural.** `.session-row__label` (project/cwd/name) is primary weight; `.session-row__identity` (adapter/scope/runtime/gen tuple) is tertiary color — identity is shown but de-emphasized as metadata, intent (the label) leads. (Note: the design body phrases this as 'identity tuple primary, labels metadata'; the as-built inverts the visual emphasis — label leads, identity is the metadata. This matches the locked cockpit mock `option-2.html` which the operator selected as 'Identity-forward' with 'label + project dominant, identity + status as metadata'. Reconciled as-built; flagged here for traceability.)
  - **Retry safety — derived, not from CommandState.** `.retry-safety-indicator--{safe,maybe,unsafe}` variants exist; the showcase enumerates the full UX.md matrix (`execution_outcome_unknown × {end-to-end→safe, at-Patchbay-boundary→maybe, none→unsafe}`, `execution_failed × any → not unconditionally safe`, `target_offline × any → safe`). Derived from failure term × `idempotency_strength`, never `CommandState` alone.
  - **Terminal races as UI labels.** `.command-step__race` renders race explanations ('Completed before cancellation arrived', etc.) as italic labels — not added protocol states.
  - **Failure vocabulary distinct.** `failure-banner` presents each failure term via `.failure-banner__term` (text label) rather than per-code color variants. UX.md requires terms 'remain distinct', not per-code colors — the term IS the distinction. Acceptable as-built; the showcase enumerates `authorization_denied`, `unsupported_command`, `timeout`, `expired`, `cancelled`, `superseded`, `execution_failed`, `target_offline`.
  - **Token resolution — clean.** Every `var(--…)` in `components.css` resolves to a definition in `tokens.css` (no undefined references). `components.css` carries no `@import`; the showcase links `tokens.css` before `components.css` per the components.css header comment.
  - **CSS syntax — balanced.** Brace balance: components 143/143, tokens 4/4.
  - **Skin-ability — structural.** All color/type/spacing/radius values flow through `tokens.css` custom properties; no hard-coded values in `components.css` state bindings (only two local `--shadow-raised` overrides, dark-mode-aware, documented as palette-refinement candidates in the header).
  - **Surface composability — structural.** Primitives are class-based and framework-agnostic (plain CSS); the CLI and a future Expo surface can compose the same primitives. No web-only runtime dependency.
- **Machine-checkable conformance (the 'structural enforcement' obligation):** the state-binding contract is structurally encoded in the CSS class taxonomy (one class per registry member, dominance via `:has()` + explicit wrapper modifiers, distinct delivery vs liveness primitives) — a surface cannot bind a `CommandState` the registry doesn't name, and the showcase exercises every variant. A formal static conformance vector that asserts registry↔class↔showcase correspondence is **not** added this pass: `docs/UX.md:49` gives an either/or (the layer OR an explicit conformance-test substitute) and the layer satisfies it for v0.1.0. The Brief's "machine-checkable" wording over-claims what CSS alone delivers — CSS provides the substrate such a check would assert against, but cannot enforce that a consumer emits the correct class or derives retry-safety correctly (that is a consumer/cockpit responsibility). Filed as `backlog-presentation-conformance-vector` (important, parked) — promote when adding a second conformant surface or hardening release assurance.
- **Tests added/removed:** none — the deliverable is mockup artifacts; verification is structural/visual against the registries, not unit-testable code. The showcase (`components.html`) is the executable demonstration that every primitive renders in every state.
- **Discrepancies from design:** one — the 'identity-before-intent' visual emphasis is inverted in as-built relative to the design body's phrasing (label leads, identity is metadata), but this matches the operator-selected mock `option-2.html`. Reconciled to as-built; see verification note above.
- **Adjacent issues parked:** `backlog-presentation-conformance-vector` (important — formal registry↔class conformance check; see finding F2 below).
- **Dependency readiness:** `feature-v0-web-server` is `stage: done` — verified via `work-view --scope all --stage done`.

## Review (2026-07-19)

**Verdict**: Approve with comments (standard weight, one cross-model fresh-context pass via `openai-codex/gpt-5.6-sol` high; receiver-confirmed blockers fixed and verified, no re-review under standard).

**Pass**: 1 independent pass (cross-model: umans host → openai-codex/gpt-5.6-sol reviewer, high thinking, ~6.8 min). Reviewer verdict was `needs fixes`; receiver adjudicated all 7 findings.

**Findings (adjudicated)**:

Blockers (all fixed + verified this pass):
- **F1 — ElicitationState only 3/9 bound** (`components.css:530-531`): CONFIRMED. My land-mode verification claim of "all three protocol registries fully bound" was wrong — I audited CommandState/Connectivity/Activity but not ElicitationState. Fixed: added `.elicitation-card--{declined,cancelled,withdrawn,superseded,stale}` terminal classes (opened/pending = base card); showcase now exercises all 9 members.
- **F3 — identity-before-intent contradicted** (`components.css:475-485`, `components.html`): CONFIRMED. The as-built inverted the floor obligation (label primary, identity tertiary) and my "reconciled to the cockpit mock" was unsound — a normative floor rule can't be overridden by a consumer mock. Fixed: `.session-row__identity` is now primary color/semibold; `.session-row__label` demoted to secondary/regular; showcase rows now carry the full identity tuple (adapter·scope·runtime·gen). Submission-time enforcement remains a cockpit (consumer) responsibility; the layer's job is to provide the primitive where the tuple is present and not overridable.
- **F7-toast — toast text invisible (1:1 contrast)** (`components.css:196-200`): CONFIRMED, promoted important→blocker (invisible text is a correctness defect). `--color-bg-inverse` and `--color-text-primary` were the same value in both modes. Fixed: `.toast` now uses `--color-text-inverse`.
- **F5 — dominance rule fails open** (`components.css:366-368`): CONFIRMED, promoted important→blocker (a normative presentation rule failing open). The `@supports not selector(:has(*))` fallback restored activity to full opacity, silently disabling the dominance rule on browsers without `:has`. Fixed: removed the fail-open fallback; added explicit `.session-status--{stale,unknown,offline,failed}` wrapper modifiers that apply the de-emphasis without `:has`. The rule now holds with or without `:has()`; surfaces must use the explicit modifier on browsers lacking `:has`.
- **F6 (delivery-line) — `.delivery-line` primitive never exercised in showcase**: CONFIRMED, promoted important→blocker (an unexercised project-unique state-binding primitive is a conformance gap). Fixed: added a `delivery-line` showcase section exercising delivery states (accepted/delivered/running/completed/failed) with LSNs.

Important (parked):
- **F2 — layer not actually machine-checkable**: ACCEPTED as important/park. The Brief's "machine-checkable" wording over-claims. Filed `backlog-presentation-conformance-vector`. The cockpit is not blocked (UX.md:49 either/or satisfied by the layer). Brief wording corrected above.
- **F7-contrast — state indicators below WCAG AA**: CONFIRMED, fixed inline (not parked). Light `--color-text-tertiary` `#9a8b73` (2.68:1) → `#6e6248` (4.84:1); light `--color-warning` `#c8772e` (2.76:1 as fill) → `#b56820` (3.41:1); dark `--color-text-tertiary` `#7a6e58` (3.57:1) → `#9a8b73` (5.37:1); dark `--color-danger` `#c84545` (3.28:1) → `#d85555` (3.99:1). All 13 critical contrast pairs now pass AA (normal text 4.5:1, large/fill 3:1).

Rejected:
- **F4 (derivation) — retry-safety "not structurally derived"**: REJECTED. CSS cannot compute retry-safety from a failure-term×idempotency input; the layer correctly provides the three outcome primitives (`--safe/--maybe/--unsafe`) and the showcase documents the full derivation matrix. Derivation is a consumer (cockpit) responsibility, not the layer's. (The "5/14 failure terms shown" sub-point is a nit — UX.md requires terms "remain distinct," not that all 14 FailureCodes be showcased.)

Nits (noted, not fixed):
- F6-common: common components `.select`/`.divider`/`.toast`/`.card--interactive` not each given a dedicated showcase cell. Showcase-completeness polish, not a floor obligation. (`.toast` is now AA-compliant regardless.)

**Verification (post-fix)**: all 4 registries fully bound + exercised in showcase (CommandState 9/9, Connectivity 5/5, Activity 3/3, ElicitationState 9/9); 13/13 critical contrast pairs pass AA; CSS brace-balanced (components 146/146, tokens 4/4); all `var()` references resolve (`--shadow-raised` is defined in components.css itself, intentional per header); dominance rule holds with and without `:has()`.

**Notes**: standard weight, single pass, no re-review. The review caught a real land-mode verification failure (F1 — I under-audited ElicitationState) plus 4 other material issues the land-mode pass missed; the fresh-context cross-model pass earned its cost.

## Implementation discovery (2026-07-19) — bounced to drafting

**The finding.** Operator review of the standard-weight review outcome surfaced that the component layer was never rigorously designed or implemented — it was generated by the `ux-ui-design:palette` + `components` mockup skills and declared "locked + implementing" in the same stride, with no separate implementation pass that built the layer against its design contract. The land-mode implementation pass then verified the artifacts by re-checking the implementer's own claims rather than independently auditing against the registries, which is how a 3/9 ElicitationState binding and a 1:1-contrast (invisible) `.toast` rule shipped through to review. The standard review caught 5 blockers and fixed them, but "fixed the findings one pass surfaced" is not "rigorously engineered" — a single standard pass is deliberately not exhaustive, and deeper accessibility (keyboard focus, ARIA semantics, screen-reader labels, `prefers-reduced-motion` for the `pb-spin`/`pb-pulse` animations — none guarded), semantic correctness of each binding (not just presence), and the layer's central "machine-checkable" obligation remain unaddressed.

**The design flaw (not an implementation defect).** The feature's Brief states the layer exists to "make the conformance floor machine-checkable (states cannot drift between surfaces)." But the feature never specified the machine-checkable mechanism as a concrete deliverable. It named the obligation in prose, shipped the CSS class taxonomy, and advanced to `implementing` on the strength of the mockup artifacts alone. The CSS taxonomy is the *substrate* a conformance check would assert against — it is not itself the check. A class taxonomy cannot enforce that a consumer emits the correct class, derives retry-safety from the failure term × idempotency_strength rather than CommandState alone, or refuses to render stale connectivity as live. Those are runtime/consumer checks; the layer's "machine-checkable" claim is hollow without a static conformance vector that asserts registry↔class↔showcase correspondence and a runtime contract that the cockpit (and future CLI/Expo) must satisfy.

This is a design gap, not a coding bug: the deliverable set was under-specified at `feature-design` time. Forcing it through to `done` on the strength of one standard review pass would silently treat a mockup-generated artifact as a rigorously engineered structural-enforcement layer — exactly the failure mode this verification program exists to prevent.

**Concrete re-design proposal (for the design gate to pick up).** The repo already has an established conformance-vector enforcement pattern that this layer should mirror:
- `contracts/vectors/*.json` + `contracts/scripts/check-vectors.mjs` — executable JSON vectors that constrain protocol behavior and trace to formal-model/stated-normative properties, with a `check-vectors.mjs` script that enforces them at build/review time and a `check-generated-drift.mjs` + `check-models.mjs` companion.
- `feature-command-state-ssot` (done) — the anti-drift sibling that consolidated command/session/failure state machines into one source of truth precisely to kill the registry-drift this layer is supposed to prevent.

The design pass should specify:
1. **A static presentation conformance vector** (likely `contracts/vectors/presentation-state-*.json` or a `contracts/scripts/check-presentation.mjs`) that asserts: every member of `CommandState` (9), `SessionConnectivityState` (5), `SessionActivityState` (3), `ElicitationState` (9) has a corresponding CSS class binding in `components.css`; no invented/divergent state names exist; every locked primitive is exercised in `components.html`; the retry-safety matrix matches the `docs/UX.md` table for all failure-term × idempotency_strength combinations.
2. **A runtime conformance contract** for consumers (cockpit/CLI/Expo) — what the layer *guarantees* (the primitives + their state-binding semantics) vs what the consumer *must* enforce (verify identity-before-submission; derive retry-safety from the inputs, never CommandState alone; never render stale as live; disable elicitation controls on terminal). The cockpit is the first consumer and must be checked against this contract, not just linked to the CSS.
3. **Accessibility as a checked property**, not a claim — an automated contrast/a11y check (none exists today; the `toast` defect proves the claim wasn't enforced) plus `prefers-reduced-motion` guards on the animations.
4. **A verification-rigor decision** for the layer's own review weight: given it is a structural-enforcement seam (not a leaf consumer), `standard` was the wrong default; the design should set the review weight (likely `thorough` or `maximum`) so the layer earns its "machine-checkable" claim.

**What stays.** The locked CSS primitives, tokens, and showcase are retained as the mockup-time analog (`docs/UX.md:49`'s wording) — they are good artifacts and the 5 review fixes are kept. They become inputs to the re-design, not the deliverable. The `palette`/`components` design decisions (Q1–Q5, Option C, the state-binding contract) are retained.

**Lifecycle.** Reverted `done → drafting` per the implement skill's design-flaw escape hatch. The feature returns to `feature-design` for the design gate to specify the conformance-vector + runtime-contract deliverable before any further implementation. The parked `backlog-presentation-conformance-vector` is absorbed into this re-design (it was the symptom; this discovery is the cause). `feature-v0-web-cockpit`'s `depends_on` edge to this feature is now genuinely unmet again — the cockpit must not proceed on the assumption that the layer is done.
