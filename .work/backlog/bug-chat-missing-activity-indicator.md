---
id: bug-chat-missing-activity-indicator
tags: [bug, ui, cockpit]
created: 2026-07-27
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
