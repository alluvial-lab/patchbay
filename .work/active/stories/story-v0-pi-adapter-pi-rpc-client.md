---
id: story-v0-pi-adapter-pi-rpc-client
kind: story
stage: implementing
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Story: pi-adapter Pi RPC client (harvested stdio driver)

The Node-side Pi RPC client. Harvests `rpc_child.ts`'s spawn/stdio-JSONL framing from outpost-pi and extends it to the full command surface (outpost-pi's only implements `sendPrompt`). Plus the harvested `transcript_projection.ts` (Pi event → typed `TranscriptEvent`) and `TranscriptEventLog` (append-only, dedup-by-eventId, `forSession` replay).

## Design (from feature-v0-pi-adapter Unit 2)

**Files**: `pi-adapter/src/pi_rpc_child.ts`, `pi-adapter/src/transcript_projection.ts`, `pi-adapter/src/transcript_event_log.ts`, `pi-adapter/tests/pi_rpc_child.test.ts`

### Harvest source

`/home/agent/projects/outpost_pi/pi-extension/src/daemon/rpc_child.ts` — the spawn/stdio-framing pattern (`resolvePiBin`, `spawn`, newline-delimited read; NOTE: Node `readline` is NOT protocol-compliant — it splits on U+2028/U+2029 which are valid inside JSON strings; use a strict `\n` splitter). And `/home/agent/projects/outpost_pi/pi-extension/src/session/transcript_projection.ts` + `transcript_event_log.ts` — the event→typed-event mapping + dedup log.

### Pi RPC command surface (from Pi's own `docs/rpc.md`)

The `PiRpcChild` maps 1:1 to the documented Pi RPC commands:

```typescript
export class PiRpcChild {
  // Spawns `pi --mode rpc --approve -n <name>` in <cwd>; speaks JSONL over stdio.
  prompt(text: string, opts?: { streamingBehavior?: "steer"|"followUp" }): Promise<Response>;
  abort(): Promise<Response>;
  getState(): Promise<SessionState>;              // get_state
  getEntries(since?: string): Promise<Entries>;  // durable cursor — snapshot source
  setModel(provider: string, modelId: string): Promise<Response>;
  setThinkingLevel(level: ThinkingLevel): Promise<Response>;
  getAvailableModels(): Promise<Model[]>;
  newSession(): Promise<Response>;               // session_new → generation bump
  compact(instructions?: string): Promise<Response>;
  respondToUiRequest(id: string, response: UiResponse): void;  // tool-call approval
  events(): AsyncIterable<PiEvent>;              // the event stream
}
```

### Transcript projection (harvested)

```typescript
// Maps Pi RPC events (message_update, tool_execution_*, turn_*, compaction_*)
// to typed TranscriptEvents with deterministic event ids (dedup key).
export function projectPiEvent(ev: PiEvent, sessionId: string): TranscriptEvent | null;
```

## Acceptance criteria

- [ ] `PiRpcChild` spawns a `pi --mode rpc` child, sends `prompt`, receives the event stream.
- [ ] All §4 commands implemented: prompt, abort, get_state, get_entries, set_model, set_thinking_level, get_available_models, new_session, compact, extension_ui_response.
- [ ] `transcript_projection` maps Pi events to typed `TranscriptEvent`s with stable dedup ids.
- [ ] `TranscriptEventLog` dedups by eventId and replays `forSession`.
- [ ] Smoke test drives a real `pi --mode rpc` child end-to-end (prompt → events → transcript).

## Notes

- **Trickiest part**: the `extension_ui` approval sub-protocol is a request/response over stdio that interleaves with the event stream. The `PiRpcChild` must correlate `extension_ui_request` ids to responses WHILE also consuming the event stream. Design the stdio reader as a single demuxer that routes frames by `type` (`response` → pending command promise; `event`/`extension_ui_request` → event stream / UI-request handler).
- **JSONL discipline**: split on `\n` only, strip optional trailing `\r`. Do NOT use Node `readline` (splits on U+2028/U+2029).
- This story is independent of Unit 1 (different write set: `pi-adapter/` vs `server/`+`contracts/`). They can run in parallel.
- `pi` must be on PATH (it's at `/home/agent/.local/bin/pi` or similar in this env). The smoke test needs a working `pi --mode rpc`.
