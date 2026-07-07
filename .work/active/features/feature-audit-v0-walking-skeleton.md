---
id: feature-audit-v0-walking-skeleton
kind: feature
stage: drafting
tags: [foundation]
parent: epic-retroactive-design-gate-audit
depends_on: [feature-v0-walking-skeleton]
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Retroactive design-gate audit — v0 walking skeleton

## Brief

`feature-v0-walking-skeleton` slipped through to `done` tagged `[prose]`, structurally skipping the design gate (alternatives evaluation + faulty-assumption hunt). It is the most load-bearing foundational feature — 13 downstream dependents — and its "Required decisions" section (operator scope, deployment topology, persistence backend, first adapter/command kinds, control surfaces, explicit exclusions) is a list of genuine architectural choices, not settled prose.

This audit re-runs the design-gate-equivalent on those decisions and records the missing alternatives traceability, plus a faulty-assumption hunt and a propagation check across the 13 dependents.

## What to read

- The target: `.work/active/features/feature-v0-walking-skeleton.md` (read FULLY — its "Required decisions" and "V0 decisions to encode").
- The docs it produced: `docs/SPEC.md` (v0 walking skeleton section, non-goals), `docs/ARCHITECTURE.md` (v0 component slice), `README.md`.
- The 13 downstream dependents (propagation check surface): `feature-research-harness-action-surfaces`, `feature-command-state-ssot`, `feature-persistence-snapshot-model`, `feature-pi-parity-checklist`, `feature-ux-v0-acceptance`, `feature-research-web-control-security`, `feature-observability-operator-admin`, `feature-session-identity-adapter-contract`, `feature-security-threat-model`, `feature-research-contract-tooling`, `feature-operator-presence-and-action-inventory`, `feature-lease-scope-decision`, `feature-extension-seams-non-foreclosure`.
- Foundation context: `docs/VISION.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `AGENTS.md`, `.agents/rules/`.

## Scope

For each load-bearing v0 decision, run:

1. **Alternatives evaluation.** Name 2-3 plausible alternatives, the tradeoff each optimizes for, and why the landed choice was taken. Record as the missing "rejected alternatives." Decisions to cover:
   - Operator scope: single-operator v0 (vs multi-operator / operator+observer).
   - Deployment topology: single authoritative core, no HA/clustering/split-brain (vs split-core / HA / leader-elected cluster).
   - Persistence backend: local durable store behind ports (vs mandated specific engine / vs remote-from-start).
   - First adapter + initial command kinds: Pi first, the initial OperationKind set (vs broader / narrower initial set).
   - Control surfaces: responsive web + CLI (vs native-first / CLI-only / web-only).
   - Exclusions: native mobile, HA, multi-operator provisioning, arbitrary adapters, leases (each — was excluding it a conscious choice with rationale, or an omission?).
2. **Faulty-assumption hunt.** Re-derive each decision from current first principles. Would the landed choice survive an honest gate today? Flag any decision that was an accident of the prose lane rather than a conscious choice. Pay special attention to: whether the single-operator assumption baked in anything that multi-human/`idea-multi-human-coordination` would have to reverse (not just defer); whether "backend behind ports" actually achieved Ports & Adapters or leaked an assumption; whether the initial OperationKind set was chosen against the harness action survey or asserted.
3. **Propagation check.** Examine the 13 dependents for any that silently assumed a posture the skipped gate would have surfaced as open. Specifically: did any dependent treat an exclusion as permanent architecture (the non-foreclosure failure), or assume a v0 scope choice was a timeless product decision?
4. **Verdict.** `holds` / `holds-with-caveats` (file refinement follow-up) / `faulty-assumption-found` (file corrective item with re-opening `depends_on`).

## Acceptance criteria

- [ ] Every load-bearing v0 decision has a recorded alternatives evaluation (the missing traceability).
- [ ] Faulty-assumption hunt complete; any accident-of-prose decisions flagged.
- [ ] Propagation check across the 13 dependents recorded.
- [ ] Verdict recorded; any `faulty-assumption-found` produced a filed corrective item with re-opening `depends_on`.

## Notes

Routes through `feature-design` (design work, not prose — `[prose]` lane retired 2026-07-07). No pre-mortem per operator direction. The target feature's review notes record it received a fresh-context deep review that "found no blockers" — this audit does not re-run that coherence review; it runs the design gate that was skipped (alternatives + faulty-assumption + propagation).
