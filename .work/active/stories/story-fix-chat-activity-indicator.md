---
id: story-fix-chat-activity-indicator
kind: story
stage: review
tags: [bug]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Chat view has no agent-activity indicator

Dogfooding report (2026-07-27): nothing in the chatbox shows the agent is
thinking/working/active — activity state only appears on the session-list
row. During a turn, the operator staring at the transcript has no "working…"
affordance; on mobile (detail drills over the list) the list isn't even
visible, so there is NO activity signal anywhere in the chat context.

The data already reaches the cockpit (`SessionActivityState` deltas); this is
a presentation gap against the UX delivery-state floor. Fix direction:
activity indicator in the session-detail header and/or composer area
(e.g. working/thinking state with elapsed turn time), sharing the existing
state vocabulary.

## Root cause + fix (2026-07-27)

The header technically renders `renderSessionStatus` (activity + detail), but
it sits at the top of the detail pane — away from where the operator's eyes
are during a turn (the end of the transcript). Fix: an in-timeline activity
row appended after the last entry whenever the session is live AND working
(`rendersLive` + `SessionActivityState.WORKING`), showing the shared
`activityDetail` vocabulary ("thinking", "using bash", …) with a pulsing dot
(reduced-motion respected). Empty timelines (fresh session mid-turn) render
the indicator after the empty-state rather than skipping it — caught by the
first test run.

## Implementation notes

- **Files changed**: `web-cockpit/src/ui/session-detail.ts` (indicator render
  + both timeline paths; `SessionActivityState`/`rendersLive` imports),
  `web-cockpit/src/ui/shell.css` (`.timeline-activity`, first-ever
  `.activity-indicator` styles + pulse), `web-cockpit/tests/shell.test.ts`
  (3 tests: working/idle/tombstoned).
- **Four-step confirmation**: (1) new tests pass; (2) full suite 66/66 green;
  (3) live repro = operator hard-reload; (4) symptom verification pending
  operator confirmation. Presentation conformance green. Indicator verified
  present in built bundle + served CSS.
- **Bounded inline review verdict**: minimal presentation-only diff; reuses
  the existing activity vocabulary and state axes; no mockup (existing
  component composition per the skip rule); aria `role="status"` for
  assistive tech; no test weakened.
- **Side discovery**: `.activity-indicator` classes used by the session list
  had NO CSS until now — the list showed activity as bare text. The new
  styles now cover both contexts.
