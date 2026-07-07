---
id: feature-audit-command-state-ssot
kind: feature
stage: drafting
tags: [foundation]
parent: epic-retroactive-design-gate-audit
depends_on: [feature-command-state-ssot]
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Retroactive design-gate audit — command/session/failure state SSOT

## Brief

`feature-command-state-ssot` slipped through to `done` tagged `[prose]`, structurally skipping the design gate. Its scope includes genuine semantic decisions: the `CommandState` lifecycle and allowed transitions, the split of control-surface-local submission state (`draft`/`submitting`) from durable state, the `SubmissionOutcome` introduction (added during its review as a fix), the first-durable-terminal-commit race semantics, and the layer-aware failure vocabulary. The review notes record "introducing `SubmissionOutcome`" as a fix — a semantic-model decision made inside the prose lane.

8 downstream dependents build on this (`feature-verification-contract-authority`, `feature-persistence-snapshot-model`, `feature-design-terminal-commit-race`, `feature-ux-v0-acceptance`, `feature-idempotency-ambiguous-execution`, `feature-session-identity-adapter-contract`, `feature-operator-presence-and-action-inventory`, `feature-formal-model-seed`).

## What to read

- The target: `.work/active/features/feature-command-state-ssot.md` (read FULLY — "Authoring decisions," "Review" notes recording the `SubmissionOutcome` introduction and terminal-commit race semantics).
- The docs it produced: `docs/PROTOCOL.md` (CommandState registry, allowed transitions, failure vocabulary, race semantics), `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`.
- The checked model that encodes these decisions: `specs/seed/command_lifecycle.qnt` (its `TERMINAL`/`NON_TERMINAL` registry, `commitTerminal`, `lateTerminalCandidate` actions).
- The 8 downstream dependents (propagation check surface) listed above.
- Foundation context: `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `AGENTS.md`, `.agents/rules/`.

## Scope

1. **Alternatives evaluation** for each load-bearing decision:
   - `CommandState` registry membership and terminal/non-terminal split (vs alternative state sets).
   - Allowed transitions (esp. the no-`accepted → completed` adjacency — was it a conscious gate or a model artifact? note `command_lifecycle.qnt`'s `commitTerminal` *allows* that adjacency, so the protocol forbids what the model permits — was that tension designed or accidental?).
   - `SubmissionOutcome` as a distinct pre-acceptance vocabulary (vs folding into `CommandState`/vs a single outcome enum) — introduced during review, so likely has no alternatives record at all.
   - First-durable-terminal-commit-wins race rule (vs last-wins / vs priority-ordered / vs nondeterministic-exposed) — note `feature-design-terminal-commit-race` re-opened this, so it has *some* gate coverage; verify the audit doesn't duplicate that.
   - Layer-aware failure vocabulary (vs flat failure enum).
2. **Faulty-assumption hunt.** Re-derive each from current first principles. Flag any accident-of-prose. Pay special attention to: the protocol/model tension on `accepted → completed` (a real fault if the protocol claims an adjacency the model doesn't check — verify whether `feature-formal-model-realignment` V1 follow-on already covers this, to avoid duplicate findings); whether `SubmissionOutcome` cleanly separates pre-acceptance refusal from `CommandState = rejected` (the review fix may have left an edge case).
3. **Propagation check** across the 8 dependents. Specifically: did `feature-idempotency-ambiguous-execution` (just done this session) assume a `CommandState` posture that the skipped gate would have surfaced? Did `feature-design-terminal-commit-race` already resolve the race-semantics open question, or is there residual debt?
4. **Verdict.** `holds` / `holds-with-caveats` / `faulty-assumption-found`.

## Acceptance criteria

- [ ] Every load-bearing state-machine decision has a recorded alternatives evaluation.
- [ ] `SubmissionOutcome` introduction has an alternatives record (it was a review fix — likely missing).
- [ ] The protocol/model `accepted → completed` tension is classified (designed vs accidental) and cross-referenced against `feature-formal-model-realignment` to avoid duplicate findings.
- [ ] Propagation check across the 8 dependents recorded.
- [ ] Verdict recorded; any `faulty-assumption-found` produced a filed corrective item with re-opening `depends_on`.

## Notes

Routes through `feature-design`. No pre-mortem per operator direction. Coordinate with `feature-formal-model-realignment` (also drafting) on the transition-adjacency modeling gap — both touch the `accepted → completed` tension; avoid filing duplicate findings.
