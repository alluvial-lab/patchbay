---
source_handle: pi-sessions
fetched: 2026-08-08
source_path: /home/agent/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/sessions.md
provenance: source-direct
---

## Summary
Pi persists conversations as JSONL sessions organized by working directory. The CLI can continue the most recent session (`-c`), browse/resume (`-r`), select an explicit session (`--session`), or fork one (`--fork`). Session files are trees, allowing in-place branch navigation; fork and clone create new files.

## Key passages
1. “Pi saves conversations as sessions so you can continue work, branch from earlier turns, and revisit previous paths.” (`Session Storage`)
2. `pi -c` continues the most recent session; `pi -r` browses and selects a past session; `pi --session <path|id>` selects a specific session; `pi --fork <path|id>` forks one into a new session. (`Session Storage`)
3. Sessions auto-save as JSONL files organized by working directory. Each file is a tree: entries carry stable `id`/`parentId` links and the current position is the active leaf. `/tree` moves within the same file, while `/fork` and `/clone` create separate session files. (`Session Storage`; `Branching with /tree`; `/tree, /fork, and /clone`; corroborated by `docs/session-format.md`, `SessionHeader` and `Tree Structure`)
4. Selecting a previous point and continuing creates a new branch rather than overwriting prior history. (`Selection Behavior`)
