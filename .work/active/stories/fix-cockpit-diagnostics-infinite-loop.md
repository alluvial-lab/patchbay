---
id: fix-cockpit-diagnostics-infinite-loop
kind: story
stage: implementing
tags: [verification]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-17
updated: 2026-08-17
---

# Fix: cockpit fires QueryDiagnostics in an infinite loop with no sessions

## Reproduction (live UAT, 2026-08-17)

Browser open on the sessions screen with ZERO sessions (fresh world). Web log
shows 65,000+ `QueryDiagnostics` calls (~8,700/min peak, still climbing),
2,157 LoadSnapshots — vs 5 Submits. The browser event loop starves; UI actions
("+", navigation) become unresponsive. Every prior UAT session compounded it.

## Root cause (diagnosed to the line)

`web-cockpit/src/ui/shell.ts` `render()` tail:

- With no sessions, `selectedSession()` → undefined → `selectedIdentity` =
  undefined; `observedSelectedKey` initialized undefined.
- The latch `observedSelectedKey === undefined ? "initial" : ...` re-arms on
  EVERY render because the assignment `observedSelectedKey = selectedIdentity`
  writes `undefined` again — the "initial" reason never latches.
- `onSelectionChange(undefined, "initial")` → `main.ts
  queryAdapterStatus(undefined, ...)` → QueryDiagnostics (adapter-unscoped,
  by design for the no-selection case) → merge → `shell.update` → render →
  loop. The `inFlightDiagnostics` key `*:initial` dedupes only concurrent
  duplicates, not re-arms.

## Fix

- Latch the no-selection state: a boolean/`null` sentinel for "observed the
  initial (including empty) selection" so "initial" fires exactly once per
  selection-transition, including the empty case. Same for the
  `reconcile-completion` path if it shares the shape.
- Regression: fold/render cycle with zero sessions must issue ZERO additional
  QueryDiagnostics after the first; a render-triggered re-arm must not query.
- Mutation: revert the latch → regression fails.

## Acceptance

- [ ] Zero-session idle browser issues exactly one adapter-status query set,
      then goes quiet (verify in web log over a 60s window).
- [ ] Selection changes still query once per transition; connectivity changes
      once per change.
- [ ] Full four groups + web-cockpit suite green.
