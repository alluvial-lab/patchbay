---
source_handle: connect-es-node
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/node/server-plugins/
additional_source_urls:
  - https://connectrpc.com/docs/node/using-clients/
  - https://raw.githubusercontent.com/connectrpc/connect-es/main/packages/connect-node/README.md
provenance: source-direct
---

# Connect-ES Node documentation

## Structural metadata

- Source type: official Connect documentation and Connect-ES package README.
- Fetched representation: documentation HTML rendered to text with `lynx`, plus raw Markdown README.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/connect-node-*.txt`.

## Paraphrased source summary

Connect for Node.js provides transports for Node clients and adapters/plugins for serving Connect RPCs. Its server-plugin documentation names vanilla Node.js, Fastify, Next.js, and Express integrations. The Fastify integration uses `@connectrpc/connect-fastify`. The Node client transport uses Node's built-in `http`, `https`, and `http2` modules.

## Key passages

1. The Node client docs say Node.js uses the same clients as Connect for Web, but with a transport from `@connectrpc/connect-node` instead of `@connectrpc/connect-web`.

2. The Node docs say transports from `@connectrpc/connect-node` use built-in Node modules `http`, `https`, and `http2` instead of the Fetch API.

3. The Node docs say HTTP/2 clients can use Connect, gRPC, or gRPC-Web and call all RPC types, while HTTP/1.1 does not support the gRPC protocol or bidirectional streaming.

4. The `@connectrpc/connect-node` README says `createConnectTransport()` lets Node.js clients talk to a server with the Connect protocol and shows `httpVersion: "1.1"` as an option.

5. The server-plugins docs say `connectNodeAdapter()` runs Connect RPCs on Node built-in HTTP modules and responds 404 for unmatched routes unless a fallback is supplied.

6. The server-plugins docs say Fastify is recommended if serving other things along with Connect RPCs, and shows installation of `fastify`, `@connectrpc/connect`, `@connectrpc/connect-node`, and `@connectrpc/connect-fastify`.

7. The Fastify server plugin example registers `fastifyConnectPlugin` with routes and optional interceptors.

8. The Fastify server-plugin section says over HTTP/2 Fastify can serve Connect, gRPC, and gRPC-Web with all RPC types; over HTTP/1.1, gRPC and bidirectional streaming are not supported.

9. The server-plugins page navigation and content list first-party Node framework integrations as Vanilla Node.js, Fastify, Next.js, and Express; no Hono, Elysia, or Oak integration appears in the fetched section.
