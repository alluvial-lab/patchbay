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
