---
id: replay-integrity-prefix-discipline-cross-projection-evidence
kind: story
stage: done
tags: [verification, protocol]
parent: replay-integrity-prefix-discipline
depends_on: [replay-integrity-prefix-discipline-shared-replay-boundary]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Cross-projection replay-integrity evidence

## Checkpoint

Prove the shared reader-side defense independently of the production append
boundary. Use scripted storage to inject gapped and `Unspecified` records into
every complete-log rebuild, plus an independent bounded-sequence oracle and
production aggregate cursor regressions.

## Acceptance evidence

- A scripted `[LSN 1, LSN 3]` read and an LSN-1 `Unspecified` event fail in
  every exported complete-log projection rebuild and in the server aggregate
  startup path; no partial projection is returned.
- Catch-up rejects before changing `last_applied_lsn`; a valid contiguous
  mixed-kind prefix still lets each projection fold its owned events and
  ignore known siblings.
- Snapshot-tail tests accept `cursor+1` and reject a missing `cursor+1`.
- A bounded property test computes expected `1..=N` independently and rejects
  a skipped LSN at every injected position.
- Mutation evidence kills both claim-breaking changes: weakening exact equality
  back to monotonic `actual > previous`, and treating `Unspecified` as a
  sibling no-op.
- Existing replay determinism, storage allocation, and Rust workspace suites
  remain green. Evidence is reported as implementation-checked only; no formal
  property or conformance vector is promoted.

## Ordering constraints

Depends on `replay-integrity-prefix-discipline-shared-replay-boundary`; the
matrix verifies integration rather than defining a second predicate. Because
this story is tagged `[verification]`, any later story-level review must use the
project deep lane, including adversarial mutation review, before advancing it
to `done`.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, xhigh; explicit caller selection for the normative cross-projection replay seam.
- Review weight: `thorough` (explicit caller selection), retained for the feature boundary. Per the implementation endpoint contract, this verification-tagged child advances directly to done on green evidence rather than entering story review.
- Files changed: `core/tests/replay_integrity.rs`, `core/tests/recovery.rs`, and the `server/src/state.rs` test module.
- Tests added/removed: one scripted-storage cross-projection matrix covering command, Elicitation, authority, operator, session, resource, security, and adapter rebuilds; direct no-mutation `Unspecified` receiver checks including diagnostics; pure contiguous-prefix and bounded gap properties; snapshot-tail cursor tests; aggregate startup/catch-up/as-of cursor regressions; valid mixed-kind aggregate replay. No tests removed.
- Mutation witnesses: `gap_mutation_witness_kills_monotonic_only_validation` fails if exact successor equality is weakened to monotonic increase; `unspecified_kind_mutation_witness_rejects_before_direct_projection_mutation` fails if `Unspecified` becomes a sibling no-op. Both use raw scripted events rather than the production append boundary.
- Simplification: one fake storage shape supplies every corrupt replay fixture; no database mutation hook, conformance-vector promotion, or second predicate was added.
- Discrepancies from design: the matrix uses harmless `RESOURCE_STATE` sibling framing for every non-resource projection and `GRANT` for resource replay, because one malformed payload kind cannot be simultaneously projection-owned and harmless across every consumer. This preserves the intended shared-boundary test rather than duplicating valid domain payload builders.
- Adjacent issues parked: none (operator forbade backlog/exclusion expansion).

## Verification evidence

- `cargo test -p patchbay-core --test replay_integrity` — 6 passed.
- `cargo test -p patchbay-core --test recovery` — 12 passed.
- `cargo test -p patchbay-core-server state::tests::replay_integrity` — 4 passed.
- `git diff --check` — passed.
- Evidence remains implementation-checked only; no model property or conformance vector was promoted.
