---
id: story-v0-web-server-sessions
kind: story
stage: done
tags: [security, protocol]
parent: feature-v0-web-server
depends_on: [story-v0-web-server-scaffold]
created: 2026-07-15
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Story: web-server operator-session store + CSRF/auth guard

The safety boundary: the in-memory session store and the Fastify preHandler that enforces the 3 preconditions (authenticated session, active session, valid CSRF proof) on every state-changing route. This is the implementation site for the 4 promoted `csrf_browser.qnt` properties.

## Design (from feature-v0-web-server Units 2, 3)

**Files**: `web-server/src/sessions.ts`, `web-server/src/routes/login.ts`, `web-server/src/middleware/csrf-auth.ts`

### Session store (`sessions.ts`)

In-memory `Map<session_id, OperatorSession>` for v0.1.0 (Q1 decision). Session ids are CSPRNG high-entropy opaque values. The `operatorActorId` comes from the verified operator record (NOT the session id — the seam's blocker-1 fix established this separation). Dead sessions (revoked/expired) stay in the map so `RevokedSessionCannotCommand` is non-tautological (mirrors `csrf_browser.qnt` keeping dead sessions in the recognized-id set).

```typescript
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
  // login(password): verifies password against the operator record (Q4 — the
  //   operator record is pre-seeded by CLI bootstrap; the web server does not
  //   create it). Creates a session, returns {sessionId, csrfToken}.
  // lookup(sessionId): returns the session or null; flips status to "expired"
  //   if past expiresAt (RevokedSessionCannotCommand covers expired too).
  // revoke(sessionId): sets status = "revoked".
  // revokeAllForOperator(actorId): rotates all sessions (lockdown, SECURITY.md:203).
}
```

### Login route (`routes/login.ts`)

- `POST /login` consumes a password (v0.1.0 primary auth per SECURITY.md:79), verifies against the operator record, creates a session, sets the `__Host-patchbay_session` cookie per SECURITY.md:102-105 (HttpOnly, Secure, SameSite=Strict, Path=/, no Domain), returns the CSRF token.
- `POST /logout` revokes the session.

### CSRF/auth guard (`middleware/csrf-auth.ts`)

A Fastify preHandler enforcing, in order:
1. Valid session cookie → `lookup`. No cookie / unknown id → 401 (`CsrfRejectsUnauthenticated`).
2. Session `status === "active"`. Revoked/expired → 403 (`RevokedSessionCannotCommand`).
3. Valid `X-Patchbay-CSRF` header matching the session's `csrfSecret` (timing-safe compare). Missing/mismatched → 403 (`CsrfRejectsMissingProof`).
4. Sets `req.verifiedOperator` + `req.verifiedSessionId` from the SERVER's session record (`browser_local_state_not_authority` — never from the browser's assertion).

Reads (`LoadSnapshot`, `GET /csrf-token`) require steps 1-2 (auth) but NOT step 3 (CSRF) — per SECURITY.md:112, only state-changing routes require CSRF.

## Acceptance criteria

- [ ] Login with a valid password creates a session, sets the `__Host-patchbay_session` cookie with the exact hardened shape, returns a CSRF token.
- [ ] Login with an invalid password returns 401, creates no session.
- [ ] Logout revokes the session.
- [ ] `lookup` returns `null` for unknown ids; returns `status: "revoked"/"expired"` for dead sessions (dead sessions stay in the map).
- [ ] A state-changing request with no cookie → 401.
- [ ] With a cookie but no CSRF header → 403.
- [ ] With a cookie + wrong CSRF header → 403.
- [ ] With a revoked session's cookie + valid CSRF → 403.
- [ ] With a valid cookie + valid CSRF → passes, and `req.verifiedOperator` is the session-record actor id (not a browser-supplied value).
- [ ] Reads require auth but not CSRF.

## Notes

- The operator password hash must be available to the web server (from the configured operator record — Q4). For v0.1.0 testing, a test fixture operator record is acceptable; the real bootstrap is `feature-v0-cli`'s job.
- The CSRF token is returned to the browser at login (and via `GET /csrf-token`) so SPAs can bootstrap it; it is NOT in a cookie-readable form (HttpOnly session cookie holds the session id, not the CSRF token).
- Timing-safe comparison for the CSRF check (avoid timing oracles).

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`, high effort; retained by the feature-owning worker because session records and the guard form one safety boundary. Direct-read only; no delegation.
- Review weight: `standard` (caller).
- Files changed: `web-server/package.json`; `web-server/package-lock.json`; `web-server/src/main.ts`; `web-server/src/sessions.ts`; `web-server/src/middleware/csrf-auth.ts`; `web-server/src/routes/login.ts`; `web-server/tests/scaffold.test.ts`; `web-server/tests/sessions.test.ts`.
- Tests added/removed: session lifecycle tests preserve revoked/expired records and exercise CSPRNG token size; HTTP tests cover password login, exact cookie security attributes, invalid login non-creation, non-loopback HTTP rejection, logout revocation, auth-only reads, and every guard branch including server-record identity. No low-value per-wrapper tests were added.
- Simplification: built-in Node scrypt and timing-safe comparison avoid a native authentication dependency; one in-memory `SessionStore` owns all status transitions; one parameterized guard handles mutation and auth-only read routes.
- Discrepancies from design: added required `PATCHBAY_OPERATOR_PASSWORD_HASH` in `scrypt$<salt>$<hash>` form because a configured operator actor id alone is not a verifiable operator record. The localhost exception is decided from the direct socket's loopback address (never Host or forwarded headers), and cookies remain `Secure` even on localhost. Fetch Metadata `cross-site` requests are rejected when the browser supplies that signal.
- Adjacent issues parked: none.
