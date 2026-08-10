---
campaign: outpost-pi-pitfall-harvest
provenance: agent-synthesis
updated: 2026-08-09
---

# Acquisition candidates — outpost_pi pitfall harvest

Consolidated from specialist returns. Research-side persist (always). Promotion into `.work/backlog/research-acquisition-queue` is **operator-confirmed** at the `/agentic-research:research-handoff` gate — never auto-fired.

## Enriching

### Pinned Herdr v0.7.5 source/schema + CLI JSON fixtures

- **Class:** `primary-doc`
- **Web-availability:** partial — Herdr docs are public (`https://raw.githubusercontent.com/herdrdev/herdr/master/docs/.../concepts.mdx`, `session-state.mdx`); the CLI JSON response schema for workspace/pane/process APIs is not published as a fixture and was inferred from one outpost_pi field assumption that failed (`[herdr-setup-pane-fix]{1}`).
- **Completes:** the herdr-multi-cwd facet's bounded-confidence claims about which response shapes are stable across Herdr versions; the workspace-id/pane-id/agent-name grammar divergence (`[herdr-restart-bulk-fix]{1}`); the multi-wrapper restart-marker isolation question left open after the foreground-TTY correction.
- **Motivation (source-bound):** the outpost_pi history proves one field assumption failed and that generated workspace IDs containing uppercase were invalid as agent names, but establishes no cross-version stability. A pinned schema + conformance transcript would let a future Patchbay herdr-style adapter (or the project/cwd seam design) distinguish "one observed break" from "stable contract."

## Blocking

None. All load-bearing sources were locally available (outpost_pi git history, code, session notes, installed Pi SDK definitions); no claim is held on an unfetchable source.
