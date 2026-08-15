---
id: research-handoff-spawn-idempotency-duplicate-handling
kind: story
stage: review
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-registration, research-handoff-spawn-crash-external-effect-evidence-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Duplicate, ambiguous-outcome, and claim reconciliation

## Redesign disposition

Rewritten. The previous “failed/cancelled/expired may retry after terminal” rule is superseded. Only durable proof of no external effect releases a claim.

## Checkpoint

Preserve exact Patchbay boundary retry while making external-effect ambiguity claim-poisoning. Exact retry returns the existing Operation, compound provenance, and claim. A distinct command/key cannot reuse an active, poisoned, promoted, or abandoned generation.

Consume typed execution/crash evidence:
- `proved_none` may durably release after proof validation;
- `may_exist` poisons and retains the delivery fence;
- `identified` reserves/reconciles the external runtime to the original claim;
- absent/contradictory evidence fails toward poison, never release.

A poisoned claim ends only through exact runtime reconciliation and promotion, later closed-vocabulary no-effect proof, or operator target abandonment. No automatic relaunch occurs.

## Design

**Files**
- `core/src/acceptance/{pipeline,index}.rs`, `core/src/session/spawn_claim.rs` — retry/claim reconciliation projection.
- `server/src/adapter_service.rs` — redelivery suppression and reconciliation ingress.
- Adapter-facing execution-evidence port; Pi implementation remains in its downstream redesign.
- Duplicate, crash, cancellation/expiry, reconciliation, and abandonment vectors/tests.

`execution_outcome_unknown`, delivered cancellation/expiry, launch-attempted loss, and unexplained stream loss transition claim state to poisoned regardless of terminal command state. Adapter `idempotency_strength` informs operator retry presentation but cannot override core claim exclusivity.

## Acceptance evidence

- [x] Exact command/key/target/payload retry returns the original state/claim; changed payload rejects.
- [x] Failed/cancelled/expired alone does not release; delivered cancellation/expiry and outcome unknown poison.
- [x] Valid no-effect proof references the exact claim/phase/source and permits one durable release.
- [x] An identified runtime reconciles only to its original logical target/claim and cannot collide in the reverse index.
- [x] Poison survives core/adapter restart and blocks delivery/reclaim until reconciliation or abandonment.
- [x] Journal/store unavailable or corrupt cannot silently execute/release.
- [x] Mutations release-on-terminal, relaunch-on-unknown, or ignore reverse ownership fail.

## Ordering constraint

Consumes atomic claim, staged identity reservation, and the crash/effect evidence contract. Completion requires this reconciliation behavior.

## Implementation notes

- Added one dedicated storage writer for `SpawnExecutionEvidence` reconciliation. The SQLite writer rebuilds the exact durable command/claim/session prefix inside its single-writer transaction, rejects malformed/current-attachment and reverse-ownership conflicts before writes, reuses the first byte-exact evidence event on retry, and atomically appends the resulting poison or no-effect release. Generic storage routes now reject this special event kind.
- Extended `SpawnClaimRegistry` with claim-to-identified-runtime and exact-external-runtime-to-claim indexes. Valid identified evidence and staged successors reserve the same original claim; conflicts cannot append or manufacture a replacement. Promotion still consumes the original active/poisoned claim. Only a proved-none release clears that claim's identified-runtime reservation.
- Extended `CommandIndex` so every durable spawn execution outcome suppresses re-offering the original command after hot catch-up or restart. Claim poison/abandonment also suppresses delivery; released generation availability is usable only by a distinct accepted command/key.
- Authenticated evidence ingress now uses the dedicated writer. Delivered cancellation, expiry, and `execution_outcome_unknown` Result observations first durably poison the exact claim. Abnormal current delivery-stream loss writes conservative `may_exist` evidence before ordinary running-command failure and retains the exact generation fence. Exact evidence redelivery returns the original receipt without a second source or disposition.
- `proved_none` releases only through the generated three-variant `NoExternalEffectProof` validator. Continuations retain their claim after proof until a durable exact prior-N live event occurs later; an exact evidence retry then performs the once-only release. If an identified staged candidate is released, candidate release and claim release commit in the same transaction.
- Added an executable draft conformance vector and refreshed generated traceability in `docs/VERIFICATION.md`. No protobuf or generated binding changed, and `core/src/session/registry.rs` generation/tombstone internals were not modified by this unit.
- Tests cover exact ambiguity/proved-none retry, restart replay and redelivery suppression, delayed continuation liveness release, invalid-evidence zero-write behavior, identified-runtime reverse collisions, authenticated evidence canonicalization, delivered ambiguous Results, and abnormal stream loss.
- Discrepancies from design: none. Adjacent issues parked: none.

