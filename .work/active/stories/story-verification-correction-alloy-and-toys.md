---
id: story-verification-correction-alloy-and-toys
kind: story
stage: implementing
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-session-elicitation]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Demote ActorIdsUnique and relocate superseded toy artifacts

## Scope

Demote `ActorIdsUnique` from `status: promoted` to `status: draft` (keeping the `check` as a non-promoted structural regression test). Relocate `patchbay-invariants.als` (superseded toy) and `Counter.qnt`/`Counter.tla`/`Counter.cfg` (hello-world tooling examples) out of `specs/seed/` into skill example directories, updating skill references.

## Unit

`Unit 3` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/patchbay-relational.als` — `@promotion` block and `check` command
- `specs/seed/patchbay-invariants.als` — superseded toy (relocate)
- `specs/seed/Counter.qnt`, `specs/seed/Counter.tla`, `specs/seed/Counter.cfg` — toy examples (relocate)
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists, seed-model summary, generated tables
- `.agents/skills/alloy/SKILL.md` — references `patchbay-invariants.als` at line 106
- `.agents/skills/quint/SKILL.md` — references `Counter.qnt` at line 152
- `.agents/skills/tla-plus/SKILL.md` — references `Counter.tla`/`Counter.cfg` at line 104

## Implementation

### ActorIdsUnique demotion

1. In `specs/seed/patchbay-relational.als`, in the `ActorIdsUnique` `@promotion` block:
   - Change `status: promoted` → `status: draft`
   - Replace `invocation` with `<TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests>`
   - Add `demotion_reason: fact-consequence check; the assert verifies the ActorIdsUnique fact holds across all instances but does not establish non-vacuity independently`

2. Keep the `check ActorIdsUniqueAssert for 5` line but add a comment above it:
   ```
   // structural regression test — NOT promoted assurance; guards against accidental fact weakening
   ```

3. In `contracts/scripts/check-vectors.mjs`:
   - Remove `ActorIdsUnique` from `CHECKED_MODEL_PROPERTIES`
   - Add it to `STATED_NORMATIVE_PROPERTIES`

### Toy artifact relocation

4. Relocate `specs/seed/patchbay-invariants.als` to `.agents/skills/alloy/examples/patchbay-invariants.als` (create the directory if needed).

5. Relocate `specs/seed/Counter.qnt` to `.agents/skills/quint/examples/Counter.qnt`, `specs/seed/Counter.tla` and `specs/seed/Counter.cfg` to `.agents/skills/tla-plus/examples/`.

6. Update skill references:
   - `.agents/skills/alloy/SKILL.md:106` — update the "Hello-world artifact" path from `specs/seed/patchbay-invariants.als` to `.agents/skills/alloy/examples/patchbay-invariants.als`
   - `.agents/skills/quint/SKILL.md:152` — update from `specs/seed/Counter.qnt` to `.agents/skills/quint/examples/Counter.qnt`
   - `.agents/skills/tla-plus/SKILL.md:104` — update from `specs/seed/Counter.tla` + `specs/seed/Counter.cfg` to the new skill example paths

### Verification

7. Run `node contracts/scripts/check-vectors.mjs` (exits 1, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then run both again to confirm exit 0.

8. Update VERIFICATION.md prose: the checked-model property list (remove `ActorIdsUnique`), the seed-model summary table, and the summary line.

## Acceptance criteria

- [ ] `ActorIdsUnique` `@promotion` block changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] `check ActorIdsUniqueAssert for 5` line kept with a comment clarifying it is not promoted assurance.
- [ ] `ActorIdsUnique` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `patchbay-invariants.als`, `Counter.qnt`, `Counter.tla`, `Counter.cfg` relocated out of `specs/seed/` to skill example directories.
- [ ] `.agents/skills/alloy/SKILL.md`, `.agents/skills/quint/SKILL.md`, `.agents/skills/tla-plus/SKILL.md` references updated to new file locations.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 on second run; `node contracts/scripts/check-models.mjs` exits 0 on second run.
- [ ] VERIFICATION.md prose updated: checked-model property list, seed-model summary.
- [ ] The two already-draft Alloy properties (`AuthorityGraphAcyclic`, `SenderMatchesClaim`) remain draft.
- [ ] The Alloy file's sigs, facts, and the `check` command are preserved.
