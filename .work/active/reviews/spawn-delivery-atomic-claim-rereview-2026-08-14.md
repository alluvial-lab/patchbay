---
id: spawn-delivery-atomic-claim-rereview-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: spawn-delivery-atomic-claim-idempotency-generation
created: 2026-08-14
updated: 2026-08-14
---

# Thorough re-review — Unit 2 atomic accepted claim and N-delivery fence

**Verdict: CLEAN.** Pass 2 reviewed fix commit `dce6872` against the original Unit 2 implementation `6f986fb`, the pass-1 findings, current source, generated contracts, focused mutation evidence, and the full clean-tree verification matrix. Both BLOCKERs and the MATERIAL finding from pass 1 are genuinely closed, and the previously confirmed atomic transaction behavior remains mutation-sensitive.

Review mode: independent fresh-context delegated story review, effective weight `thorough`, pass 2. Adjacent Unit 8 commit `ab83bda` was treated only as consumer context.

## Findings

No current BLOCKER, MATERIAL, or NIT findings.

## Pass-1 closure

| Pass-1 finding | Disposition |
|---|---|
| Delivery discarded the exact claim and both-Grant provenance | **Closed.** Generated `Delivery.accepted_spawn` is committed in the proto plus Rust/TypeScript bindings. Managed delivery decodes and validates the exact durable `SpawnClaimAccepted` from the `SpawnClaimEvent` on both live and restart scans, while preserving the nested Operation compatibility view. Hot/restart tests assert semantic and encoded-byte equality and explicitly check claim, spawning Grant, replacement Grant, exact prior, and generation. Missing claim or each named authority/generation field fails before adapter delivery. `npm run check:drift` regenerated both bindings and found no drift. |
| Continuations entered the legacy broad-Grant completion tail | **Closed.** `SpawnDescendantTail` returns before translating any accepted claim whose `expected_prior` is present. The regression combines continuation acceptance, delivered state, successful Result, session evidence, and replacement-Grant revocation and observes no legacy audit/descendant-Grant/completion action. Removing the exclusion produces `RecordAudit` and fails the test. A separate fresh managed-claim regression preserves the compatibility bridge. |
| Dedicated writer admitted payload/claim disagreement | **Closed.** The shared accepted-envelope validator now composes canonical spawn-payload validation, authority-carriage validation, claim/fence validation, and exact-prior checks before the transaction inserts anything. This enforces fresh iff no prior and generation 1; request/claim/provenance/fence prior agreement; non-empty distinct Grants; and exact N+1. Four one-field storage tests prove malformed candidates leave the log empty. Removing authority-carriage binding makes the fresh/continuation disagreement test accept and is killed. |
| Previously confirmed exclusivity, atomic fence, dedup, and guard removal | **Preserved.** No production storage transaction, claim-exclusivity, dedup, prior-work derivation, or acceptance-pipeline ownership was weakened. All four pass-1 mutation oracles were re-run and killed their mutants. |

## Mutation matrix

Every mutation was made on the main tree, exercised with one focused test, and immediately reverted with `git restore`. The tree was clean before full verification.

| Mutation | Result | Focused oracle |
|---|---|---|
| Remove managed delivery-envelope population | **KILLED** — delivery had no `accepted_spawn` | `cargo test -p patchbay-core-server adapter_service::tests::managed_spawn_delivery_preserves_the_exact_durable_envelope_hot_and_after_restart -- --exact` |
| Re-admit managed continuations to the legacy completion tail | **KILLED** — legacy `RecordAudit` appeared | `cargo test -p patchbay-core --test authority_spawn_tail managed_continuation_never_enters_the_legacy_one_grant_completion_tail -- --exact` |
| Drop canonical payload/authority-carriage binding in the dedicated writer | **KILLED** — fresh payload plus continuation claim was durably accepted | `cargo test -p patchbay-core --test spawn_claim_acceptance dedicated_writer_rejects_fresh_payload_with_continuation_claim_without_writes -- --exact` |
| Remove transactional claim conflict enforcement and staged exclusivity validation | **KILLED** — two durable owners appended instead of one | `cargo test -p patchbay-core --test spawn_claim_acceptance distinct_continuations_race_to_exactly_one_durable_owner -- --exact` |
| Remove pending-replacement activation from the accepted-event claim fold | **KILLED** — prior-N work remained `Accepted` instead of `Superseded` | `cargo test -p patchbay-core --test spawn_claim_acceptance acceptance_fence_barrier_has_one_before_or_after_winner_and_exact_retry_survives -- --exact` |
| Bypass idempotency reconciliation before claimability | **KILLED** — exact retry returned `SpawnClaimConflict` instead of the original durable acceptance | `cargo test -p patchbay-core --test spawn_claim_acceptance exact_retry_returns_original_claim_and_changed_payload_is_inert -- --exact` |
| Re-introduce the Unit 1 continuation rejection guard | **KILLED** — continuation returned `Rejected` instead of `Accepted` | `cargo test -p patchbay-core --test acceptance_pipeline continuation_acceptance_round_trips_compound_claim_while_fresh_uses_single_grant -- --exact` |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; generated Rust/TypeScript trees drift-clean, 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests on the current tree.

The working tree was clean after verification.

## Recommendation

**Advance `spawn-delivery-atomic-claim-idempotency-generation` to `done`.** The intentional downstream Pi supervisor consumption of `accepted_spawn` remains owned by the Pi feature and is not a Unit 2 finding.
