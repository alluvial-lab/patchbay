---
source_handle: hono-docs
fetched: 2026-07-07
source_url: https://hono.dev/docs/concepts/middleware
additional_source_urls:
  - https://hono.dev/docs/helpers/cookie
  - https://hono.dev/docs/middleware/builtin/csrf
  - https://hono.dev/docs/helpers/websocket
  - https://hono.dev/docs/helpers/streaming
provenance: source-direct
---

# Hono documentation

## Structural metadata

- Source type: official Hono documentation pages.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/hono-*.txt`.

## Paraphrased source summary

Hono documents an onion-style middleware model around Web-standard handlers, cookie helpers, an Origin/Fetch-Metadata CSRF middleware, WebSocket helpers for multiple adapters, and streaming helpers including SSE.

## Key passages

1. Hono calls the primitive that returns a `Response` a "Handler" and says middleware executes before and after the handler, handling the `Request` and `Response` in an onion structure.

2. The Cookie Helper page says it provides an interface to set, parse, and delete cookies. It exposes `getCookie`, `getSignedCookie`, `setCookie`, `setSignedCookie`, `generateCookie`, and `generateSignedCookie` from `hono/cookie`.

3. Hono's cookie options for `setCookie` and `setSignedCookie` include `domain`, `expires`, `httpOnly`, `maxAge`, `path`, `secure`, `sameSite`, `priority`, `prefix`, and `partitioned`.

4. Hono's Cookie Helper says it supports `__Secure-` and `__Host-` prefixes, and that parsing throws when a `__Host-` cookie lacks `secure`, lacks `path=/`, or has `domain` set.

5. Hono's CSRF middleware says it protects by checking both the `Origin` header and `Sec-Fetch-Site` header and allows a request if either validation passes.

6. Hono's CSRF middleware warns that old browsers without Origin headers, or reverse proxies that remove those headers, may not work well and says to use other CSRF token methods in such environments.

7. Hono's WebSocket Helper is described as server-side WebSockets in Hono applications with Cloudflare Workers/Pages, Deno, Bun, and Node.js adapters available. The Node.js example imports `serve` and `upgradeWebSocket` from `@hono/node-server` and requires `ws`.

8. The WebSocket Helper warns that middleware that modifies headers on a WebSocket route may conflict because `upgradeWebSocket()` modifies headers internally.

9. Hono's Streaming Helper says it provides streaming responses, including `stream`, `streamText`, and `streamSSE`; `streamSSE()` streams Server-Sent Events and the example loops while not aborted, writing event data, event name, and id.
