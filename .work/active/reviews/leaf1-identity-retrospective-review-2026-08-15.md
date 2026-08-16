---
id: leaf1-identity-retrospective-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-logical-target-identity-contract
created: 2026-08-15
updated: 2026-08-15
---

# Retrospective deep review — Leaf 1 logical-target identity

## Retrospective status

This is the retained retrospective artifact requested after the integrated feature review found that the original independent Leaf 1 review had not been preserved. It judges the landed implementation (`7e95273` plus review fix `62ed1cb`, closed by `c41a392`) against the current tree at `090e0bd`. Ten later reviewed spawn units consume and extend this identity spine; that downstream convergence is context, not proof of Leaf 1 correctness.

Review mode: independent fresh-context, two-phase completeness then adversarial mutation review. Mutations were made one at a time on the main tree, exercised only with focused tests, and reverted with `git restore`. No temporary worktree was created. `/` began with 55G free. The tracked tree was clean before the first mutation, after every Leaf 1 restore, before the final standard groups, and after those groups.

## Verdict

**CLEAN.** Every behavioral acceptance row has non-vacuous executable evidence on the current tree, all four requested invariant mutations were killed, and the four standard clean-tree verification groups passed. No current-cycle finding or concrete fix is required.

## Completeness review

| Acceptance obligation | Evidence and disposition |
|---|---|
| Distinct generated logical-target and external-runtime id spaces | **PASS.** `contracts/proto/patchbay/common.proto:54-71` defines three separate messages: `LogicalTargetId`, `ExternalRuntimeRef`, and their typed `RuntimeGenerationRef` composition. Generated Rust and TypeScript expose separate named message types; contract drift and both language builds pass. Current Rust/TypeScript consumers construct the nested typed shape rather than a flattened string tuple. |
| Fresh managed generation is positive | **PASS.** `core/src/session/logical_target.rs:830-851` rejects missing/zero generation before reservation. `core/tests/logical_target_identity.rs:58-86` checks exact error/non-mutation with generation 0 and other malformed dimensions; `core/tests/logical_target_proptest.rs:81-101` ranges the remaining identity dimensions. `Generation` is protobuf `uint64`/Rust `u64`, so a negative value is unrepresentable; removing the zero half of the non-positive guard was killed. |
| Stable logical identity across replacement; adapter/deployment are immutable target scope | **PASS.** `core/src/session/logical_target.rs:268-430` preserves the record key and requires exact prior/current/candidate references. `slot_transitions_are_exact_and_tombstones_retain_ownership` asserts replacement keeps the same `LogicalTargetId`; `cross_adapter_scope_and_runtime_ref_mismatches_are_non_mutating` rejects adapter/deployment mutation. Existing session property/regression tests at `core/tests/sessions_proptest.rs:724` and `core/tests/sessions_registry.rs:1550` independently prove project/cwd/name/model changes preserve runtime identity. The private logical-target checkpoint shape at `contracts/proto/patchbay/sessions.proto:320-335` contains none of those metadata fields. |
| Current, reserved-candidate, and tombstone slots are mutually constrained and replay-identical | **PASS.** `core/tests/logical_target_identity.rs:88-139` proves exact current→tombstone and reserved→current movement; `:191-341` exercises illegal empty/current/candidate/retired transitions with exact errors and whole-registry non-mutation; `:510-548` reconstructs current+tombstone+reserved state exactly from checkpoint. `server/src/checkpoint.rs:562` round-trips the production writer through both recovered session consumers. |
| One exact external runtime has at most one logical owner, including current/reserved/tombstoned state and recovery | **PASS.** `ExternalRuntimeKey` and `external_runtime_key` include `(authority_domain_id, adapter_id, deployment_scope, runtime_session_id, generation)` at `core/src/session/logical_target.rs:27-34,774-800`. `core/tests/logical_target_proptest.rs:40-78` proves one-owner behavior hot and after checkpoint recovery; `:143-189` independently distinguishes every key dimension and exposes each omission mutant. `core/tests/logical_target_identity.rs:409-478` proves hot/restart duplicate rejection, and `server/src/checkpoint.rs:562` proves production checkpoint recovery retains current and candidate ownership. Tombstone ownership is asserted directly at `core/tests/logical_target_identity.rs:123-136`. The canonical error remains `duplicate-native-reference`. |
| Leaf 1 does not import downstream claim/evidence contracts or accept an Operation | **PASS for the landed leaf boundary.** The original `7e95273:core/src/session/logical_target.rs:1-15` explicitly depended only on generated identity/projection types and stated that it had no Operation, claim, evidence, target-resolution, or authority dependency. The historical `sessions.proto` identity mutations likewise carried only logical target, adapter/deployment, and external runtime fields. Their hot/replay tests fold `SessionStateEvent` identity mutations directly; no acceptance API is introduced. The current file also hosts a later-added read-only reconciled fence adapter for downstream units, but `LogicalTargetRegistry` remains an identity-only projection and imports no Operation acceptance type. No Leaf 1 event or method accepts an `Operation`. |
| Project/cwd/name/model are metadata, never identity | **PASS.** Besides the generated checkpoint omission above, `relabel_preserves_identity` changes project/cwd/name and retains the exact session identity, while `model_change_preserves_identity_and_rejects_mismatched_prior_value` changes model and retains it. The logical-target transition APIs have no metadata parameter from which routing identity could be rewritten. |

