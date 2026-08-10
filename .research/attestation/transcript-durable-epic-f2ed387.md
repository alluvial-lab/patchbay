---
source_handle: transcript-durable-epic-f2ed387
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/epics/epic-durable-transcript-ownership.md
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The durable-transcript epic names lossy SDK-message re-derivation as the divergence class, proposes extension-owned durable transcript events via SDK custom entries, and separates LLM-context ownership from transcript ownership.

## Key passages

> "Today the extension's transcript is a lossy re-derivation from SDK messages. TranscriptEventLog ... is purely in-memory; on every Pi process restart it is rebuilt by projecting the SDK's durable messages."

> "That re-derivation is the root of an entire divergence class: the extension's live events ... and the SDK's durable messages ... are two different sources of truth, and restart backfill silently picks the SDK's."

> "This epic makes the extension the authoritative owner of its own durable transcript event log, persisted alongside SDK messages via the SDK's custom-entry API, with the SDK's messages becoming one input (for LLM context), not the source of transcript truth."

> "The reconciliation is the design-bearing core: SDK messages remain authoritative for LLM context; extension entries are authoritative for the transcript."

> "Stable replay contract — the app's session_history becomes exactly what was rendered live, durably; no re-derivation drift."

> "F1 — Durable transcript event log (foundation) ... backfill from buildContextEntries() preferring validated Outpost-Pi events over SDK-derived."

## Metadata
- Commit: `f2ed387fecf7f854de85d0a7d8a7c798a63d2d74`
- Author date: 2026-08-04
- Source kind: local git history / work-item document
