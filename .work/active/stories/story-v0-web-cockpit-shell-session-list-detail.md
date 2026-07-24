---
id: story-v0-web-cockpit-shell-session-list-detail
kind: story
stage: done
tags: [ux]
parent: feature-v0-web-cockpit
depends_on: [story-v0-web-cockpit-markdown-rendering, story-v0-web-cockpit-elicitation-handling]
created: 2026-07-20
updated: 2026-07-21
release_binding: v0.1.0
gate_origin: null
---

# Story: Cockpit Unit 5 — shell + session list + responsive detail

Implements Unit 5 of `feature-v0-web-cockpit`. Composes Units 2/3/4 into the
locked shell.

## Scope

- `web-cockpit/src/ui/shell.ts` — the responsive shell: desktop two-pane
  (list + live detail side-by-side), mobile drill-in (list home, tap →
  full-screen detail + back).
- `web-cockpit/src/ui/session-list.ts` — session rows: identity-before-intent
  (identity tuple primary, labels metadata), connectivity×activity badges
  (separate channels, per the split), and the needs-you state.
- `web-cockpit/src/ui/session-detail.ts` — message timeline + delivery badges
  + composer.

## Design reference

The locked mock is `.mockups/screens/feature-v0-web-cockpit/option-2.html`
(selected 2026-07-16, option-2 Identity-forward). It is self-contained
(inlined tokens+components) and interactive (mobile drill-in works via
tap/back). Translate it into real components driven by live protocol state —
do not copy its inlined CSS; consume `tokens.css` + `components.css`.

### v0.1.0 scope: sessions shell only

Operator decision (2026-07-17): pare down to just the sessions shell for
v0.1.0. The Attention destination is deferred (EC4) — elicitations surface
inline + via `needs-you` badge.

### Responsive IA (committed A / reserved B)

- Desktop: two-pane (list + live session-detail side-by-side).
- Mobile: drill-in (list is home, tap a session → full-screen detail with back
  button).
- B (drill-in) is both the reserved seam AND the natural mobile mode.
  Promotion to desktop drill-in is additive (container change), not a rebuild.
- Detail-pane header hidden on desktop (redundant with the active sidebar
  row); kept on mobile (drill-in needs back button + which-session context).

### Session detail (folded into the shell's right pane)

