---
id: story-fix-cockpit-scroll-anchor
tags: [bug]
created: 2026-07-27
---

# Cockpit transcript scroll resets to top when a tool call finishes

**Reported** 2026-07-27 during live dogfooding: if the operator is not
scrolled fully to the bottom of the session transcript, a tool-call
completion event scrolls the view to the very top (losing reading position).

**Suspected mechanism**: the shell re-renders the transcript on delta fold
(`web-cockpit/src/ui/shell.ts` / session-detail) without preserving scroll
anchor — likely a full-list replace that resets scrollTop, or an
auto-scroll-to-bottom guard that miscomputes "was at bottom" and instead
collapses to top. Expected behavior: if the user was at the bottom, keep
pinned to bottom; if scrolled up, preserve position (ideally with a "new
activity" affordance).

**Repro**: run a session with enough transcript to scroll; scroll up; wait
for a tool call to finish.

## Root cause

`web-cockpit/src/ui/shell.ts` `render()` does a full `root.replaceChildren()`
rebuild. It only preserves position in the near-bottom case ("stick to
bottom"); when the user is scrolled UP, the captured `stickToBottom` is false
and NO position is restored, so the fresh timeline renders at `scrollTop = 0`
— the very top. Every streamed event therefore yanks a user reading history
to the top (reported as "scrolls to the very top when a tool call finishes").

## Fix approach

Anchor-based scroll preservation across the rebuild: before
`replaceChildren`, capture the first partially-visible timeline entry
(`[data-observation-id]`, `[data-command-id]`, `[data-diagnostic-id]`) and
its offset from the timeline viewport top. After the rebuild, re-query the
same entry by data attribute and adjust `scrollTop` so it sits at the same
viewport offset. Fall back to the previous raw `scrollTop` (clamped) when the
anchor entry vanished. Stick-to-bottom behavior unchanged.

## Regression test

`web-cockpit/tests/scroll-anchor.test.ts` — pure capture/restore logic with
stubbed geometry: first-visible-entry selection, exact offset restoration,
fallback to raw scrollTop when the anchor entry is gone, and clamping when
content shrank.

## Implementation notes (2026-07-27)

- **Execution capability**: inline host — focused single-package fix.
- **Files changed**: `web-cockpit/src/ui/scroll-anchor.ts` (new:
  `captureAnchor`/`restoreAnchor` DOM glue + pure `pickAnchor`/
  `restoredScrollTop`), `web-cockpit/src/ui/shell.ts` (render() captures the
  anchor when not sticking to bottom, restores after rebuild),
  `web-cockpit/tests/scroll-anchor.test.ts` (new, 5 tests).
- **Four-step confirmation**: (1) new tests pass; (2) full suite 63/63 green;
  anchor code verified present in the built browser bundle (bundle
  verification added to the checklist after the f6e7053 lost-edit incident);
  (3) live repro = operator hard-reload; (4) symptom verification pending
  operator confirmation.
- **Bounded inline review verdict**: minimal diff; stick-to-bottom behavior
  untouched; anchor candidates come back in document order (single combined
  selector); vanished-anchor falls back to clamped raw scrollTop; no test
  weakened.
- **Known limitation**: mobile (≤760px) scrolls the PAGE, not the internal
  timeline — this anchor covers the internal-timeline scroll (desktop). A
  mobile page-scroll anchor would use window.scrollY; add if dogfooding on
  mobile shows the same symptom.
