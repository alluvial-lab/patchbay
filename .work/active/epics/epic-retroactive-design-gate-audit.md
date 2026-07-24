---
id: epic-retroactive-design-gate-audit
kind: epic
stage: done
tags: [foundation]
depends_on: []
parent: null
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: v0.1.0
---

# Epic: Retroactive design-gate audit of foundational decisions

## Why this exists

A full audit of the `[prose]` tag (2026-07-07) found that four foundational features slipped through to `done` still tagged `[prose]` — `feature-v0-walking-skeleton`, `feature-command-state-ssot`, `feature-persistence-snapshot-model`, `feature-security-threat-model`. All four predate the 2026-07-06 codification of the prose black-box test, and all four involve genuine design decisions that the `prose-author` lane structurally skipped: alternatives evaluation and pre-mortem. The decisions landed (they received deep review and held up), but the *alternative space was never honestly evaluated and recorded*.

That gap matters because these four are load-bearing: every downstream feature in the foundation-hardening arc depends on at least one of them (`feature-v0-walking-skeleton` alone has 13 dependents). "Review checked coherence" is not the same as "the skipped design gate would not have surfaced a faulty assumption or a rejected alternative that should have been chosen." A faulty assumption in a foundational decision propagates downward and is expensive to find late.

## What this epic does

For each of the four slipped-through features, run a **full design-gate-equivalent pass** — not a pre-mortem (operator direction: skip the pre-mortem), but the rest of what `feature-design` would have done before the writing pass:

1. **Alternatives evaluation.** For each load-bearing decision the feature made, name 2-3 plausible alternatives, the tradeoff each optimizes for, and why the landed choice was taken over them. Record this as the missing "rejected alternatives" traceability.
2. **Faulty-assumption hunt.** Re-derive each decision from current first principles (the current foundation docs + code + research), and check whether the landed choice would survive an honest gate *today*. Flag any decision that was an accident of the prose lane rather than a conscious choice.
3. **Propagation check.** For each decision, examine the downstream dependents (listed per child) and verify none has already propagated a fault — i.e., no downstream feature silently assumed a posture that the skipped gate would have surfaced as open.
4. **Verdict.** Per feature: `holds` (decision sound, alternatives record added), `holds-with-caveats` (decision sound but a refinement is needed — file a follow-up), or `faulty-assumption-found` (file a corrective item that re-opens anything downstream built on the fault).

If a `faulty-assumption-found` verdict lands on a decision with downstream dependents, the corrective item carries `depends_on` entries that re-open the affected dependents for re-review — the audit does not silently fix propagation.

## Why full-gate, not lightweight-triage

Operator direction: run the full design-gate-equivalent on all four, not a triage pass. These are foundational; "probably fine" is not airtight. The cost is ~4 design passes; the cost of an undetected faulty assumption propagating through 13 dependents is substantially higher.

## Why no pre-mortem

Operator direction: skip the pre-mortem phase. The alternatives evaluation + faulty-assumption hunt + propagation check cover the "what could go wrong" surface; the pre-mortem is redundant for decisions whose consequences are already partially observable in the downstream features that built on them.

## Scope

- Four child audit features, one per slipped-through foundational feature (below).
- Each child is a genuine design-gate pass, not prose — it evaluates alternatives and hunts for faulty assumptions. It routes through `feature-design` (the `[prose]` lane is retired; these are design work regardless).
- Each child's verdict is recorded in its body. `faulty-assumption-found` verdicts produce filed follow-up items with re-opening `depends_on`.

## Out of scope

- Re-doing the landed work. The audit *evaluates* and *records alternatives*; it does not rewrite the foundational docs unless a `faulty-assumption-found` verdict requires it.
- Auditing the 7 caught-and-stripped misroutes — those *did* go through the design gate (after retag), so their alternatives record exists. Only the 4 that slipped through to `done` still tagged `[prose]` are in scope.
- Auditing the 2 genuine-prose items (`feature-bank-formal-methods-skills`, `feature-extension-seams-non-foreclosure`) — those were correctly routed; no design gate was skipped.

## Acceptance criteria

- All four child audit features reach `done` with a recorded verdict (`holds` / `holds-with-caveats` / `faulty-assumption-found`).
- Every load-bearing decision in each of the four foundational features has a recorded alternatives evaluation (the missing traceability).
- Any `faulty-assumption-found` verdict has produced a filed corrective item with re-opening `depends_on` on affected downstream dependents — no fault is silently absorbed.
- The propagation check for each child is recorded (which downstream dependents were examined and what was verified).

## Children

- `feature-audit-v0-walking-skeleton` — v0 scope decisions (operator, topology, backend, surfaces, exclusions). 13 downstream dependents — the largest propagation surface.
- `feature-audit-command-state-ssot` — `SubmissionOutcome` introduction, first-durable-terminal-commit race semantics, transition rules, failure-vocabulary layering. 8 downstream dependents.
- `feature-audit-persistence-snapshot-model` — persistence backend abstraction, event ordering, snapshot revision/cursor, crash recovery, adapter snapshot tiers. 2 downstream dependents.
- `feature-audit-security-threat-model` — v0 threat model, principal/grant posture, adapter trust boundary, replay protection, emergency revocation. 3 downstream dependents.

Children have no inter-dependencies (each audits an independent foundational feature) and may run in parallel; each depends on its target feature being `done` (all four are).

## Notes

Origin: 2026-07-07 `[prose]` tag audit (see `epic-foundation-hardening` "Lane routing discipline" § "`[prose]` tag retired"). The retirement of the `[prose]` routing tag (same date) prevents future misroutes; this epic closes the debt on the 4 that already slipped through.

## Audit results (2026-07-07)

All four audits ran in parallel as fresh-context design-gate auditors (`openai-codex/gpt-5.5`). Verdicts:

| Audit | Verdict | Corrective actions |
|---|---|---|
| `feature-audit-v0-walking-skeleton` (13 dependents) | **holds** | none |
| `feature-audit-command-state-ssot` (8 dependents) | **holds-with-caveats** | none |
| `feature-audit-persistence-snapshot-model` (2 dependents) | **holds-with-caveats** | none |
| `feature-audit-security-threat-model` (3 dependents) | **holds-with-caveats** | none |

**Zero faulty-assumption-found verdicts. Zero corrective actions filed. Zero dependents re-opened.** The four foundational decisions hold up under honest re-examination. The skipped design gates left *traceability debt* (missing alternatives records), now filled by the audits — but not *faulty-design debt*. No downstream feature propagated a fault from a missed assumption.

The three `holds-with-caveats` caveats are all already-tracked verification/design debt, not hidden problems:
- `accepted → completed` protocol/model tension → owned by `feature-formal-model-realignment` (drafting).
- crash-recovery / snapshot-recovery property draftness → VERIFICATION property-graded tier (stated-normative, not checked).
- `CompoundIssuer` / authority safety draftness + web↔core evidence shape → deferred to `feature-web-core-protocol-seam` (backlog).
- malicious/compromised enrolled adapter posture → conscious v0 decision (boundary checks + conformance + audit), not an accidental assumption.

The `v0-walking-skeleton` audit (largest propagation surface, 13 dependents) came back a clean **holds** with no caveats — its one weak original area (initial command/action set) was already re-opened and resolved by later grounded harness research and the normative OperationKind registry, evidence that the project's later rigor compensated for the early skipped gate.

Epic complete. The `[prose]` tag retirement (same date) prevents future misroutes; this audit closes the debt on the four that already slipped through.
