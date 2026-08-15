---
id: generation-monotonicity-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-generation-monotonicity-tombstoning
created: 2026-08-15
updated: 2026-08-15
---

# Re-review pass 3: promotion fold, exact monotonicity, and tombstones

## Verdict

**MATERIAL** — return the story to `implementing`.

The two committed same-runtime asymmetry regressions now fail closed, their discriminator mutants are killed, complete managed hydration retains reverse owners for N and N+1, and the promotion/formal regression set remains green. The new current-slot heuristic is nevertheless neither sound nor complete: it rejects a replay-reachable legacy lineage after explicit current assignment, while a changed-runtime managed lineage can still lose its logical tombstone and pass as legacy when the retired native slot has since been reused.

## Findings

### MATERIAL — current-slot inference rejects replay-reachable legacy history after explicit assignment

**Location:** `core/src/session/registry.rs:1283-1299`

The fallback added in round 2 treats any logical target whose current runtime shares the session tombstone's adapter/deployment/runtime slot as proof that the tombstone came from a managed promotion. That is not necessarily true. The explicit initial-current path can adopt a pre-provisioned or discovered runtime after it already accumulated ordinary session-generation history.

Reviewer probe built and hot-folded this valid four-event prefix:

1. legacy `SessionRegistered(runtime-a, generation 1)`;
2. legacy `SessionGenerationBumped(runtime-a, 1→2)`, retaining the session tombstone;
3. `LogicalTargetCreated(target-a)`;
4. `LogicalTargetInitialCurrentAssigned(target-a, runtime-a generation 2)`.

Hot replay accepted the complete prefix. Hydrating its exact sessions, session tombstone, and logical-target checkpoint records at LSN 4 returned `CorruptRecord("session checkpoint tombstone has no later current generation in its legacy runtime slot or managed logical-target lineage")`. The temporary equality/compatibility oracle exited 101. This is a genuine replay/checkpoint divergence and a false rejection of real legacy state, not a speculative malformed shape.

### MATERIAL — changed-runtime managed history can hide a missing logical tombstone behind a reused legacy slot

**Location:** `core/src/session/registry.rs:1283-1336`

The inverse escape remains possible because managed detection uses only exact retained ownership or a current same-runtime slot. Starting from a valid changed-runtime managed promotion (`runtime-a@1 → runtime-b@2`), the reviewer added a later valid live session in the retired `runtime-a` slot at generation 2. The complete checkpoint—with both managed tombstones—hydrated and owned both N and N+1. Removing only the managed logical-target tombstone then erased exact prior ownership; the managed current was in `runtime-b`, so the scan found no same-runtime marker; and the later `runtime-a@2` session satisfied the legacy fallback. Hydration returned `Ok`, dropping the logical reverse owner for `runtime-a@1` relative to full replay. The temporary expected-rejection oracle exited 101.

Together the two probes show that current checkpoint shape cannot reliably infer lineage provenance from slot coincidence. The durable/private checkpoint projection needs an explicit managed-vs-legacy tombstone discriminator (preferably the managed logical-target identity/provenance derived from the promotion event). Old ambiguous checkpoints can remain disposable and fall back to replay; freshly replayed legacy generation bumps must remain unmarked, while promotion-created tombstones must require their exact logical counterpart.

## Mutation matrix

All temporary edits were applied one at a time on the main tree and reverted with `git restore`. The tree was clean before the full suite and before this review file was written.

| Probe / mutant | Focused oracle | Result |
|---|---|---|
| Clean round-2 same-runtime session-without-logical asymmetry | `same_runtime_managed_checkpoint_rejects_missing_logical_tombstone` | **PASS**; semantic rejection, with the complete fixture asserting reverse ownership for N and N+1. |
| Clean round-2 same-runtime logical-without-session asymmetry plus bare session-only legacy | `same_runtime_managed_checkpoint_rejects_missing_session_tombstone_but_legacy_hydrates` | **PASS**; managed asymmetry rejects and the no-managed-marker legacy fixture hydrates. |
| Remove the current same-runtime-slot discriminator | missing-logical regression above | **Killed**, exit 101 at the expected managed-lineage rejection. |
| Suppress the retained-owner managed branch | `snapshot::tests::promotion_checkpoint_retains_changed_runtime_tombstone_and_reverse_reservation` | **Killed**, exit 101; the valid changed-runtime checkpoint was rejected. |
| Fresh replay-reachable legacy-adoption probe | temporary four-event replay → exact checkpoint hydration | **Implementation failed the compatibility oracle**, exit 101; hot replay succeeded but hydration rejected. MATERIAL finding 1. |
| Fresh mixed-lineage craft probe | valid changed-runtime managed checkpoint + later old-slot live lineage, then remove only logical tombstone | **Survived implementation**, causing the expected-rejection probe to exit 101; hydration returned `Ok`. MATERIAL finding 2. |
| Deterministic disposable-checkpoint recovery | `checkpoint::tests::complete_checkpoint_round_trips_tombstones_source_cursor_and_tail` | **PASS**; semantic rejection uses LSN-0 replay and converges sessions, tombstones, logical projection, and late-target classification with full replay. |
| Exact promotion pre-state/order and prior reverse retention | `promotion_append_binds_exact_generation_prestate_and_rejects_double_or_out_of_order`; `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion` | **PASS**. |
| Quint exact-promotion scenarios and independent oracle | six named `session_generation_promotion` runs; `promotion_fold_exact_and_atomic` through 10 steps | **PASS**. |

## Formal and artifact checks

- `./formal/run-model-checks.sh`: **PASS**, 20/20.
- All six exact-promotion Quint scenarios: **PASS**.
- `promotion_fold_exact_and_atomic`, Apalache through 10 steps: **PASS**.
- Fresh quiet Quint 0.32.0 TLA+ compile produced a 585-line body byte-for-byte equal to `specs/seed/session_generation.emitted.tla` after its three-line header. The committed artifact contains no checker diagnostics or absolute output path.

## Full clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 killed mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38).
- Final `git diff --check`: **PASS**.

## Recommendation

**Return to implementing.** Replace slot-coincidence lineage inference with explicit managed tombstone provenance, reject either missing counterpart for promotion-created tombstones, preserve replay-reachable legacy generation history even after explicit logical-target assignment, and add both reviewer craft shapes as permanent replay/checkpoint regressions before the next thorough pass.
