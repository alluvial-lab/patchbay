---
id: session-registry-replay-domain-soundness-integration-evidence
kind: story
stage: review
tags: [verification, protocol]
parent: session-registry-replay-domain-soundness
depends_on: [session-registry-replay-domain-soundness-bound-registry-contract]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Session integration and property evidence

## Checkpoint

Exercise the bound registry through the real acceptance target-resolution seam,
attack malformed/conflicting replay records as a table, and broaden the session
property oracle across collision-prone adapter/scope/runtime identities. Keep
production decision-gate serialization as separate composition-root evidence.

## Acceptance evidence

- `acceptance_pipeline` uses a real populated `SessionRegistry` for at least one
  accepted runtime-session Operation and one cross-domain rejection; the latter
  creates no command record even though the grant test double otherwise
  authorizes it.
- Table-driven registry cases cover missing/empty/wrong outer domain, missing
  LSN, inner/outer domain mismatch, missing mutation/identity/state, unknown
  enum values, exact redelivery, conflicting same-LSN payload, unseen older
  owned events, and duplicate registration at a new LSN. Rejected cases leave
  the projection unchanged.
- Ingest tests prove all single- and multi-delta successes warm immediately,
  domain mismatch appends nothing in either domain, partial-append failure
  preserves only the committed prefix, and exact caller redelivery remains
  inert.
- A multi-identity proptest varies adapter id, deployment scope, runtime id,
  and generation using deliberate one-dimension collisions. Its independent
  per-identity oracle checks non-decreasing live generation, retained
  tombstones, hot/rebuilt equality, and that a report cannot mutate another
  identity.
- Mutation evidence catches a faulty identity key that omits an adapter, scope,
  or runtime dimension; no test derives its expected key/equality from the
  production registry helper.
- The existing server regression
  `concurrent_conflicting_model_reports_leave_a_replayable_log` stays green and
  is reported only as evidence for the composition-root `CoreDecisionGate`, not
  as a core-writer safety guarantee.
- Evidence remains implementation-checked. No formal property, conformance
  vector, wire contract, or normative assurance tier is promoted.

## Ordering constraints

Depends on
`session-registry-replay-domain-soundness-bound-registry-contract`; these tests
verify the integrated behavior instead of defining a second replay/domain
predicate. Because this story is tagged `[verification]`, its later story-level
review follows the project deep lane and attacks the equality and identity-key
mutation witnesses before advancing to `done`.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (explicit caller selection for mutation-sensitive protocol evidence); direct-read implementation with no nested delegation.
- Review weight: project-mandated `[verification]` two-phase deep lane for this story, plus caller-selected `thorough` at the later feature boundary.
- Files changed: `core/tests/acceptance_pipeline.rs` and `core/tests/sessions_proptest.rs`; registry malformed/conflict and writer warm-path tables landed with the dependency checkpoint in commit `4d7b3f6` so their evidence stays adjacent to the contract under test.
- Tests added/strengthened: real populated `SessionRegistry` acceptance success and cross-domain `target_not_found`/zero-command append; 100-case multi-identity action sequences over adapter/scope/runtime collision pairs; a test-owned tuple oracle checking only the addressed key, generation monotonicity, retained tombstones, other-key exact stability, and hot/cold registry equality; explicit adapter/scope/runtime omission mutants.
- Simplification: removed property-test caller-managed event warming; properties now exercise the production append-then-fold writer directly and compare against independent cold replay.
- Discrepancies from design: none. The new identity-focused generator keeps state axes fixed to isolate key interference while the retained single-identity generator continues varying connectivity/activity/metadata and invalid-transition paths.
- Adjacent issues parked: none.
- Assurance wording: implementation-checked only; no formal property, model metadata, conformance vector, wire contract, or normative tier was changed or promoted.

## Verification evidence

- `cargo test -p patchbay-core --test acceptance_pipeline` — 25 passed, including real-registry same-domain acceptance and cross-domain no-command rejection.
- `cargo test -p patchbay-core --test sessions_proptest` — 9 passed at 100 configured cases, including the collision-matrix oracle and all three omission-mutant witnesses.
- `cargo test -p patchbay-core --test sessions_registry` — 15 passed; malformed/conflicting cases preserve exact registry equality and exact old-prefix redelivery remains inert.
- `cargo test -p patchbay-core --test sessions_ingest` — 17 passed; single/multi-delta immediate warming, both-domain no-append checks, committed-prefix failure, and hot/cold equality are green.
- `cargo test -p patchbay-core-server concurrent_conflicting_model_reports_leave_a_replayable_log` — pass, retained solely as `CoreDecisionGate` composition-root race evidence.
- Story intentionally stops at `stage: review`; the driver owns the required completeness → adversarial convergence lane before `done`.

## Deep-lane Phase-1 fix — exact-envelope equality

