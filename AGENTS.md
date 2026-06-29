# AGENTS.md — Patchbay

Patchbay is a deployment-neutral human control plane for operating agent sessions across machines. It leads with human control surfaces, starts with a Pi adapter for workflow migration, and keeps core semantics adapter-neutral and formally specified.

## Orientation

Read these foundation docs before designing or changing project behavior:

1. `docs/VISION.md`
2. `docs/SPEC.md`
3. `docs/ARCHITECTURE.md`
4. `docs/PROTOCOL.md`
5. `docs/SECURITY.md`
6. `docs/VERIFICATION.md`
7. `docs/UX.md`
8. `docs/GLOSSARY.md`

## Substrates

Patchbay uses two coordinated but independent substrates:

- `.work/` — agile-workflow operational work items: what is being scoped, designed, implemented, reviewed, or parked.
- `.research/` — agentic-research/ARD-style grounded research: source-backed findings that inform design decisions.

Do not put operational task state in `.research/`. Do not put source-grounded research claims only in `.work/` chat prose. Link across the boundary with `research_refs:` and `research_origin:` when needed.

## Conversion / adoption guidelines

When standing up or refreshing these substrates, follow the installed plugin discipline rather than inventing a local shape:

- For `.work/`, use the agile-workflow `convert` workflow when available. It bootstraps `.work/`, `CONVENTIONS.md`, `work-view`, and agent entrypoint guidance. Preserve-only is the default for any legacy material.
- For `.research/`, use the agentic-research `convert` workflow when available. It scaffolds `.research/`, routes raw sources to `reference/`, routes claim-bearing legacy synthesis to holding for rigor uplift, and never silently imports ordinary docs as research.
- Patchbay is a greenfield repo, so the initial conversion has no legacy research or tracking artifacts to migrate. Future migrations must still run discovery, classification, content-integrity, and reference-integrity checks before destructive cleanup.
- The work/research handoff follows the agentic-research pairing: `.work` commissions/cites research via `research_refs:` and `[research]` items with `research_dials:`; completed research may propose work via `research_origin:` only after operator confirmation.

## Work item routing

- `[research]` items route to agentic-research research-orchestrator.
- `[prose]` items are documentation/config-as-prose work and route through prose-author before implementation.
- `[refactor]` is behavior-preserving only.
- `[perf]` is performance work.

## Design principles

- Ports & Adapters: domain logic stays independent of DB/filesystem/HTTP/time/randomness.
- Single Source of Truth: growing variant sets have one registry; types, validation, routing, and display derive from it.
- Generated Contracts: boundary types come from schema/router/DB inference or generation instead of hand copies.
- Fail Fast: unknown input is validated at system boundaries and internal preconditions are asserted early.

## Verification posture

Patchbay's safety claims should become formal models, generated contracts, conformance vectors, and property tests before implementation treats them as product semantics. Do not implement a protocol enum, state machine, or authority rule from scattered prose lists; first consolidate the source of truth.
