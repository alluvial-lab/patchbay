---
source_handle: transcript-provenance-sweep-306dd7f
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/stories/story-canonical-transcript-ordering-systematic-ts-provenance-sweep.md
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The systematic sweep records an exhaustive extension live/history table, an app live-construction table, a schema table, and a scope contradiction involving app-facing mesh tool notifications.

## Key passages

> "Exhaustive read-only enumeration of every extension live-broadcast site and every app authoritative-event-creation site. Confirms the 4 review gaps + finds 3 more (app-origin user-confirmation competing ts, tool-result restart backfill divergence, user_message echo/dedupe ts omission) + a scope contradiction (_deliverMeshMessageToAgent broadcasts app-facing tool frames w/o ts)."

> "Operator decision 2026-08-03: do a systematic enumeration of every live-broadcast site (extension) and every authoritative-event-creation site (app), rather than chasing the current four findings piecemeal — so no fifth gap is missed."

> "tool_execution_start → tool_request ... normally the earlier assistant message_end SDK timestamp ... The broadcast stamps a second Date.now() instead of looking up the recorded request."

> "tool_execution_end → tool_result ... yes in-process; no across restart ... There are still two timestamp owners for one logical result; restart can move the bubble."

> "_deliverMeshMessageToAgent → tool_request + tool_result (tool='agent-network') ... the app persists them as ToolRequested/ToolFinished, and both enter authoritativeMessages. They therefore use phone time today."

> "Every authoritative-bubble-producing event kind carries server ts on the live path OR is documented as render-excluded."

> "The sweep also found two additional multi-owner cases beyond the review's four: app-origin user confirmation (_confirmUserDelivery versus SDK message_end) and tool-result restart backfill (tool_execution_end versus SDK message_end). A revised design must name the durable SDK timestamp owner for early/late hooks rather than relying only on first-writer-wins process-local dedupe."

## Metadata
- Commit: `306dd7f0789326e1442463bafb6769b0d103ee55`
- Author date: 2026-08-03
- Source kind: local git history / work-item document
