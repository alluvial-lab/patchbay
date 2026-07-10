---
id: story-verification-correction-prose
kind: story
stage: implementing
tags: [verification, foundation]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-draft-formulas]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Fix stale PROTOCOL.md prose and audit emitted TLA+

## Scope

Fix stale PROTOCOL.md, VERIFICATION.md, and ADAPTER-PI.md assertions that contradict current HEAD, then audit emitted TLA+ files for any prose presenting them as independent evidence.

## Unit

`Unit 5` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `docs/PROTOCOL.md` — stale correlation, adjacency, Elicitation-tier, extension-pressure, and extension-seams assertions
- `docs/VERIFICATION.md` — descendant-grant and Elicitation first-answer checked-model descriptions
- `docs/ADAPTER-PI.md` — stale `LateGenerationInert` verification claim
- `specs/seed/*.emitted.tla` — generated inspection artifacts (audit only)

## Implementation

### PROTOCOL.md fixes

1. **The `reply_correlation.qnt` coverage claims** (~lines 75 and 94) — currently say response Operation → Elicitation is a new stated-normative obligation and that `reply_correlation.qnt` does not cover it.

   Current HEAD: `TypedCorrelation` in `reply_correlation.qnt` now covers both Reply → Command/Message AND response Operation (`approval-response`/`elicitation-response`) → Elicitation typed references across disjoint id spaces. Update to reflect that the coverage exists, while noting it is checked-model (not checked-normative until vectors are promoted).

2. **The transition-adjacency claim** (~line 142) — currently says: "the current checked model permits any non-terminal state to commit any terminal candidate, so adjacency rules such as no `accepted → completed` require a strengthened lifecycle model..."

   Current HEAD: `NoAcceptedToCompleted` is now a checked-model property, and `allowedTransition` enforces the exact PROTOCOL transition table. Update to reflect that the no-`accepted → completed` adjacency is now checked, while the full transition graph and read/query fast-path rule remain stated-normative.

   Note: Unit 1 (`story-verification-correction-command-lifecycle`) fixes the `OperationState` ⇿ `CommandState` refinement section (~line 140) which lists demoted properties as checked. This story runs after the demotion chain and fixes the remaining stale prose at lines 75, 94, 142, 270, 568, and 603.

3. **The Elicitation lifecycle classification** (~line 270) — currently calls the entire registry stated-normative until promoted. Update it to say the lifecycle has partial checked-model coverage, with the demoted timeout/grant obligation remaining stated-normative.

4. **The extension pressure classification** (~line 568) — currently calls the Elicitation lifecycle stated-normative until promoted. Update it to the same partial-coverage description.

5. **The extension seams registry** (~line 603) — currently classifies "Elicitation, spawn-authority, subscription, and response-correlation models" as "stated-normative, reserved model ids" despite partial checked-model coverage. Update to reflect that these models have partial checked-model coverage (some properties promoted, some demoted to stated-normative).

### VERIFICATION.md and ADAPTER-PI.md fixes

6. **Descendant-grant tier** (`docs/VERIFICATION.md:43`) — after `SpawnCreatesDescendantGrant` is demoted, describe fleet-spawn authorization as checked-model but descendant-grant creation as stated-normative.

7. **First-answer scope** (`docs/VERIFICATION.md:93`) — narrow `ElicitationFirstAnswerWins` to the first valid answer terminal; the formula does not cover decline-terminal selection.

8. **Pi replacement-window wording** (`docs/ADAPTER-PI.md:147`) — stop presenting demoted `LateGenerationInert` as verified; retain it as a stated-normative migration requirement.

### Emitted TLA+ audit

9. VERIFICATION.md already states that `*.emitted.tla` files are generated inspection artifacts, not an independent verification lane. Audit all prose (docs, README, work items) for any claim that presents emitted TLA+ as independent evidence. If found, correct to "generated inspection artifact, not independently checked." Expected outcome: no corrections needed (the discipline is already honest), but verify.

## Acceptance criteria

- [ ] PROTOCOL.md `reply_correlation.qnt` coverage claims (~lines 75 and 94) corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md transition-adjacency claim (~line 142) corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] PROTOCOL.md Elicitation lifecycle classification (~line 270) corrected to partial checked-model coverage.
- [ ] PROTOCOL.md extension pressure classification (~line 568) corrected to partial Elicitation checked-model coverage.
- [ ] PROTOCOL.md extension seams registry (~line 603) corrected: Elicitation, spawn-authority, subscription, and response-correlation models no longer classified as purely "stated-normative, reserved model ids" — they have partial checked-model coverage.
- [ ] VERIFICATION.md:43 no longer calls descendant-grant creation checked-model.
- [ ] VERIFICATION.md:93 narrows `ElicitationFirstAnswerWins` to answer-terminal behavior.
- [ ] ADAPTER-PI.md:147 no longer describes `LateGenerationInert` as verified.
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0.
