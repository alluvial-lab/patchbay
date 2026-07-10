---
id: story-verification-correction-command-lifecycle
kind: story
stage: review
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

Demote `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` from `status: promoted` to `status: draft`. These five properties have formulas too narrow to support their product-claim names. The real failure-boundary modeling (crash/restart, competing pre-append candidates, retry-input identity, returned-record identity) is v1 formal-gate work owned by `epic-public-product-contract-executable-release-assurance`.

## Unit

`Unit 1` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/command_lifecycle.qnt` — five `@promotion` blocks
- `contracts/scripts/check-vectors.mjs` — `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` arrays
- `docs/VERIFICATION.md` — prose lists, refinement table, property definitions, seed-model summary, generated tables
- `docs/PROTOCOL.md` — `OperationState` ⇿ `CommandState` refinement section (~line 140)

## Implementation

For each of the five properties:

1. In the `@promotion` block in `command_lifecycle.qnt`:
   - Change `status: promoted` → `status: draft`
   - Replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`
   - Add `demotion_reason: <explanation from the design>`

   Rename the property-section heading at `command_lifecycle.qnt:165` so it describes mixed promotion metadata rather than claiming every following block is promoted.

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`

3. Run `node contracts/scripts/check-vectors.mjs` (exits 0, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then `node contracts/scripts/check-models.mjs` again (exits 0, confirms current). This two-pass sequence for `check-models` is required because it regenerates its table and exits 1 when the table changes on the first run.

4. Update VERIFICATION.md prose that is NOT generated:
   - The checked-model property list (the line listing `command_lifecycle.qnt` properties) — remove the five demoted names, keep `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`
   - The `OperationState` ⇿ `CommandState` refinement table — rows referencing `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` should mark these as stated-normative
   - The property definitions (`LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`) — note they are stated-normative, not checked
   - The seed-model summary table — move the five to the draft column; ADD `NoAcceptedToCompleted` which is currently missing from this table
   - The summary line — update promoted/draft counts

5. Update PROTOCOL.md `OperationState` ⇿ `CommandState` refinement section (~line 140): it currently lists `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` as checked properties inherited by `OperationState`. Mark these as stated-normative.

## Demotion reasons

- `CommandDurability`: formula is `CMD_IDS.forall(c => state.keys().contains(c))` — commands are pre-installed at init and no action removes keys. Proves map-domain persistence inside the abstraction, not durable acceptance across a failure boundary. No crash, restart, torn commit, or reconstruction is modeled.
- `PreAppendTerminalChoice`: only proves that a transition into terminal assigns a positive terminalLsn. Does not model two pre-append candidates competing before durable append, or relate the selected state to the chosen candidate. Does not even constrain post-assignment stability despite its comment.
- `LsnDeterminesTerminalWinner`: effectively `terminal state implies terminalLsn > 0`. Does not retain competing candidates, compare their LSNs, or establish minimum-LSN selection. The `lateTerminalCandidate` action is a no-op, so the second candidate cannot even commit.
- `RetryReusesIdAndKey`: only proves the command→key map never changes after init. Never observes an attempted retry's actual command id or key — the driver only calls `retry("c1", "k1")`, and the action rejects mismatches structurally. Does not check the named retry-input obligation.
- `RetryAfterTerminalReturnsExisting`: formula is identical to `TerminalFinality` (terminal stasis). No returned-record identity, record count, retry-attempt discriminator, or candidate-creation state. Does not check the named return-existing-record behavior.

## Acceptance criteria

- [ ] Five `@promotion` blocks in `command_lifecycle.qnt` changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] The property-section heading at `command_lifecycle.qnt:165` no longer labels the mixed promoted/draft blocks as all promoted.
- [ ] Five ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; `node contracts/scripts/check-models.mjs` exits 0 on second run; generated tables reflect the demotion.
- [ ] VERIFICATION.md prose updated: checked-model property list, refinement table, property definitions, idempotent-retry section (~line 213), seed-model summary (including adding `NoAcceptedToCompleted`).
- [ ] PROTOCOL.md `OperationState` ⇿ `CommandState` refinement section (~line 140) updated: the five demoted properties marked stated-normative.
- [ ] PROTOCOL.md ~line 380 updated: `RetryAfterTerminalReturnsExisting` no longer called checked.
- [ ] The 3 genuine promoted properties in `command_lifecycle.qnt` remain `status: promoted`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`.
- [ ] `quint parse specs/seed/command_lifecycle.qnt` exits 0.

## Implementation notes

- Files changed: `specs/seed/command_lifecycle.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, `docs/PROTOCOL.md`.
- Formal-model metadata: demoted the five overclaiming lifecycle properties with the design-specified reasons and placeholder invocations; retained `TerminalFinality`, `BoundaryDedup`, and `NoAcceptedToCompleted` as promoted.
- Traceability and prose: moved the five ids to stated-normative, regenerated both verification tables, and aligned lifecycle/refinement/idempotency prose and seed-model summaries with the supported tiers. The generated model summary is 27 promoted / 17 draft modeled properties plus 3 reserved-unmodeled properties; `NoAcceptedToCompleted` now appears in the checked seed-model row.
- Verification: `quint parse specs/seed/command_lifecycle.qnt` exited 0; `check-vectors.mjs` exited 0; the first `check-models.mjs` run exited 1 after regenerating the stale table as expected; the second run exited 0 and confirmed the table current.
- Tests added: none; this story's verification surface is Quint parsing plus the model/vector metadata checkers.
- Discrepancies from design: the generated current-state counts are 27 checked-model and 20 stated-normative total (17 modeled draft plus 3 reserved-unmodeled), rather than the parent design's future multi-unit counts; generated tables remain authoritative.
- Dispatch: direct-read inline implementation; the four named files and formulas made the integration surface explicit, so no exploratory agent was needed.
- Adjacent issues parked: none.
