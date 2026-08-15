---
id: generation-monotonicity-rereview3-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-generation-monotonicity-tombstoning
created: 2026-08-15
updated: 2026-08-15
---

# Re-review pass 4: promotion fold, exact monotonicity, and tombstones

## Verdict

**CLEAN** — advance the story to `done`.

The round-3 explicit provenance model closes both pass-3 discriminator holes. Format 3 carries deterministic per-lineage managed markers derived by the session fold; format 2 decodes with no markers as legacy input. Marked tombstones require exact three-way agreement among marker, session tombstone, and logical-target tombstone, while an unmarked logical-target tombstone is rejected as disposable cross-version state. No material findings or nits remain.

## Findings

None.

Direct inspection confirmed:

- accepted managed claims, staged successors, promotions, and promotion-created tombstones all record managed provenance in `SessionRegistry`;
- production checkpoint materialization carries those records through the local format-3 wrapper, while the old unmarked encoder is not used by the production writer;
- hydration checks marker → session/logical symmetry and logical → marker/session symmetry before accepting a marked successor;
- unmarked session-only history retains the legacy same-runtime successor rule, including replay-reachable initial-current adoption;
- the old-slot-reuse craft cannot make a marked changed-runtime lineage pose as legacy;
- format-2 decoding remains supported, and the change has no `.proto`, generated-contract, or public wire impact;
- semantic checkpoint rejection still enters the single deterministic LSN-0 recovery path and converges with full replay.

## Mutation matrix

Every mutation was applied alone on the main tree and reverted with `git restore`; the focused clean oracle passed after each restoration.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Remove promotion/tombstone-path insertion into `managed_tombstone_owners` | `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion` | **Killed**, exit 101; writer-derived marker tombstones became empty (`left: 0`, `right: 1`). Restored oracle passed. |
| Ignore managed-marker contents during checkpoint hydration and treat retained session tombstones as legacy | `marked_changed_runtime_rejects_missing_logical_tombstone_despite_old_slot_reuse` | **Killed**, exit 101; the old-slot masquerade was accepted and the expected-rejection assertion failed. Restored oracle passed. |
| Weaken Quint `foldGuard` to only `phase == "pending"` | `wrong_prior_is_inert` | **Killed**, exit 1; the named scenario failed. Restored scenario passed. |
| Clean legacy-adoption probe | `unmarked_legacy_adoption_hydrates_after_session_generation_history` | **PASS**, non-vacuous library test. |
| Clean old-slot-reuse probe | `marked_changed_runtime_rejects_missing_logical_tombstone_despite_old_slot_reuse` | **PASS**, non-vacuous library test. |
| Symmetric checkpoint fallback | `complete_checkpoint_round_trips_tombstones_source_cursor_and_tail` | **PASS**; both asymmetry directions reject, fall back to LSN 0, and match full-replay sessions, tombstones, logical projection, and late-runtime classification. |

## Formal and regression evidence

- `./formal/run-model-checks.sh`: **PASS**, 20/20.
- All six named `session_generation_promotion` scenarios: **PASS**.
- `promotion_fold_exact_and_atomic` via Apalache through 10 steps: **PASS**.
- Fresh quiet Quint compilation of `session_generation_promotion` produced a 585-line TLA+ body byte-for-byte equal to the committed inspection artifact after its three-line header.
- Rust monotonicity, tombstone-retention, exact-promotion, reverse-index, and injected-decrease/mutation oracles all pass in the workspace suite.

## Full clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 killed mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38).
- Final tree status before writing this review: **clean**.

## Recommendation

**Advance to done.** The explicit format-3 provenance model is mutation-sensitive, preserves genuine format-2/legacy hydration, rejects both managed asymmetry directions including the old-slot masquerade, and retains deterministic full-replay convergence.
