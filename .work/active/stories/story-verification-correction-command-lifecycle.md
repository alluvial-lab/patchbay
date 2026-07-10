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

2. In `contracts/scripts/check-vectors.mjs`:
   - Remove `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`

3. Run `node contracts/scripts/check-vectors.mjs` FIRST (regenerates the conformance-vector table and updates the registry), then `node contracts/scripts/check-models.mjs` (regenerates the model-promotion table). This order is critical: `check-models.mjs` validates the existing generated conformance-vector table before rewriting its own and exits 1 when its table changes.

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
- [ ] Five ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; then `node contracts/scripts/check-models.mjs` exits 0; generated tables reflect the demotion.
- [ ] VERIFICATION.md prose updated: checked-model property list, refinement table, property definitions, seed-model summary (including adding `NoAcceptedToCompleted`).
- [ ] PROTOCOL.md `OperationState` ⇿ `CommandState` refinement section updated: the five demoted properties marked stated-normative.
- [ ] The 3 genuine promoted properties in `command_lifecycle.qnt` remain `status: promoted`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`.
- [ ] `quint parse specs/seed/command_lifecycle.qnt` exits 0.
