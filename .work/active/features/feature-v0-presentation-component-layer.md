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
updated: 2026-07-16
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
