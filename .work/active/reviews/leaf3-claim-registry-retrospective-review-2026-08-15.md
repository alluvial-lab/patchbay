---
id: leaf3-claim-registry-retrospective-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-claim-registry-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-15
updated: 2026-08-15
---

# Retrospective deep review — Leaf 3 claim registry

## Verdict

**APPROVE (CLEAN).** No blocker, material finding, or nit survived completeness and adversarial review of the landed claim-registry snapshot at `090e0bd`. The registry preserves one durable exclusive owner, keeps command terminality independent from claim disposition, fails closed around evidence-dependent release/reconciliation, and exposes the exact active/poisoned continuation fence without delivering spawn work.

## Completeness

| Contract surface | Landed evidence | Verdict |
|---|---|---|
| Exclusive key: authority domain + logical target + expected prior generation | `SpawnClaimKey`, `claim_key`, and `exclusive_claims` enforce the complete key. Active, poisoned, promoted, and abandoned claims retain ownership; only proved no-effect release removes it. Exact retries return the original record, competing commands conflict, and cold replay reconstructs the same owner. | **PASS** |
| Disposition transitions | `allowed_spawn_claim_transition` is the exact active/poisoned adjacency table. `apply_disposition_change` verifies projected pre-state, legal adjacency, and closed evidence; `SpawnPromotionCommitted` separately consumes only the exact active/poisoned claim. Released, promoted, and abandoned dispositions cannot reactivate. | **PASS** |
| No-effect release | Release requires a referenced typed `SpawnExecutionEvidence`, exact claim/domain correlation, current attachment provenance, a closed proof variant, a proof newer than the latest disposition, no later contradictory effect evidence, and exact post-proof prior-N liveness for continuation. Silence and terminal command state are not proofs. | **PASS** |
| Poison and reconciliation | Phase/effect/failure classification poisons ambiguous or identified failures, retains key and fence, and allows only later exact proof, promotion, or authenticated target abandonment to resolve the claim. Exact evidence retries remain idempotent across replay. | **PASS** |
| Delivery-fence query | `delivery_fence` and `delivery_fence_for_target_scope` return canonical `superseded/replacement_pending` only for an exact active/poisoned continuation claim that still owns its key. Release, promotion, and abandonment clear the fence. | **PASS** |
| Accepted-continuation effects | One `SpawnClaimAccepted` replay unit carries claim, compound authority, exact-prior fence, and explicit prior-work effects. Validation requires canonical supersession for accepted/not-offered work and quiesce/outcome reconciliation for delivered/running work, with unique command ids. Tests exercise atomic visibility, complete derived effects, before/after fence races, post-fence rejection, and exact retry. | **PASS** |
| No spawn delivery in Leaf 3 | `spawn_claim.rs` is a durable fold, validator, encoder, and read-only query port. It observes delivery lifecycle facts but contains no delivery producer, adapter invocation, launch path, or hidden hold queue. | **PASS** |

The acceptance rows have behavior-bearing tests rather than construction-only assertions: `continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically`, `exact_retry_projects_original_while_changed_or_competing_claim_conflicts`, `every_terminal_command_state_is_claim_inert`, `all_three_closed_no_effect_proofs_can_release_only_with_typed_evidence`, `storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row`, `claim_acceptance_derives_complete_prior_effects_and_replay_suppresses_delivery`, and `acceptance_fence_barrier_has_one_before_or_after_winner_and_exact_retry_survives` cover the required rows and replay consequences.

## Findings

- **Blockers:** none.
- **Material:** none.
- **Nits:** none.

## Mutation matrix

Each mutation was applied alone to `core/src/session/spawn_claim.rs` on the main tree, exercised with one focused test, and reverted with `git restore` before the next mutation. No mutation was committed.

| Injected mutant | Focused oracle | Result |
|---|---|---|
| Release and clear the fence when a command enters `failed`, `cancelled`, or `expired` | `terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant` | **KILLED** — observed `ReleasedNoExternalEffect` where `Active` was required. |
| Admit terminal `released_no_external_effect → active` disposition transition | `disposition_table_matches_every_legal_cell` | **KILLED** — exhaustive table rejected the extra edge. |
| Omit active/poisoned claims from the delivery-fence query | `continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically` | **KILLED** — query returned `Open` instead of canonical `ReplacementPending`. |
| Remove the duplicate exclusive-key rejection in claim acceptance | `distinct_commands_never_share_one_active_generation_kills_reclaim_mutant` | **KILLED** — the second distinct command incorrectly appended and the property failed. |

The tree was clean after every restore and after the final verification.

## Clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**, including 39 claim-registry tests and the compile-fail checkpoint-authority contract.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; generated drift clean, 57 vectors, 17 promoted vectors, 26 implementation checks, and 38 registered mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 27/27.
4. `cd pi-adapter && npm test` — **PASS**, 38/38, including the real core/adapter restart loop.

`git diff --check` also passed.

## Retrospective note

The story landed through implementation `7d7c9cc`, safety fix `6324a57`, and done transition `03c778b`, but the independent review artifact cited by the story was not retained. This artifact reconstructs that missing audit evidence against the landed code rather than treating the story's completion note as proof. Later Units 2/4/6/7 consuming and reviewing the registry were considered integration context only; the verdict rests on direct source/test inspection, the four independent mutation kills above, and the clean four-group run.
