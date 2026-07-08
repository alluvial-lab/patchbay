---
source_handle: opencode-server-docs-current
fetched: 2026-07-07
source_url: https://opencode.ai/docs/server/
provenance: source-direct
---

# Attestation: OpenCode server documentation

## Structural metadata

- Publisher/site: OpenCode documentation.
- Page title observed: Server | OpenCode.
- Source kind: server/API documentation.

## Paraphrased summary

OpenCode runs a TUI and server, where the TUI is a client talking to the server. The server supports multiple clients, exposes OpenAPI 3.1 documentation, can be started standalone with `opencode serve`, supports optional HTTP basic auth, exposes session/message/config/provider/project/file/tool/agent/logging/auth APIs, and streams events with SSE.

## Key passages

1. **Usage command.** The server page documents `opencode serve` as the server startup command. Source anchor: usage section, lines 80-120 in fetched HTML.

2. **Server options.** Options include `--port`, `--hostname`, `--mdns`, `--mdns-domain`, and repeatable `--cors`, with defaults shown in the options table. Source anchor: options table, lines 118-120 in fetched HTML.

3. **Authentication.** Setting `OPENCODE_SERVER_PASSWORD` protects the server with HTTP basic auth; username defaults to `opencode` or can be overridden with `OPENCODE_SERVER_USERNAME`, applying to both `opencode serve` and `opencode web`. Source anchor: authentication section, lines 122-124.

4. **TUI/server architecture.** The page says running `opencode` starts a TUI and a server, and the TUI is the client that talks to the server. Source anchor: how-it-works section, lines 126-129.

5. **Programmatic clients.** The page says this architecture supports multiple clients and programmatic interaction with OpenCode. Source anchor: how-it-works section, lines 127-132.

6. **Standalone server.** `opencode serve` starts a standalone server; if the TUI is running, `opencode serve` starts a new server. Source anchor: lines 131-133.

7. **Connect to existing server and TUI endpoint.** The page says TUI startup randomly assigns port/hostname unless flags override, and the `/tui` endpoint can drive the TUI, such as prefilling or running a prompt; IDE plugins use this setup. Source anchor: lines 135-137.

8. **OpenAPI spec.** The server publishes an OpenAPI 3.1 spec at `/doc`; clients can use it to generate clients or inspect request and response types. Source anchor: lines 139-142.

9. **Global event stream.** The global API table includes `GET /global/event` for global events as an SSE stream. Source anchor: lines 169-171.

10. **Sessions API.** The sessions table includes `GET /session` to list sessions and `POST /session` to create a new session. Source anchor: lines 315-435.

11. **Messages API.** The messages table includes `GET /session/:id/message` to list messages in a session. Source anchor: lines 437-485.

12. **Events API.** The events table includes `GET /event`, described as a server-sent events stream whose first event is `server.connected`, followed by bus events. Source anchor: lines 769-787.
