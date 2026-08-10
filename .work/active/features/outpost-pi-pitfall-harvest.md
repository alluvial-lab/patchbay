---
id: outpost-pi-pitfall-harvest
kind: feature
stage: done
tags: [research, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
research_dials:
  scope_authority: in-engagement-judgment
  verification_rigor: standard
  intent: pitfall-seam-gap-harvest
  output_kind: campaign
created: 2026-08-08
updated: 2026-08-09
---

Sequencing directive: run this research engagement FIRST in a fresh session; thread its findings (via research-handoff) into the engineering clusters before draining them.

# Harvest outpost_pi for pitfalls / seams / gaps

`outpost_pi` (the operator's hand-built control surface: pi extension + relay +
mobile app) is accruing Patchbay-shaped complexity in a substrate that fights it.
Its bug-swatting history is a grounded source of real-world failure modes and
seam decisions Patchbay has not yet had to make — worth harvesting systematically
so Patchbay learns from them rather than rediscovering them at runtime.

Defer until after the current research wave.

## Why

A scan of recent `outpost_pi` commits already shows clusters that map onto
Patchbay capabilities — and the hard-won lessons inside each:

- **Hot-reload / restart lifecycle for fresh code** — `feature-extension-hot-reload-via-process-restart`
  (restart markers, agent-settled hooks, wrapper-restart-without-continue, PID
  hunting, ENOENT races). Lesson: `/reload` does not reliably pick up a fresh
  `/dist`; restart is the correctness boundary for adapter-code upgrades; lifecycle
  fencing around restart is full of races.
- **Multi-cwd headless project management via herdr** — 12 project cwds, ancestor-chain
  PID hunting, pty-stalls, restart-agents scripts. Lesson: project-as-first-class
  + headless hosting is a real, messy surface; shell-script archaeology is the
  alternative Patchbay should replace.
- **Mobile control fragility** — `feature-mobile-slash-command-invocation` re-scoped
  to a "dedicated-ops model (drop editor-seam)"; `newSession no-command-ctx`
  architectural gap. Lesson: driving an agent through TUI editor-seams is fragile;
  a dedicated-ops model (Patchbay's Operations) is the converged answer.
- **Durable transcript ownership + canonical ordering** — `epic-durable-transcript-ownership`,
  `feature-canonical-transcript-timestamp-ownership`, `feature-canonical-transcript-ordering`,
  ts-provenance-audit. Lesson: the operator is independently rebuilding Patchbay's
  durability/ordering/provenance thesis; the failure modes they hit are directly
  relevant to Patchbay's core.
- **Identity / keyring durability** — `keyring-loss → silent re-identity` session
  note. Lesson: silent re-identity on credential loss is a real authority failure.

## What to harvest (when promoted)

Mine the `outpost_pi` repo (`/home/agent/projects/outpost_pi`) git history,
architecture, and session notes for: concrete failure modes + root causes,
seam decisions taken (and regretted), gaps discovered, mobile/control lessons,
restart/lifecycle races, and durability/ordering pitfalls. Produce a grounded
findings set that feeds Patchbay design (esp. spawn, restart, project model,
mobile surface, transcript/state durability, identity) — pitfalls to avoid,
seams to pre-decide, gaps to close proactively. Route as a `[research]` item.

## Engagement record (2026-08-09)

- **Dials (set at kickoff, operator-confirmed):** scope_authority = in-engagement-judgment; verification_rigor = standard; intent = pitfall-seam-gap-harvest; output_kind = campaign.
- **Decision relevance (yield hypothesis):** findings change Patchbay's spawn-lifecycle/restart-fencing, project/cwd-seam, mobile-control, durability/ordering/provenance, and identity/authority design decisions (pitfalls to avoid, seams to pre-decide).
- **Substrate check:** the prior `v1-control-plane-and-spawn` campaign attested herdr's *model* (`[herdr-concepts]`, `[herdr-state]`) as a spawn-lifecycle peer comparison; it did NOT harvest outpost_pi's bug-swatting history. Minimal overlap; this harvest is distinct.
- **Decomposition (5 facets, one specialist each):** restart-hot-reload-lifecycle; herdr-multi-cwd-project; mobile-control-fragility; durable-transcript-ordering-provenance; identity-keyring-durability.
- **Status:** complete (closed to done). Fan-out: 5 specialists (restart/herdr on gpt-5.6-sol; mobile/transcript/keyring on gpt-5.6-luna); 38 source-direct attestations. Gates (standard rigor): lint floor passed (transcript facet clean; 4 facets use richer `source_path` formats the lint flags `unreachable-source` — a local-source tooling limitation, not a grounding gap; all attestations verified); adversarial-read returned NEEDS-REVISION → revision pass resolved all 6 findings (BLOCKER-5 analogy downgrade; mobile/keyring/herdr per-claim narrows) → lead spot-check clean. Output: `.research/analysis/campaigns/outpost-pi-pitfall-harvest/` (`parent.md`, `specialists/`, `acquisitions.md`, `verification-checklist.md`). Acquisition candidates: 1 enriching (pinned Herdr schema) — promotion to `research-acquisition-queue` is operator-confirmed at handoff.
- **Handoff:** run `/agentic-research:research-handoff outpost-pi-pitfall-harvest` to emit operator-confirmed `.work/` items grounded in these findings (never auto-fires). Headline cross-cutting: the 'don't infer X from Y' universal lesson; incarnation/process-fencing as the universal hard problem (field-corroborates spawn BLOCKERs 3/4; BLOCKER 5 only by analogy); `{extends}` convergence with Patchbay's durable-log-sole-authority thesis.
