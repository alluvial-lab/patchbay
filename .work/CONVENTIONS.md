# Patchbay work conventions

Patchbay tracks operational work in `.work/` using agile-workflow markdown items with YAML frontmatter.

## Layout

- `.work/active/epics/` — capability arcs and multi-feature refinement programs.
- `.work/active/features/` — design-bearing work items.
- `.work/active/stories/` — implementation-sized work items.
- `.work/backlog/` — parked ideas not yet scoped.
- `.work/archive/` — completed items if/when terminal retention needs a filesystem stub.
- `.work/releases/` — not used until Patchbay has release bundles.

## Frontmatter

Required fields:

```yaml
id: <filename-without-.md>
kind: epic | feature | story
stage: drafting | implementing | review | done
tags: []
depends_on: []
created: YYYY-MM-DD
updated: YYYY-MM-DD
gate_origin: null | security | tests | cruft | docs | patterns | refactor
release_binding: null | <version>
```

Optional fields:

```yaml
parent: <item-id>
research_refs: []
research_origin: <slug>
research_dials:
  scope_authority: in-engagement-judgment | pre-registered | mixed
  verification_rigor: floor | standard | full
  intent: <open inventory>
  output_kind: <open inventory>
```

## Tags

Patchbay routes all design-bearing work (including docs-only features) through `feature-design`. There is no separate `[prose]` routing tag or `prose-author` lane. The historical `[prose]` tag was removed from the routing vocabulary on 2026-07-07 after an audit found a 56% misroute rate (9 of 16 items that ever carried `[prose]` in the `epic-foundation-hardening` arc were misroutes — 7 caught and stripped, 2 caught this session, 4 slipped through to `done` still tagged and are under retroactive design-gate audit). The tag was applied by deliverable format ("it produces docs") rather than work-nature ("does it involve choosing between approaches"), and the asymmetry of harm — a misroute skips the design gate, alternatives evaluation, and pre-mortem on *foundational* decisions, and the cost propagates downward — made the tag net-negative.

### Work-nature test (applied inside feature-design)

Docs-only features are not exempt from the design gate. `feature-design` Phase 4.5 applies the same work-nature test that the old prose black-box gate performed *before* routing: surface genuine ambiguities (choosing between approaches, pinning a semantic model, making an architectural commitment, an integration seam, an error path). If the surface is genuinely zero — authoring a checklist, inventory, mapping, or config-as-prose of *already-settled* material — Phase 4.5 records "zero questions" and the design collapses into a lightweight design-body + the writing pass (roughly what `prose-author` did, but inside the design lane so the gate is not structurally skipped). When in doubt, prefer surfacing the question: the design gate's cost is low; the cost of a semantic commitment made silently through prose is high.

Items that currently still carry `[prose]` in `tags:` (legacy, from before the retirement) are inert for routing — treat them as design features and route through `feature-design`. Do not add `[prose]` to new items.
- `[research]` — grounded research engagement. Route to agentic-research research-orchestrator. Do not bind research items to releases.
- `[protocol]` — protocol semantics, schemas, state machines, wire contracts, or conformance vectors.
- `[security]` — threat model, grants, principals, auth, revocation, replay resistance.
- `[verification]` — formal models, property tests, conformance, traceability.
- `[ux]` — human control surface flows, presentation states, and operator experience.
- `[adapter]` — adapter contracts or adapter-specific capability mapping.
- `[foundation]` — rolling foundation docs.

## Work/research handoff

- Use `research_refs:` when a work item consumes or is gated by a `.research/` artifact.
- Use `research_origin:` when a completed research engagement emits a work item.
- A `[research]` item may carry `research_dials:` to pre-register scope authority, rigor, intent, and output kind.
- `depends_on:` always points to `.work` item ids, never directly to `.research` slugs.

## Release mapping

Patchbay has no release process yet. Keep `release_binding: null` until release conventions are explicitly designed.

## Query tool

Use `.work/bin/work-view`:

```bash
.work/bin/work-view --ready
.work/bin/work-view --stage drafting --paths
.work/bin/work-view --blocking <item-id>
.work/bin/work-view --research-refs <slug>
```
