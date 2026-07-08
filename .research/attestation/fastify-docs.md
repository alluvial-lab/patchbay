---
source_handle: fastify-docs
fetched: 2026-07-07
source_url: https://fastify.dev/docs/latest/Reference/Plugins/
additional_source_urls:
  - https://fastify.dev/docs/latest/Reference/Hooks/
  - https://fastify.dev/docs/latest/Reference/Middleware/
provenance: source-direct
---

# Fastify documentation

## Structural metadata

- Source type: official Fastify documentation pages.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/fastify-Reference-*.txt`.

## Paraphrased source summary

Fastify documents an extensible Node.js server framework organized around plugins, encapsulation scopes, and lifecycle hooks. Plugins can add routes, decorators, or other functionality. Hooks give access to the request/reply lifecycle and can be scoped through Fastify's encapsulation model.

## Key passages

1. Fastify says it can be extended with plugins, which may be a set of routes, a server decorator, or other functionality, and that `register` is used to add plugins.

2. Fastify says `register` creates a new scope by default, so changes to the Fastify instance affect descendants but not the current context's ancestors; the docs describe this as plugin encapsulation and inheritance that forms a directed acyclic graph and avoids cross-dependency issues.

3. Fastify says creating a plugin is easy: make a function with `fastify`, an options object, and `done`, or use an async function. It notes `register` can be used inside another `register`.

4. Fastify says hooks are registered with `fastify.addHook` and allow code to listen to application or request/response lifecycle events.

5. Fastify lists request/reply hooks in execution order, including `onRequest`, `preParsing`, `preValidation`, `preHandler`, `preSerialization`, `onError`, `onSend`, `onResponse`, `onTimeout`, and `onRequestAbort`.

6. Fastify says hooks are affected by encapsulation and can be applied to selected routes via scopes.

7. Fastify's middleware reference identifies middleware as a reference topic but the core request lifecycle emphasis in the fetched docs is the hook system rather than Express-style middleware.
