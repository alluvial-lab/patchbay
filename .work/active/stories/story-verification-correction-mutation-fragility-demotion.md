---
id: story-verification-correction-mutation-fragility-demotion
kind: story
stage: done
tags: [verification, protocol, bug]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-trace-fidelity-demotion]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Remove leftover demoted formulas and demote mutation-fragile surviving properties

## Scope

Two defects found in the round-4 deep review:

1. **Leftover demoted formulas (Blocker 2).** Units 1 and 2 demoted 9 properties' `@promotion` status to draft but left their `val`/`temporal` definitions in place. This contradicts the feature's own removal discipline (established in Unit 4): removing the definition entirely ensures `quint verify --invariant <name>` fails because the invariant doesn't exist, rather than passing on a narrow/defective formula. Currently `quint verify --invariant command_durability` still *passes* on a draft property — exactly the vacuous-pass the feature exists to prevent.

2. **Mutation-fragile surviving promoted properties (Blocker 1).** 9 surviving promoted properties have invariants that inspect state written by the same action that decides acceptance (the trace-fidelity defect). The host independently confirmed all 9 via mutation tests: a coordinated lie (mutating the accepting action to record the expected values regardless of input) makes each property pass. This is the same defect class as the 4 properties demoted in Unit 7, now confirmed pervasive across the elicitation (6), subscription (2), and reply-correlation (1) model families.

## Unit

`Unit 8` from `epic-public-product-contract-verification-claim-correction` review (round 4). Review-discovered correction: rounds 1–3 under-corrected. Round 4 found Units 1–2 left formulas that should have been removed (a Phase 7 verification miss), and the surviving promoted set still contains mutation-fragile properties whose demotion is in-scope per the feature's brief ("correcting remaining overclaims is this feature's responsibility").

## Origin

Deep review round 4 (`openai-codex/gpt-5.6-sol`, xhigh, fresh-context). The host independently verified every claim before filing:

- **Leftover formulas:** confirmed 9 demoted properties (5 in `command_lifecycle.qnt`, 3 in `session_generation.qnt`, 1 in `elicitation_lifecycle.qnt`) still have `val`/`temporal` definitions. `quint verify --invariant <name>` passes on each.
- **Mutation-fragility:** the host ran mutation tests on all 9 surviving promoted properties. Every one passes under a coordinated-lie mutation:
  - `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationWithdrawalFinality`, `ElicitationStaleTargetInert`: mutating `commitTerminal` to allow re-terminalization from any state + lying about the `firstTerminalState` baseline → all pass.
  - `ElicitationCorrelationTyped`, `ElicitationInvalidResponseRejected`: mutating `recordedResponseIndependentOk` to `true` → pass (antecedent vacuously false).
  - `SubscriptionAudited`: mutating `auditRecords' = SubscriptionEstablishAttempts + 1` (lie about audit) → passes. (A "skip audit" mutation is caught, but a coordinated lie is not.)
  - `SubscriptionCursorReplayAuthorized`: mutating `replayedEventIndependentOk` to `true` → passes.
  - `TypedCorrelation`: mutating `createResponseOperation` to record canonical correlation regardless of input → passes.

The host's round-3 dispute of this defect was **wrong**: the round-3 test mutated the wrong branch (the already-terminal branch, which the `__saved_` baseline catches) and missed the first-terminal-branch mutation the reviewer used. Round 4's reviewer found the right mutation; the host reproduced it.

## Files

