---
id: idea-check-models-draft-discipline-enforcement
kind: backlog
stage: backlog
tags: [verification, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# check-models.mjs does not enforce the demotion/draft discipline

## Origin

Deep review (Phase 2 adversarial) of `epic-public-product-contract-verification-claim-correction`. Filed as an important finding.

## Finding

`contracts/scripts/check-models.mjs` validates `@promotion` blocks but does not enforce the discipline that the verification-claim correction relied on:

- **`demotion_reason` is not required** for `status: draft` blocks. A demoted property without a `demotion_reason` passes silently.
- **Draft invocation is not validated.** A draft block with a concrete `quint verify ...` invocation (instead of `<TBD...>`) passes silently. Invocation validation (`check-models.mjs:198-209`) only applies to `status: promoted` blocks.
- **Executable-definition presence is not checked.** A `status: draft` property whose `val`/`temporal` definition was removed (the intended state for the 11 stubbed properties) is indistinguishable to the checker from one that still has a misleading `= true` formula. The misleading-formula defect this feature corrected can recur while all metadata checks remain green.

## Why this matters

The verification-claim correction removed `val` definitions entirely (rather than `= true`) precisely so `quint verify --invariant <name>` fails honestly. But `check-models.mjs` — the metadata/traceability gate — does not verify this invariant. A future contributor could re-add a `= true` stub to a draft property, or demote a property without recording why, and CI would stay green.

The parked `idea-proto-prose-registry-consistency-check.md` addresses prose drift; this is the complementary checker-side gap for block discipline.

## Suggested work

Strengthen `check-models.mjs` validation:
- draft blocks must have `invocation` starting with `<TBD`.
- demoted drafts (those with a `demotion_reason` field, or a new explicit `demoted: true` marker) must carry `demotion_reason`.
- consider: for `status: draft` properties, warn (or fail) if a matching `val`/`temporal` definition exists in the model file — the property is stated-normative and an executable formula that passes vacuously is the defect to prevent. (This requires the checker to cross-reference block names with parsed definitions, which is a non-trivial extension; a lighter version cross-references the `invocation` `<TBD>` marker.)

Scope this as a `[verification]` feature under `epic-public-product-contract-public-compatibility` (which owns the long-term drift-detection mechanisms) or as a standalone hardening item.
