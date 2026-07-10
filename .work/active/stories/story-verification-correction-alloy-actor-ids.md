---
id: story-verification-correction-alloy-actor-ids
kind: story
stage: implementing
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Demote tautological Alloy ActorIdsUniqueAssert

## Scope

Demote `ActorIdsUnique` from `status: promoted` to `status: draft`. The `ActorIdsUniqueAssert` checks the same injectivity constraint imposed by the `ActorIdsUnique` fact — it is a fact-consequence check, not a genuine independent verification. Actor uniqueness belongs in generated/database constraints plus executable negative tests. Alloy remains the reserved relational tool for real delegation, authority-graph, routing, and lease/fencing problems.

## Unit

`Unit 2` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/patchbay-relational.als` — `@promotion` block and `check` command
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists and generated tables

## Implementation

1. In `specs/seed/patchbay-relational.als`:
   - In the `ActorIdsUnique` `@promotion` block:
     - Change `status: promoted` → `status: draft`
     - Replace `invocation` with `<TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests>`
     - Add `demotion_reason: fact-consequence check; the assert verifies the ActorIdsUnique fact holds across all instances but does not establish non-vacuity independently`
   - Remove the `check ActorIdsUniqueAssert for 5` line at the end of the file
   - Preserve the `ActorIdsUnique` fact, the `sig Actor`, `sig Identity`, and the two already-draft reserved properties (`AuthorityGraphAcyclic`, `SenderMatchesClaim`) — these are the relational vocabulary future delegation/authority-graph work needs

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `ActorIdsUnique` from `CHECKED_MODEL_PROPERTIES`
   - Add it to `STATED_NORMATIVE_PROPERTIES`

3. Regenerate the VERIFICATION.md tables:
   - `node contracts/scripts/check-models.mjs`
   - `node contracts/scripts/check-vectors.mjs`

4. Update VERIFICATION.md prose that is NOT generated:
   - Line 36: the checked-model property list — remove `ActorIdsUnique`
   - Line 552: the seed-model summary table — move `ActorIdsUnique` to the draft column
   - Summary line: update the promoted/draft counts

## Acceptance criteria

- [ ] `ActorIdsUnique` `@promotion` block changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] `check ActorIdsUniqueAssert for 5` line removed from `patchbay-relational.als`.
- [ ] `ActorIdsUnique` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0; generated table shows `ActorIdsUnique` as stated-normative.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0.
- [ ] VERIFICATION.md prose lists updated: line 36, seed-model summary.
- [ ] The two already-draft Alloy properties (`AuthorityGraphAcyclic`, `SenderMatchesClaim`) remain draft.
- [ ] The Alloy file's sigs and facts are preserved (only the tautological `check` command is removed).
