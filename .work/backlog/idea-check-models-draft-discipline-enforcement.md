---
id: idea-check-models-draft-discipline-enforcement
kind: backlog
created: 2026-07-10
updated: 2026-07-11
tags: [verification, protocol]
research_refs: []
---

# Backlog: check-models.mjs should enforce the demotion/draft discipline

Filed from the round-2 deep review of `epic-public-product-contract-verification-claim-correction`.

`contracts/scripts/check-models.mjs` validates `@promotion` block shape but does not enforce the discipline the verification-claim correction relied on:

- `demotion_reason` is not required for `status: draft` blocks.
- draft `invocation` is not validated (a concrete `quint verify ...` instead of `<TBD...>` passes; invocation validation at `check-models.mjs:198-209` only applies to promoted blocks).
- executable-definition presence is not checked — a `status: draft` property whose `val`/`temporal` was removed (the intended state for stubbed properties) is indistinguishable from one with a misleading `= true` formula.

The misleading-formula defect this feature corrected can recur while all metadata checks stay green.

## Three-way distinction the check needs

A blanket ban on draft executable definitions would be wrong — not all drafts are formula-less. The checker must distinguish:

1. **Formula-less reservations** (`status: draft`, no `val`/`temporal`, invocation `<TBD>`): the intended state for demoted properties whose misleading formulas were removed, and for reserved-unmodeled ids. No executable definition should exist.
2. **Demoted-but-retained formulas** (if any future draft keeps a genuine-but-insufficient formula): `status: draft` with a real formula. Currently none exist in the seed models, but the discipline should not forbid them outright.
3. **Forbidden vacuous stubs** (`= true`, `always(true)`): the defect class to detect and reject.

Suggested check shape: for `status: draft`, require `invocation` starts with `<TBD`; require `demotion_reason` when a `demoted:` marker is present (or introduce an explicit draft-disposition field); and warn/fail when a draft block has a `val`/`temporal` whose body is a literal `true`/`always(true)` (the vacuous-stub detector). The formula-less-vs-retained distinction can be left to author judgment unless a stricter rule is wanted.

Scope under `epic-public-product-contract-public-compatibility` (which owns long-term drift-detection mechanisms) or as a standalone hardening item.
