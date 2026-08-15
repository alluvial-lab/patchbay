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
