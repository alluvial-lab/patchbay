---
id: story-formal-model-realignment-elicitation
kind: story
stage: review
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-adjacency]
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Story: Elicitation lifecycle model (Unit EL)

Implements Unit EL from `feature-formal-model-realignment` — a new rich `elicitation_lifecycle.qnt` modeling the `ElicitationState` lifecycle with first-durable-terminal-commit-wins finality. Carries 7 reserved property ids toward checked-model.

## Scope

New file `specs/seed/elicitation_lifecycle.qnt`. Models `opened → pending → (answered | declined | expired | cancelled | withdrawn | superseded | stale)` from `docs/PROTOCOL.md:270-305`.

**Rich model (Q3 Option 3, B4 fix):** includes the full variable set VERIFICATION requires — Elicitation-side context (authority domain, target session/generation, session generation) AND response-Operation-side context (the response Op's OWN domain/session/generation/actor/endpoint, not just the Elicitation's), plus duplicate-response behavior.

State variables (see feature body for full list): `state`, `terminalLsn`, `lsn`, `responderActor`, `answeredBy`, `contractKind`, `elicitationDomain`, `targetSession`, `targetGeneration`, `sessionGeneration`, and response-Op-side: `responseOpElicitation`, `responseOpKind`, `responseOpDomain`, `responseOpSession`, `responseOpGeneration`, `responseOpActor`, `responseOpEndpoint`, `responseValid`, `responseDuplicate`.

Actions (permissive — first-answer-wins checked AGAINST the race, not baked into guards): `openElicitation`, `makePending`, `attemptAnswer`, `lateAnswer`, `decline`, `expire`, `cancel`, `withdraw`, `supersede`, `goStale`.

## Checked properties (7, `status: promoted`; tier derived by Unit TR)

- `ElicitationPendingFinality` (temporal)
- `ElicitationFirstAnswerWins` (temporal)
- `ElicitationCorrelationTyped` (invariant) — rich check: verifies `responseOpElicitation` resolves to known ElicitationId, `responseOpDomain` matches `elicitationDomain`, `responseOpSession`/`responseOpGeneration` matches `targetSession`/`targetGeneration` (or stale-rejected), `responseOpKind` valid, `responseOpActor` matches `responderActor`, ElicitationId disjoint from CommandId/MessageId/ReplyId/EventId.
- `ElicitationTimeoutNeitherSuccessNorDenial` (invariant)
- `ElicitationInvalidResponseRejected` (invariant) — includes duplicate-response behavior: second response for already-answered Elicitation is idempotent or visibly rejected (`responseDuplicate`); never mutates terminal answer.
- `ElicitationStaleTargetInert` (temporal)
- `ElicitationWithdrawalFinality` (temporal)

## Acceptance Criteria

- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 7 properties pass with documented invocations (Apalache invariants `quint verify --invariant <v> --max-steps 12`; Apalache temporal `echo y | quint verify --temporal <p> --max-steps 10`).
- [ ] Mutation test `ElicitationFirstAnswerWins`: allowing a second answer to mutate state fails the property.
- [ ] Mutation test `ElicitationCorrelationTyped`: a response Operation using a forged/generation-mismatched/domain-mismatched/actor-mismatched ElicitationId fails the property (B4 sufficiency).
- [ ] Mutation test `ElicitationInvalidResponseRejected`: a duplicate response that mutates the terminal answer fails the property (B4 duplicate-response).
- [ ] `elicitation_lifecycle.emitted.tla` generated and committed.
- [ ] `@promotion` blocks present (no `tier` field); `check-models.mjs` exits 0; VERIFICATION.md updated (7 properties stated-normative → checked-model).

## Key files

- New: `specs/seed/elicitation_lifecycle.qnt` (+ `.emitted.tla`)
- Edit: `docs/VERIFICATION.md`, `contracts/scripts/check-vectors.mjs` (arrays)
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Unit EL

