---
id: story-v0-web-server-scaffold
kind: story
stage: implementing
tags: [security, protocol]
parent: feature-v0-web-server
depends_on: []
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Story: web-server scaffold + core client

The `patchbay-web-server` Fastify app skeleton and the gRPC client to the Rust core. No domain logic — just the process that terminates HTTP and can reach the core.

## Design (from feature-v0-web-server Unit 1)

**Files**: `web-server/package.json`, `web-server/tsconfig.json`, `web-server/src/main.ts`, `web-server/src/core-client.ts`

- Fastify app. Wires `createGrpcTransport()` + `createClient(ControlService, ...)` to a configurable core address, mirroring the spike's confirmed pattern (`spikes/connect-tonic-interop/ts/src/run.ts`).
- Reads config from env: `PATCHBAY_CORE_ADDR`, `PATCHBAY_CORE_SECRET`, `PATCHBAY_WEB_BIND_ADDR`, `PATCHBAY_TLS_CERT`/`PATCHBAY_TLS_KEY`, `PATCHBAY_OPERATOR_ID`.
- Fail-safe: refuses to start without `PATCHBAY_CORE_SECRET` and without a configured operator record (Q4 enrollment decision).
- A `createGrpcTransport` interceptor adds `x-patchbay-core-secret` to every core call (the web server authenticates as a principal).
- `GET /healthz` returns 200 without auth (the one unauthenticated route).

```typescript
// core-client.ts
import { createClient } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { ControlService } from "@patchbay/contracts";

export function makeCoreClient(coreAddr: string, coreSecret: string) {
  const transport = createGrpcTransport({
    baseUrl: coreAddr,
    interceptors: [coreSecretInterceptor(coreSecret)],
  });
  return createClient(ControlService, transport);
}
```

## Acceptance criteria

- [ ] `patchbay-web-server` starts, binds a Fastify listener (HTTP or HTTPS per TLS config), refuses to start without `PATCHBAY_CORE_SECRET`.
- [ ] `GET /healthz` returns 200 without auth.
- [ ] The core client can reach a running `patchbay-core-server` (verified by a smoke test that calls `LoadSnapshot` or `Subscribe` and gets a response, not an auth error).

## Notes

- Reuses `@patchbay/contracts` (the existing `contracts/ts` package) for the `ControlService` client — no new contract generation.
- Mirror the spike's `createGrpcTransport` usage exactly (it's verified-correct against tonic).
- `CARGO_HOME`/npm cache: npm needs `--cache /home/agent/projects/patchbay/.npm-cache` (the `~/.npm` cache is read-only in this sandbox); see the spike's environment notes in `story-connect-node-tonic-interop-spike.md`.
