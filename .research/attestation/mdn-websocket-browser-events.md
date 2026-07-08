---
source_handle: mdn-websocket-browser-events
fetched: 2026-07-07
source_url: https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API
provenance: source-direct
---

# Attestation: MDN — WebSocket API

## Summary

MDN describes the WebSocket API as enabling a two-way interactive communication session between a browser and a server. It says the stable `WebSocket` interface has broad support but does not support backpressure, so fast-arriving messages can fill memory or cause high CPU/unresponsiveness. It describes `WebSocketStream` as using the Streams API for backpressure but as non-standard and currently limited to one rendering engine.

## Key passages

1. The page states that the WebSocket API makes it possible to open "a two-way interactive communication session between the user's browser and a server."

2. It says that with this API, a browser can send messages to a server and receive responses without polling.

3. The page says the WebSocket API provides `WebSocket` and `WebSocketStream` mechanisms.

4. It states that the `WebSocket` interface is stable and has good browser and server support.

5. It says the `WebSocket` interface does not support backpressure.

6. It says that when messages arrive faster than the application can process them, the application may fill device memory by buffering messages, become unresponsive due to 100% CPU, or both.

7. It says `WebSocketStream` uses the Streams API so socket connections can take advantage of stream backpressure automatically.

8. It states that `WebSocketStream` is non-standard and currently supported in only one rendering engine.