### Fix round — 2026-08-15 thorough review findings

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read fix round because the authoritative review named the three bounded reconciliation gaps and exact ownership surfaces. Review weight remains `thorough` from the autopilot caller; a fresh independent re-review follows.
- Effect-before-ack Results: authenticated, exactly correlated ambiguous spawn Results now admit `accepted` as well as `delivered`/`running` pre-state. The dedicated evidence write therefore poisons the exact claim before ordinary terminalization even when the delivery acknowledgement was lost. Focused server coverage exercises cancellation, expiry, and `execution_outcome_unknown` both with and without the acknowledgement.
- Identified-success staging: the SQLite writer now derives the claim consequence from phase, failure, and disposition. Identified progress with no failure at `external_identity_known`, `handshake_reconciling`, or `success_evidence_reported` reserves the exact runtime while leaving the claim active; identified failure evidence and `may_exist` ambiguity still poison. The chosen phase split follows the authoritative failure-phase table: known-identity progress is active absent failure, whereas launch ambiguity does not gain success semantics merely from carrying an identity.
- Logical-target ownership: before observing any new identified evidence in the claim projection, the same writer transaction consults the authoritative logical-target reverse index. A current, reserved-candidate, or tombstoned runtime owned by another logical target returns `DuplicateNativeReference` with zero writes; the original target's exact evidence retry remains valid. File-backed hot/restart tests cover all three owner slots.
- Promotion continuity: successful identified evidence is durable and byte-idempotent across restart, remains `active`, accepts the matching staged successor for the original claim, and reaches the ordinary atomic promotion path.
- Files changed: `server/src/adapter_service.rs`, `server/src/adapter_service/tests.rs`, `core/src/storage/rusqlite.rs`, `core/tests/runtime_evidence_promotion.rs`, and this story file. No protobuf, generated binding, `core/src/session/registry.rs`, or `specs/seed/` file was changed by this fix round.
- Tests added/expanded: one six-cell effect-before-ack/acknowledged Result oracle; successful identified evidence retry/replay/staging/promotion; logical-target current/reserved/tombstone collision oracles across file restart. Simplification: one shared server case helper and shared storage-fixture helpers avoid duplicate scenario bodies. Discrepancies from design: none. Adjacent issues parked: none.

#### Fix-round mutation evidence

Each mutant was applied alone, its focused oracle failed with exit 101, and its file was restored before the next probe:

- remove `accepted` from ambiguous-Result poisoning — killed by `ambiguous_spawn_results_with_or_without_ack_poison_the_exact_claim` (`Cancelled`, no acknowledgement remained `Active`);
- route `success_evidence_reported + identified + unspecified` back into poison/rejection — killed by `successful_identified_evidence_stays_active_and_reaches_original_claim_promotion`;
- remove the logical-target reverse-owner consultation — killed by `identified_evidence_respects_current_and_reserved_logical_target_owners_after_restart`.

The four pass-1 mutants were re-confirmed killed: release on terminal; infer no effect from terminal state/ack silence; re-offer a claim after execution evidence; and ignore the claim-level external-runtime reverse owner. The restored `spawn_claim_registry` suite passed 38/38.

#### Fix-round verification evidence

- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**, including 38 spawn-claim tests, 31 runtime-evidence/promotion tests, 77 server unit tests, all workspace integration/property tests, doctests, and warnings-denied clippy.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 55 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart e2e.
- `cargo fmt --all -- --check` and `git diff --check`: **PASS**.

### Fix round 2 — 2026-08-15 identified-launch and running-oracle findings

- Execution capability: `openai-codex/gpt-5.6-sol` at `xhigh` thinking.
- Consolidated the storage write-path decision on `execution_evidence_poisons_claim`: `LaunchAttempted + Identified` now always poisons the exact claim generation, including `failure_code = unspecified`; identified evidence in later phases poisons only when it carries failure evidence, so identified success remains promotable.
- Added a storage/replay consequence matrix over every allowed phase/disposition row. Each case proves disposition, exact continuation-fence retention, identified-runtime ownership binding when applicable, competing-owner suppression, and cold-replay equivalence.
- Expanded the real adapter-service ambiguous-Result oracle to start independently from `Accepted`, `Delivered`, and actual `Running` command prestates, then prove terminal failure, exact-claim poison, retained fence, and hot/restart delivery suppression.
- No production change was needed in `server/src/adapter_service.rs`: `Running` was already eligible; the missing protection was a `Running`-sensitive oracle.
- Files changed: `core/src/session/spawn_claim.rs`, `core/src/storage/rusqlite.rs`, `core/tests/spawn_claim_registry.rs`, and `server/src/adapter_service/tests.rs`.
- Design discrepancies: none. The implementation makes the existing phase-aware design explicit without broadening failure codes or adapter semantics.

