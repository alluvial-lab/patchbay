---
id: story-v0-web-server-rpc-bridge
kind: story
stage: done
tags: [security, protocol]
parent: feature-v0-web-server
depends_on: [story-v0-web-server-sessions]
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
research_origin: null
---

# Story: web-server Connect-Web RPC bridge + integration tests

The browser↔core translation layer: exposes the core's `ControlService` over Connect-Web (gRPC-Web) to the browser, forwarding the server-verified operator identity as gRPC metadata to the core. Plus the integration tests that prove the 4 `csrf_browser.qnt` promoted properties hold at the HTTP boundary.

## Design (from feature-v0-web-server Units 4, 5)

**Files**: `web-server/src/routes/rpc.ts`, `web-server/src/routes/csrf-token.ts`, `web-server/tests/integration.test.ts`

### RPC bridge (`routes/rpc.ts`)

Each browser RPC: (1) runs the CSRF/auth guard (state-changing ones only — already implemented in the sessions story), (2) reads the *server-verified* `operator-id` + `operator-session-id` from `req.verifiedOperator`/`req.verifiedSessionId`, (3) forwards them as gRPC metadata to the core (matching the seam's `MetadataIssuerContext` headers `x-patchbay-operator-id` + `x-patchbay-operator-session-id` + `x-patchbay-core-secret`), (4) proxies the call.

```typescript
// browser Connect-Web → core gRPC translation
app.post("/patchbay.ControlService/Submit", { preHandler: requireOperatorSession },
  async (req, reply) => {
    const result = await coreClient.submit(
      req.body.operation,
      { headers: {
        "x-patchbay-core-secret": coreSecret,
        "x-patchbay-operator-id": req.verifiedOperator,       // from session record
        "x-patchbay-operator-session-id": req.verifiedSessionId,
      } },
    );
    reply.send(result);
  });

// Subscribe: server-streaming passthrough (browser Connect-Web client receives
// the async iterable; the web server does not buffer or hold state).
app.post("/patchbay.ControlService/Subscribe", { preHandler: requireOperatorSession },
  async (req, reply) => {
    const stream = coreClient.subscribe(req.body, { headers: coreHeaders(req) });
    reply.hijack(); // stream the async iterable back as gRPC-Web frames
    for await (const ev of stream) reply.raw.write(encodeGrpcWebFrame(ev));
  });
```

### CSRF token route (`routes/csrf-token.ts`)

`GET /csrf-token` (auth-required, NOT CSRF-required — it *issues* the token) returns the session's CSRF token for SPAs that need to bootstrap after login.

### Integration tests (`tests/integration.test.ts`)

The verification-grade tests — these are the implementation evidence for the 4 promoted checked-model properties:

- [ ] `CsrfRejectsUnauthenticated`: state-changing request with no cookie → 401, no core call made.
- [ ] `CsrfRejectsMissingProof`: valid session, missing/wrong `X-Patchbay-CSRF` → 403, no core call made.
- [ ] `RevokedSessionCannotCommand`: a revoked session's cookie + valid CSRF → 403, no core call made.
- [ ] `browser_local_state_not_authority`: a request whose browser-supplied operator-id differs from the session record's operator-id still uses the *session record's* operator-id at the core (verified by asserting the forwarded metadata reaches the core with the server-record value).

## Acceptance criteria

- [ ] A browser Connect-Web `Submit` through the web server reaches the core and returns the `SubmissionResult`; unauthenticated → 401, bad CSRF → 403.
- [ ] `Subscribe` streams events from core to browser; reconnect with a new cursor resumes.
- [ ] The `operator-id`/`operator-session-id` forwarded to the core come from the server's session record, not the browser request body.
- [ ] The 4 `csrf_browser.qnt` property integration tests pass (listed above).
- [ ] `GET /csrf-token` returns the session's CSRF token (auth-required, no CSRF-required).

## Notes

- The browser Connect-Web client speaks gRPC-Web to the web server; the web server uses `@connectrpc/connect-node` gRPC to the core. Types are shared via `@patchbay/contracts`.
- **Highest risk (flagged in the feature Risks section)**: streaming gRPC events back to the browser as gRPC-Web frames requires manual frame encoding (`reply.hijack()` + gRPC-Web framing in Fastify). The v0-stack-tooling research noted Fastify lacks a first-party SSE helper and Connect-Web server-side fit was a deferred question. If gRPC-Web framing in Fastify proves too costly, fall back to SSE for the event stream (still server-streaming, still typed via Connect-Web for the unary RPCs). This is the one place Q2a (Connect-Web end-to-end) could realistically force a hybrid (Q2c). Spike the streaming-bridge shape early in this story.
- The `x-patchbay-csrf` header is checked by the guard (sessions story) BEFORE this handler runs; the RPC bridge only does metadata forwarding + transport translation.
- `LoadSnapshot` (read) requires auth but NOT CSRF (per SECURITY.md:112).

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; the same feature-owning worker retained the transport and safety context. Direct-read only; no delegation.
- Review weight: `standard` (caller).
- Files changed: `web-server/package.json`; `web-server/package-lock.json`; `web-server/src/main.ts`; `web-server/src/routes/rpc.ts`; `web-server/src/routes/csrf-token.ts`; `web-server/tests/integration.test.ts`.
- Tests added/removed: six HTTP/actual-Connect-Web integration tests cover each of the four promoted CSRF properties, unary framing, metadata forwarding, sender re-stamping, CSRF-token issuance, auth-only snapshot reads, server-streaming frames, and cursor-based reconnect. Rejection tests assert zero core calls; revoked records remain present.
- Simplification: the generated `ControlService` schemas are the only DTO source; the bridge contains one small binary gRPC-Web frame codec and directly proxies the generated messages. No second browser contract or buffering/event state was introduced.
- Discrepancies from design: the early streaming spike succeeded with Fastify `reply.hijack()` and standards-shaped binary data/trailer frames, verified through the real `@connectrpc/connect-web` client, so the documented SSE fallback was not needed. `Subscribe` is treated as an authenticated read/subscription establishment and therefore does not require CSRF; `Submit` remains CSRF-protected. Submit also replaces the payload sender actor with the server-record actor before forwarding, in addition to authoritative metadata forwarding.
- Adjacent issues parked: none.
- Verification: `npm install`, `npm run build`, 15 tests, the real-core authenticated LoadSnapshot smoke, `npm audit` (0 vulnerabilities), and `cargo build -p patchbay-core-server` all pass.
