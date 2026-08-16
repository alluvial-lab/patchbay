---
id: leaf5-crash-evidence-retrospective-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-crash-external-effect-evidence-contract
created: 2026-08-15
updated: 2026-08-15
---

# Retrospective deep review — Leaf 5 spawn crash/external-effect evidence

## Verdict

**CLEAN.** The landed Leaf-5 contract is complete at its release/poison boundary and the requested adversarial mutations all fail focused oracles. No blocker, material finding, or nit survived. The story's original implementation (`86d1c9f`), review fixes (`bc7cc9d`), and completion transition (`9d3f7f3`) are consistent with the current landed code.

## Completeness

| Required property | Landed evidence | Verdict |
|---|---|---|
| Typed execution phases and effect dispositions | `contracts/proto/patchbay/adapter_control.proto:47-64` owns both generated enums. `allowed_external_effect_disposition` rejects every cell outside the closed table (`core/src/session/spawn_claim.rs:1644`), and `execution_phase_disposition_table_is_closed_and_complete` enumerates the sentinel and every generated value (`core/tests/spawn_claim_registry.rs:715`). | **PASS** |
| Exact claim correlation | `SpawnExecutionEvidence.exact_claim` carries the complete claim (`adapter_control.proto:85`); replay requires full equality with the durable accepted claim and its authority domain (`core/src/session/spawn_claim.rs:1527`). Authenticated ingress canonicalizes producer/attachment provenance and rejects a wrong claim without append (`server/tests/spawn_execution_evidence.rs:53`). | **PASS** |
| Optional bounded external identity | `MAY_EXIST` may omit identity; `IDENTIFIED` requires it. When present, validation binds logical target, claimed generation, authority domain, and current claim adapter (`core/src/session/spawn_claim.rs:1595-1637,2013`). `identified_external_runtime_is_bounded_to_original_claim` supplies the negative oracle (`core/tests/spawn_claim_registry.rs:914`). | **PASS** |
| Closed no-effect proof vocabulary | `NoExternalEffectProof` has only core pre-delivery terminal, authenticated current-adapter refusal-before-responsibility, and exact supervisor/journal pre-launch variants (`contracts/proto/patchbay/sessions.proto:211-234`). The fold exhaustively validates producer, phase, failure, attachment, delivery responsibility, and the exact durable core transition (`core/src/session/spawn_claim.rs:1685-1841`). All three positive paths are exercised by `all_three_closed_no_effect_proofs_can_release_only_with_typed_evidence` (`core/tests/spawn_claim_registry.rs:1290`). | **PASS** |
| Only typed evidence releases or poisons after acceptance/delivery | `validate_transition_evidence` admits release only through a referenced `PROVED_NONE` execution event and poison only through phase-aware ambiguous/identified evidence. Terminal command state and silence are inert; delivered cancellation/expiry/outcome-unknown poison and retain the fence (`core/tests/spawn_claim_registry.rs:795,817,859,1209,1368`). Promotion and operator abandonment remain separate typed decision families. | **PASS** |
| Acceptance rows have behavioral tests | The closed table test constrains admission, while `storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row` drives every allowed row through the real SQLite writer, hot fold, restart replay, claim consequence, identity reservation, and redelivery suppression (`core/tests/spawn_claim_registry.rs:1832`). Server ingress separately covers authenticated canonicalization, exact retry, and zero-write wrong-claim rejection. | **PASS** |

## Findings

None.

## Mutation matrix

Every mutant was applied directly to the main working tree, exercised by one focused test, and reverted with `git restore`. The OPEN/unknown proof case used a reviewer-only focused probe that represented an unrecognized protobuf oneof arm after decoding as an outer proof with no recognized variant; the probe and mutant were both removed afterward.

| Adversarial mutation | Focused oracle | Result |
|---|---|---|
| Accept an OPEN/unknown `NoExternalEffectProof` arm by treating an empty recognized oneof as valid | Reviewer-only `open_or_unknown_no_effect_proof_variant_cannot_release_claim` | **KILLED** — clean code passed; the permissive fallback mutant failed with exit 101 because the claim released. |
| Treat silence / missing evidence id as proof of no effect | `missing_evidence_event_id_is_silence_not_proof` | **KILLED** — exit 101; the mutant released the active claim. |
| Remove full equality between referenced evidence and the exact accepted claim | `another_claims_typed_evidence_cannot_poison_claim` | **KILLED** — exit 101; another claim's event poisoned the target claim. |
| Downgrade poison-worthy external-effect evidence to `ReleasedNoExternalEffect` in the reconciled writer | `storage_replay_consequence_matrix_commits_every_allowed_phase_disposition_row` | **KILLED** — exit 101 at `offered/may_exist`; the independent release validator rejected the downgrade because the evidence did not prove absence. |

## Clean-tree verification

All required groups passed on the restored tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**, including 39 spawn-claim tests, the server ingress test, workspace integration tests, doctests, and warnings-denied clippy.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 57 vectors, 17 promoted vectors, 26 implementation checks, 38 mutation witnesses, 54 model-promotion blocks, and generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 27/27.
4. `cd pi-adapter && npm test` — **PASS**, 38/38.

The first group-2 attempt found the ignored `operator-domain/dist` prerequisite absent. Group 3 rebuilt that package; the exact unmodified group-2 command then passed. No tracked file changed. `git diff --check` and final pre-report status were clean.

## Retrospective note

This is a fresh retrospective judgment of the landed code, not a reconstruction of the missing 2026-08-13 reviewer output. Units 6 and 9 demonstrate that downstream code consumes the phase/effect vocabulary, but they were treated only as regression context and not as proof of Leaf-5 correctness. The retained evidence above comes from the Leaf-5 generated contracts, claim validation/fold, storage reconciliation path, adapter ingress, acceptance tests, reviewer mutations, and current clean-tree verification.
