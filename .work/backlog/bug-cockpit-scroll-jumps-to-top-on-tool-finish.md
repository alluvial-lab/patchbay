---
id: bug-cockpit-scroll-jumps-to-top-on-tool-finish
tags: [bug, ui, cockpit]
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
