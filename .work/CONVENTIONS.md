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

- `[prose]` — docs, specifications, conventions, or config-as-prose. Route drafting features through prose-author; implementation is usually the writing pass.

### Prose black-box test (lane routing)

Apply this honestly to every candidate `[prose]` item before routing. **`[prose]` is for items whose deliverable is authored prose and whose design collapses into the writing** — authoring a checklist, classification rule, inventory, mapping, or config-as-prose where the choices are settled or obvious. If the scope item involves **choosing between approaches, pinning a semantic model, or making an architectural commitment**, route to `feature-design` instead, even when the deliverable is docs-only. The prose-author lane skips the design gate, pre-mortem, and alternatives evaluation; a semantic commitment made silently through prose is high-cost and hard to reverse.

**Going forward:** when in doubt, prefer design — the design gate's cost is low; the cost of a semantic commitment made silently through prose is high. (Origin: `epic-foundation-hardening` lane-routing discipline, 2026-06-28; codified project-wide 2026-07-06 after `feature-foundation-doc-completeness-gaps` was initially misrouted as `[prose]`.)
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
