---
source_handle: oak-docs
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/oakserver/oak/main/README.md
additional_source_urls:
  - https://oakserver.org/
provenance: source-direct
---

# oak documentation

## Structural metadata

- Source type: official oak README and homepage.
- Fetched representation: raw Markdown README plus homepage HTML rendered to text.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/oak-*.txt`.

## Paraphrased source summary

Oak is a middleware framework and router originally for Deno and now documented for Deno, Node.js, Cloudflare Workers, and Bun. It offers an application/context middleware stack, cookies with signing when application keys are configured, server-sent events via `sendEvents()`, and WebSocket upgrades via `upgrade()`. The README notes limitations for non-Deno runtimes.

## Key passages

1. The README says oak is a middleware framework for Deno's native HTTP server, Deno Deploy, Node.js 16.5 and later, Cloudflare Workers, and Bun, and includes a middleware router.

2. The README says the examples target Deno CLI or Deno Deploy and recommends pinning a version for actual workloads.

3. For Node.js usage through JSR, the README notes that Send, WebSocket upgrades, and serving over TLS/HTTPS are not currently supported.

4. For Cloudflare Workers and Bun usage, the README notes that Send and WebSocket upgrades are not currently supported.

5. The README says `Application` coordinates the HTTP server, middleware, and errors; middleware is added with `.use()` and `.listen()` starts the server.

6. The middleware stack passes a context and `next` method to each middleware and lets each middleware control response flow.

7. Application `.keys` are used for signing and verifying cookies; an array of keys can be managed through `KeyStack`, which allows key rotation without re-signing data values.

8. `context.cookies` allows reading request cookies and setting response cookies, and automatically secures cookies if application `.keys` is set; the APIs are asynchronous because signing and validation use Web Crypto.

9. `context.sendEvents()` converts the current connection into a server-sent event response and returns a `ServerSentEventTarget` for streaming messages and events to the client.

10. `context.upgrade()` attempts to upgrade the connection to a WebSocket connection and returns a `WebSocket` interface.

11. The fetched README does not describe a built-in server-side session plugin or built-in CSRF token middleware.
