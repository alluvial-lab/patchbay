---
source_handle: fastify-session
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/fastify/session/main/README.md
provenance: source-direct
---

# @fastify/session README

## Structural metadata

- Source type: official package README in the Fastify GitHub organization.
- Fetched representation: raw Markdown.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/fastify-session.txt`.

## Paraphrased source summary

`@fastify/session` is a Fastify session plugin that requires `@fastify/cookie`, stores session data server-side in a configured session store, exposes a `request.session` object, signs the session cookie with a secret, and supports configurable cookie attributes, stores, session regeneration, reload, save, and destruction.

## Key passages

1. The README says `@fastify/session` is a Fastify session plugin and requires `@fastify/cookie`.

2. Usage registers `@fastify/cookie` and then `@fastify/session` with a secret of minimum length 32 characters.

3. The plugin decorates requests with `sessionStore` and a `session` object; it says session data is stored server-side using the configured session store.

4. The required `secret` signs the cookie and must be an array of strings, a string length 32 or greater, or a custom signer; arrays can be used to rotate signing secrets.

5. The optional `cookieName` defaults to `sessionId`.

6. The `cookie` option generates the session cookie's `Set-Cookie` header and includes `path` default `/`, `httpOnly` default true, `secure` default true with optional `auto`, `sameSite`, `domain`, `expires`, `maxAge`, and experimental `partitioned`.

7. The README says if HTTPS is terminated at a reverse proxy, Fastify's `trustProxy` setting is needed to use secure cookies.

8. The store interface requires `set`, `get`, and `destroy`; the default is a simple in-memory store, and the README warns it should not be used in production because it leaks memory.

9. `saveUninitialized` defaults to true, and `rolling` defaults to true.

10. `Session#regenerate` generates a new `sessionId` and persists it to the store; `Session#destroy` destroys the session in the store; `reload`, `save`, `get`, and `set` are also provided.
