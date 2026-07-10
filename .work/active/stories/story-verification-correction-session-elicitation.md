---
id: story-verification-correction-session-elicitation
kind: story
stage: review
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

Demote `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, and `LateGenerationInert` (from `session_generation.qnt`) and `ElicitationTimeoutNeitherSuccessNorDenial` (from `elicitation_lifecycle.qnt`) from `status: promoted` to `status: draft`. These four properties have formulas too narrow to support their product-claim names.

## Unit

`Unit 2` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/session_generation.qnt` — three `@promotion` blocks (`SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `LateGenerationInert`)
- `specs/seed/elicitation_lifecycle.qnt` — one `@promotion` block (`ElicitationTimeoutNeitherSuccessNorDenial`)
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists, seed-model summary, generated tables
- `docs/ADAPTER-PI.md` — lines 75-79 reference `LabelsCannotOverrideIdentity` and `LateGenerationInert` as checked

## Implementation

For each of the four properties:

1. In the `@promotion` block:
   - Change `status: promoted` → `status: draft`
   - Replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`
   - Add `demotion_reason: <explanation from the design>`

   Rename the property-section headings at `session_generation.qnt:108` and `elicitation_lifecycle.qnt:538` so they describe mixed promotion metadata rather than claiming every following block is promoted.

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `LateGenerationInert`, `ElicitationTimeoutNeitherSuccessNorDenial` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`

3. Run `node contracts/scripts/check-vectors.mjs` (exits 0, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then `node contracts/scripts/check-models.mjs` again (exits 0, confirms current).

4. Update VERIFICATION.md prose that is NOT generated:
   - The checked-model property lists for `session_generation.qnt` and `elicitation_lifecycle.qnt`
   - The seed-model summary tables for both models — move the demoted properties to the draft column
   - The summary line — update promoted/draft counts
   - The sentence near line 566 — `ElicitationTimeoutNeitherSuccessNorDenial` is now a stated-normative Elicitation-specific obligation, not a checked-model analog

5. Update `docs/ADAPTER-PI.md:75-79` — `LabelsCannotOverrideIdentity` and `LateGenerationInert` are referenced as checked properties. Mark these as stated-normative.

## Demotion reasons

- `SessionIdentityTuple` (in `session_generation.qnt`): adapter id, deployment scope, and runtime id are constants (`ADAPTER_IDS = Set("a1")`, etc.), not per-session identity state. The formula checks generation mirroring and that three singleton sets have size one, not the four-field identity tuple named in the metadata.
- `LabelsCannotOverrideIdentity` (in `session_generation.qnt`): proves labels use strings disjoint from three constant singleton sets and that generation mirrors remain equal. Models no routing/target-selection path in which a label could override identity.
- `LateGenerationInert` (in `session_generation.qnt`): the formula proves only that generation and `identityGeneration` do not change. Its promoted semantics additionally claim a `stale_event` audit record, but no audit state is modeled.
- `ElicitationTimeoutNeitherSuccessNorDenial` (in `elicitation_lifecycle.qnt`): metadata and VERIFICATION.md claim timeout never implies "grant," but the model has no grant state. The formula checks only answer/decline fields.

## Acceptance criteria

- [ ] Four `@promotion` blocks changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] Four ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; `node contracts/scripts/check-models.mjs` exits 0 on second run.
- [ ] VERIFICATION.md prose updated: checked-model property lists, seed-model summary tables, and the stale `ElicitationTimeoutNeitherSuccessNorDenial` checked-model-analog sentence near line 566.
- [ ] The property-section headings at `session_generation.qnt:108` and `elicitation_lifecycle.qnt:538` no longer label mixed promoted/draft blocks as all promoted.
- [ ] `docs/ADAPTER-PI.md:75-79` updated: `LabelsCannotOverrideIdentity` and `LateGenerationInert` marked stated-normative.
- [ ] The genuine promoted properties in those models remain promoted: `GenerationMonotonic`, `TypedCorrelation`, `ElicitationCorrelationTyped`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`.
- [ ] `quint parse specs/seed/session_generation.qnt` and `quint parse specs/seed/elicitation_lifecycle.qnt` exit 0.

## Implementation notes

- Files changed: `specs/seed/session_generation.qnt`, `specs/seed/elicitation_lifecycle.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, and `docs/ADAPTER-PI.md`.
- Demoted `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `LateGenerationInert`, and `ElicitationTimeoutNeitherSuccessNorDenial` with formula-specific reasons, draft invocations, and matching registry tiers; retained all genuine promoted properties named by the design.
- Regenerated both traceability tables. The resulting model inventory is 44 modeled properties: 23 promoted and 21 draft; all four demoted ids are stated-normative in both generated tables.
- Verification: both required `quint parse` commands exited 0; `check-vectors.mjs` exited 0; the first `check-models.mjs` run regenerated the model table and exited 1 as expected; the confirming second run exited 0.
- Tests added: none; the production verification surface is the existing Quint parser and traceability checker suite.
- Discrepancies from design: none. Direct reads confirmed every demotion reason against the formula at HEAD.
- Adjacent issues parked: none.