No acceptance oracle was tautological: the transition tests assert externally visible slots, exact errors, reverse ownership, and full pre/post registry equality; replay/checkpoint tests rebuild through production folds; and the reverse-index dimension oracle compares independently varied tuples rather than restating the production equality predicate.

## Findings

None.

## Adversarial mutation matrix

| Injected mutation | Focused oracle | Result |
|---|---|---|
| Accept generation 0 by weakening `validate_external_ref` to reject only a missing generation | `cargo test -p patchbay-core --test logical_target_proptest generation_zero_never_reserves` | **KILLED**, exit 101: mutation returned `Ok(())` instead of `NonPositiveGeneration`; the generated proptest regression artifact was removed and source restored. Negative generation remains structurally unrepresentable by protobuf `uint64`/Rust `u64`. |
| Accept a duplicate external-runtime owner by removing the existing-owner branch from `reserve_external` | `cargo test -p patchbay-core --test logical_target_identity slot_transitions_are_exact_and_tombstones_retain_ownership` | **KILLED**, exit 101: the second logical target returned `Ok(())` instead of `DuplicateNativeReference`. |
| Drop the prior runtime's reverse reservation when moving current to a tombstone | same `slot_transitions_are_exact_and_tombstones_retain_ownership` test | **KILLED**, exit 101: `owner_of(prior)` became `None` instead of the original logical target. |
| Confuse reserved/current slots by writing `reserve_candidate` into `current` | `cargo test -p patchbay-core --test logical_target_identity release_removes_only_the_candidate_reservation` | **KILLED**, exit 101: exact candidate release failed with `ReservedCandidateMismatch`. |

After each run, `git restore --worktree core/src/session/logical_target.rs` restored the production file and `git diff --quiet -- core/src/session/logical_target.rs` confirmed it matched `HEAD`.

## Final clean-tree verification

All required groups ran after the last mutation restore:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**, including 10 logical-target integration tests, 3 logical-target property tests, production checkpoint/restart tests, all workspace tests/doctests, and warnings-denied clippy.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**: generated Rust/TypeScript bindings drift-clean; 57 vectors, 17 promoted vectors, 26 implementation checks, 38 mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 27/27.
4. `cd pi-adapter && npm test` — **PASS**, 38/38, including the real core/AgentSession generation-bump, reconnect, and restart loop.

Final `git status --short`, `git diff --check`, and `git diff --exit-code` were clean before this retained artifact was written.