#### Fix-round-2 mutation evidence

Both newly requested mutants were killed, then restored:

1. Restoring the old evidence-only poison predicate (`failure_code != unspecified` or `execution_outcome_unknown`) failed `storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row` on the `launch_attempted / identified / unspecified` row (`cargo test -p patchbay-core --test spawn_claim_registry ...`, exit `101`).
2. Removing `Running` from ambiguous-Result poison eligibility failed `ambiguous_spawn_results_with_or_without_ack_poison_the_exact_claim` from the actual-running prestate (`cargo test -p patchbay-core-server --lib ...`, exit `101`).

All seven prior pass-2 mutations were reconfirmed as killed (exit `101` for each focused oracle):

1. remove `Accepted` from server poisoning eligibility;
2. poison identified success evidence;
3. bypass logical-target ownership validation;
4. release claims on terminal command state;
5. release from terminal-state silence;
6. suppress typed-effect poisoning;
7. omit identified-runtime claim-owner binding.

Every mutation was restored with `git restore`; no mutant remained in the final tree.

#### Fix-round-2 verification

All four requested verification groups passed:

```text
cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build
cd operator-domain && npm run build && npm test  # 23 passed
cd pi-adapter && npm test                         # 38 passed
```

Additional final checks passed: `cargo fmt --all -- --check`, `git diff --check`, the 39-test spawn-claim registry target, the 31-test runtime-evidence promotion target, and the 77-test server library target. Contract drift reported no generated changes; no schema or generated binding was edited.

## Mutation evidence

Three required claim-breaking probes were applied one at a time, each focused oracle failed with exit 101, each probe was reverted, and the restored `spawn_claim_registry` suite passed 38/38:

- release claims directly from failed/cancelled/expired `CommandTransition` — killed by `terminal_command_states_never_release_or_clear_the_fence_kills_release_mutant`;
- omit spawn execution evidence from delivery suppression (relaunch on unknown) — killed by `reconciled_ambiguity_poisons_once_survives_restart_and_suppresses_relaunch`;
- ignore the external-runtime reverse-owner check — killed by `identified_runtime_is_reserved_to_its_original_claim_at_ingress`.

## Verification evidence

- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** on 2026-08-15, including 38 spawn-claim tests, 77 server unit tests, all workspace integration/property tests, doctests, and warnings-denied clippy.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; generated bindings clean, 55 vectors, 17 promoted vectors, 22 implementation checks, and 38 registered mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart e2e.
- `cargo fmt --all -- --check` and `git diff --check`: **PASS**.

### Fix round 3 — 2026-08-15 exact lost-stream offer evidence

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read implementation over the one review-confirmed disconnect race. Review weight: `thorough`, supplied by the autopilot caller.
- The delivery tail now records the exact managed-spawn claim ids it actually emits. Abnormal-loss reconciliation poisons durable `Delivered`/`Running` claims and `Accepted` claims present in that exact lost-stream offer set; an `Accepted` claim that arrived only after the stream disappeared remains active with its continuation fence retained.
- The barrier-controlled regression drops an empty stream while holding the shared decision gate, accepts the claim only after the drop, waits for disconnect reconciliation to finish, and proves active disposition, zero execution-evidence events, retained fence, and delivery on a replacement stream. The existing accepted-and-actually-offered case remains poisoned and redelivery-suppressed.
- `core/src/storage/rusqlite.rs` was inspected but needed no final change: the writer already rebuilds and validates the exact durable claim inside its transaction. Exact per-stream offer evidence exists at the delivery boundary, so excluding never-offered claims before invoking that writer avoids manufacturing durable ambiguity while preserving its atomic checks.
- Files changed: `server/src/adapter_service.rs`, `server/src/adapter_service/tests.rs`, and this story file. No protobuf, generated binding, `core/src/session/registry.rs`, or foundation document was changed by this round.
- Simplification: one per-tail `HashSet<CommandId>` is consumed by the existing disconnect callback; no new protocol state, persistence record, or generalized tracking layer was introduced. Design discrepancies: none. Adjacent issues parked: none.

