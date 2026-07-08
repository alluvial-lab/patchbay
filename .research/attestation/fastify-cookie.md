---
source_handle: fastify-cookie
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/fastify/fastify-cookie/main/README.md
provenance: source-direct
---

# @fastify/cookie README

## Structural metadata

- Source type: official package README in the Fastify GitHub organization.
- Fetched representation: raw Markdown.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/fastify-cookie.txt`.

## Paraphrased source summary

`@fastify/cookie` is a Fastify plugin for reading and setting cookies. It parses cookies through a Fastify hook, can sign cookies, and exposes options that map to `Set-Cookie` attributes including `Domain`, `Path`, `HttpOnly`, `SameSite`, and `Secure`.

## Key passages

1. The README says `@fastify/cookie` is a plugin for Fastify that adds support for reading and setting cookies.

2. The README says cookie parsing works via Fastify's `onRequest` hook and should be registered before other `onRequest` hooks that depend on it.

3. The example registration accepts `secret` for cookie signatures, `hook` for selecting the parsing hook, and `parseOptions` for cookie parsing.

4. The README says the `secret` option can be a string, array, buffer, or object, and an array can be used for key rotation.

5. Under security considerations, the README recommends `sha256` or stronger and a secret at least 20 bytes long.

6. The `domain` option sets the `Domain` attribute; by default no domain is set and most clients consider the cookie to apply only to the current domain.

7. The `httpOnly` option sets the `HttpOnly` attribute when truthy, but the default is not to set it; the README notes compliant clients will not let client-side JavaScript see the cookie in `document.cookie` when it is true.

8. The `sameSite` option accepts `true` for Strict, `false` for no attribute, `'lax'`, `'none'`, or `'strict'`.

9. The `secure` option sets the `Secure` attribute when truthy, but the default is not to set it; the README warns that when true, compliant clients will not send the cookie back without HTTPS.
