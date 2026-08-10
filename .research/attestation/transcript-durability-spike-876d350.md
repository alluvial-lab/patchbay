---
source_handle: transcript-durability-spike-876d350
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/stories/story-canonical-transcript-timestamp-ownership-ownership-foundation.md
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The Outpost-Pi durability spike records a FEASIBLE verdict for a durable custom-entry overlay, and documents SDK hook ordering, timestamp traces, replay behavior, and a six-step implementation plan.

## Key passages

> "Read-only spike verdict: the SDK does not immutably own message.timestamp, but assistant message_end fires BEFORE tool_execution_start (can't retrofit execution ts into an already-persisted assistant message, esp. multi-tool). Full live==durable agreement is FEASIBLE via the SDK durable custom-entry API (appendEntry -> appendCustomEntry -> session JSONL -> buildContextEntries backfill): persist hook-owned canonical events alongside SDK messages and prefer them on restart backfill."

> "Therefore a request hook cannot retroactively replace the already-appended assistant message's timestamp. One assistant message can also contain multiple tool calls ... each with a different execution-start clock, so the single assistant-message timestamp is not a sufficient durable representation."

> "Custom entries are specifically intended to reconstruct extension state after reload and are ignored by LLM context ... Thus Unit A can persist each hook-owned canonical event inside the SDK session and prefer it over the ordinary SDK-message projection during restart backfill."

> "So current request disagreement is live S != in-process history A == restart history A; current result disagreement is live E == in-process history E != durable/restart R."

> "1. pi-extension/src/session/transcript_event_log.ts — add an event-id index ... Test that a duplicate with a different ts cannot change the returned owner."

> "6. Tests — add a producer-order regression that drives assistant message_end -> tool_execution_start -> tool_execution_end -> toolResult message_end and asserts live request/result ts equal in-process history. Add a real-file SessionManager integration test ... assert request S and result E survive exactly (also cover two tool calls in one assistant message)."

## Metadata
- Commit: `876d3501f2dc06c63b80d1c778ce29bfbeba8e70`
- Author date: 2026-08-03
- Source kind: local git history / work-item document
