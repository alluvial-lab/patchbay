---
id: adapter-report-source-ordering-conformance
kind: story
stage: done
tags: [verification, protocol]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-core-fence, adapter-report-source-ordering-pi-sequencer]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
---

# Promote source-ordering model and executable vector evidence

## Checkpoint

Establish `SessionReportSourceOrdering` with a focused, trace-faithful Quint
model, a claim-breaking stale-guard mutation, and a promoted conformance vector
executed through authenticated server ingress. Regenerate model/vector
traceability without hand-editing generated blocks.

## Acceptance evidence

- Environment arrival and core application are separate model steps; the
  property inspects pending pre-state evidence that the apply action cannot
  rewrite.
- The real checker passes at the documented bound, emitted TLA is committed only
  as an inspection artifact, and weakening the source comparison admits the
  delayed rollback and fails the property.
- The vector applies `A/r1`, `B/r3`, then delayed `A/r2`; it observes stale
  status/audit, no stale session event, snapshot `B/r3`, and hot/replay equality.
- The same runner proves a newer adapter or runtime-session generation can reset
  local revision without letting the prior producer mutate state.
- Vector/property registries, invariant expectation, exact implementation-check
  ids, constrained proto paths, and generated verification tables agree.
- Checked-model, promoted-vector, and checked-normative language is used only if
  every genuine-checking and promotion gate passes.
- Because this story is tagged `[verification]`, its later story review follows
  the project deep lane and attacks the mutation witness before `done`.

## Ordering constraints

Runs only after `adapter-report-source-ordering-core-fence` and
`adapter-report-source-ordering-pi-sequencer`. It is the final checkpoint before
the integrated feature enters caller-selected `thorough` review.

## Current-HEAD reconciliation (2026-08-10)

- The live registries currently derive 53 vectors (16 promoted), 53 modeled properties (8 promoted), and zero checked-normative properties. `SessionReportSourceOrdering` becomes the first checked-normative intersection only after both its promoted model block and promoted authenticated server vector pass; generated counts/tables must be regenerated from artifacts.
- The server runner already dispatches exact requested `rust-server` case ids and supports compiled conformance faults. This checkpoint adds one exact authenticated session-report case and a source-comparison fault without disturbing the token-commune mutation profile.
- The model must use a separate environment-arrival pending phase and raw pending pre-state. Mutation evidence will weaken the apply comparison while leaving the oracle unchanged; a nonzero Quint counterexample exit is expected mutation success, not a tool failure.
- Per project convention, implementation may establish green model/vector evidence but this `[verification]` child stays at `stage: review` for the independent deep lane rather than being self-closed.

## Implementation and deep review (2026-08-10)

- Landed in WIP commit `078ccae`: the trace-faithful Quint model and emitted TLA inspection artifact, promoted authenticated server vector, exact runner/mutation registration, bounded Rust property coverage, and generated verification traceability.
- Completeness phase: focused core/server/Pi suites passed; Quint parse/compile and the delayed-report/mutant traces passed; `check-models.mjs`, `check-vectors.mjs`, and generated-contract drift passed. The real temporal check passed at the documented 10-step bound.
- Adversarial phase: re-read the raw pending-evidence oracle and comparison-weakening model mutation, then exercised the production `accept-nonincreasing-session-revision` mutation through the exact server vector. The oracle remains independent of the production guard and the runner proves stale audit, no stale session append, durable watermark, snapshot state, hot/replay equality, and both generation resets.
- Review disposition: no receiver-confirmed material blocker or smaller surviving finding. Global `cargo fmt --all -- --check` remains red because unrelated pre-existing test files are not formatted; this story changed none of those files and all in-scope diffs pass `git diff --check`.
