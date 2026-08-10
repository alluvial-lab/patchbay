---
source_handle: transcript-ordering-review-70900a5
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/features/feature-canonical-transcript-ordering.md
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The standard review accepted the core render-sort but returned REQUEST CHANGES for four remaining timestamp-provenance paths.

## Key passages

> "Core fix (Unit 3) sound; neither prior regression reintroduced; tool min-ts, backward-compat, agent-network exclusion all confirmed safe. But the single-clock invariant Unit 3 relies on is not yet fully achieved."

> "Tool live/history ts divergence. message_end records tool_requested with the SDK assistant ts ... tool_execution_start broadcasts a fresh Date.now() whose history append is discarded (duplicate event-id, first-writer-wins). So live tool ts != replay tool ts."

> "Buffered assistant fallback narration ... uses DateTime.now() while the tool uses wire ts."

> "AgentDone producer gap. The audit's app-side consume is dead code: the extension records a terminal ts but omits it from the live agent_done frame."

> "Error diagnostics — no ts on the wire ... the app creates an authoritative AssistantMessageCommitted with phone time."

> "This is the third round of clock-provenance discovery ... strong signal that fully closing the single-clock invariant is a deeper arc than one feature pass."

## Metadata
- Commit: `70900a5cc656c383ef07cf2d1a0f8acc4622a7a8`
- Author date: 2026-08-03
- Source kind: local git history / work-item document
