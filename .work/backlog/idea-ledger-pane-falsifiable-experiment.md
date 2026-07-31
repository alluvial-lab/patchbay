---
id: idea-ledger-pane-falsifiable-experiment
created: 2026-07-30
updated: 2026-07-30
tags: [ux, experiment, mockup-first]
---

# Ledger pane as a cheap falsifiable experiment

Surfaced as the legitimate residue of the combined-surface vision (round 5,
both reviewers). If the combined-view hypothesis is worth testing at all, this
is the cheapest way to test it — and the repo's own AGENTS.md mandates
mockup-first for new surfaces, so it's also the disciplined next step.

## The hypothesis

Viewing Patchbay's durable delivery contract (intent + reported outcome +
failure vocabulary) alongside a git-backed work ledger and pooled-fuel state,
in one cockpit, changes an operator decision the incumbent (agile-workflow +
chat harness) gets wrong.

## Why this is a backlog item, not a vision

Five review rounds proved the combined-surface *vision* was always Patchbay's
delivery contract viewed alongside other things — the delivery contract *is*
Patchbay. The committed work (token-commune adapter, public product contract)
proceeds on its own tracks. The only genuine uncommitted delta was the ledger
pane. So: mock the pane, test the hypothesis, and only build seams if the
hypothesis survives.

## The cheapest test (mockup-first, over the existing substrate)

Per AGENTS.md's mockup-first convention: a single-file HTML mockup (or set of
options) under `.mockups/screens/` rendering the delivery contract timeline
alongside the existing agile-workflow `.work/` ledger (no Workbench conversion
— the agile-workflow substrate is what's actually running, and converting
mid-build loses load-bearing `work-view` tooling for no gain). Render it
alongside a token-commune panel placeholder (the fuel adapter is two drafting
epics away; a static panel is fine for the mock).

No generic ledger contract. No public-core API. No Workbench-native migration.
No adapter-instance relabeling. No correlation-validation machinery. Just:
does seeing these three together change a decision?

## Kill criteria (what falsifies the hypothesis)

The reviews were explicit that the vision supplied no failure case where the
combined view would have produced a materially better result than the
incumbent. So the experiment needs concrete kill criteria:

- If the operator never encounters a recurring failure that the combined view
  would have caught (missed start, stale-item execution, lost intent on
  handoff), the pane doesn't earn demand. "Context switching is annoying" is
  not enough.
- If the async-detached-commissioning pattern (the one scenario where the
  delivery contract genuinely earns its keep — operator fires agents across
  machines and walks away, one silently never starts) is not a real recurring
  pattern for anyone beyond the builder, the combined view has no audience.
  See `idea-combined-surface-demand-research`.
- If the incumbent (git-backed ledger + chat log) already covers intent/
  delivery/failure well enough in practice, the pane is theoretical rigor, not
  product value.

## What the experiment does NOT justify (even if it succeeds)

Per the round-5 reviews, a successful pane mock does not establish pressure
for: agents-as-principals, multi-operator visibility, the IDE extension, the
public-core API, the generic ledger seam, or Workbench-native migration. Each
of those needs its own demonstrated pressure. The pane tests one thing: whether
co-location plus a durable delivery contract changes operator decisions.

## Source

Combined-surface vision review round 5 (GPT verdict + Kimi finding 1), against
`.mockups/` (no ledger-pane mock exists) and AGENTS.md's mockup-first mandate.
