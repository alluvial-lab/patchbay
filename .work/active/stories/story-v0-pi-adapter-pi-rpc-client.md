---
id: story-v0-pi-adapter-pi-rpc-client
kind: story
stage: done
tags: [adapter, protocol]
parent: feature-v0-pi-adapter
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
research_origin: null
---

# Story: pi-adapter Pi `AgentSession` driver (harvested in-process session layer)

The Node-side Pi driver. Harvests outpost-pi's **in-process session layer** — `sdk_session_projection.ts` (the `AgentSession` driving surface), `transcript_projection.ts` (event → typed `TranscriptEvent`), `transcript_event_log.ts` (dedup replay log), `turn_state.ts` (turn projection) — re-housed behind Patchbay's adapter port. The adapter calls `createAgentSession()` directly (NOT `pi --mode rpc` subprocess) and sets `beforeToolCall` for the tool-call approval gate.

## Why not `pi --mode rpc` subprocess (verification, 2026-07-15)

`pi --mode rpc` emits **zero** tool-approval frames. The `beforeToolCall` gate (`agent-session.js:191`) is wired exclusively to the in-process extension runner — `runner.emitToolCall(...)`; with no `tool_call` handler, tools auto-execute. The `extension_ui_request` sub-protocol is for generic extension UI (`ctx.ui.select`/`confirm`), NOT the tool-call permission gate. outpost-pi's own source confirms: tool approval is `pi.on("tool_call")` with `{ block: true }` — an in-process hook. Therefore a pure-RPC client cannot intercept or approve tool calls. Pi's `docs/rpc.md` endorses the programmatic `AgentSession` path for Node consumers. **Decision: (a) programmatic `AgentSession` (operator-confirmed).**

## Design (from feature-v0-pi-adapter Unit 2)

**Files**: `pi-adapter/src/pi_session.ts`, `pi-adapter/src/transcript_projection.ts`, `pi-adapter/src/transcript_event_log.ts`, `pi-adapter/src/turn_state.ts`, `pi-adapter/tests/pi_session.test.ts`

### Harvest source

`/home/agent/projects/outpost_pi/pi-extension/src/session/`:
- `sdk_session_projection.ts` — the `AgentSession` driving surface (`sendMessage`/`sendUserMessage` + event capture + `SessionHistorySnapshot`). NOTE: outpost-pi built this as a Pi *extension* (`ExtensionFactory`/`ExtensionAPI`); re-housing it as a direct `createAgentSession()` caller is the "real adapter-implementation work, not a copy" the harvest idea cautions about. The projection logic harvests; the extension-shape wiring does not.
- `transcript_projection.ts` — Pi event → typed `TranscriptEvent` with deterministic ids (dedup key).
- `transcript_event_log.ts` — append-only, dedup-by-eventId, `forSession` replay.
- `turn_state.ts` — turn projection.

### PiSession driver

```typescript
import { createAgentSession, type AgentSession, type AgentSessionEvent } from "@earendil-works/pi-coding-agent";

export class PiSession {
  static async create(opts: { cwd: string; name?: string; model?: string }): Promise<PiSession>;
  prompt(text: string): Promise<void>;        // instruct — sendUserMessage
  cancel(): void;                              // cancel/interrupt — abort
  getState(): SessionState;                    // query — getState
  getEntries(since?: string): Entries;        // snapshot cursor — getEntries
  setModel(provider: string, modelId: string): Promise<void>;   // reconfigure
  setThinkingLevel(level: ThinkingLevel): Promise<void>;         // reconfigure
  getAvailableModels(): Model[];               // query
  newSession(): Promise<void>;                // session-management → report generation bump
  compact(instructions?: string): Promise<void>;                 // session-management (no gen bump)
  // beforeToolCall is wired in the constructor as the approval gate.
}
```

## Acceptance criteria

- [x] `PiSession` creates an `AgentSession` in-process, sends a prompt via `sendUserMessage`, receives the typed event stream.
- [x] `beforeToolCall` is wired and can block/approve a tool call.
- [x] All §4 actions implemented: prompt, cancel, getState, getEntries, setModel, setThinkingLevel, getAvailableModels, newSession, compact.
- [x] `transcript_projection` maps `AgentSessionEvent`s to typed `TranscriptEvent`s with stable dedup ids.
- [x] `TranscriptEventLog` dedups by eventId and replays `forSession`.
- [x] Smoke test drives a real `AgentSession` end-to-end (prompt → events → transcript → approval gate fires).

## Notes

- **Approval gate**: `beforeToolCall` is a direct typed async hook. For v0.1.0 minimal slice, this may stub (auto-proceed with audit) and add the full Elicitation loop (opens an `approval-response` Elicitation the operator answers) as a follow-on — but that's an explicit documented scope cut, not a silent gap.
- **Harvest fidelity**: the outpost-pi session layer was built as a Pi extension; re-housing as a direct `createAgentSession()` caller is real work, not a copy. The projection logic (event → TranscriptEvent, dedup, replay) harvests; the `ExtensionFactory`/`ExtensionAPI` wiring does not.
- This story is independent of Unit 1 (different write set: `pi-adapter/` vs `server/`+`contracts/`). They can run in parallel.
- Requires `@earendil-works/pi-coding-agent` as a dependency (the SDK the adapter hosts).

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; selected by the caller for the SDK lifecycle and approval-boundary risk.
- Review weight: `standard` (caller).
- Files changed: `pi-adapter/package.json`, lockfile/TypeScript config, `src/pi_session.ts`, harvested transcript/turn modules, focused tests, and `.gitignore` build-output entries.
- Tests added/removed: added a real in-process `createAgentSession()` smoke test using Pi's faux provider. It proves prompt/event capture, both blocked and approved tool calls, transcript replay, model/thinking/query surfaces, generation bump, and cancel; added a focused stable-id/dedup replay test. No tests removed.
- Simplification: harvested only the typed event projection, append-only log, and turn reducer; all `ExtensionFactory`/peer/room transport wiring was discarded. The adapter drives `AgentSession` directly.
- Discrepancies from design: installed Pi 0.80.7 exposes the tool gate as the public `session.agent.beforeToolCall` hook and events as `session.subscribe(...)`, not `session.beforeToolCall` / `session.on(...)`. Entries are exposed by `session.sessionManager.getEntries()` rather than `session.getEntries()`. The driver wraps those current typed APIs behind the designed `PiSession` surface. Full approval Elicitation round-trip is the explicit minimal-slice cut: the default gate auto-proceeds while recording the request in the transcript; Unit 3 can install an async approval handler, and the smoke test proves both allow and block decisions.
- Verification: `npm install --cache /home/agent/projects/patchbay/.npm-cache`, `npm run build`, and `npm test` passed under Node with a real `AgentSession`.
- Adjacent issues parked: none.

### Review-response implementation update

- Replaced the same-object `SessionManager.newSession()`/agent reset with Pi 0.80.7's programmatic `AgentSessionRuntime.newSession()` lifecycle. The old `AgentSession` is disposed (including extension-context invalidation), a fresh runtime/session is created, and approval/event hooks are rebound before Patchbay reports the generation bump.
- Event and approval bindings capture an immutable generation and become inert synchronously during replacement; persisted `SessionManager.getEntries()` values can now be projected into the partial `TranscriptEventLog` snapshot for reconnect replay.
- Regression coverage: the real Pi smoke test verifies the Pi-internal session id changes and a delayed response from the disposed context cannot be labeled as generation 2; snapshot projection is exercised by the reconnect e2e.
