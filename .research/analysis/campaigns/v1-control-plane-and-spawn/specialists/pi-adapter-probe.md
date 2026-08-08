---
provenance: agent-synthesis
updated: 2026-08-08
---

# Pi adapter probe

## Bottom line

Current Pi supports process-isolated RPC over strict LF-delimited JSONL, and its session store gives an adapter a durable append-order cursor for reconnect/replay. It supports extension/resource reload in-process, but the documented reload contract is not a general Pi/runtime code-upgrade contract; restarting the Pi process is the reliable boundary for upgrading the running package/runtime. A restarted process can continue the saved session, but only persisted session data survives; in-memory runtime state and the process's cache/generation do not.

## (a) Reconnect/replay and event ordering

RPC emits responses and asynchronous events as JSONL. The documented lifecycle is structured (`agent_start`/`agent_end`, `turn_start`/`turn_end`, message lifecycle, tool lifecycle), but the docs do **not** promise one universal total order for every event. In particular, parallel tool execution starts in assistant source order, updates may interleave, tool ends are completion order, and final tool-result message events remain assistant source order. Streaming `message_update` records are deltas; `message_end.message` is authoritative. [pi-rpc]{1}

`get_entries` is the recovery primitive: it returns the append-only session tree in append order, excluding the header. Entry ids are stable durable cursors; `get_entries({since: last_seen_id})` returns only entries strictly after that id, including pre-compaction history and abandoned branches, and includes `leafId`. If the cursor is not found, the response fails rather than guessing. Therefore the adapter can persist its last accepted entry id and, after a broken connection/process restart, request the suffix and reconcile the current leaf. This is session-entry replay, not a promise that the transient stdout event stream itself is replayable. [pi-rpc]{2}

Operational implication: Patchbay should treat RPC events as live notifications and `get_entries(since)` as authoritative gap recovery. It should persist the cursor only after handling the corresponding entry and handle an unknown cursor as a resynchronization/error path (for example, fetch the full entry set or rebind to a new session), not as an empty suffix.

## (b) `/reload` versus restart for code upgrades

`/reload`/`ctx.reload()` has a defined in-process resource/runtime lifecycle: current extension runtime receives `session_shutdown`; resources are rediscovered; a new extension runtime receives `session_start` with `reason: "reload"`; future commands/events/tools use the new extension version. The invoking command remains an old call frame, so the documented safe form is `await ctx.reload(); return`. [pi-extensions]{1}

The installed loader source clarifies the boundary. Pi clears its own extension-factory cache and creates jiti with `moduleCache: false` when importing extension entrypoints, so a changed auto-discovered extension entrypoint can be re-read. However, the same loader aliases Pi runtime packages to the already-running installed `dist` entrypoints. Neither the docs nor loader source defines `/reload` as replacing the running Pi executable/package graph. Thus a freshly built `/dist` of the Pi/runtime package is not a reliable hot-upgrade mechanism: use process termination and respawn for code upgrades. This is a limitation of the reload contract, not a claim that every extension source import remains cached. [pi-loader]{1}[pi-extensions]{2}

## (c) Restart-as-continuation

Pi documents the continuation pattern rather than a dedicated restart RPC command: sessions auto-save as JSONL; a new process can use `--continue` for the most recent session or `--session <path|id>` for a specific session. Session entries form a tree with stable ids and parent links, and the header records the session id/cwd/version. [pi-sessions]{1}[pi-sessions]{2}

What survives a process restart is the persisted session file: transcript entries, tree/branch structure, compaction entries, model/thinking changes, and extension custom entries that were appended. Full history remains in JSONL even when context is compacted. [pi-sessions]{2}

What does not survive automatically is process-local runtime state: active subscriptions, open resources, in-memory extension variables, and loader/cache generation. Extensions must reconstruct state from persisted custom entries/config during `session_start`; the loader's extension cache generation is process-local and is reset/created anew. [pi-extensions]{3}[pi-loader]{1}

