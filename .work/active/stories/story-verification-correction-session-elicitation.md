---
id: story-verification-correction-session-elicitation
kind: story
stage: implementing
tags: [verification, protocol]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-command-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Demote overclaiming session_generation and elicitation_lifecycle properties

## Scope

Demote `SessionIdentityTuple`, `LabelsCannotOverrideIdentity` (from `session_generation.qnt`), `ElicitationTimeoutNeitherSuccessNorDenial`, and `LateGenerationInert` (from `elicitation_lifecycle.qnt`) from `status: promoted` to `status: draft`. These four properties have formulas too narrow to support their product-claim names.

## Unit

`Unit 2` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/session_generation.qnt` — two `@promotion` blocks
- `specs/seed/elicitation_lifecycle.qnt` — two `@promotion` blocks
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists, seed-model summary, generated tables

## Implementation

For each of the four properties:

1. In the `@promotion` block:
   - Change `status: promoted` → `status: draft`
   - Replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`
   - Add `demotion_reason: <explanation from the design>`

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `ElicitationTimeoutNeitherSuccessNorDenial`, `LateGenerationInert` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`

3. Run `node contracts/scripts/check-vectors.mjs` FIRST, then `node contracts/scripts/check-models.mjs`.

4. Update VERIFICATION.md prose that is NOT generated:
   - The checked-model property lists for `session_generation.qnt` and `elicitation_lifecycle.qnt`
   - The seed-model summary tables for both models — move the demoted properties to the draft column
   - The summary line — update promoted/draft counts

## Demotion reasons

- `SessionIdentityTuple`: adapter id, deployment scope, and runtime id are constants (`ADAPTER_IDS = Set("a1")`, etc.), not per-session identity state. The formula checks generation mirroring and that three singleton sets have size one, not the four-field identity tuple named in the metadata.
- `LabelsCannotOverrideIdentity`: proves labels use strings disjoint from three constant singleton sets and that generation mirrors remain equal. Models no routing/target-selection path in which a label could override identity.
- `ElicitationTimeoutNeitherSuccessNorDenial`: metadata and VERIFICATION.md claim timeout never implies "grant," but the model has no grant state. The formula checks only answer/decline fields.
- `LateGenerationInert`: the formula proves only that generation and `identityGeneration` do not change. Its promoted semantics additionally claim a `stale_event` audit record, but no audit state is modeled.

## Acceptance criteria

- [ ] Four `@promotion` blocks changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] Four ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; then `node contracts/scripts/check-models.mjs` exits 0.
- [ ] VERIFICATION.md prose updated: checked-model property lists, seed-model summary tables.
- [ ] The genuine promoted properties in those models remain promoted: `GenerationMonotonic`, `TypedCorrelation`, `ElicitationCorrelationTyped`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`.
- [ ] `quint parse specs/seed/session_generation.qnt` and `quint parse specs/seed/elicitation_lifecycle.qnt` exit 0.
