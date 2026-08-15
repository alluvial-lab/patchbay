---
id: generation-monotonicity-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-generation-monotonicity-tombstoning
created: 2026-08-15
updated: 2026-08-15
---

# Review: promotion fold, exact monotonicity, and tombstones

## Verdict

**MATERIAL** — return the story to `implementing`.

The ordered authority → session → claim → command fold, dedicated transactional append, exact `∅→1` / `N→N+1` guards, reverse-index retention, and genuine Quint oracle all survived direct review and mutation. One checkpoint compatibility gap still permits restart state to diverge from full replay by dropping the session tombstone half of a managed promotion. The committed Quint inspection artifact also needs non-blocking cleanup.

## Findings

### MATERIAL — checkpoint hydration accepts a missing managed session tombstone

**Location:** `core/src/session/registry.rs:164`

`from_checkpoint_with_logical_targets` validates in only one direction: every supplied `SessionTombstone` must have a current successor and, for a changed native runtime id, an exact matching logical-target tombstone. It never performs the inverse check that every retained managed logical-target tombstone has the exact corresponding `SessionTombstone` with the same external identity and superseding LSN.

Reviewer probe: starting from the valid changed-runtime promotion checkpoint in `promotion_checkpoint_retains_changed_runtime_tombstone_and_reverse_reservation`, clearing only `StoredSessionCheckpoint.tombstones` left the logical-target current/tombstone/reverse index intact, and `decode_compatible_session_checkpoint` still returned `Ok`. The temporary assertion that this semantically incomplete checkpoint must reject failed with exit 101. Full log replay retains both tombstone projections, while this accepted checkpoint resumes with `SessionRegistry::tombstones()` empty. The adapter recovery path consumes this registry, and `classify_runtime_target` consults the session tombstone map rather than the logical-target map, so exact late-generation classification/audit context can also differ after restart.

This violates the story's required checkpoint/replay equality and indefinite tombstone retention. The retained logical reverse index still fences the separately logical-target-aware report classifier, so the probe did not demonstrate publication of a stale generation; the blocker is the proven recovery divergence and lost session audit fact.

**Concrete fix:** after constructing both checkpoint projections, require every logical-target tombstone to have one exact `SessionTombstone` counterpart (identity plus `superseded_at_lsn`), while preserving legacy session-only lineages where no logical target exists. Add a regression that removes the outer session tombstone from an otherwise valid changed-runtime managed checkpoint and proves decode rejects and `recover_session_registry` falls back to full replay; compare both tombstone collections and late-target classification with full replay.

### NIT — the generated Quint inspection artifact is polluted and omits the reviewed module

**Location:** `specs/seed/session_generation.emitted.tla:4`

The artifact contains Apalache console diagnostics, an absolute timestamped output path, and `#` lines before the TLA+ module. It also emits only the default `session_generation` module, not `session_generation_promotion`, so it does not inspect the exact-atomic model added by this story. This does not weaken the actual Quint source/checker result because the file explicitly is not an independent check lane.

**Concrete fix:** regenerate through Quint's `--out` path with `--verbosity 0` and `--main session_generation_promotion`, update the regeneration header accordingly, and keep console diagnostics out of the committed TLA+ text.

## Mutation matrix

All mutants were applied one at a time on the main tree and reverted with `git restore`. Focused clean tests were rerun after restoration, and the tree was clean before the review file was written.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Wrong projected current N fails open | `promotion_append_binds_exact_generation_prestate_and_rejects_double_or_out_of_order` | **Killed**, exit 101 at the wrong-current assertion (`runtime_evidence_promotion.rs:3031`) |
| Projected claim / claimed N+1 mismatch fails open | same focused test | **Killed**, exit 101 at the wrong-claim assertion (`runtime_evidence_promotion.rs:3120`) |
| Double-promotion append reports success instead of the descendant-id/pre-state conflict | same focused test | **Killed**, exit 101 in the zero-write rejection helper (`runtime_evidence_promotion.rs:1954`) |
| Candidate-release/out-of-order promotion reports success with no reserved successor | same focused test | **Killed**, exit 101 in the zero-write rejection helper (`runtime_evidence_promotion.rs:1954`) |
| Promotion removes the prior tombstone's reverse-index reservation | `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion` | **Killed**, exit 101; prior owner became `None` (`runtime_evidence_promotion.rs:2791`) |
| Quint production `foldGuard` weakened to `phase == "pending"` | `wrong_prior_is_inert` scenario | **Killed**, exit 1; expectation failed |
| Same Quint guard mutant | `promotion_fold_exact_and_atomic`, max 4 steps | **Killed**, exit 1; Apalache found a counterexample |
| Managed checkpoint omits only the session tombstone | temporary regression assertion in the existing changed-runtime checkpoint test | **Survived implementation**, causing the MATERIAL finding; decoder returned `Ok` and the expected-rejection probe exited 101 |

Clean formal evidence:

- all six exact promotion scenarios pass when selected explicitly;
- `promotion_fold_exact_and_atomic` passes Apalache through 10 steps;
- the invariant is independent of `foldGuard` and catches the behavior-changing guard mutant;
- the implementation retains one shared `fold_spawn_promotion_ordered`; no parallel promotion fold machinery was found.

## Clean-tree verification

- `cargo build --workspace --all-targets`: **PASS**.
- `cargo test --workspace`: **PASS** on clean-tree retry. The first chained attempt reached the doctest phase and transiently failed to resolve the present `tokio` rlib while another reviewer was using the shared Cargo artifact directory; an immediate unchanged-tree rerun passed every workspace test and doctest.
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (55 vectors, 22 implementation checks, 38 mutation witnesses).
- `cd operator-domain && npm run build && npm test`: **PASS** (23/23).
- `cd pi-adapter && npm test`: **PASS** (38/38).
- Focused restored-tree Rust promotion, tombstone, and checkpoint tests: **PASS**.
- Quint six-scenario run and exact-atomic invariant check: **PASS**.
- Final `git diff --check`: **PASS** before committing this review.

## Recommendation

**Return to implementing.** Add symmetric managed-tombstone checkpoint validation plus the fallback/full-replay regression, regenerate the non-authoritative Quint inspection artifact, rerun the clean-tree suite, and submit the security-critical story for the next thorough review pass.