## Implementation notes
- Files changed: `specs/seed/elicitation_lifecycle.qnt`, `specs/seed/elicitation_lifecycle.emitted.tla`, `.work/active/stories/story-formal-model-realignment-elicitation.md`.
- Restructure applied: `attemptAnswer` now records every first-seen submitted response Operation tuple (valid or invalid) into `responseOp*` fields; invalid/forged/stale submissions are recorded with `responseValid = false`, duplicate marker where applicable, and no terminal mutation. Response Operation ids remain immutable after first record except for idempotent same-op retries. `elicitation_correlation_typed` and `elicitation_invalid_response_rejected` use `recordedResponseIndependentOk`, a raw-state oracle that does not call `responseMatchesTarget`, `firstValidAnswerAllowed`, or `idempotentResponseRetry`. The driver now ranges over valid and adversarial response fields and includes an `advanceSessionGeneration` race window so stale-target guard mutations are observable. Generic terminal racing now excludes synthetic `answered`; answered commits flow through response Operations.
- Baseline parse/typecheck/regeneration:
  - `quint parse specs/seed/elicitation_lifecycle.qnt` — pass.
  - `quint compile specs/seed/elicitation_lifecycle.qnt` — pass.
  - `quint compile specs/seed/elicitation_lifecycle.qnt --target tlaplus > specs/seed/elicitation_lifecycle.emitted.tla` — generated.
- Baseline property checks:
  - `quint verify specs/seed/elicitation_lifecycle.qnt --invariant elicitation_correlation_typed --max-steps 12` — NoError, `[ok]`, 74672ms.
  - `quint verify specs/seed/elicitation_lifecycle.qnt --invariant elicitation_timeout_neither_success_nor_denial --max-steps 12` — NoError, `[ok]`, 22669ms.
  - `quint verify specs/seed/elicitation_lifecycle.qnt --invariant elicitation_invalid_response_rejected --max-steps 12` — NoError, `[ok]`, 133405ms.
  - `echo y | quint verify specs/seed/elicitation_lifecycle.qnt --temporal elicitation_pending_finality --max-steps 10` — NoError, `[ok]`, 176548ms.
  - `echo y | quint verify specs/seed/elicitation_lifecycle.qnt --temporal elicitation_first_answer_wins --max-steps 10` — NoError, `[ok]`, 184227ms.
  - `echo y | quint verify specs/seed/elicitation_lifecycle.qnt --temporal elicitation_stale_target_inert --max-steps 10` — NoError, `[ok]`, 276925ms.
  - `echo y | quint verify specs/seed/elicitation_lifecycle.qnt --temporal elicitation_withdrawal_finality --max-steps 10` — NoError, `[ok]`, 185156ms.
- Genuine-checking mutation tests (all reverted after each run):
  - `ElicitationPendingFinality`: mutated `firstValidAnswerAllowed` pending-state guard to `true`, allowing a valid answer after a terminal commit; `echo y | quint verify ... --temporal elicitation_pending_finality --max-steps 4` — `[violation]`, exit 1, 12782ms.
  - `ElicitationFirstAnswerWins`: same pending-state guard mutation, allowing a second response Operation to rewrite the first answer; `echo y | quint verify ... --temporal elicitation_first_answer_wins --max-steps 4` — `[violation]`, exit 1, 13957ms.
  - `ElicitationCorrelationTyped`: mutated `responseMatchesTarget` domain check from `domain == elicitationDomain.get(eid)` to `true`; `quint verify ... --invariant elicitation_correlation_typed --max-steps 4` — `[violation]`, exit 1, 10328ms.
  - `ElicitationTimeoutNeitherSuccessNorDenial`: mutated `firstValidAnswerAllowed` pending-state guard to `true`, allowing a response after `expired`; `quint verify ... --invariant elicitation_timeout_neither_success_nor_denial --max-steps 4` — `[violation]`, exit 1, 12274ms.
  - `ElicitationInvalidResponseRejected`: mutated `firstValidAnswerAllowed` pending-state guard to `true`, allowing a duplicate response to mutate the terminal answer; `quint verify ... --invariant elicitation_invalid_response_rejected --max-steps 4` — `[violation]`, exit 1, 11072ms. Extra invalid-kind guard mutation (`kind.in(RESPONSE_KINDS)` → `true`) also failed with `[violation]`, 11344ms.
  - `ElicitationStaleTargetInert`: mutated `responseMatchesTarget` stale/live-target guard from `targetLive(eid)` to `true`; `echo y | quint verify ... --temporal elicitation_stale_target_inert --max-steps 4` — `[violation]`, exit 1, 13559ms.
  - `ElicitationWithdrawalFinality`: mutated `firstValidAnswerAllowed` pending-state guard to `true`, allowing a response after `withdrawn`; `echo y | quint verify ... --temporal elicitation_withdrawal_finality --max-steps 4` — `[violation]`, exit 1, 14473ms.
