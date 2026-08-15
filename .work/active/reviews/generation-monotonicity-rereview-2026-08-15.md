---
id: generation-monotonicity-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-generation-monotonicity-tombstoning
created: 2026-08-15
updated: 2026-08-15
---

# Re-review: promotion fold, exact monotonicity, and tombstones

## Verdict

**MATERIAL** — return the story to `implementing`.

The pass-1 logical-target-tombstone → session-tombstone gap is fixed, its checkpoint-rejection/full-replay regression is non-vacuous, and the quiet TLA+ artifact is correct. The inverse direction is still incomplete for a managed continuation that keeps the same native runtime id: removing only the logical-target tombstone is misclassified as a permissible legacy session-only lineage, so checkpoint hydration accepts state that diverges from full replay and drops the prior generation's logical reverse reservation.

## Findings

### MATERIAL — same-runtime managed session tombstone can hydrate without its logical counterpart

**Location:** `core/src/session/registry.rs:1283`, `core/src/session/registry.rs:1317`

`checkpoint_tombstone_has_current_successor` enters managed validation only when `logical_targets.owner_of(&superseded)` already finds the exact prior runtime reference. If the malformed checkpoint has removed that exact logical-target tombstone, the lookup is necessarily absent. When the managed successor retains the same adapter/deployment/runtime id at generation N+1, the later legacy fallback finds the live session in that same slot and accepts the session tombstone as a legacy lineage.

Reviewer probe: starting from the committed valid promotion checkpoint fixture, I changed the managed successor to retain `runtime-a`, kept the generation-1 `SessionCheckpointTombstone`, and removed only the matching logical-target tombstone. A temporary assertion that `decode_compatible_session_checkpoint` must return `Err(SessionCheckpointRejection::Semantic)` failed with exit 101: the decoder returned `Ok`. The returned state retained the session tombstone but had an empty logical-target tombstone collection and no generation-1 logical reverse owner. Full replay of the managed promotion retains both projections and that reverse reservation.

This is the unclosed “vice versa” half of the requested symmetric hydration contract. It creates restart/full-replay divergence and permits the exact tombstoned native reference to become available to another logical target after checkpoint recovery. The existing changed-runtime test does not expose it because its legacy live-session key differs and therefore happens to reject.

**Required direction:** classify a session tombstone as legacy only when no logical-target-owned lineage/current slot identifies it as managed, including the same-runtime N→N+1 case where the missing logical tombstone erased the exact prior reverse entry. Add a regression that removes only the logical-target tombstone from an otherwise valid same-runtime managed checkpoint, proves decode rejection and deterministic LSN-0 recovery, and compares both tombstone projections plus reverse ownership with full replay. Preserve true session-only legacy lineages where no logical target owns the lineage.

## Mutation matrix

All temporary edits were made one at a time on the main tree and reverted with `git restore` before the full suite.

| Probe / mutant | Focused oracle | Result |
|---|---|---|
| Committed missing-session-tombstone regression | `promotion_checkpoint_retains_changed_runtime_tombstone_and_reverse_reservation`; `complete_checkpoint_round_trips_tombstones_source_cursor_and_tail` | **PASS** on the clean tree; checkpoint decode rejects and the server regression proves checkpoint rejection, LSN-0 replay, projection convergence, and tombstoned late-target classification. |
| Remove the new inverse logical-target → session validation call | same two focused tests | **Killed**; both exited 101 at their expected-rejection assertions. |
| Fresh mutant: suppress the managed-owner branch and force the legacy fallback | changed-runtime promotion checkpoint test | **Killed**, exit 101; the valid changed-runtime checkpoint was rejected. |
| Reviewer probe: same-runtime managed checkpoint retains the session tombstone but omits only the logical-target tombstone | temporary expected-rejection assertion in the promotion checkpoint fixture | **Survived implementation**; decoder returned `Ok`, and the probe exited 101. This is the MATERIAL finding. |
| Exact projected pre-state / ordering regression | `promotion_append_binds_exact_generation_prestate_and_rejects_double_or_out_of_order` | **PASS**. |
| Tombstone and reverse-reservation regression | `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion` | **PASS**. |
| Quint exact-promotion scenarios | all six `session_generation_promotion` runs | **PASS**. |
| Quint independent exact-and-atomic oracle | `promotion_fold_exact_and_atomic`, Apalache through 10 steps | **PASS**. |

## Artifact hygiene

`specs/seed/session_generation.emitted.tla` is clean: no checker diagnostics or absolute paths, and its module is `session_generation_promotion`. A fresh Quint 0.32.0 compile using `--main session_generation_promotion --target tlaplus --out ... --verbosity 0` produced a 585-line body byte-for-byte equal to the committed artifact after its three-line generated header.

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38).
- Final `git diff --check`: **PASS**; the tree was clean before this review file was written.

## Recommendation

**Return to implementing.** Close the same-runtime session-tombstone → logical-target-tombstone hydration gap and add the missing rejection/fallback/full-replay regression. The monotonic promotion fold, reverse-reservation behavior under normal replay, formal checks, generated artifact, and repository-wide suites otherwise remain green.
