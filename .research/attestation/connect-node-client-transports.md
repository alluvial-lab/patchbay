---
source_handle: connect-node-client-transports
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/node/using-clients/
provenance: source-direct
---

# Attestation: Connect for Node — client transports

## Summary

The Connect for Node client documentation says Node.js uses the same clients as Connect for Web, with transports from `@connectrpc/connect-node`. It documents Connect, gRPC, and gRPC-Web transports. It states that Node transports use Node's built-in `http`, `https`, and `http2` modules rather than the Fetch API; with HTTP/2, clients can use Connect, gRPC, or gRPC-Web and call all RPC types; with HTTP/1.1, gRPC and bidirectional streaming are not supported.

## Key passages

1. The page states: "On Node.js, you use the same clients as you do with Connect for Web, but with a transport from `@connectrpc/connect-node` instead of from `@connectrpc/connect-web`."

2. Its example imports `createClient` from `@connectrpc/connect`, a service definition, and `createConnectTransport` from `@connectrpc/connect-node`, then constructs a client with `createClient(ElizaService, transport)` and calls `client.say(...)`.

3. The page says the `@connectrpc/connect-node` transports use built-in Node modules `http`, `https`, and `http2` instead of the Fetch API.

4. It states: "With HTTP/2, clients can use the Connect, gRPC, or gRPC-Web protocol, and call all types of RPCs. With HTTP 1.1, the gRPC protocol and bidirectional streaming are not supported."

5. The Connect section says `createConnectTransport()` creates a transport for the Connect protocol and shows the `httpVersion: "2"` option.

6. The gRPC section says `createGrpcTransport()` creates a transport for the gRPC protocol and notes: "The gRPC transport requires HTTP/2."

7. The gRPC-web section says `createGrpcWebTransport()` creates a transport for the gRPC-web protocol and says Connect for Node and connect-go support gRPC-web out of the box.
