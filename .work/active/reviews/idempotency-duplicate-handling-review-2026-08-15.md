---
id: idempotency-duplicate-handling-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-idempotency-duplicate-handling
created: 2026-08-15
updated: 2026-08-15
---

# Review: duplicate, ambiguous-outcome, and claim reconciliation

## Verdict

**MATERIAL** — return the story to `implementing`.

The closed no-effect paths, exact evidence retry, poison redelivery suppression, and the four requested mutation oracles are strong. Three reconciliation gaps remain: an authenticated ambiguous Result before the delivery acknowledgement does not poison, successful identified evidence is rejected instead of remaining active for promotion, and identified evidence can claim an external runtime already owned by another logical target.

## Findings

### MATERIAL — effect-before-ack ambiguity is terminalized without poisoning

**Location:** `server/src/adapter_service.rs:583`

`poison_ambiguous_spawn_result` accepts only commands already projected as `Delivered` or `Running`. An authenticated, exactly correlated `Cancelled`, `Expired`, or `ExecutionOutcomeUnknown` Result received while the command is still `Accepted` therefore skips Leaf 5 evidence. The ordinary observation path can then terminalize `Accepted -> Failed` while retaining the original failure code, leaving the claim `Active` rather than `PoisonedPendingReconciliation`. Absence of the delivery acknowledgement cannot establish that the adapter had no delivery responsibility; the Result itself is the required effect-before-ack ambiguity signal.

Reviewer probe: removing the acknowledgement from `delivered_ambiguous_spawn_results_poison_the_exact_claim` failed with exit 101; for `Cancelled`, the projected claim was `Active` instead of `PoisonedPendingReconciliation`.

**Concrete fix:** admit the exact `Accepted` spawn pre-state for these authenticated ambiguous Results, append `MAY_EXIST` Leaf 5 evidence through the dedicated writer before ordinary terminalization, and add focused no-ack cases for cancellation, expiry, and outcome-unknown. Preserve the target/adapter/exact-command checks.

### MATERIAL — successful identified evidence cannot remain active for promotion

**Location:** `core/src/storage/rusqlite.rs:3381`

The writer groups every `Identified` disposition with `MayExist` and always attempts `Active -> PoisonedPendingReconciliation`. That contradicts `docs/PROTOCOL.md:412`, where `success_evidence_reported + identified` reserves the runtime while the claim remains active until promotion. Because poison evidence validation requires a failure code, a valid `SuccessEvidenceReported / Identified / Unspecified` record is rolled back as corrupt instead of being durably reconciled.

Reviewer probe: changing the existing identified-runtime ingress case to `SuccessEvidenceReported` with `FailureCode::Unspecified` failed with exit 101: `claim poison requires typed ambiguous or identified external-effect failure evidence`.

**Concrete fix:** derive the claim consequence from phase and failure, not disposition alone. Reserve every valid identified runtime, poison launch ambiguity and identified failure evidence, but leave successful identified evidence active and delivery-suppressed for the original claim's promotion. Add focused storage/replay tests for successful identified evidence, exact retry, active disposition, and subsequent original-claim promotion.

### MATERIAL — execution-evidence reconciliation bypasses the logical-target ownership index

**Locations:** `core/src/storage/rusqlite.rs:3315`, `core/src/storage/rusqlite.rs:3352`, `core/src/session/spawn_claim.rs:490`

The dedicated writer rebuilds `SessionRegistry`, but before appending identified evidence it validates ownership only through `SpawnClaimRegistry.external_runtime_claims`. That index contains identified/staged claim evidence, not every current, reserved, or tombstoned owner in the authoritative logical-target projection. Identified evidence for claim B can therefore reserve an external tuple already owned by logical target A, making the claim and logical-target reverse indexes disagree. Staging rejects the collision later, but the wrong-claim evidence and poison are already durable.

Reviewer probe: with the external runtime installed as logical target A's current runtime, claim B's identified evidence was accepted; the focused expected-`DuplicateNativeReference` test failed with exit 101.

**Concrete fix:** inside the same writer transaction, consult `sessions.logical_targets().owner_of(external_runtime)` before `claims.observe`. Reject an owner different from the evidence's exact logical target/claim as `DuplicateNativeReference` with zero writes. Cover current, reserved-candidate, and tombstone owners across hot replay and restart, while allowing the same original claim/target's exact retry.

## Mutation matrix

All mutants were applied one at a time on the main tree, run through one focused test, and reverted with `git restore`. The tree was clean after every probe.

| Mutant | Focused oracle | Result |
|---|---|---|
| Terminal command state releases the claim and clears its fence | `terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant` | **Killed**, exit 101; expected `Active`, observed `ReleasedNoExternalEffect` |
| No-effect inferred from terminal state / absence of acknowledgement | `silence_without_delivered_ack_is_not_no_effect_proof` | **Killed**, exit 101; unproved release was admitted |
| Poisoned attempt may be offered again | `reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch` | **Killed**, exit 101; delivery-suppression assertion failed |
| External runtime reverse-owner check ignored | `identified_runtime_is_reserved_to_its_original_claim_at_ingress` | **Killed**, exit 101; second claim was not rejected |

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (including 38 spawn-claim tests and 77 server unit tests).
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38, including the real restart e2e).
- Final `git diff --check`: **PASS**.

## Recommendation

**Return to implementing.** Fix the three MATERIAL reconciliation gaps, add the focused regressions described above, rerun the clean-tree suite, and submit a new thorough review pass.
