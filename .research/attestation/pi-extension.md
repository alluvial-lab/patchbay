---
source_handle: pi-extension
fetched: 2026-07-04
source_path: /home/agent/projects/remote_pi/pi-extension/src/
provenance: source-direct
---

# Pi extension — action surface (grounded via remote_pi)

Pi's operator/agent/harness action surface is grounded via remote_pi's `pi-extension` source (a Pi SDK extension that hooks Pi events and issues control actions). The extension is at `/home/agent/projects/remote_pi/pi-extension/`.

## Outbound (agent→operator events — pi.on hooks)

From `pi-extension/src/index.ts` — the extension subscribes to these Pi events:
- `turn_start`, `turn_end` — session working/idle
- `message_update`, `message_end` — streaming + final message content
- `tool_execution_start`, `tool_execution_end` — tool-call lifecycle (the approval gate fires via `tool_call`)
- `tool_call` — tool-call request (the approval surface)
- `model_select`, `thinking_level_select` — reconfiguration
- `session_before_compact`, `session_compact` — compaction lifecycle
- `agent_end` — agent/turn completion
- `input`, `resources_discover` — input + resource discovery

## Inbound (operator→agent control — ClientMessage types)

From `pi-extension/src/protocol/generated/protocol.generated.ts` (CLIENT_MESSAGE_TYPES):
- `user_message` — drive (send prompt/content)
- `approve_tool` — approve a pending tool call (Request/gate)
- `cancel` — interrupt a running turn (Request/lifecycle-acting)
- `session_sync` — sync/refresh session state (Query)
- `session_new` — reset the session's conversation (Request/session-mgmt) — *does not spawn a new process*; `handleSessionNew` calls `ctx.newSession()` which resets the attached session's conversation
- `session_compact` — compact the session (Request/session-mgmt)
- `model_set`, `thinking_set` — reconfigure (Request)
- `list_models` — query available models (Query)
- `ping` — liveness query
- `pair_request`, `queued_message_set`, `queued_message_clear` — pairing/queue (transport, not agent control)

## Provisioning

From `pi-extension/src/bin/supervisord.ts` and `pi-extension/src/session/setup_wizard.ts`: `pi-supervisord` is a long-running daemon supervisor that spawns `pi --mode rpc` children, managed by systemd/launchd (`service-templates/` has launchd.plist, systemd.service, etc.). The setup wizard explicitly excludes daemon mode: "Daemon mode (run agents 24/7 via systemd/launchd) is intentionally NOT in the wizard — it's an explicit, separate opt-in via `/remote-pi install`." So provisioning is **out-of-band sysadmin**, not an operator action in the control API.

## No-grant informational replyable content

No operator-originated no-grant `Message` type — `user_message` is the drive action. Agent-originated question/elicitation is not a distinct Pi concept in the surveyed wire types (the `tool_call` event is the closest, but it's a tool-approval gate, not a free-form question).
