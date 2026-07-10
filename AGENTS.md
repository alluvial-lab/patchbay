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
- All design-bearing work — including docs-only features — routes through `feature-design`. There is no `[prose]` routing tag or `prose-author` lane (retired 2026-07-07; see `.work/CONVENTIONS.md`). `feature-design` Phase 4.5 applies the work-nature test and collapses to a lightweight writing pass when the design surface is genuinely zero.
- `[refactor]` is behavior-preserving only.
- `[perf]` is performance work.

## Design principles

- Ports & Adapters: domain logic stays independent of DB/filesystem/HTTP/time/randomness.
- Single Source of Truth: growing variant sets have one registry; types, validation, routing, and display derive from it.
- Generated Contracts: boundary types come from schema/router/DB inference or generation instead of hand copies.
- Fail Fast: unknown input is validated at system boundaries and internal preconditions are asserted early.

## Verification posture

Patchbay's safety claims should become formal models, generated contracts, conformance vectors, and property tests before implementation treats them as product semantics. Do not implement a protocol enum, state machine, or authority rule from scattered prose lists; first consolidate the source of truth.

## Extension pressure-test checklist

Patchbay ships a narrow v0.1.0 that must not foreclose future directions. Run this checklist before committing any decision to v0.1.0 (and before advancing a foundation-hardening item past design). The standing discipline and per-seam registry live in `docs/SPEC.md` ("Non-foreclosure discipline") and `docs/PROTOCOL.md` ("Extension seams registry").

**Classify the decision:**

- [ ] Is this **committed v0.1.0**, **reserved seam**, or **explicitly rejected**? Tag it explicitly using the three-way vocabulary in `docs/SPEC.md`.
- [ ] If **committed v0.1.0**: is it in the single source-of-truth registry for its kind (OperationKind / Operation·Session·Elicitation state enum / adapter capability manifest / failure vocabulary / `response_contract.contract_kind`)? Does it have checked-model + conformance-vector coverage where it carries a normative safety/security claim (see `docs/VERIFICATION.md` property-graded baseline)?
- [ ] If **reserved seam**: is the seam named in the registry/protocol (wire-present where forward-compatibility matters) rather than omitted? Is delivery behavior defined (typically `validation_failed` / `unsupported_command` in v0.1.0)?
- [ ] If **explicitly rejected**: is the rationale recorded? Is a future promotion visibly a reversal (a protocol-change ceremony), not a quiet gap-fill?

**Check the framing:**

- [ ] Is the v0.1.0 assumption written as v0.1.0-only ("v0.1.0 has...", "v0.1.0 ships...") rather than timeless architecture ("Patchbay has...", "Patchbay ships...")?
- [ ] If a future variant is likely (second operator, second control surface, second adapter, second storage backend, federation, delegation), does the v0.1.0 shape carry the future-relevant demarcator (authority-domain id in the key, reserved enum value, capability manifest field) rather than baking in a single-value assumption?
- [ ] Does a capability registry/manifest exist where future variants are likely, rather than scattering the variant set across prose?

**Check the seams (adapter-neutrality and surface-neutrality):**

- [ ] Are Pi-specific (or any single-adapter-specific) capabilities adapter-declared features, not core protocol primitives?
- [ ] Are surface-specific presentations surface-declared features, not core UX primitives? Does the control-surface design treat web/CLI as instances of a conformance floor, not the closed set?

**Check the parked ideas:**

- [ ] Does this decision foreclose a parked idea — `idea-multi-human-coordination`, `idea-desktop-app-surface`, `idea-agent-to-agent-mesh-seam`, `idea-operator-customizable-ux-skins`? If it touches one, is that idea treated as a pressure-test input (informs the seam inventory), not a v0.1.0 requirement?
- [ ] Is the decision recorded in the item's "Extension pressure classification" section using the three-way vocabulary, so the central registry in `docs/PROTOCOL.md` can consolidate it later?

<!-- ux-ui-design:installed -->
## UI/UX Design Convention

**Mockup-first.** All UI/UX design is done as standalone HTML/CSS/JS mockups
before any production code is written. Mockups are committed.

**Location.** Mockups live in `.mockups/` with three buckets:

- `.mockups/design-system/` — palette, typography, tokens (project-wide)
- `.mockups/screens/<feature-id>/` — single-screen options per feature
- `.mockups/flows/<flow-name>/` — multi-page user journeys

`<feature-id>` matches the agile-workflow item id when applicable, else a
kebab-case short name.

**Process.**
- Single screen with options to align on: `/ux-ui-design:screens`
- Multi-page user flow for sign-off: `/ux-ui-design:flows`
- Palette / typography / design tokens: `/ux-ui-design:palette`
- Convention reference (auto-loads): `/ux-ui-design:ux-ui-principles`

**Tech rule.** Single-file HTML per mock, vanilla CSS in `<style>`, vanilla JS
in `<script>`. No build step, no CSS framework CDNs. Hosted fonts (Google
Fonts, etc.) are fine when the palette specifies one.

**Linking.** Each substrate item with mocks gets a `## Mockups` section in its
body pointing at the relevant `.mockups/` paths.

**Skip mocking** for trivial copy changes, bug fixes that don't shift visual
structure, behind-the-scenes refactors, or feature-level UI that cleanly
reuses existing components and patterns. Mock new surfaces, design-system
shifts, and multi-screen epics.