- Adjudication: confirmed an evidence gap, not a production defect. Before this fix, temporary payload-only and decoded-semantic equality mutations both passed the old conflicting-redelivery test; production already compared the complete `StoredEventPayload`.
- `core/tests/sessions_registry.rs` now isolates kind-only changes to another owned kind and to a sibling kind while preserving the original bytes, plus a bytes-only valid Protobuf re-encoding whose unknown field is discarded on decode. Test-owned mutant predicates prove those fixtures fool payload-only and decoded-semantic equality respectively.
- Mutation results: the temporary payload-only production mutation failed the strengthened test on the kind-only owned candidate; the temporary decoded-semantic mutation failed on the semantically equal alternate encoding (both exit 101). Both mutations were reverted, and the production test passed.
- Verification: registry 16/16; ingest 17/17; replay/resolver 9/9; session properties 9/9; acceptance 25/25; named server gate regression passed; `cargo test --workspace`, workspace Clippy with warnings denied, and model/vector checks passed (53 models; 53 vectors, 21 implementation checks, 37 existing vector mutation witnesses).
- Assurance/lifecycle: implementation-checked evidence only; no model/vector/wire promotion. This child and its parent remain at `stage: review` for the continuing deep lane.

## Deep-lane Phase-2 fix — security-lockdown replay ledger

- Adjudication: confirmed a second evidence gap, not a production defect. Before this fix, all three temporary reviewer mutations passed the complete 16-test `sessions_registry` suite (exit 0): omitting `SecurityLockdown` from `applied_events`, treating any same-LSN lockdown envelope as exact, and inserting a lockdown envelope before its fold validated successfully. Production already ledgers the exact envelope only after a successful fold.
- `core/tests/sessions_registry.rs` now asserts the old entry immediately after exit and the old exit immediately after a later entry, preventing the two faulty folds from canceling before comparison. Dedicated cases reject a conflicting same-LSN enter/exit envelope without mutation and prove a missing-transition lockdown event neither mutates nor poisons a corrected envelope at the same LSN.
- Mutation kills (each temporary production edit was restored):
  - omit `SecurityLockdown` ledgering — `cargo test -p patchbay-core --test sessions_registry old_security_redeliveries_do_not_cancel_later_lockdown_posture -- --exact` failed on old-entry reactivation, exit 101;
  - accept any same-LSN lockdown as exact — `cargo test -p patchbay-core --test sessions_registry conflicting_same_lsn_security_envelope_rejects_without_mutation -- --exact` failed because the conflict returned success, exit 101;
  - claim the lockdown LSN before validation — `cargo test -p patchbay-core --test sessions_registry malformed_security_event_does_not_poison_corrected_same_lsn -- --exact` failed because the malformed envelope appeared in `applied_events`, exit 101.
- Focused verification: registry 18/18; ingest 17/17; replay/resolver 9/9; session properties 9/9; acceptance 25/25; named server `CoreDecisionGate` regression passed. `cargo test --workspace`, workspace Clippy with warnings denied, model checks (53 modeled properties), and vector checks (53 vectors, 21 implementation checks, 37 existing vector mutation witnesses) passed.
- Hygiene/lifecycle: scoped `rustfmt --edition 2021 --check core/tests/sessions_registry.rs`, `git diff --check`, and a production-source zero-diff check passed. No production, model, vector, wire, or foundation artifact changed; assurance remains implementation-checked, and both child and parent intentionally remain at `stage: review` for Phase-2 adversarial convergence.

## Deep-lane Phase-2 follow-up — lockdown envelope and semantic preclaim evidence

- Adjudication: confirmed both findings as evidence gaps, not production defects. Production already compares the complete applied `StoredEventPayload`, checks the lockdown payload's inner authority domain, and records replay identity only after all transition-specific validation succeeds.
- Applied-lockdown equality evidence now isolates a kind-only/same-bytes candidate and a same-kind valid Protobuf re-encoding with an unknown field discarded on decode. Test-owned payload-only and decoded-semantic predicates accept their respective weak fixtures, while the real registry rejects each as conflicting same-LSN history with exact non-mutation.
- Actual equality mutant kills (temporary production edits restored):
  - payload-only `applied.payload == candidate.payload` — `lockdown_exact_envelope_equality_kills_payload_only_and_decoded_semantic_mutants` failed on the kind-only candidate, exit 101;
  - decoded `SessionStateEvent`/`SecurityLockdownEvent` semantic equality — the same focused test failed on the unknown-field re-encoding, exit 101.
- A table now covers correct outer/conflicting inner domain, affected-runtime-session-count mismatch, and exit while inactive. Every row proves exact registry non-mutation, then applies a corrected envelope at the identical `(authority_domain_id, LSN)` successfully.
- Actual validation mutant kills (temporary production edits restored):
  - removing the inner-domain equality guard — `rejected_lockdown_semantics_are_non_mutating_and_do_not_claim_their_lsn` failed on the conflicting-inner-domain row, exit 101;
  - claiming `applied_events[event_lsn]` after common domain validation but before transition-specific checks — the same table failed exact non-mutation on the affected-count row, exit 101.
- Verification: registry 20/20; ingest 17/17; replay/resolver 9/9; session properties 9/9; acceptance 25/25; named server `CoreDecisionGate` regression passed; `cargo test --workspace`, workspace Clippy with warnings denied, model checks (53 modeled properties), and vector checks (53 vectors, 21 implementation checks, 37 existing vector mutation witnesses) passed.
- Hygiene/lifecycle: scoped rustfmt, `git diff --check`, and production-source zero-diff passed. No production, model, vector, wire, foundation, or unrelated item changed; assurance remains implementation-checked, and child/parent remain at `stage: review` for caller-owned adversarial convergence.