Chat-aligned timeline (operator right / agent left, capped 860px column,
560px left-side content width), markdown rendering in agent bubbles (the
mobile-readability differentiator — Unit 3), delivery state as a compact
badge below each message showing the current `CommandState` and last
transition (v0.1.0 scope per `docs/SPEC.md` § observability — a full
expandable per-command delivery trace is a reserved seam; see the feature
body's Q2 revision), binary approval = direct buttons,
multi-option question = radio + free-text option (EC1) + answer-and (EC2),
grouped multi-question card (EC3). Mobile: bottom-sheet for elicitations,
fixed composer, page scroll. Teaser previews on mobile: clamped prompt +
'Tap to answer' affordance.

## Implementation notes

- Uses the presentation-component layer primitives
  (`.session-row`, `.session-status`, `.connectivity-indicator`,
  `.activity-indicator`, `.composer`, `.elicitation-card`, `.command-step`,
  `.delivery-line`, `.attention-badge`, etc.) from `components.css` — no
  inline protocol-state rebinding. If the cockpit introduces new
  state-bearing CSS that bypasses the component layer, that is a
  conformance-floor violation.
- The drill-in (mobile) is a container swap, not a separate screen — the
  `session-detail` content component is identical in both modes (the reserved
  B seam).
- Composer: textarea + attach + send; contextual actions (Cancel/Interrupt)
  appear near running commands. No composer-level OperationKind selector —
  actions surface contextually where relevant (Q4).
- Identity-before-intent: show stable target identity (adapter/scope/runtime/
  gen) before allowing submission (the conformance-floor obligation). The
  composer must not enable Send until the selected target identity is stable
  and non-superseded.

## Acceptance criteria

- [x] Desktop: list + detail side-by-side; selecting a session fills the detail pane
- [x] Mobile: list is home; tap drills into full-screen detail; back returns to list
- [x] Session rows show identity tuple + connectivity/activity (separate channels)
  + needs-you state
- [x] All state-binding uses the presentation-component layer primitives (no
  bespoke protocol-state CSS)
- [x] Composer does not enable Send until stable, non-superseded target identity
  is selected (identity-before-submission)
- [x] The mobile drill-in and desktop two-pane share one `session-detail`
  component (the reserved-B seam is a container swap, not a fork)
- [x] Delivery badges render the compact current `CommandState` + last
  transition only (a full expandable per-command delivery trace is a reserved
  seam, deferred per `docs/SPEC.md` § observability — v0.1.0 does not carry a
  trace-timeline UI)

## Verification evidence

- Visual/conformance: the shell must pass the presentation conformance check
  (`contracts/scripts/check-presentation.mjs`) if it touches
  `components.css`/`tokens.css` — but as a *consumer*, not a re-definer.
- Interface test: identity-before-submission (Send disabled until stable
  target); stale-never-live (a session marked stale by reconcile does not
  render as live).
- The shell composes the lower units; its tests are the integration of the
  conformance-floor properties (identity-before-submission, stale-never-live,
  delivery-state separation, reconnect reconciliation).

## Risk

If the shell finds it needs a protocol-state presentation that the component
layer's locked primitives don't cover, that is a conformance-floor gap —
surface it (extend the component layer, do not bypass it). The component-layer
arc's lesson applies forward: a claimed-but-not-enforced conformance surface
is a liability.

## Implementation completion notes (2026-07-20)

- Execution capability: inline feature-owning worker; the three UI modules share
  one projection and one responsive container policy, so keeping ownership
  together reduced integration risk.
- Review weight: standard (project/default); child story closes on green
  verification and receives no independent review.
- Files changed: `web-cockpit/src/ui/shell.ts`, `session-list.ts`,
  `session-detail.ts`, `shell.css`, and `web-cockpit/tests/shell.test.ts`.
- Tests added: 6 shell/interface tests covering desktop composition, mobile
  drill-in/container reuse, generated identity-before-submission cases,
  generated stale-never-live cases, markdown/delivery/action/Elicitation
  integration, and CSS non-rebinding.
- Simplification: desktop and mobile use the same `session-detail` instance;
  native `<details>` provides compact-by-default delivery history without a
  second disclosure state machine; contextual command actions stay out of the
  composer.
- Discrepancies from design: added `shell.css` for surface-only responsive and
  chat layout. It consumes locked tokens/primitives and deliberately contains no
  connectivity, activity, command, or Elicitation protocol-state bindings.
- Adjacent issues parked: none.

## Verification result (2026-07-20)

- `cd web-cockpit && npm test` — PASS (25 tests).
- Property evidence: 100 generated target-shape cases enforce
  identity-before-submission; 100 generated reconciliation/state cases enforce
  stale-never-live at the session-row binding.
- Acceptance walk: desktop two-pane, mobile drill-in/back, identity-first rows,
  separate connectivity/activity, one detail component, compact delivery
  disclosure, contextual actions, and composer gating all pass.

## Review-fix completion (2026-07-20)

- Replaced the expandable full-history/LSN disclosure with the committed
  current `CommandState` + last-transition-only badge; the full trace remains a
  reserved seam.
- Integrated same-correlation question groups into session detail as one
  `elicitation-card`, while preserving independent single-answer forms and
  terminal state per Elicitation.
- Added locked-primitive failure, deduplication/retry-safety, reconnect, stale,
  and offline surfaces.
- Verification: `cd web-cockpit && npm test` PASS (36 tests), including the
  integrated grouped-card, reduced-delivery, failure, deduplication, and
  degraded-banner assertions.
