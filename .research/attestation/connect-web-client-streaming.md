---
source_handle: connect-web-client-streaming
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/web/using-clients/
provenance: source-direct
---

# Attestation: Connect for Web — client usage and server-streaming

## Summary

The Connect for Web client documentation describes generated promise and callback clients created from a service definition and transport. It states that server-streaming RPC methods return async iterable response streams for promise clients and use callbacks for response messages and stream completion for callback clients. It also says callback clients can help migrate existing gRPC-Web codebases.

## Key passages

1. The page says the `createClient` function gives a client that uses ECMAScript promise objects.

2. The example imports `createClient` from `@connectrpc/connect`, constructs `createClient(ElizaService, transport)`, and calls `client.say(...)` with `await`.

3. The page states: "For server-streaming RPCs, the corresponding method on the client will return an async iterable stream of response messages that can be used with the `for await...of` statement."

4. The callback-client section states that `createCallbackClient` returns a callback-based client.

5. For server-streaming RPCs with callback clients, the page says the corresponding method takes two callback functions: one called each time a response message arrives, and one called at the end of the stream.

6. The page says the callback client is useful for migrating an existing code base from gRPC-web to Connect clients.

7. The client-management section shows a React hook that creates a transport with `@connectrpc/connect-web`, memoizes `createClient(service, transport)`, and returns a typed client.
