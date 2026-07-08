---
source_handle: elysia-docs
fetched: 2026-07-07
source_url: https://elysiajs.com/at-glance.md
additional_source_urls:
  - https://elysiajs.com/essential/plugin.md
  - https://elysiajs.com/essential/life-cycle.md
  - https://elysiajs.com/patterns/cookie.md
  - https://elysiajs.com/patterns/websocket.md
  - https://elysiajs.com/plugins/jwt.md
  - https://elysiajs.com/plugins/cors.md
provenance: source-direct
---

# Elysia documentation

## Structural metadata

- Source type: official Elysia documentation Markdown pages.
- Fetched representation: Markdown served by `elysiajs.com`.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/elysia-*.txt`.

## Paraphrased source summary

Elysia describes itself as an ergonomic TypeScript backend framework optimized for Bun, with platform-agnostic deployment claims, plugin/lifecycle mechanisms, reactive cookies, cookie signatures and rotation, WebSocket support via `Elysia.ws()`, JWT and CORS plugins, and Bun-centered installation examples.

## Key passages

1. The at-a-glance page says Elysia is an ergonomic web framework for building backend servers with Bun, designed for simplicity and type safety, optimized for Bun.

2. The at-a-glance page says Elysia is optimized for Bun but not limited to Bun; it says WinterTC compliance allows deployment on Bun, Node.js, Deno, and Cloudflare Worker.

3. The Cookie page says Elysia provides a mutable signal for interacting with cookies, where values can be read and updated directly, and that cookie changes update response headers automatically.

4. The Cookie page says cookie attributes can be updated by setting properties directly or with `set`/`add`; examples set `domain` and `httpOnly`.

5. The Cookie page says cookie signatures append a cryptographic hash generated with a secret key to verify authenticity and integrity, and that `t.Cookie` can automatically sign and unsign values when configured.

6. The Cookie page says Elysia handles cookie secret rotation automatically, including an array of `cookie.secrets`, and supports transition from unsigned to signed cookies with a warning to limit unsigned cookies to the transition period.

7. The Cookie config says `domain` defaults to no domain, `httpOnly` defaults to false, `sameSite` accepts `true` for Strict, `false`, `'lax'`, `'none'`, or `'strict'`, and `secure` is a boolean that is not set by default.

8. The WebSocket page says WebSocket is a real-time protocol and Elysia uses uWebSocket, which Bun uses under the hood, with the same API; WebSocket support is exposed by calling `Elysia.ws()`.

9. Elysia WebSocket schema validation can validate incoming message, query, params, header, cookie, and response, and incoming stringified JSON messages are parsed as objects by default for validation.

10. The JWT plugin page says the plugin adds JWT support in Elysia handlers and shows an example setting a JWT in a cookie with `httpOnly: true`.

11. The fetched Elysia docs include cookie, JWT, bearer, and CORS plugins, but no official server-side session plugin or CSRF plugin surfaced in the fetched pages.
