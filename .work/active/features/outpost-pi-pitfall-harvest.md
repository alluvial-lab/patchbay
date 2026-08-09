---
id: outpost-pi-pitfall-harvest
kind: feature
stage: drafting
tags: [research, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
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