- `specs/seed/command_lifecycle.qnt` — remove 5 leftover `val`/`temporal` definitions
- `specs/seed/session_generation.qnt` — remove 3 leftover `val`/`temporal` definitions
- `specs/seed/elicitation_lifecycle.qnt` — remove 1 leftover + demote 6 + remove 6 `val`/`temporal` definitions
- `specs/seed/subscription_authority.qnt` — demote 2 + remove 2 `val` definitions
- `specs/seed/reply_correlation.qnt` — demote 1 + remove 1 `val` definition
- `contracts/scripts/check-vectors.mjs` — move 9 ids from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES`
- `docs/VERIFICATION.md` — checked-model property lists, authority/subscription/elicitation/correlation sections, seed-model summaries, generated tables, verification-floor prose
- `docs/ADAPTER-PI.md` — any checked-model references to demoted properties
- `docs/SPEC.md` — verification floor (~line 70)
- `docs/PROTOCOL.md` — any checked-model references to demoted properties
- `docs/GLOSSARY.md` — any checked-model references

## Implementation

### Part A: Remove 9 leftover demoted formulas (Units 1-2 cleanup)

Remove the `val`/`temporal` definition for each of these 9 properties (already `status: draft` from Units 1-2; only the definition was left behind). Keep the `@promotion` block.

**`command_lifecycle.qnt`** (5):
- `command_durability` (line ~182, single line)
- `pre_append_terminal_choice` (line ~220, multi-line temporal — read full extent)
- `lsn_determines_terminal_winner` (line ~241, multi-line)
- `retry_reuses_id_and_key` (line ~279, multi-line)
- `retry_after_terminal_returns_existing` (line ~296, multi-line)

**`session_generation.qnt`** (3):
- `session_identity_tuple` (line ~124, multi-line)
- `late_generation_inert` (line ~180, multi-line temporal)
- `labels_cannot_override_identity` (line ~201, multi-line)

**`elicitation_lifecycle.qnt`** (1):
- `elicitation_timeout_neither_success_nor_denial` (line ~610, multi-line)

### Part B: Demote 9 mutation-fragile surviving promoted properties

For each: change `status: promoted` → `draft`, replace `invocation` with `<TBD — demoted; formula inspects state recorded by the accepting action, not independent attempted evidence; not a mutation-survivable oracle; v1 formal gate owns the real property>`, add `demotion_reason`, remove the `val`/`temporal` definition, keep the `@promotion` block.

**`elicitation_lifecycle.qnt`** (6):
- `ElicitationPendingFinality` (~553) — `temporal elicitation_pending_finality`
- `ElicitationFirstAnswerWins` (~571) — `temporal elicitation_first_answer_wins`
- `ElicitationCorrelationTyped` (~589) — `val elicitation_correlation_typed`
- `ElicitationInvalidResponseRejected` (~630) — `val elicitation_invalid_response_rejected`
- `ElicitationStaleTargetInert` (~661) — `temporal elicitation_stale_target_inert`
- `ElicitationWithdrawalFinality` (~680) — `temporal elicitation_withdrawal_finality`

**`subscription_authority.qnt`** (2):
- `SubscriptionAudited` (~233) — `val subscription_audited`
- `SubscriptionCursorReplayAuthorized` (~250) — `val subscription_cursor_replay_authorized`

**`reply_correlation.qnt`** (1):
- `TypedCorrelation` (~273) — `val typed_correlation`

**Per-property demotion reasons:**

- `ElicitationPendingFinality`: the temporal formula checks `state == firstTerminalState`, but `commitTerminal` writes both `state'` and `firstTerminalState'` in the same action. A mutation allowing re-terminalization from any state while lying about the baseline passes. Not a mutation-survivable oracle for terminal finality.
- `ElicitationFirstAnswerWins`: same `firstTerminalState`/`answeredBy`/`answeredResponseOp` baseline pattern; same coordinated-lie mutation passes.
- `ElicitationCorrelationTyped`: inspects `recordedResponseIndependentOk` which reads action-recorded response fields; forcing it to `true` makes the property pass vacuously.
- `ElicitationInvalidResponseRejected`: formula is `not(recordedResponseIndependentOk(ro)).implies(...)`; forcing `recordedResponseIndependentOk = true` makes the antecedent false (vacuous pass).
- `ElicitationStaleTargetInert`: same `firstTerminalState` baseline pattern; coordinated-lie mutation passes.
- `ElicitationWithdrawalFinality`: same `firstTerminalState` baseline pattern; coordinated-lie mutation passes.
- `SubscriptionAudited`: compares `auditRecords == SubscriptionEstablishAttempts`, both written by the establishment action; a mutation setting `auditRecords' = SubscriptionEstablishAttempts + 1` (lie about audit) passes.
- `SubscriptionCursorReplayAuthorized`: checks `replayedEventIndependentOk` which reads action-recorded grant/scope/stream/filter state; forcing it to `true` passes.
- `TypedCorrelation`: `createResponseOperation` records correlation evidence in the same action that decides recordability; a mutation recording canonical correlation regardless of input passes.

### Part C: Update check-vectors.mjs

Move the 9 ids from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES`, keeping both arrays alphabetically sorted: `ElicitationCorrelationTyped`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationPendingFinality`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, `SubscriptionAudited`, `SubscriptionCursorReplayAuthorized`, `TypedCorrelation`.

### Part D: Reconcile prose

Update all non-generated prose across VERIFICATION.md, ADAPTER-PI.md, SPEC.md, PROTOCOL.md, GLOSSARY.md:
- Checked-model property lists: remove the 9 demoted names.
- The elicitation/subscription/correlation sections: these models now have ZERO promoted properties (like authority.qnt after Unit 7). Reflect that.
- Seed-model summary tables: move the 9 to draft column; update counts.
- SPEC.md:70 verification floor: remove "selected Elicitation lifecycle properties", "subscription audit/cursor-replay authorization", "typed reply/response correlation" from checked-model coverage; add to stated-normative.
- Grep each demoted property name across docs/; mark any checked-model/verified reference as stated-normative.

