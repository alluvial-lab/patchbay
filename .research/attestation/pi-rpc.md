---
source_handle: pi-rpc
fetched: 2026-08-08
source_path: /home/agent/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/rpc.md
provenance: source-direct
---

## Summary
Pi RPC is a strict LF-delimited JSONL stdin/stdout protocol. Commands receive correlated responses when an id is supplied, while agent events stream asynchronously. Prompt acceptance is distinct from later execution outcome. RPC exposes explicit session replacement (`new_session`, `switch_session`, `fork`, `clone`) and state inspection (`get_state`, `get_entries`); entry ids can serve as durable cursors across client restarts. The session-entry cursor is append-order persistence, not a universal ordering guarantee for the live event stream.

## Key passages
1. RPC commands are JSON objects on stdin, responses have `type: "response"`, and events stream asynchronously. Framing is strict JSONL: LF (`\n`) is the only record delimiter; clients may strip a trailing CR from CRLF input but must not split on Unicode separators. (`Protocol Overview`; `Framing`)
2. A prompt response with `success: true` means accepted, queued, or handled; failures after acceptance appear in the normal event/message stream rather than as a second response. (`prompt`)
3. `new_session` starts a fresh session; `switch_session` loads another file; `fork` and `clone` create new session variants. (`new_session`; `switch_session`; `fork`; `clone`)
4. `get_entries` returns all session entries in append order, excluding the header. It accepts `since`, treats stable entry ids as durable cursors across client restarts, returns only entries strictly after the cursor, includes pre-compaction history and abandoned branches, and returns the current `leafId`. (`get_entries`)
5. `agent_end` is a low-level run completion; `agent_settled` means no automatic retry, compaction retry, or queued continuation remains. (`Events`; `agent_end`; `agent_settled`)
6. In parallel-tool mode, tool starts are emitted in assistant source order, updates may interleave, tool ends are emitted in completion order, and final tool-result message events are emitted later in assistant source order. `message_update` records are deltas, while `message_end.message` is authoritative. (`Events`; `message_update`; `tool_execution_start / tool_execution_update / tool_execution_end`; corroborated by `docs/extensions.md`, `tool_execution_*`)
7. With `get_entries({ since: id })`, the returned suffix is strictly after the matching entry id. The sequence is append order over persisted session entries; it is not specified as replay of the transient stdout event stream or as a universal total order over live events. (`get_entries`; `Events`)
8. If `since` does not match any entry id, `get_entries` returns `success: false` rather than treating the cursor as an empty suffix or guessing a replacement position. (`get_entries`)
9. The documented RPC command inventory covers prompting/queues, abort, state/model/thinking controls, compaction/retry, bash, session statistics/export/replacement/tree access, and command discovery. It documents no process `restart` command. `get_commands` lists extension commands, prompt templates, and skills, while built-in interactive-only commands are excluded and do not execute when sent through RPC `prompt`. (`Commands`; `get_commands`)
