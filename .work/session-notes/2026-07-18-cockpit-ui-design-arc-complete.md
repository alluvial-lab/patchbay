# Session note — 2026-07-18 (review queue normalized; cockpit UI/UX design arc complete)

A durable handoff note for the next session. Read this before continuing.

## Where we are

`epic-v0-1-0-implementation` — **4/6 layers done** (core + seam + web-server + pi-adapter). This session did NOT ship a new layer; it did two things: (1) drained the stale review queue, and (2) completed the **full UI/UX design arc** for the last phone-usable layer (`feature-v0-web-cockpit`) plus its prerequisite (`feature-v0-presentation-component-layer`). Both features are now at `stage: implementing` with complete designs, ready for implementation.

The remaining work: implement the two cockpit features (component layer + cockpit), then the CLI. The phone-usable critical path's last layer is designed and unblocked.

## What happened this session

### 1. Review queue normalized (0 → 0, 8 stale → done)

The review queue had 8 items — all child stories of `feature-v0-core-sessions`, which was **already `stage: done`** since commit `f843dc0` (full B1–B5 deep-review convergence + delta re-review). The 8 children were never normalized from `review` → `done` when the feature advanced (legacy bookkeeping). Per the review skill, **child stories never enter review** — green verification advances them directly to `done`. Verified the tree green (build + test + clippy clean), normalized all 8 to `done` (commit `7430f2c`). Board: review 8→0, done 96→104.

### 2. UI/UX design arc — `feature-v0-presentation-component-layer` → `implementing`

Scoped the **shared presentation-component layer** as a separate sibling feature (operator decision Q1a — it's a pre-requisite, not absorbed into the cockpit). This is the structural enforcement of the UX conformance floor that `feature-ux-v0-acceptance` named but deferred.

Ran the locked `ux-ui-design` pipeline:
- **Palette** (`palette.html` → `tokens.css`): 3 options explored (Nostromo/LCARS, Soviet cybernetics, Swiss/Editorial) + 3 hybrids. **Locked Variant 1 — Nostromo/LCARS** (amber phosphor on warm instrument-panel, dark-first). WCAG AA verified both modes.
- **Typography**: 2 options. **Locked IBM Plex Mono / Plex Sans hybrid** (mono chrome + readable sans body — resolves the mobile-markdown tension).
- **Components** (`components.css` + `components.html` showcase): common starter set + **11 project-unique state-binding primitives** that make the floor structural. Aesthetic: subtle depth / mixed corners / dual density.

Two sharp design inputs from the operator's outpost_pi experience, grounded against PROTOCOL before deciding:
- **Connectivity/activity are separate visual channels** (not a merged label) — two sub-primitives (`.connectivity-indicator`, `.activity-indicator`) + `.session-status` composition wrapper applying the dominance rule. Aligns with PROTOCOL's two-axis composition.
- **`working` stays a 3-value protocol axis** (Option C) — thinking-vs-executing is a *presentation detail composed from the Observation stream*, not a registry promotion. Avoids reversing the reserved "richer activity details" seam.

### 3. UI/UX design arc — `feature-v0-web-cockpit` → `implementing`

The operator's primary control surface (v0.1.0 product center). Mockup-first through a real review loop; the shell is honed.

**Scouted current LLM-app UIs** (Codex desktop, Claude Code desktop redesign, Cursor 3, Antigravity) — all converged on sidebar-as-control-plane + multi-agent supervision + status-forward chrome. Validated operator-console (Q4b) over Claude-app-warmth; warmth lives in message rendering only.

**Design decisions pinned (Q1–Q4 + EC1–EC4):**
- Q1: two-pane desktop / drill-in mobile (committed A / reserved B — B is both the reserved seam AND the natural mobile mode)
- Q2: delivery badge below message (compact, expandable debug detail; LSNs hidden by default)
- Q3: chat alignment (operator right / agent left, capped 860px column, 560px left-side content)
- Q4: text-first composer + contextual actions + attach button
- Q5a: thin translator, operator domain browser-only (reserved-seam posture confirmed)

**3 response-contract-shape decisions (EC1–EC3) + Attention deferral (EC4)** — all grounded against PROTOCOL:
- EC1: free-text option within `question` contract → committed
- EC2: "answer-and" composed response (selection + clarification in one Operation) → committed
- EC3: grouped multi-question = N independent single-answer Elicitations as one card → committed grouping (true multi-answer contract is a reserved seam, PROTOCOL:312)
- EC4: Attention destination deferred from v0.1.0 (elicitations surface inline + via needs-you badge; mock preserved on disk)

**5 implementation units** written into the feature body with acceptance criteria + implementation order: protocol client + cursor-reconcile, presentation model fold, markdown rendering (the mobile-readability differentiator), elicitation handling (3 shapes + mobile sheet), shell + list + detail.

**Mock review loop** (iterative, ~12 commits): fixed FAB overlap, header flex, mobile transparency, mobile scroll lock, redundant approval option-list, elicitation width consistency, radio/checkbox mixing, multi-question blank-on-mobile teaser, mobile sheet content cloning, nav wiring across screens, and finally folded the standalone detail mock into the shell (one coherent product mock). The shell is at `.mockups/screens/feature-v0-web-cockpit/option-2.html` (self-contained, interactive mobile drill-in).

## What's next

Both cockpit features are at `stage: implementing` with deps satisfied:
- `feature-v0-presentation-component-layer` — depends on `feature-v0-web-server` (done). Design = the design-system pipeline (already locked).
- `feature-v0-web-cockpit` — depends on `feature-v0-web-server` (done) + `feature-v0-presentation-component-layer` (implementing).

**Next:** implement the two cockpit features. `feature-v0-presentation-component-layer` first (it's the cockpit's dependency), then `feature-v0-web-cockpit`. Either via `/agile-workflow:implement-orchestrator` or inline `/implement`. After those land, the CLI (`feature-v0-cli`) is the last v0.1.0 layer.

## Key artifacts produced this session

- `.mockups/design-system/tokens.css` — locked Nostromo/LCARS + Plex tokens
- `.mockups/design-system/components.css` + `components.html` — 11 state-binding primitives
- `.mockups/design-system/palette.html`, `typography.html`, `typography-in-palettes.html` — design-system previews
- `.mockups/screens/feature-v0-web-cockpit/option-2.html` — the locked, honed sessions shell (self-contained)
- `.mockups/screens/feature-v0-web-cockpit/attention/attention.html` — designed-but-deferred Attention destination (preserved, not wired)
- `.work/active/features/feature-v0-presentation-component-layer.md` — full design (drafting → implementing)
- `.work/active/features/feature-v0-web-cockpit.md` — full design + 5 units + pinned decisions (drafting → implementing)

## Notes for the implementer

- The cockpit consumes `tokens.css` + `components.css` for all state-binding — it does not re-bind protocol states. The mocks are the visual reference; translate into real components driven by live protocol state.
- **One flagged risk** (potential blocker): the EC1–EC3 response-payload shapes are new and may require a proto extension to the `elicitation-response` Operation. If that extension is a semantic 50/50 (not mechanical), surface it as a blocker per the harness rule — do not resolve with judgment.
- The markdown renderer choice (Unit 3) should be spiked early — must be small + safe + streaming-friendly.
- Reconnect correctness (Unit 1) is load-bearing: the snapshot-correctness rule means an unreconciled snapshot must never render as live. Property-test the reconcile path.
