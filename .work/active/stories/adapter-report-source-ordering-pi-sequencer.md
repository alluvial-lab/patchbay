---
id: adapter-report-source-ordering-pi-sequencer
kind: story
stage: implementing
tags: [adapter, protocol]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-contract-foundation]
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Emit ordered Pi session-report cursors

## Checkpoint

Give each Pi runtime entry and runtime-session generation an adapter-local
report sequence, include the current adapter generation in the generated source
cursor, and capture each complete report snapshot when it is enqueued rather
than when the promise tail later executes.

## Acceptance evidence

- One runtime generation emits revisions `1, 2, ...` in enqueue order; separate
  sessions have independent counters.
- Runtime-session generation replacement resets the local revision, and a
  replacement adapter generation may also begin at one. A same-process
  reattach preserves its sequence.
- The identity, connectivity/activity request, model, and cursor are captured as
  one immutable unit before queueing, so an old revision cannot acquire newer
  mutable state while waiting.
- Authentication retry reuses the same cursor and payload rather than allocating
  a second revision; uint64 overflow fails before wire construction.
- Producer code uses generated cursor/report types. Promise-tail serialization
  remains defense in depth rather than source authority.
- Pi unit, delivery/reconnect, transcript, model-change, and real-process E2E
  coverage remain green.

## Ordering constraints

Consumes `adapter-report-source-ordering-contract-foundation`. It is file-disjoint
from the core checkpoint after generation, but both are one feature contract
and must converge before promotion evidence runs.
