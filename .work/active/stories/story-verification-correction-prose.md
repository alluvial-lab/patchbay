---
id: story-verification-correction-prose
kind: story
stage: implementing
tags: [verification, foundation]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-command-lifecycle]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Fix stale PROTOCOL.md prose and audit emitted TLA+

## Scope

Fix stale PROTOCOL.md assertions that contradict current HEAD, and audit emitted TLA+ files for any prose presenting them as independent evidence. Also fix the stale model classification in the PROTOCOL.md extension seams registry.

## Unit

`Unit 5` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `docs/PROTOCOL.md` — three stale assertions + extension seams registry
- `specs/seed/*.emitted.tla` — generated inspection artifacts (audit only)

## Implementation

### PROTOCOL.md fixes

1. **The `reply_correlation.qnt` coverage claim** (~line 94) — currently says: "The existing `reply_correlation.qnt` does **not** cover response Operation → Elicitation. Extending typed correlation is a new verification obligation."

   Current HEAD: `TypedCorrelation` in `reply_correlation.qnt` now covers both Reply → Command/Message AND response Operation (`approval-response`/`elicitation-response`) → Elicitation typed references across disjoint id spaces. Update to reflect that the coverage exists, while noting it is checked-model (not checked-normative until vectors are promoted).

2. **The transition-adjacency claim** (~line 142) — currently says: "the current checked model permits any non-terminal state to commit any terminal candidate, so adjacency rules such as no `accepted → completed` require a strengthened lifecycle model..."

   Current HEAD: `NoAcceptedToCompleted` is now a checked-model property, and `allowedTransition` enforces the exact PROTOCOL transition table. Update to reflect that the no-`accepted → completed` adjacency is now checked, while the full transition graph and read/query fast-path rule remain stated-normative.

   Note: Unit 1 (`story-verification-correction-command-lifecycle`) fixes the `OperationState` ⇿ `CommandState` refinement section (~line 140) which lists demoted properties as checked. This story fixes the remaining stale prose at lines 94, 142, and 603. Coordinate to avoid conflicting edits.

3. **The extension seams registry** (~line 603) — currently classifies "Elicitation, spawn-authority, subscription, and response-correlation models" as "stated-normative, reserved model ids" despite partial checked-model coverage. Update to reflect that these models have partial checked-model coverage (some properties promoted, some demoted to stated-normative).

### Emitted TLA+ audit

4. VERIFICATION.md already states that `*.emitted.tla` files are generated inspection artifacts, not an independent verification lane. Audit all prose (docs, README, work items) for any claim that presents emitted TLA+ as independent evidence. If found, correct to "generated inspection artifact, not independently checked." Expected outcome: no corrections needed (the discipline is already honest), but verify.

## Acceptance criteria

- [ ] PROTOCOL.md `reply_correlation.qnt` coverage claim (~line 94) corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md transition-adjacency claim (~line 142) corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] PROTOCOL.md extension seams registry (~line 603) corrected: Elicitation, spawn-authority, subscription, and response-correlation models no longer classified as purely "stated-normative, reserved model ids" — they have partial checked-model coverage.
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0.
