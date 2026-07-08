---
source_handle: connect-es-web
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/web/getting-started/
additional_source_urls:
  - https://connectrpc.com/docs/web/using-clients/
  - https://connectrpc.com/docs/web/choosing-a-protocol/
  - https://connectrpc.com/docs/web/cancellation-and-timeouts/
  - https://raw.githubusercontent.com/connectrpc/connect-es/main/packages/connect-web/README.md
provenance: source-direct
---

# Connect for Web documentation

## Structural metadata

- Source type: official Connect documentation and Connect-ES package README.
- Fetched representation: documentation HTML rendered to text with `lynx`, plus raw Markdown README.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/connect-web-*.txt` and `connect-web-readme.txt`.

## Paraphrased source summary

Connect for Web supports type-safe browser clients generated from Protocol Buffers, with transports for Connect and gRPC-web protocols. Browser streaming is limited by browser request-stream support: server-streaming is supported, but client streaming is not generally available. The clients expose promise-based and callback-based APIs, including async iterables for server-streaming RPCs. Transports are fetch-based and can use custom fetch functions for credentials.

## Key passages

1. The Getting Started guide says Connect can call remote procedures from a web browser and provides a type-safe client without having to think about serialization.

2. The guide says the Connect protocol supports all streaming RPC types, but browsers do not support streaming from the client side across the board, so browser streaming can be used only for server streaming.

3. The guide installs `@connectrpc/connect` and `@connectrpc/connect-web` with generated client packages.

4. The Using Clients page says `createClient` gives a client using ECMAScript promises and `await`.

5. For server-streaming RPCs, the promise client method returns an async iterable stream usable with `for await...of`.

6. The callback client supports server-streaming RPCs with one callback called for each response message and one called at stream end.

7. The Using Clients page shows creating a shared `createConnectTransport` from `@connectrpc/connect-web` and a React hook that memoizes `createClient` for a service.

8. The Choosing a Protocol page says `@connectrpc/connect-web` supports both the Connect protocol and gRPC-web protocol.

9. `createConnectTransport()` uses the Fetch API for network operations, with options including `baseUrl`, `useBinaryFormat`, `interceptors`, `useHttpGet`, custom `fetch`, and `jsonOptions`.

10. The docs recommend JSON format for web browsers because browser network inspectors can show what is sent over the wire.

11. `createGrpcWebTransport()` creates a gRPC-web transport; the page says Connect for Node and connect-go support gRPC-web out of the box.

12. The custom fetch option can include browser credentials, with an example `fetch: (input, init) => fetch(input, {...init, credentials: "include"})`.

13. The cancellation/timeouts page says client-specified timeouts are sent in a request header understood by Connect, gRPC, and gRPC-web servers and can help streaming calls in fragile networks.
