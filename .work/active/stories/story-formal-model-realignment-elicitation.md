---
id: story-formal-model-realignment-elicitation
kind: story
stage: implementing
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
