---
id: feature-research-formal-methods-tooling
kind: feature
stage: drafting
tags: [research, verification, foundation]
parent: epic-foundation-hardening
depends_on: []
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: null
research_dials:
  scope_authority: in-engagement-judgment
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: Formal-methods tooling for Patchbay verification models

Patchbay's verification posture (`docs/VERIFICATION.md`) commits to TLA+ as the semantic baseline, Quint as the ergonomic authoring candidate, and Alloy for bounded relational invariants. The model-promotion rule requires a *stable checked artifact* with a documented tool invocation and expected pass/fail status before any property is treated as product behavior. `feature-verification-contract-authority` (done) settled the authority order and the property-graded normative baseline; `feature-formal-model-seed` (drafting, blocked on this research) will author the first checked models.

Before authoring models that carry safety-critical claims, bank the toolchain: verify current syntax, the exact CLI to author→compile→check, installation in this environment, and idioms for the property shapes Patchbay needs. Plausible-but-wrong models that don't actually run or check give false confidence — the worst failure mode for a safety-claiming model.

## Why this research

The implementor's self-assessment: TLA+/TLC is passable but unpinned (stale CLI flags/config risk); Quint is the highest risk (newer, smaller-community, evolving API, and the chosen primary authoring language); Alloy 6 specifics need verifying (temporal operators vs relational-only for v0). None of the three has any banked research in `.research/` or any auto-loading reference skill in `.agents/skills/`. Banking all three upgrades "I think I know this" to "verified against current docs/toolchain" and empirically validates Q1 (Quint-primary-checked-via-TLC) of the seed-model design — if the toolchain can't install or the Quint→TLA+→TLC path doesn't run here, Q1 flips to pure-TLA+, which is a legitimate outcome this research surfaces.

## Seed questions

- **Quint**: current syntax for state machines, temporal properties (`always`/`eventually`), the `action`/`nondet`/`run` shape; the exact CLI to author→compile→check; whether `quint verify` uses Apalache or TLC and how to drive it; installation in this environment; idioms for terminal-finality, idempotency, and monotonic-generation properties.
- **TLA+/TLC**: current `tla2tools.jar` CLI and `.cfg` config-file shape; invariant vs temporal-property checking; counterexample format; how Quint-emitted TLA+ is checked through TLC.
- **Alloy 6**: current syntax (incl. whether temporal operators are in scope for our use or we stay relational); CLI `check` invocation and counterexample output; relational-only sufficiency for v0 identity/authority-graph/anti-spoofing shapes.
- **Verified "this runs here" artifact**: a trivial hello-world model per language that compiles and passes a check, as the promotion-gate floor and the empirical Q1 validation.

## Scope (in-engagement-judgment)

- Investigate the current (2026) toolchains for Quint, TLA+/TLC, and Alloy 6 from primary sources (official docs, repos, release notes).
- Verify installation and a working author→check round-trip in this environment for each.
- Produce a synthesis brief with: per-language current syntax essentials, the exact tool invocations, installation steps, idioms for the Patchbay property shapes, and the verified hello-world artifacts.
- Produce auto-loading reference skills under `.agents/skills/` (Quint, TLA+, Alloy) so future model-authoring work has banked, version-pinned API knowledge.
- Surface the Q1 outcome: does Quint-primary-checked-via-TLC work here, or does the seed fall back to pure-TLA+?

## Expected output

- Synthesis brief: `.research/analysis/briefs/formal-methods-tooling.md`
- Source attestations: `.research/attestation/{quint,tla2tools,alloy6,...}.md`
- Verified hello-world artifacts: one per language in the brief or a linked `specs/seed/` location
- Reference skills: `.agents/skills/{quint,tla-plus,alloy}/SKILL.md`
- A confirmed Q1 answer (Quint-primary-checked-via-TLC vs pure-TLA+ fallback) handed to `feature-formal-model-seed`

## Relationship to consuming work

- `feature-formal-model-seed` (drafting) depends on this research. Its Q1 (authoring language), Q2 (decomposition), and Q3 (Alloy scope) design questions are all informed by what the toolchains can actually do here.
- `feature-protocol-idl-and-conformance` (drafting) is downstream: its conformance vectors trace to the model properties this research enables.
- `docs/VERIFICATION.md` is not modified by this research; it already commits to the tool positions. This research banks the *how*, not the *whether*.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: the tool choice (Quint vs pure-TLA+) is a committed v0 decision once the research concludes; the seam to switch tools later is preserved by keeping model intent portable (already a VERIFICATION.md principle). No v0 assumption should be encoded as permanent architecture unless intentionally rejected.
