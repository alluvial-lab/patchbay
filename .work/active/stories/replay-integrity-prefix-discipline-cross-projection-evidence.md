---
id: replay-integrity-prefix-discipline-cross-projection-evidence
kind: story
stage: implementing
tags: [verification, protocol]
parent: replay-integrity-prefix-discipline
depends_on: [replay-integrity-prefix-discipline-shared-replay-boundary]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
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