### Verification

```
export PATH="$HOME/.npm-global/bin:$PATH"
quint parse specs/seed/command_lifecycle.qnt
quint parse specs/seed/session_generation.qnt
quint parse specs/seed/elicitation_lifecycle.qnt
quint parse specs/seed/subscription_authority.qnt
quint parse specs/seed/reply_correlation.qnt
node contracts/scripts/check-vectors.mjs
node contracts/scripts/check-models.mjs
node contracts/scripts/check-models.mjs
```

## Acceptance criteria

- [ ] 9 leftover `val`/`temporal` definitions removed (Part A); `@promotion` blocks preserved.
- [ ] 9 mutation-fragile properties demoted to `status: draft` with `demotion_reason` and `<TBD>` invocation (Part B); their `val`/`temporal` definitions removed; `@promotion` blocks preserved.
- [ ] 9 ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `quint parse` exits 0 for all 5 affected model files.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; `node contracts/scripts/check-models.mjs` exits 0 on second run.
- [ ] No surviving hand-authored prose calls any of the 9 demoted properties "checked-model" or "verified."
- [ ] The 8 genuinely-promoted properties remain promoted: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`, `GenerationMonotonic`, `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`.
- [ ] Final promoted count: 8 (17 − 9). Final stated-normative: 39 (30 + 9).

## Notes

- The 8 surviving promoted properties were confirmed clean: the 4 CSRF properties inspect `attemptedSession`/`attemptedProof` (raw evidence, fixed in `story-fix-csrf-trace-and-ssot-drift`); `TerminalFinality`/`BoundaryDedup`/`NoAcceptedToCompleted`/`GenerationMonotonic` check transition/structural invariants not recorded by the accepting action (round 3 confirmed).
- This unit supersedes the host's round-3 scope-routing argument. The routing was correct for the *re-architecture* (attempted-evidence state is v1-gate work) but wrong for the *demotion* of properties with confirmed mutation failure — that is in-scope claim correction.
- The `idea-csrf-trace-fidelity` backlog item and the `executable-release-assurance` design-input section (both updated in the round-3 disposition) remain accurate: they route the *re-architecture* and the *untested-property* question to the v1 gate. After this unit, there are no untested surviving promoted properties in the affected model families.


## Review (2026-07-11)

**Verdict**: Approve - fast-lane advance.

Story verified by implement (green `quint parse` + checkers); the cumulative diff across all 8 units was covered by the feature's 6-round deep-review convergence loop, which confirmed the final state (8 promoted / 39 stated-normative, 24 demotions, 24 formulas removed, 8 survivors mutation-confirmed sound).
## Implementation notes

- Files changed: `specs/seed/command_lifecycle.qnt`, `specs/seed/session_generation.qnt`, `specs/seed/elicitation_lifecycle.qnt`, `specs/seed/subscription_authority.qnt`, `specs/seed/reply_correlation.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, `docs/SPEC.md`, `docs/PROTOCOL.md`, `docs/GLOSSARY.md`, and `docs/ADAPTER-PI.md`. The adapter doc had no references to the nine newly demoted ids; its adjacent `LateGenerationInert` wording was corrected because Part A removed that already-draft formula.
- Removed all 9 leftover draft formulas from Units 1–2 and all 9 mutation-fragile promoted formulas. Preserved every `@promotion` block; each new demotion records the exact coordinated-lie mutation and now has a `<TBD>` invocation.
- Moved the 9 ids from the checked-model registry to the stated-normative registry. Final tier derivation is 8 checked-model and 39 stated-normative properties.
- Updated stale model headers/comments in the Elicitation, subscription, and reply-correlation models so they no longer claim promoted or independent-oracle coverage after demotion.
- Verification: all five required `quint parse` commands exited 0; `check-vectors.mjs` exited 0; the first `check-models.mjs` run exited 1 after regenerating the model table as expected; the second exited 0 and reported 8 checked-model / 39 stated-normative properties. `git diff --check` passed.
- Tests added: none; this correction is verified by Quint parsing plus the vector/model registry checkers.
- Discrepancies from design: none. Approximate line numbers had shifted, but every named block and formula matched the described scope. Direct-read only; no exploratory dispatch was needed because the story named every integration point and mutation result.
- Adjacent issues parked: none.
