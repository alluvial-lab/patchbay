---
id: feature-v0-web-server
kind: feature
stage: done
tags: [security, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-protocol-seam]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-11
updated: 2026-07-15
---

# Feature: TypeScript web server

## Brief

Build the TypeScript web server that terminates HTTP/HTTPS for the browser cockpit. The web server is a control surface, not a core: it never writes the durable log or makes authority decisions. It owns operator sessions, cookies, and CSRF protection, and speaks the generated Protobuf/Connect contract to the Rust coordination core.

The web server is an authenticated endpoint/principal with respect to the core, subject to the same grant and audit rules as other control surfaces. It translates browser-facing requests into core protocol calls, and streams core events back to the browser. The browser runs the shared TypeScript operator domain (protocol client, delivery/reconnect state machines, presentation model) as a client of the web server.

v0.1.0 may run the web server as a thin HTTP→protocol translator with the operator domain executing only in the browser; promoting delivery/reconnect state machines or SSR to the server is a reserved seam.

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: on the phone-usable critical path, between the protocol seam and the web cockpit. The cockpit cannot run until the web server terminates HTTP and speaks to the core.

## Foundation references

- `docs/ARCHITECTURE.md` — v0.1.0 process topology (two-process split: Rust core + TS web server), reserved seams (server-side operator-domain reuse, web↔core internal protocol)
- `docs/SECURITY.md` — operator sessions, CSRF, web server as principal
- `docs/PROTOCOL.md` — authority grants, audit
- `docs/UX.md` — shared presentation-component layer (named seam, implementation deferred)
- `contracts/ts/` — generated TS bindings (the starting contract for the web server's types)
- `contracts/proto/patchbay/*.proto` — generated contract source
- Formal model: `csrf_browser.qnt` — `CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`

## Design decisions (operator-confirmed leans)

- **Operator-session store**: (a) in-memory map for v0.1.0. Single-operator localhost; the session record is server-side (satisfies `SECURITY.md:89`); restart = re-login, not data loss (no durable operator state lives in the session store — it's all in the core's log). SQLite-backed sessions are the natural promotion for split-deploy/multi-session, reserved. Honestly bounded as v0.1.0-only.
- **Browser↔web-server transport**: (a) Connect-Web (gRPC-Web) end-to-end. Single typed client stack from browser to core. The seam's server-streaming `Subscribe` maps directly to Connect-Web's browser server-streaming (no bidi needed, confirmed by research + the seam's server-streaming-only decision). The web server is a Connect-Web server that translates to gRPC to the core. REST/JSON doubles the contract surface and discards generated types.
- **CSRF mechanism**: (a) synchronizer-token. Server stores a per-session CSRF secret; browser sends the token in a custom header (`X-Patchbay-CSRF`) on every state-changing request; server compares before acceptance. This is the OWASP stateful pattern for session-backed auth AND matches the `csrf_browser.qnt` model exactly — the model's `csrfProofs: session -> proof` is a server-bound proof, and `CsrfRejectsMissingProof` asserts a missing/invalid proof is rejected before command acceptance. Double-submit is weaker against subdomain cookie injection and doesn't match the model's server-bound-proof shape. The session store from Q1 makes proof storage free.
- **Enrollment / first-operator bootstrap**: (a) CLI bootstrap, web server login-only. The web server has NO unauthenticated setup page — it trusts the operator record + grant already exist (created by `patchbay setup` in `feature-v0-cli`, or a pre-seeded record). Honors `SECURITY.md:77-78` and the lockdown-exit channel-distinction property (`SECURITY.md:208`). Dependency note: the web server refuses to start (or refuses logins) if no operator record is configured, rather than creating one. `feature-v0-cli` owns bootstrap.
- **TLS termination**: (a) direct termination with localhost exception. Fastify terminates TLS for non-localhost; localhost uses the browser's secure-cookie exception (`SECURITY.md:97` permits, forbids generalizing to LAN/IP/container). Reverse-proxy termination is a deployment variant, reserved.

## Architectural choice

**A Fastify web server (`patchbay-web-server`) that is a thin HTTP→protocol translator.** It owns exactly three concerns: (1) operator-session lifecycle (login, cookie, revocation), (2) CSRF synchronizer-token enforcement on every state-changing route, and (3) translation between browser Connect-Web calls and the core's gRPC `ControlService`. It owns NO domain logic — no delivery/reconnect state machines, no authority decisions, no durable writes (those stay in the browser operator domain and the core respectively, per the reserved-seam note in `ARCHITECTURE.md`).

This realizes the committed v0.1.0 two-process topology: Rust core (authority, no HTTP) + TS web server (HTTP termination, sessions, CSRF, speaks gRPC to core). The browser runs the shared TS operator domain as a client of the web server.

Chosen over:
- *Embedding operator-domain state machines server-side* — rejected: explicitly a reserved seam (`ARCHITECTURE.md` "server-side operator-domain reuse"); v0.1.0 keeps the operator domain in the browser. Promoting later is internal to the web server crate.
- *A second Connect contract for the browser* — rejected: single generated contract is the single source of truth (Q2 decision). The browser-facing and internal surfaces share `patchbay` types; the web server translates transport, not types.

## Safety contract (the 4 promoted `csrf_browser.qnt` properties)

This feature is the implementation site for the 4 promoted checked-model properties. Each maps to a concrete enforcement point:

- **`CsrfRejectsUnauthenticated`** — every state-changing route requires an authenticated operator-session cookie; missing/invalid → 401 before any core call.
- **`CsrfRejectsMissingProof`** — every state-changing route requires a valid `X-Patchbay-CSRF` header matching the session's stored proof; missing/mismatched → 403 before any core call.
- **`RevokedSessionCannotCommand`** — revoked/expired sessions (status != `active`) are rejected at the session-lookup step, even if the cookie is otherwise well-formed. The in-memory store tracks session status; revocation flips it.
- **`browser_local_state_not_authority`** — the server independently verifies session + CSRF from its own server-side state, never trusting browser-local claims. The forwarded `operator-id`/`operator-session-id` to the core come from the *server's* verified session record, not the browser's assertion.

## Implementation Units

### Unit 1: Web-server crate scaffold + core client
**File**: `web-server/package.json`, `web-server/tsconfig.json`, `web-server/src/main.ts`, `web-server/src/core-client.ts`
**Story**: `story-v0-web-server-scaffold`

The `patchbay-web-server` Fastify app. Wires a `createGrpcTransport()` client to the core (mirroring the spike's confirmed pattern) pointing at a configurable core address, authenticating with the configured `PATCHBAY_CORE_SECRET`. Reads config from env (`PATCHBAY_CORE_ADDR`, `PATCHBAY_CORE_SECRET`, `PATCHBAY_WEB_BIND_ADDR`, `PATCHBAY_TLS_CERT`/`PATCHBAY_TLS_KEY`, `PATCHBAY_OPERATOR_ID`). Fail-safe: refuses to start without `PATCHBAY_CORE_SECRET` and without a configured operator record (Q4).

```typescript
// web-server/src/core-client.ts — the gRPC client to the Rust core
import { createClient } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { ControlService } from "@patchbay/contracts";

export function makeCoreClient(coreAddr: string, coreSecret: string) {
  const transport = createGrpcTransport({
    baseUrl: coreAddr,
    // The shared secret authenticates the web server as a principal to the core.
    // Interceptor adds x-patchbay-core-secret to every call.
    interceptors: [coreSecretInterceptor(coreSecret)],
  });
  return createClient(ControlService, transport);
}
```

**Acceptance Criteria**:
- [ ] `patchbay-web-server` starts, binds a Fastify listener (HTTP or HTTPS per TLS config), refuses to start without `PATCHBAY_CORE_SECRET`.
- [ ] A health-check route (`GET /healthz`) returns 200 without auth.
- [ ] The core client can reach a running `patchbay-core-server` (verified by a smoke test).

---

### Unit 2: Operator-session store + login/logout
**File**: `web-server/src/sessions.ts`, `web-server/src/routes/login.ts`
**Story**: `story-v0-web-server-sessions`

The in-memory session store (`Map<session_id, {operatorActorId, status, csrfSecret, createdAt, ...}>`), the login route (consumes a one-time setup/login secret issued by CLI bootstrap — Q4), and the logout route. Sets the `__Host-patchbay_session` cookie per `SECURITY.md:102-105` (HttpOnly, Secure, SameSite=Strict, Path=/, no Domain). Session ids are CSPRNG-generated high-entropy opaque values.

```typescript
// web-server/src/sessions.ts
export interface OperatorSession {
  sessionId: string;          // opaque, high-entropy, CSPRNG
  operatorActorId: string;    // from the verified operator record (NOT the session id)
  status: "active" | "revoked" | "expired";
  csrfSecret: string;         // per-session synchronizer-token secret
  createdAt: number;
  lastUsedAt: number;
  expiresAt: number;
}

export class SessionStore {
  private sessions = new Map<string, OperatorSession>();
  // login(secret): verifies the one-time secret against the configured operator
  //   record, creates a session, returns {sessionId, csrfToken}.
  // lookup(sessionId): returns the session or null; flips expired if past expiresAt.
  // revoke(sessionId): sets status = "revoked" (RevokedSessionCannotCommand).
  // revokeAllForOperator(actorId): rotates all sessions (lockdown, SECURITY.md:203).
}
```

**Acceptance Criteria**:
- [ ] Login with a valid one-time secret creates a session, sets the `__Host-patchbay_session` cookie with the exact hardened shape, returns a CSRF token.
- [ ] Login with an invalid/expired secret returns 401 and creates no session.
- [ ] Logout revokes the session.
- [ ] `SessionStore.lookup` returns `null` for unknown ids and returns `status: "revoked"/"expired"` for dead sessions (they stay in the map so `RevokedSessionCannotCommand` is non-tautological — mirrors `csrf_browser.qnt` keeping dead sessions in the recognized-id set).

---

### Unit 3: CSRF + auth guard (the safety boundary)
**File**: `web-server/src/middleware/csrf-auth.ts`
**Story**: `story-v0-web-server-sessions` (same story — the guard is the session store's enforcement face)

A Fastify preHandler hook that enforces all three preconditions on every state-changing route: (1) valid session cookie → `lookup`, (2) session `status === "active"` (rejects revoked/expired), (3) valid `X-Patchbay-CSRF` header matching the session's `csrfSecret`. This is the implementation site for `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand`, and `browser_local_state_not_authority`.

```typescript
// web-server/src/middleware/csrf-auth.ts
export async function requireOperatorSession(req, reply) {
  // 1. Cookie → sessionId → SessionStore.lookup. No cookie / unknown id → 401
  //    (CsrfRejectsUnauthenticated).
  const sessionId = readSessionCookie(req);
  const session = sessionId ? store.lookup(sessionId) : null;
  if (!session) return reply.code(401).send({ error: "unauthenticated" });

  // 2. Session must be active. Revoked/expired → 403 (RevokedSessionCannotCommand).
  if (session.status !== "active")
    return reply.code(403).send({ error: "session " + session.status });

  // 3. CSRF synchronizer-token: X-Patchbay-CSRF header must equal the
  //    session's stored proof (CsrfRejectsMissingProof). Missing/mismatched → 403.
  const proof = req.headers["x-patchbay-csrf"];
  if (!proof || !timingSafeEqual(proof, session.csrfSecret))
    return reply.code(403).send({ error: "csrf proof missing or invalid" });

  // 4. browser_local_state_not_authority: the verified operator actor comes
  //    from the SERVER's session record, not the browser's assertion.
  req.verifiedOperator = session.operatorActorId;
  req.verifiedSessionId = session.sessionId;
}
```

**Acceptance Criteria**:
- [ ] A state-changing request with no cookie → 401.
- [ ] With a cookie but no CSRF header → 403.
- [ ] With a cookie + wrong CSRF header → 403.
- [ ] With a revoked session's cookie + valid CSRF → 403.
- [ ] With a valid cookie + valid CSRF → passes, and `req.verifiedOperator` is the server-record actor id (not a browser-supplied value).
- [ ] Non-state-changing reads (`GET /healthz`) do not require the guard.

---

### Unit 4: Connect-Web RPC bridge (browser → core translation)
**File**: `web-server/src/routes/rpc.ts`
**Story**: `story-v0-web-server-rpc-bridge`

Exposes the core's `ControlService` over Connect-Web to the browser. Each browser RPC: (1) runs the CSRF/auth guard (state-changing ones only), (2) reads the *server-verified* `operator-id` + `operator-session-id` from the session, (3) forwards them as gRPC metadata to the core (matching the seam's `MetadataIssuerContext` headers `x-patchbay-operator-id` + `x-patchbay-operator-session-id`), (4) proxies the call. For `Subscribe`, streams core events back to the browser as Connect-Web server-streaming.

```typescript
// web-server/src/routes/rpc.ts — browser Connect-Web → core gRPC translation
app.post("/patchbay.ControlService/Submit", { preHandler: requireOperatorSession },
  async (req, reply) => {
    // The browser sends an Operation; the server re-stamps the sender from
    // its verified session (browser_local_state_not_authority — the browser
    // never asserts the operator actor).
    const result = await coreClient.submit(
      req.body.operation,
      { headers: {
        "x-patchbay-core-secret": coreSecret,           // web-server principal
        "x-patchbay-operator-id": req.verifiedOperator, // from session record
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

**Implementation Notes**:
- The browser Connect-Web client speaks gRPC-Web to the web server; the web server uses `@connectrpc/connect-node` gRPC to the core. This is the transport translation; types are shared (single `@patchbay/contracts` package).
- The `x-patchbay-csrf` header is checked by the guard (Unit 3) BEFORE this handler runs; the RPC bridge only does metadata forwarding + transport translation.
- For v0.1.0, `LoadSnapshot` (read) may or may not require the CSRF guard depending on whether reads are state-changing — per `SECURITY.md:112` ("every state-changing web route"), reads do NOT require CSRF, but DO require an authenticated session (so the operator-id is server-verified). The guard's auth-check applies; the CSRF-check is skipped for reads.

**Acceptance Criteria**:
- [ ] A browser Connect-Web `Submit` through the web server reaches the core and returns the `SubmissionResult`; unauthenticated → 401, bad CSRF → 403.
- [ ] `Subscribe` streams events from core to browser; reconnect with a new cursor resumes.
- [ ] The `operator-id`/`operator-session-id` forwarded to the core come from the server's session record, not the browser request body.

---

### Unit 5: CSRF token issuance route + integration tests
**File**: `web-server/src/routes/csrf-token.ts`, `web-server/tests/integration.test.ts`
**Story**: `story-v0-web-server-rpc-bridge` (same story)

A `GET /csrf-token` route (auth-required, not CSRF-required — it *issues* the token) that returns the session's CSRF token to the browser (for SPAs that need to bootstrap the token after login). Plus the integration test suite that proves the 4 `csrf_browser.qnt` properties hold at the HTTP boundary.

**Acceptance Criteria** (the verification-grade tests — these are the implementation evidence for the promoted properties):
- [ ] `CsrfRejectsUnauthenticated`: state-changing request with no cookie → 401, no core call made.
- [ ] `CsrfRejectsMissingProof`: valid session, missing/wrong `X-Patchbay-CSRF` → 403, no core call made.
- [ ] `RevokedSessionCannotCommand`: a revoked session's cookie + valid CSRF → 403, no core call made.
- [ ] `browser_local_state_not_authority`: a request whose browser-supplied operator-id differs from the session record's operator-id still uses the *session record's* operator-id at the core (verified by asserting the forwarded metadata).

## Implementation Order

1. `story-v0-web-server-scaffold` (Unit 1) — no deps; unblocks everything (needs the core client to talk to).
2. `story-v0-web-server-sessions` (Units 2, 3) — depends on scaffold; the session store + the CSRF/auth guard are one cohesive safety boundary.
3. `story-v0-web-server-rpc-bridge` (Units 4, 5) — depends on sessions; the RPC bridge + integration tests.

## Simplification

- The web server owns NO domain logic — no delivery/reconnect state machines, no authority, no durable writes. Pure translation + the three safety concerns. This is the elimination pass.
- In-memory session store (not SQLite) for v0.1.0 — honestly bounded, documented as v0.1.0-only.
- No SSR, no server-side operator-domain promotion (reserved seam).
- No browser bidi streaming (Connect-Web is server-streaming-only in browsers, which is all the seam needs).
- Reads (`LoadSnapshot`, `GET /csrf-token`) require auth but NOT CSRF (per SECURITY.md:112 — only state-changing routes require CSRF).

## Testing

- **Interface/integration tests (Unit 5)**: the 4 `csrf_browser.qnt` property tests at the HTTP boundary — these are the load-bearing verification evidence for the promoted checked-model properties.
- **Session-lifecycle tests (Unit 2)**: login/logout/revoke/expire, cookie shape, CSPRNG session ids.
- **RPC-bridge smoke (Unit 4)**: end-to-end submit + subscribe against a real (or stubbed) core.
- No unit test per route handler beyond the above — they're thin translations; the integration tests cover them.

## Risks

- **Connect-Web server-streaming bridge (highest risk)**: streaming gRPC events back to the browser as gRPC-Web frames requires careful frame encoding (`reply.hijack()` + manual gRPC-Web framing in Fastify). The v0-stack-tooling research noted Fastify lacks a first-party SSE helper and Connect-Web server-side fit was a deferred question. Mitigation: spike the streaming-bridge shape first; if gRPC-Web framing in Fastify proves too costly, fall back to SSE for the event stream (still server-streaming, still typed via Connect-Web for the unary RPCs). This is the one place the transport choice (Q2a) could realistically force a hybrid (Q2c).
- **One-time login secret vs operator password**: `SECURITY.md:79` commits to password/passphrase for v0.1.0 primary auth, but `SECURITY.md:78` mentions a one-time setup secret for bootstrap. The web server's login route needs to know which it's consuming. Resolution: v0.1.0 login consumes a password verified against the operator record (the CLI bootstrap sets the password hash); the one-time setup secret is a CLI-only bootstrap concern, not a web-server login credential. Filed as an implementation note, not a blocker — but worth confirming during implementation.
- **TLS localhost exception scope creep**: the localhost secure-cookie exception must not generalize to LAN/IP/container. Enforced by checking the request's remote address, not just a config flag.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; one feature-owning worker implemented the three dependency-ordered child checkpoints end to end. Direct-read only and no delegation, per caller.
- Review weight: `standard` (caller); implementation stops at `review` for the orchestrator's independent feature review.
- Delivered: a fail-closed Fastify HTTP/HTTPS composition root; generated-contract gRPC client; scrypt-backed configured operator record; in-memory session lifecycle with retained dead records; hardened host cookie; loopback-only HTTP exception; timing-safe synchronizer-token guard; CSRF token issuance; and unary/server-streaming gRPC-Web translation to the core.
- Safety evidence: the four integration tests named for `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand`, and `browser_local_state_not_authority` exercise the real HTTP boundary. Rejections assert no core call, while the authority test asserts both re-stamped request sender and forwarded operator/session metadata originate in the server record.
- Streaming spike: manual binary gRPC-Web data/trailer framing worked with `@connectrpc/connect-web`, including server-streaming completion and cursor-based reconnect; SSE fallback was not needed.
- Integrated verification: `npm install --cache /home/agent/projects/patchbay/.npm-cache`; `npm run build`; `npm test` (15/15); `npm run test:core-smoke` against `patchbay-core-server`; `npm audit` (0 vulnerabilities); and `CARGO_HOME=/home/agent/projects/patchbay/.cargo-home PATH="/home/agent/.cargo/bin:$PATH" cargo build -p patchbay-core-server` all pass.
- Discrepancies from design: a real operator record requires `PATCHBAY_OPERATOR_PASSWORD_HASH` in addition to `PATCHBAY_OPERATOR_ID`; the format is `scrypt$<base64url-salt>$<base64url-hash>`. `Subscribe` is auth-required but CSRF-exempt as a read/subscription establishment. No contracts, core, or server source was changed.
- Simplification: the server remains a thin translator with no durable write, authority decision, reconnect state machine, duplicate DTO, or second protocol contract.
- Adjacent issues parked: none.

## Review response

- **Blocker addressed — interactive login throttling (`docs/SECURITY.md:85`)**: added a bounded process-local limiter for the configured operator-account dimension and the direct socket-address dimension. Failed-attempt windows decay after a fixed interval; successful authentication resets the applicable windows; no permanent account lockout or durable web-server-local state was introduced. The limiter also caps concurrent password verifications so a burst cannot queue unbounded scrypt work before failed results are recorded.
- **Pre-scrypt enforcement and audit**: `/login` checks the limiter before password verification, returns `429 login_throttled` with bounded `Retry-After`, and emits structured secret-free `interactive_login` audit lines for success and failure (including configured operator actor id and direct socket address). Production Fastify logging is enabled by default.
- **Regression coverage**: added HTTP-boundary tests proving account throttling across addresses, network throttling for one address, successful legitimate login after window decay, and zero password-verifier/scrypt calls for a throttled request. Suite is now 19/19 passing.
- **Re-verification**: `cd web-server && npm run build && npm test` passes; `CARGO_HOME=/home/agent/projects/patchbay/.cargo-home PATH="/home/agent/.cargo/bin:$PATH" cargo build -p patchbay-core-server` passes from the repository root.

## Review outcome

Standard-weight feature review (fresh-context, same-model `gpt-5.6-sol` — same harness, NOT cross-model). One independent pass found 1 material current-cycle blocker; fixed in-stride and re-verified green; no re-review per standard contract.

- Blocker (unthrottled login, violating SECURITY.md:85): fixed with a bounded in-memory account + network-dimension limiter, checked before scrypt, with decay (no permanent lockout) and concurrent-verification cap. 4 regression tests added (account throttle, network throttle, decay/recovery, pre-scrypt rejection).
- Important finding (no durable audit path for auth events) correctly PARKED as a follow-up: it crosses the web↔core seam (no audit-ingress RPC exists yet); web-server-local durable storage was deliberately NOT introduced. Tracked for a future typed audit sink into the core-owned durable audit log.

Final verification: 19 web-server tests pass (incl. 4 csrf_browser.qnt property tests + 4 throttle tests + streaming bridge); `cargo build -p patchbay-core-server` clean; `core/`/`server/`/`contracts/` unmodified by the web-server work. Reviewer confirmed all 4 safety properties genuine and the compound-issuer headers match the seam exactly.

Advanced `review → done`.