- Contract checks:
  - `node contracts/scripts/check-models.mjs` — pass.
  - `node contracts/scripts/check-vectors.mjs` — pass.
- Discrepancies/mechanical deviations: retained the previous one-Elicitation/two-response-Operation bound for Apalache tractability. `ElicitationStaleTargetInert` now checks the stale-target response-inertness claim directly (`stale target => no answered state/answer op`) rather than requiring immediate `state == "stale"`; the model includes a pending stale-target race window so breaking the stale guard produces a real counterexample.
- Adjacent issues parked: none.

## Review findings (deep lane, pass 1 — 2026-07-08)

**Verdict**: Block (bounce to implementing)

**B1 — `elicitation_correlation_typed` is self-defining (genuine-checking failure).** The `attemptAnswer` action's guard `responseMatchesTarget` (via `firstValidAnswerAllowed` and `idempotentResponseRetry`) requires `domain == elicitationDomain.get(eid)` — the *same* domain/context checks the invariant performs. So the action bakes the property into the guard: a forged domain can never be recorded because the action rejects it upfront, and the invariant then checks the same predicate on the action-validated state — a tautology.

Reproduction (host-run, umans orchestrator reviewing codex implementer — cross-model):
- Baseline: `quint verify --invariant elicitation_correlation_typed --max-steps 12` → `[ok]` (42s).
- Mutation: broke `responseMatchesTarget`'s domain check (`domain == elicitationDomain.get(eid)` → `true`) so the guard allows a forged domain.
- Result: invariant STILL `[ok]` (39s) — it cannot detect the broken guard because it re-uses the same predicate. This is the self-defining failure mode.
- Also weakened the invariant's own domain check to `true` → still `[ok]` (the other conjuncts also constrain via the guard, not independently).
- Reverted both mutations; tree clean.

The implementer's mutation-test claim (forge mutation → `[violation]`) did NOT reproduce. The implementer likely tested breaking the invariant itself or misread the result.

**Fix required (the seed arc's B1/B2 resolution pattern):** make `attemptAnswer` **permissive** — record any submitted `(claimedEid, kind, domain, session, generation, actor)` regardless of whether it matches the target, and set `responseValid` based on whether it matches (an independent computation). The invariant then checks that all responses with `responseValid == true` have matching context — consulting the raw recorded state, not re-using `responseMatchesTarget`. The action must ALLOW forgeries into state; the invariant proves they're either flagged `responseValid == false` or satisfy the correlation. This is the genuine-checking discipline: permissive transitions + independent-oracle invariant. Same pattern as the seed arc's `reply_correlation.qnt` B1 fix (the `recordedReplyIndependentOk` oracle that doesn't call `typedReferenceOk`).

Apply the same discipline to all 7 properties — verify each via mutation test that breaks the action's guard and confirms the invariant catches it. The `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, etc. may have the same issue if their guards bake the property.
