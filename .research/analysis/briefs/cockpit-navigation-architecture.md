---
updated: 2026-07-28
---

# Cockpit Navigation Architecture (2025–2026 adaptive practice)

Synthesis of two research passes: prior-art mining in
`~/projects/SNC/platform` (ux-decisions.md, responsive-design brief,
implemented nav code) and current canonical/exemplar research (Material 3
adaptive layouts, Apple HIG split views, VS Code/Slack/Linear/GitHub/Discord
practice, ARIA window splitter, Primer/Carbon shells, modern CSS).

## The one-sentence answer

**Destination navigation and in-destination pane hierarchy are two
independent adaptive layers.** Destinations adapt bar↔rail by available
space; each destination's list→detail hierarchy adapts drill-in↔side-by-side
independently. Preserve the information architecture and selection state
across sizes — never the column geometry.

## Canonical guidance (Material 3 / Apple HIG)

- Compact: bottom navigation bar for destinations; one pane at a time
  (list→detail is drill-in with Back). Expanded: navigation rail; list+detail
  side by side; optional third "extra/supporting" pane only when space and
  task justify it. (Android Developers — Build adaptive navigation; Build a
  list-detail layout; Build a supporting-pane layout.)
- Apple HIG mirrors this: sidebar → content → optional inspector; compact
  collapses hierarchy into push/back navigation; the inspector is subordinate
  and disappears first. (HIG — Split views, Sidebars, NavigationSplitView.)
- Window size classes are adaptive inputs (compact/medium/expanded/large/xl,
  width AND height separately), not a device taxonomy.

## Patchbay pane taxonomy (fills the SNC prior-art gap)

| Region | Role | Narrow-screen fate |
|---|---|---|
| **Rail** | Global destination switcher (Sessions, Security, Diagnostics, Files, Git, Settings) | Becomes bottom bar (≤5 items + More overflow; 6 destinations means one overflows or two merge) |
| **List pane** | The current destination's collection (session list, grant list, file tree) | The destination's landing view; selection drills into detail |
| **Detail** | Primary work surface (transcript, security section, diff) | Full-screen route with identity + Back in the header |
| **Inspector** | Supporting/secondary (adapter status, command detail, file preview) | Sheet, dialog, or subroute — never required for core control; first region to disappear |

The session list is NOT a seventh destination — it is the Sessions
destination's list pane. This was the SNC platform's main un-answered
question too (no nested-pane taxonomy, no pane-state contract).

## Collapse-handle convention (resolves our inconsistency)

- A **sash/splitter resizes**; a **chevron collapses**. Don't conflate them.
  ARIA window splitter pattern for keyboard-accessible resizing.
- One visual language, role-distinct placement: the **rail** toggles via a
  global control in its own header; a **pane** toggles via a directional
  chevron at its boundary; an **inspector** is always summonable/dismissable
  and easiest to close.
- A pane toggle is only offered when the remaining pane stays usable alone.
- Persist collapse/width per user/device, but never restore a multi-pane
  state blindly into a narrow window.
- (Sources: ARIA APG window splitter; Primer PageLayout pane-local policy;
  Carbon global header; exemplar practice.)

## Mobile mapping for the cockpit

Bottom destination nav → destination's list → drill-in detail (identity +
Back in header) → inspector as sheet/subroute. No persistent three-column
geometry on phones — 2025-2026 guidance (incl. Android's June 2026 update)
continues to formalize exactly this. Slack/Linear/GitHub/Discord all do the
same: stable tiny destination switcher + drill-in everything else.

## Modern CSS enablers

- Viewport media queries decide SHELL topology (bar vs rail, columns
  allowed). Container queries decide PANE-LOCAL composition (toolbar → icons,
  metadata stacking) — a pane beside other panes can be narrow on a wide
  viewport. (`container-type: inline-size`; MDN container queries.)
- `dvh` for full-height transient compositions; `svh` where browser chrome
  could obscure controls (composer!). `env(safe-area-inset-*)` on every fixed
  bar. (MDN viewport units, env().)
- SNC lessons worth importing: bottom tabs beat hamburger (NNG 2025: −20%
  discoverability, +39%/15% task time); 3–5 tabs with overflow; SSR/CSS-first
  adaptation (no JS layout branching on first render); content-driven
  breakpoints, not one universal 768px.

## What the SNC prior art got right vs needs revisiting

Right: mobile-first, bottom tabs, hybrid overflow, token discipline, dvh,
touch targets. Needs revisiting: universal 768px switch (fragile),
viewport-only adaptation (no container queries in nav), `100vh` legacy in
layouts, missing safe-area on the fixed bar, no nested-pane taxonomy or pane
state contract — all addressed above.

## Sources

Canonical: developer.android.com (adaptive navigation, list-detail,
supporting pane, window size classes), Apple HIG (split views, sidebars).
Exemplar: VS Code UI docs, GitHub Mobile docs. Patterns: ARIA APG window
splitter, Primer PageLayout, Carbon global header. Prior art:
SNC/platform docs/ux-decisions.md, .research/analysis/briefs/
responsive-design-best-practices.md + implemented nav code.