#### Fix-round-3 mutation evidence

Every mutant was applied alone, its focused oracle failed with exit 101, and its file was restored before the next probe. The new regression killed unconditional `Accepted` disconnect poisoning at `left: PoisonedPendingReconciliation, right: Active`.

The 12 prior kills were also reconfirmed: old failure-only identified-launch poisoning; removal of `Running` and removal of `Accepted` from ambiguous-Result eligibility; false poison of handshake progress and successful identified progress; omission of identified-runtime replay reservation; bypass of logical-target and claim-level external-runtime ownership; release on terminal state and on terminal-state silence; omission of execution-evidence delivery suppression; and clearing the continuation fence on poison. The effect-before-ack and actual-`Running` oracles remained non-vacuous.

#### Fix-round-3 verification

- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**, including 39 spawn-claim tests, 31 runtime-evidence/promotion tests, 78 server unit tests, all workspace integration/property tests, doctests, and warnings-denied clippy.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 55 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart e2e.
- `cargo fmt --all -- --check`, `git diff --check`, generated-contract diff, and final focused abnormal-stream-loss regressions: **PASS**.

### Fix round 4 — 2026-08-15 durable offered-claim reconstruction

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read implementation for the single pass-4 MATERIAL. Review weight remains `thorough` from the autopilot caller; a fresh independent pass-5 review follows.
- Chosen architecture: durable reconstruction from the existing audited authority-domain prefix, not a protobuf change. Immediately before a managed spawn crosses the delivery stream boundary, `DeliveryTail` commits one canonical `CommandDelivered` audit marker (`managed_spawn_delivery_offered`) linked to the exact accepted claim event. The marker shares the decision gate and stream epoch, so an obsolete stream cannot commit an offer and a delivery cannot be yielded until its marker is durable.
- `CommandIndex` now folds those exact markers into a replay-reconstructed managed-spawn offer set. The set suppresses a second delivery while the command remains `Accepted`, and startup/replacement-stream reconciliation maps an offered active claim to the existing `Offered / MayExist / ExecutionOutcomeUnknown` evidence path. An accepted claim with no marker remains active and deliverable. This preserves ordinary delivered-command retry semantics outside the managed-spawn lane.
- Added `offered_without_ack_survives_core_restart_as_ambiguous_without_redelivery`: yield one managed claim, suppress the in-process Drop callback to model abrupt core death, rebuild from the unchanged prefix, prove the offer set is identical, prove the claim is poisoned with its fence retained and the exact ambiguous evidence row, reattach generation 2, and prove no redelivery. The prior accepted-after-drop and same-process offered-before-drop regressions remain green.
- Updated the delivery/continuation barrier oracle to distinguish an actually offered replacement from a never-offered replacement: only the latter remains reconstructible for delivery after replay.
- Files changed: `core/src/acceptance/{index,mod}.rs`, `server/src/adapter_service{,/tests}.rs`, and this story file. No protobuf, generated binding, foundation document, or other `.work/` item changed. Design discrepancy: none. Rationale: the audited marker is the smallest durable-log projection consistent with existing replay architecture and avoids a second protocol state family.

#### Fix-round-4 mutation evidence

Every mutant was applied alone on the main tree, exercised by one focused oracle, and restored with `git restore`; the tree was clean after each.

- New replay-reconstruction mutant: skipping `AuditRecord -> managed offer` folding was killed by `offered_without_ack_survives_core_restart_as_ambiguous_without_redelivery` (exit 101 before delivery because the committed marker did not enter the projection).
- All 13 prior mutants were re-confirmed killed (exit 101): unconditional `Accepted` disconnect poisoning; old failure-only identified-launch poisoning; removal of `Running` and removal of `Accepted` from ambiguous-Result eligibility; false poison of handshake progress and successful identified progress; omission of identified-runtime replay reservation; bypass of logical-target and claim-level external-runtime ownership; release on terminal state and admission of terminal-state silence as no-effect proof; omission of execution-evidence delivery suppression; and clearing the continuation fence on poison.

#### Fix-round-4 verification

- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**, including 39 spawn-claim tests, 31 runtime-evidence/promotion tests, 79 server unit tests, all workspace integration/property tests and doctests, and warnings-denied clippy.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; generated paths clean, 55 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart e2e.
- `cargo fmt --all -- --check`, `git diff --check`, generated-contract diff, and final clean-tree status: **PASS**.
