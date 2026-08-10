---
source_handle: transcript-wire-0a048de
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/pi-extension/src/index.ts
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The ordering implementation reuses one `Date.now()` value for each tool history event and its live frame, and adds optional non-negative `ts` fields to tool request/result wire messages.

## Key passages

> "const ts = Date.now();"

> "_appendTranscriptEvent({ ... ts, ... });"

> "_owners.broadcast({ ... ts, });"

> "Canonical server timestamp for the live tool request. Optional (compat); new clients use it to preserve transcript ordering with session_history replay."

> "Canonical server timestamp for the live tool result. Optional (compat); new clients use it to preserve transcript ordering with session_history replay."

## Metadata
- Commit: `0a048de2657712f227b6133826560772fdf81eb0`
- Author date: 2026-08-03
- Source kind: local git history / source code and schema
