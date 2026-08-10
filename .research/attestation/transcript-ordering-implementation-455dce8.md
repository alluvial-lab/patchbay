---
source_handle: transcript-ordering-implementation-455dce8
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/app/lib/domain/transcript/transcript_projection.dart
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The projection implementation preserves arrival-order lifecycle reduction, then sorts authoritative bubbles by canonical server timestamp with arrival-index tie-breaking; tool bubbles use the minimum request/result timestamp.

## Key passages

> "Lifecycle state reduces in arrival order above. Rendered authoritative bubbles use canonical server time with arrival as the stable tiebreaker."

> "authoritativeMessages.sort((a, b) { final byTs = (messageTs[a.id] ?? 0).compareTo(messageTs[b.id] ?? 0); if (byTs != 0) return byTs; return (messageArrival[a.id] ?? 0).compareTo(messageArrival[b.id] ?? 0); });"

> "Tool sort key: messageTs[toolCallId] = min(existing, ts) on every tool upsert, so the bubble sorts by REQUEST time regardless of arrival order."

## Metadata
- Commit: `455dce867dcd6a4bbe1e8d0c48db521fabe4fd9a`
- Author date: 2026-08-03
- Source kind: local git history / source code
