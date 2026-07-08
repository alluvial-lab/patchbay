---
source_handle: codex-appserver-current
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/openai/codex/main/codex-rs/app-server/README.md
provenance: source-direct
---

# Attestation: Codex app-server README

## Structural metadata

- Publisher/site: OpenAI Codex GitHub repository raw README.
- Source path in repository: `codex-rs/app-server/README.md`.
- Source kind: app-server protocol and API documentation.

## Paraphrased summary

`codex app-server` is a rich-client interface for Codex. It uses bidirectional JSON-RPC-like messaging over stdio, websocket, or Unix socket transports. The README defines thread/turn/item primitives, thread lifecycle and read methods, turn start/steer/interrupt methods, item/turn notification streams, server-initiated approvals and user input requests, standalone command/process execution utilities, remote-control status/pairing controls, and generated schema output.

## Key passages

1. **Purpose.** The README says `codex app-server` is the interface Codex uses to power rich interfaces such as the Codex VS Code extension. Source anchor: line 3.

2. **Transport/protocol.** It supports bidirectional communication using JSON-RPC 2.0-like messages, with transports including stdio, websocket, Unix socket, and off. Source anchor: lines 22-29.

3. **Local control-plane socket.** The Unix socket transport is intended for local app-server control-plane clients and `codex app-server proxy` connects stdin/stdout to that socket. Source anchor: lines 39-41.

4. **Backpressure.** The server uses bounded queues between ingress, request processing, and outbound writes; saturated ingress returns retryable overload error `-32001`. Source anchor: lines 51-53.

5. **Version-specific schema generation.** `generate-ts` and `generate-json-schema` outputs are specific to the Codex version used and guaranteed to match that version. Source anchor: lines 57-61.

6. **Primitives.** Thread is a conversation, Turn is one turn usually starting with user message and finishing with agent message, and Item represents user inputs and agent outputs persisted for future context. Source anchor: lines 67-72.

7. **Thread/turn lifecycle overview.** `thread/start`, `thread/resume`, and `thread/fork` open or continue conversations; `turn/start` sends user input and returns a turn object; stream notifications report progress; `turn/interrupt` interrupts and final `turn/completed` reports final state/usage. Source anchor: lines 76-81.

8. **Thread APIs.** API overview entries include create/resume/fork/list/read, settings update, archive/unarchive, compact, shell command, status changes, and memory/goal changes. Source anchor: lines 140-170.

9. **Turn controls.** `turn/start` adds user input and begins Codex generation; `turn/steer` adds input to an in-flight regular turn; `turn/interrupt` requests cancellation by `(thread_id, turn_id)` and the turn finishes with `status: "interrupted"`. Source anchor: lines 171-174.

10. **Standalone process utilities.** `command/exec` runs a sandboxed command without a thread/turn; `process/spawn` experimentally spawns a standalone process without Codex sandbox on the app-server host and emits output/exited notifications. Source anchor: lines 181-191.

11. **Remote-control controls.** Experimental `remoteControl/enable`, `disable`, `status/read`, pairing start/status, client revoke, and status changed notification expose current status (`disabled`, `connecting`, `connected`, `errored`), server name, environment ID, pairing artifacts, and revocation. Source anchor: lines 222-229.

12. **Server-originated user input.** `tool/requestUserInput` prompts the user with 1–3 short questions for a tool call and returns answers. Source anchor: line 234.
