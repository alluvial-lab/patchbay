---
source_handle: fastify-websocket
fetched: 2026-07-07
source_url: https://raw.githubusercontent.com/fastify/fastify-websocket/master/README.md
provenance: source-direct
---

# @fastify/websocket README

## Structural metadata

- Source type: official package README in the Fastify GitHub organization.
- Fetched representation: raw Markdown.
- Local fetched copy: `.research/fetched/v0-stack-tooling/ts-web-and-browser/fastify-websocket.txt`.

## Paraphrased source summary

`@fastify/websocket` adds WebSocket support to Fastify using `ws`. It allows WebSocket handling on Fastify routes, can use hooks before upgrade for authentication and request processing, and states limitations after a connection is upgraded.

## Key passages

1. The README says `@fastify/websocket` provides WebSocket support for Fastify and is built on `ws@8`.

2. Usage registers the plugin and marks a `.get` route with `{ websocket: true }`, whose handler receives the WebSocket connection and Fastify request object.

3. The README says WebSocket route handlers must attach event handlers synchronously during handler execution to avoid dropping messages that arrive before handlers are attached.

4. Routes registered with the plugin respect Fastify plugin encapsulation contexts.

5. The README says hooks that run before WebSocket connection establishment are called, including `onRequest`, `preParsing`, `preValidation`, and `preHandler`, and those hooks can be used for authentication or other request-level processing.

6. The README says response serialization/transmission hooks such as `preSerialization` and `onSend` do not run for WebSocket routes after upgrade because message handling is outside Fastify's HTTP lifecycle.

7. The plugin uses the same router as Fastify; WebSocket route handlers follow the usual request lifecycle for hooks, error handlers, and decorators, but the plugin must be registered before all routes to intercept WebSocket connections.
