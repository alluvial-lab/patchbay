---
id: story-verification-correction-command-lifecycle
kind: story
stage: implementing
tags: [verification, protocol]
parent: epic-public-product-contract-verification-claim-correction
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Demote overclaiming command_lifecycle.qnt properties

## Scope

Demote `CommandDurability`, `PreAppendTerminalChoice`, and `LsnDeterminesTerminalWinner` from `status: promoted` to `status: draft`. These three properties have formulas too narrow to support their product-claim names. The real failure-boundary modeling (crash/restart, competing pre-append candidates) is v1 formal-gate work owned by `epic-public-product-contract-executable-release-assurance`.

## Unit

`Unit 1` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/command_lifecycle.qnt` — three `@promotion` blocks
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists and generated tables

## Implementation

For each of the three properties:

1. In the `@promotion` block in `command_lifecycle.qnt`:
   - Change `status: promoted` → `status: draft`
   - Replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`
   - Add `demotion_reason: <one-line explanation of the formula/name gap>`

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`

3. Regenerate the VERIFICATION.md tables:
   - `node contracts/scripts/check-models.mjs` (regenerates the model-promotion table)
   - `node contracts/scripts/check-vectors.mjs` (regenerates the conformance-vector table)

4. Update VERIFICATION.md prose that is NOT generated:
   - Line 29: the checked-model property list — remove the three demoted names
   - Line 82: the refinement table row `first durable terminal commit | existing PreAppendTerminalChoice + LsnDeterminesTerminalWinner` — mark these as stated-normative
   - Lines 189–190: the property definitions — note they are stated-normative, not checked
   - Line 545: the seed-model summary table — move the three to the draft column
   - Summary line: "32 promoted, 12 draft" → "29 promoted, 15 draft"

## Demotion reasons

- `CommandDurability`: formula is `CMD_IDS.forall(c => state.keys().contains(c))` — commands are pre-installed at init and no action removes keys. Proves map-domain persistence inside the abstraction, not durable acceptance across a failure boundary. No crash, restart, torn commit, or reconstruction is modeled.
- `PreAppendTerminalChoice`: only proves that a transition into terminal assigns a positive terminalLsn. Does not model two pre-append candidates competing before durable append, or relate the selected state to the chosen candidate.
- `LsnDeterminesTerminalWinner`: effectively `terminal state implies terminalLsn > 0`. Does not retain competing candidates, compare their LSNs, or establish minimum-LSN selection. The `lateTerminalCandidate` action is a no-op, so the second candidate cannot even commit.

## Acceptance criteria

- [ ] Three `@promotion` blocks in `command_lifecycle.qnt` changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] Three ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0; generated table shows 29 promoted / 15 draft.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; generated table shows the three as stated-normative.
- [ ] VERIFICATION.md prose lists updated: line 29 list, line 82 refinement row, lines 189–190 definitions, seed-model summary.
- [ ] The 5 genuine promoted properties in `command_lifecycle.qnt` remain `status: promoted`: `TerminalFinality`, `BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`, `NoAcceptedToCompleted`.
- [ ] `quint parse specs/seed/command_lifecycle.qnt` exits 0.
