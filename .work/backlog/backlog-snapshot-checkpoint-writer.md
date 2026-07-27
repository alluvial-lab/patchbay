---
id: backlog-snapshot-checkpoint-writer
tags: [perf, protocol, foundation]
created: 2026-07-27
---

# Snapshot checkpoint writer (bound recovery replay cost)

Docs-audit finding (2026-07-27): PROTOCOL claimed periodic snapshot
checkpointing bounds recovery replay. Reality: the storage API is
snapshot-capable (`core/src/storage/audited.rs:284-290`) but has no
production writer; `LoadSnapshot` materializes on demand
(`server/src/service.rs:247-289`) and recovery replays the whole log. Fine at
dogfooding scale; becomes a real recovery-time concern as the durable log
grows. Prose corrected to describe current behavior; the checkpoint writer +
scheduling policy is open v0.1+ work.
