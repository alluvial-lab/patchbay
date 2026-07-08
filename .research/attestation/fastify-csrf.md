---
source_handle: fastify-csrf
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/fastify/csrf-protection/main/README.md
provenance: source-direct
---

# @fastify/csrf-protection README

## Structural metadata

- Source type: official package README in the Fastify GitHub organization.
- Fetched representation: raw Markdown.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/fastify-csrf.txt`.

## Paraphrased source summary

`@fastify/csrf-protection` is a Fastify utility plugin for CSRF protection. It supports storage of CSRF secrets in cookies, `@fastify/session`, or `@fastify/secure-session`, exposes `reply.generateCsrf()` and `fastify.csrfProtection`, and allows token extraction from body or common CSRF headers. The README explicitly tells developers not to treat the plugin as sufficient without understanding and configuring CSRF mitigations.

## Key passages

1. The README says the plugin helps protect Fastify servers against CSRF attacks and tells developers to study the OWASP Cross-Site Request Forgery Prevention Cheat Sheet in depth.

2. The security disclaimer says CSRF security is the developer's responsibility, third-party modules should not be fully trusted, and the plugin provides utilities developers can use to secure an application.

3. With `@fastify/cookie`, the CSRF secret is added to response cookies; by default the cookie is named `_csrf`, and `cookieOpts` override defaults, with a warning to restore secure defaults if overriding.

4. With `@fastify/session`, the CSRF secret is added to the session; the default session key is `_csrf`, and `sessionKey` can rename it.

5. The session example registers a session plugin, registers CSRF protection with `sessionPlugin: '@fastify/session'`, generates a token with `reply.generateCsrf()`, and protects a POST route by adding `onRequest: fastify.csrfProtection`.

6. The README's secret guidance says secrets should never be hard-coded or committed, should be stored in KMS/Vault-like services, read at runtime, be significant in length, and be truly random; it also says HTTPS is extremely important.

7. The options table includes `cookieKey`, `cookieOpts`, `sessionKey`, `getToken`, `getUserInfo`, `sessionPlugin`, `csrfOpts`, and `logLevel`.

8. `fastify.csrfProtection` can be used as a hook to protect routes or plugins; the README generally recommends `onRequest`, but says body-token use requires `preValidation` or `preHandler`.

9. The default `getToken` checks `req.body._csrf`, `csrf-token`, `xsrf-token`, `x-csrf-token`, and `x-xsrf-token`; the README recommends a custom `getToken` for performance and security reasons.
