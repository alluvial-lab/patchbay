---
source_handle: transcript-ownership-decision-c3d5bdc
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi/.work/active/features/feature-canonical-transcript-timestamp-ownership.md
provenance: source-direct
substrate_confidence: direct
---

# Source-direct attestation

## Summary
The recorded operator decisions assign timestamp ownership to execution/delivery hooks, make mesh tool notifications authoritative, and make the extension the sole timestamp authority.

## Key passages

> "Q1 (timestamp ownership) = B: execution/delivery hook owns the canonical ts, message_end reuses it (live == replay == durable)."

> "The execution/delivery hook owns each event's canonical ts (it fires first AND broadcasts live, so its ts is available at broadcast time); message_end is changed to reuse the already-recorded ts instead of stamping a fresh one."

> "the extension (Pi/SDK) is the sole authoritative ts owner; the app is consumer-only ... The relay has no transcript-ts role (opaque transport)."

> "Q2 — app-facing mesh tool notifications: (a) authoritative. The tool='agent-network' cards ... are real transcript bubbles."

> "The extension stamps a server ts on these frames; the app consumes it."

## Metadata
- Commit: `c3d5bdc4fc9f20d1e117de6b2cf16c85de725d46`
- Author date: 2026-08-03
- Source kind: local git history / work-item document