There is no documented `restart` command in the local RPC command list. The adapter's restart operation should therefore own: quiesce/abort policy, terminate process, preserve/verify the session path, respawn with `--session` (or `--continue` only where unambiguous), then use RPC state/cursor reconciliation. The SDK offers a same-process runtime replacement API (`newSession`, `switchSession`, `fork`, clone, import), but that replaces sessions/runtimes; it is not a process-code upgrade. [pi-sdk]{1}

## (d) Capability surface and minimum manifest

Pi exposes two viable adapter substrates:

1. **RPC subprocess:** strict JSONL command/event channel, prompt/steer/follow-up, state/model/thinking controls, compaction, session open/switch/fork/clone, `get_entries`, `get_tree`, and extension UI sub-protocol. It provides process isolation and language-agnostic integration. [pi-rpc]{1}[pi-rpc]{2}
2. **SDK embedding:** `createAgentSession()` gives typed prompt/queue/event/state/model/compaction/tree/abort/dispose access. `AgentSessionRuntime` adds new-session, switch, fork/clone, and JSONL import; after replacement, callers must re-subscribe and rebind extensions. `DefaultResourceLoader` supplies cwd-scoped extensions, skills, prompts, themes, and context files. [pi-sdk]{1}

For v1, a Pi adapter capability manifest should declare at least:

- `transport`: `rpc-jsonl` or `sdk` (and whether process isolation is available);
- `prompting`: prompt plus steering/follow-up queue semantics;
- `events`: lifecycle/streaming events, with the explicit parallel-tool ordering caveat;
- `cursor_replay`: `get_entries(since)` with stable entry-id cursor, append-order replay, `leafId`, and unknown-cursor failure behavior;
- `session_persistence`: JSONL path/session id, continue/open, tree branches, compaction/history semantics;
- `session_replacement`: new/switch/fork/clone/import if SDK-backed, or adapter-owned terminate/respawn continuation if RPC-backed;
- `reload`: resource/extension reload only, with process restart required for reliable Pi/runtime code upgrade;
- `resource_scope`: cwd, project trust, extensions, skills, prompts, themes, context files; and
- `state_rehydration`: whether adapter/extension state is persisted and how it is reconstructed on `session_start`.

This manifest is an adapter synthesis from the cited capability surface, not a Pi-native manifest type. {inferred: consolidate}

## Disconfirming analysis

- For ordering, the RPC event documentation's parallel-tool exception disconfirms treating the stream as a universal total order; the recovery conclusion relies on the separate append-order `get_entries` contract. [pi-rpc]{1}
- For reload, the extension docs say future calls use the new extension version, while the loader source shows jiti module caching disabled for extension imports; these disconfirm the overly broad statement that `/reload` never sees fresh extension code. The narrower conclusion is that reload is not a documented replacement of the running Pi/runtime package graph. [pi-extensions]{1}[pi-loader]{1}
- For restart, session docs confirm continuation via saved files, but no local RPC section documents a restart command. This disconfirms presenting restart as a Pi RPC primitive; it is an adapter-supervised process lifecycle. [pi-sessions]{1}[pi-rpc]{2}

## Contradictions

No source pair presents an incompatible position. There is a scope tension: extension documentation describes hot reload of auto-discovered extensions, while the loader source separately shows process-level runtime aliases and cache behavior. They qualify different layers rather than contradicting: extension entrypoint reload is supported; replacing the running Pi/runtime package is not established by the reload contract. [pi-extensions]{1}[pi-loader]{1}

## Gaps shaping v1

- Pi does not document a universal event ordering guarantee; Patchbay must not infer one from lifecycle names.
- `get_entries(since)` fails for an unknown id; v1 needs an explicit cursor-loss/full-resync policy.
- `/reload` is not a reliable deployment upgrade primitive for Pi/runtime code; v1 needs supervised respawn for upgrades.
- Session continuation preserves persisted JSONL, not arbitrary in-memory adapter/extension state; v1 needs state rehydration and process identity/generation handling.
- RPC itself does not provide a documented process supervisor/restart command; Patchbay owns spawn, termination, backoff, and reconnect orchestration.

## Revisit if

Re-open this facet if current Pi documents add an explicit restart RPC command, define a stronger total event-order guarantee, change `get_entries` cursor semantics, expose a documented runtime package hot-swap path, or add durable process/generation state to session continuation.
