---
source_handle: mdn-sse-browser-events
fetched: 2026-07-07
source_url: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
provenance: source-direct
---

# Attestation: MDN — using server-sent events

## Summary

MDN describes server-sent events as a browser API for streaming events from a server to the front end using `EventSource`. It states that SSE is a one-way connection, so clients cannot send events to the server through that channel. It documents receiving server messages, named events, the `text/event-stream` MIME type, automatic restart behavior, and HTTP/1.x connection-count limitations that are relaxed when using HTTP/2 streams.

## Key passages

1. The MDN page says developing a web application with server-sent events needs server code to stream events to the front end, and client-side handling works similarly to websockets for incoming events.

2. It states: "This is a one-way connection, so you can't send events from a client to a server."

3. The page says the server-sent event API is contained in the `EventSource` interface.

4. It says creating an `EventSource` opens a connection to the server to begin receiving events from it.

5. The page says messages without an `event` field are received as `message` events, while messages with an `event` field are received as named events.

6. It notes that when not used over HTTP/2, SSE has a per-browser/per-domain maximum open-connection limit that can be painful with multiple tabs, and that with HTTP/2 the maximum simultaneous HTTP streams is negotiated between server and client.

7. The server-side section says the script should respond using MIME type `text/event-stream` and each notification is a block of text terminated by a pair of newlines.

8. The page says that by default, if the client-server connection closes, the connection is restarted, and it can be terminated with `.close()`.
