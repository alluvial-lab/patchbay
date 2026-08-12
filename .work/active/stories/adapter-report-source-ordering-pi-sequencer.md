---
id: adapter-report-source-ordering-pi-sequencer
kind: story
stage: done
tags: [adapter, protocol]
parent: adapter-report-source-ordering
depends_on: [adapter-report-source-ordering-contract-foundation]
release_binding: v0.2.0
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

## Current-HEAD reconciliation (2026-08-10)

- Pi uses one per-runtime-id promise tail for transcript and session observations, but `#identity(entry, model)` is currently evaluated only when the tail executes. The implementation allocates/captures the cursor and complete report values before chaining without disturbing transcript ordering.
- `PatchbayCoreClient.#postAttach` retries one closure after an unauthenticated reattach. Passing a preallocated generated cursor plus captured primitives into that closure preserves the exact cursor/payload across the retry; revision allocation must stay outside `reportSession` and outside the retry closure.
- Runtime entries are stable objects across their Pi session-generation changes, so a per-entry generation/revision sequence can reset on a strict runtime-generation bump and survive same-process attachment refresh. The configured adapter generation remains the producer epoch and a replacement process starts its own counters.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, maximum reasoning (caller-selected for asynchronous producer-order and retry semantics); direct-read single-owner delivery, no nested agents or peeragent.
- Review weight: `thorough` (explicit caller override); child checkpoint review is not applicable.
- Files changed: `pi-adapter/src/{main,core_client,session_report_sequencer}.ts`; `pi-adapter/tests/{delivery,core_client,session_report_sequencer}.test.ts`.
- Tests added/removed: added a pure sequence boundary/overflow suite and a real Connect reattach-retry test; extended the real Pi model-change test to block revision 2 while revision 3 captures its own model/activity; added independent-session plus runtime/adapter-generation reset coverage. No useful existing tests were removed.
- Simplification: sequencing is one domain-owned pure function; the process stores one tuple per runtime id and passes a minimal frozen order to the transport. Promise tails only deliver already-captured values.
- Discrepancies from design: no material deviation. The generated `SessionReportSourceCursor` is constructed at the transport boundary from a frozen internal order rather than retained as a mutable generated object; both retry attempts rebuild byte-equivalent generated reports from the same captured primitives.
- Adjacent issues parked: none.

## Verification evidence

- `npm --prefix pi-adapter run build` — passed.
- `node --test pi-adapter/dist/tests/core_client.test.js pi-adapter/dist/tests/delivery.test.js pi-adapter/dist/tests/session_report_sequencer.test.js` — passed (11 focused tests).
- `npm --prefix pi-adapter test` — passed (29 tests), including real AgentSession model/activity ordering, generation bump/reconnect/core restart E2E, exact reattach retry, diagnostics, transcript, and overflow coverage.
- `git diff --check` — passed.
