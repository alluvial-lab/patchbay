---
id: story-fix-cockpit-render-amplification
kind: story
stage: done
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Cockpit renders once per subscription event — text turns freeze the tab

## Symptom

Operator report (2026-07-27, live dogfooding): "I only get the tool call, not
the text." The chatbox shows tool rows but assistant text never appears.

## Root cause

NOT adapter-side and NOT the presentation fold (both verified against the real
durable database): the turn in question ingested 1,867 `assistant_delta`
events plus the committed message, and replaying the full durable stream
through the cockpit's `fold()` produces a complete, correctly-keyed model.

The defect is render amplification in `web-cockpit/src/main.ts`: the subscribe
loop runs `shell.update(projection.model)` once per folded event, and
`update()` synchronously re-renders the entire timeline (marked parse +
DOMPurify + DOM rebuild of every message). A normal streaming turn emits
~1,900 events, so the browser main thread is saturated for the whole turn;
tool rows (folded early) paint, while the ~1,800 text re-renders queue behind
and never catch up. The per-event `cloneModel` in the fold adds avoidable CPU
on top but is not the primary defect.

## Fix approach

Coalesce rendering: fold every event as it arrives, but schedule at most one
`shell.update` per animation frame (rAF), always rendering the latest model.
Introduce a small injectable `createRenderCoalescer(schedule, render)` helper
so the policy is unit-testable without a browser frame loop; the subscribe
loop in `main.ts` consumes it (flush on `stop()`).

## Regression test

`web-cockpit/tests/render-coalescer.test.ts` — N synchronous event arrivals
schedule exactly one render, which renders the latest model; a second burst
after flush schedules exactly one more; `flush()` renders pending state
synchronously; no render occurs when nothing arrived.

## Implementation notes (2026-07-27)

- **Execution capability**: inline host implementation — small, single-package,
  well-understood after diagnosis; delegation overhead unjustified.
- **Files changed**: `web-cockpit/src/ui/render-coalescer.ts` (new),
  `web-cockpit/src/main.ts` (subscribe loop renders via coalescer; optional
  `scheduleFrame` injection with rAF default + setTimeout fallback),
  `web-cockpit/tests/render-coalescer.test.ts` (new).
- **Four-step confirmation**: (1) new tests pass; (2) full web-cockpit suite
  57/57 green; (3) live repro = operator reload (browser bundle rebuilt);
  (4) symptom verification pending operator reload — see below.
- **Bounded inline review verdict**: minimal diff; every event still folds
  (cursor/model stay current); rendering is now ≤1 per frame with latest
  model; stale pre-flush frames are no-ops by contract (tested); no test
  weakened (one test assertion corrected to the actual contract — stale
  frame no-op — after the harness modeled the scheduler queue incorrectly).
- **Parked**: per-fold `cloneModel` O(events × model-size) CPU on initial
  full-log replay (pre-existing, unchanged by this fix; candidate for
  structural-sharing or batch-fold optimization later).
  `bug-cockpit-scroll-jumps-to-top-on-tool-finish` remains separate — render
  coalescing removes most scroll thrash but the anchor logic itself is
  unfixed.

## Operator confirmation (2026-07-27)

Hard-reload verification passed: a fresh prompt produced live tool rows AND
the assistant's committed text rendered in the chatbox. Symptom resolved;
story closed to done.
