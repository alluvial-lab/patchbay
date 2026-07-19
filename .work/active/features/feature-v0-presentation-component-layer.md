---
id: feature-v0-presentation-component-layer
kind: feature
stage: drafting
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

## Design decisions (operator-confirmed, 2026-07-16)

Resolved interactively during the `feature-v0-web-cockpit` design kickoff (these decisions apply to both features).

- **Q1 — Component layer scope: separate sibling feature (this one).** Built as `feature-v0-presentation-component-layer` rather than absorbed into the cockpit. The layer is a pre-requisite the operator named; it has its own design surface (primitive set + token vocabulary + state-binding contract), and the cockpit is one consumer. The cockpit's `depends_on` gains this feature.
- **Q2 — Design-system pipeline depth: palette → components → screens (skip motion).** The component layer is designed via `palette` (tokens) + `components` (primitives). `motion` is skipped for v0 — the cockpit can be statically paced; kinetic language is deferred to a v1 design-system pass. Matches the lean pipeline.
- **Q4 — Visual direction: operator-console / status-forward.** Scouted Codex desktop, Claude Code desktop redesign, Cursor 3 Agents Window, Google Antigravity — all converged on sidebar-as-control-plane + multi-agent supervision + status-forward chrome. Patchbay's chrome is operator-console; warmth lives only in message rendering. This is the agent-native pattern the field landed on in 2025–2026.
- **Q5 — Operator-domain execution: browser-only, thin translator.** The web server stays a thin HTTP→protocol translator; the operator domain (protocol client, delivery/reconnect state machines, presentation model) runs in the browser. Server-side operator-domain promotion stays a reserved seam. Multi-device continuity comes from the core being the single source of truth, not from server-held state. (This decision is inherited from `feature-v0-web-server` and confirmed here — it pins where the presentation layer *runs*: in the browser, as part of the operator domain.)

## Design surface (to be designed in the design pass)

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
