---
id: idea-combined-surface-demand-research
created: 2026-07-30
updated: 2026-07-30
tags: [research, demand, parking]
research_refs: []
---

# Combined-surface demand research (empirical, not architectural)

Surfaced across all five review rounds. This is the one item from the
combined-surface exploration worth following up with *research* rather than
parking as a finding — because it's an empirical question the architecture
can't answer.

## The question

The combined-surface vision's residual value (after stripping committed and
standalone work) rested on one scenario: **async detached commissioning** —
the operator fires agents across machines and walks away; one silently never
starts; the durable `accepted` record with no `running` transition is the only
place that failure is visible without reading scrollback.

The reviews found this scenario was asserted as "the common case" with zero
usage evidence. Every grounded signal in the repo describes an n-of-1 product
built by its primary user. The demand intersection (operators who run Patchbay
*and* Workbench *and* token-commune) is n-of-1 by construction.

So the question: **is async detached commissioning a real recurring failure
pattern for anyone beyond the builder-operator, and does the combined view
change a decision the incumbent gets wrong?**

## Why this is research, not architecture

More architecture can't answer it. The reviews proved the architecture was
sound enough to attack for five rounds — the problem was always that the
payoff was asserted, not demonstrated. The only way forward is empirical:
observe actual operator behavior, count failures, test whether a combined view
changes outcomes.

## What research would look like

Per the agentic-research discipline (source-bound, not fabrication): this
isn't a web-searchable fact. It's a usage study of the builder-operator's own
workflow over a representative period — log when commissions go detached, when
they fail silently, when the incumbent's chat + git + work-view combination
loses intent/delivery/failure that a durable delivery contract would have
caught. Then: would a second operator (if one materializes via v1.0.0
self-hosting) have the same pattern?

## Why it's parked, not commissioned

The research is worth doing *if* the ledger-pane experiment
(`idea-ledger-pane-falsifiable-experiment`) is run and produces ambiguous
results. If the pane mock clearly succeeds or clearly fails, the demand
question answers itself. The research is the fallback for the ambiguous case,
and there's no point commissioning it until the cheaper mockup test runs.

## What the research would NOT justify

Even positive demand research does not, by itself, justify the four-seam
program. It would justify the pane. Agents-as-principals, multi-operator
visibility, the IDE extension, and the public-core API each need their own
demonstrated pressure.

## Source

Combined-surface vision review rounds 1–5 (consensus finding across both
model classes), against `.work/` substrate state (n-of-1 signals) and
`docs/SPEC.md:26` (v1.0.0 self-hosting as the demand hypothesis).
