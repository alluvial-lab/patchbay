---
id: story-verification-correction-prose
kind: story
stage: review
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
9. **Subscription grant description** (`docs/VERIFICATION.md:139`) — says `SubscriptionGrantChecked` checks an "actor/session" grant, but the model has no session principal. Narrow to match the model: "actor and stream/filter scope".
10. **Verification floor** (`docs/SPEC.md:70`) — says "checked-model seed coverage for command acceptance, idempotent retry, and session identity". After demotion, no retained property establishes accepted-command durability or the identity tuple. Update to reflect the actual retained checked-model coverage (terminal finality, boundary dedup, no-accepted-to-completed, generation monotonicity, typed correlation, Elicitation lifecycle, spawn fleet authority, subscription authority, CSRF).
11. **ElicitationState glossary** (`docs/GLOSSARY.md:63`) — says "Stated-normative until promoted — not checked" despite partial checked-model coverage. Update to reflect partial checked-model coverage.
12. **Correlation context glossary** (`docs/GLOSSARY.md:71`) — says response-Operation → Elicitation correlation is "a new stated-normative obligation" despite `TypedCorrelation` now covering it. Update to reflect checked-model coverage.

### Emitted TLA+ audit

13. VERIFICATION.md already states that `*.emitted.tla` files are generated inspection artifacts, not an independent verification lane. Audit all prose (docs, README, work items) for any claim that presents emitted TLA+ as independent evidence. If found, correct to "generated inspection artifact, not independently checked." Expected outcome: no corrections needed (the discipline is already honest), but verify.

## Acceptance criteria

- [ ] PROTOCOL.md `reply_correlation.qnt` coverage claims (~lines 75 and 94) corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md transition-adjacency claim (~line 142) corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] PROTOCOL.md Elicitation lifecycle classification (~line 270) corrected to partial checked-model coverage.
- [ ] PROTOCOL.md extension pressure classification (~line 568) corrected to partial Elicitation checked-model coverage.
- [ ] PROTOCOL.md extension seams registry (~line 603) corrected: Elicitation, spawn-authority, subscription, and response-correlation models no longer classified as purely "stated-normative, reserved model ids" — they have partial checked-model coverage.
- [ ] VERIFICATION.md:43 no longer calls descendant-grant creation checked-model.
- [ ] VERIFICATION.md:93 narrows `ElicitationFirstAnswerWins` to answer-terminal behavior.
- [ ] ADAPTER-PI.md:147 no longer describes `LateGenerationInert` as verified.
- [ ] VERIFICATION.md:139 corrected: `SubscriptionGrantChecked` no longer claims "actor/session" grant; narrowed to "actor and stream/filter scope".
- [ ] SPEC.md:70 corrected: verification floor updated to reflect actual retained checked-model coverage after demotions.
- [ ] GLOSSARY.md:63 corrected: ElicitationState no longer wholly "stated-normative until promoted"; partial checked-model coverage.
- [ ] GLOSSARY.md:71 corrected: response-Operation → Elicitation correlation no longer "a new stated-normative obligation"; checked-model coverage exists.
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0.

## Implementation notes

- Delivery mode: direct-read prose reconciliation; the target surfaces and current model metadata were explicit, so no exploratory agent was needed.
- Files changed: `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/ADAPTER-PI.md`, `docs/SPEC.md`, `docs/GLOSSARY.md`, and this story.
- Tests added: none (prose-only correction).
- Fix 1 — applied both `PROTOCOL.md` correlation corrections: `TypedCorrelation` now covers Reply → Command/Message and response Operation → Elicitation as checked-model, with checked-normative status still gated on promoted vectors.
- Fix 2 — applied the transition-adjacency correction: `allowedTransition` constrains actions to the canonical table and `NoAcceptedToCompleted` independently checks that one adjacency; the remaining graph and read/query fast-path rule stay stated-normative.
- Fix 3 — applied the `ElicitationState` lifecycle correction with the six promoted properties named and `ElicitationTimeoutNeitherSuccessNorDenial` retained as stated-normative.
- Fix 4 — applied the same precise partial-coverage classification in the protocol extension-pressure section.
- Fix 5 — corrected the extension-seams row to distinguish committed checked-model properties from remaining stated-normative obligations across the Elicitation, spawn-authority, subscription, and response-correlation families.
- Fix 6 — removed `SpawnCreatesDescendantGrant` from both non-generated checked-model inventories in `VERIFICATION.md`; fleet-spawn authorization remains checked-model, while descendant-grant creation is stated-normative. The stale seed-model summary row was an additional Unit 4 carryover not called out by the approximate line reference.
- Fix 7 — narrowed `ElicitationFirstAnswerWins` to persistence of the first valid answer terminal and explicitly excluded decline-terminal selection.
- Fix 8 — changed the Pi replacement-window checklist entry from a verification claim to a stated-normative migration requirement and stated that the current `LateGenerationInert` draft formula is not checked-model evidence.
- Fix 9 — narrowed `SubscriptionGrantChecked` from an actor/session grant to an actor grant over stream/filter scope.
- Fix 10 — replaced the stale `SPEC.md` verification-floor shorthand with the retained 21-property coverage areas and explicitly separated the demoted stated-normative obligations.
- Fix 11 — updated the `ElicitationState` glossary entry to name its partial checked-model coverage and remaining timeout/grant obligation.
- Fix 12 — updated the correlation-context glossary entry to describe `TypedCorrelation` checked-model coverage and the remaining vector gate.
- Fix 13 — audited all six `specs/seed/*.emitted.tla` artifacts and searched `docs/`, root `README.md`, and `.work/` prose for emitted-TLA evidence claims. No prose presents an emitted file as independent evidence; existing references consistently call the files generated inspection artifacts or explicitly reject an independent verification lane, so no emitted file or audit-only prose needed correction.
- Verification: `node contracts/scripts/check-models.mjs` ran twice; both runs exited 0 with 21 checked-model, 0 checked-normative, and 26 stated-normative properties, and reported the generated table already current.
- Discrepancies from design: the story's `Files` summary omitted `docs/SPEC.md` and `docs/GLOSSARY.md` even though implementation items 10–12 required them; both were corrected. Unit 4 had also left `SpawnCreatesDescendantGrant` in the non-generated checked-model seed summary, so this reconciliation removed it. No semantic conflict was found.
- Adjacent issues parked: none.
