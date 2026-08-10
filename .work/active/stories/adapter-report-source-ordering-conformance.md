---
id: adapter-report-source-ordering-conformance
kind: story
stage: implementing
tags: [verification, protocol]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-core-fence, adapter-report-source-ordering-pi-sequencer]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
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
