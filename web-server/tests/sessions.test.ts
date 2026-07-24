import assert from "node:assert/strict";
import test from "node:test";

import {
  CSRF_HEADER_NAME,
  requireOperatorSession,
  SESSION_COOKIE_NAME,
} from "../src/middleware/csrf-auth.js";
import { buildApp, type WebServerConfig } from "../src/main.js";
import { hashPassword, SessionStore, verifyPassword } from "../src/sessions.js";

const password = "correct horse battery staple";
const operatorPasswordHash = await hashPassword(password, Buffer.alloc(16, 2));
const config: WebServerConfig = {
  coreAddr: "http://127.0.0.1:50051",
  coreSecret: "test-core-secret",
  bindHost: "127.0.0.1",
  bindPort: 3000,
  authorityDomainId: "default",
  operatorId: "operator-from-record",
  operatorPasswordHash,
};
const unusedCoreClient = {} as never;

test("dead sessions remain recognized with revoked or expired status", () => {
  let now = 100;
  const sessions = new SessionStore({ now: () => now, sessionTtlMs: 50 });
  const revoked = sessions.create("operator-a");
  const expired = sessions.create("operator-a");

  assert.equal(sessions.lookup("unknown"), null);
  assert.equal(sessions.revoke(revoked.sessionId), true);
  now = 151;

  assert.equal(sessions.lookup(revoked.sessionId)?.status, "revoked");
  assert.equal(sessions.lookup(expired.sessionId)?.status, "expired");
  assert.equal(sessions.size, 2, "dead records must stay in the recognized-session map");
});

test("session ids and CSRF secrets are independent 256-bit CSPRNG tokens", () => {
  const session = new SessionStore().create("operator-a");
  assert.notEqual(session.sessionId, session.csrfSecret);
  assert.equal(Buffer.from(session.sessionId, "base64url").length, 32);
  assert.equal(Buffer.from(session.csrfSecret, "base64url").length, 32);
});

test("login verifies the operator record and sets the hardened host cookie", async () => {
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    passwordVerifier: verifyPassword,
    logger: false,
  });

  const invalid = await app.inject({ method: "POST", url: "/login", payload: { password: "bad" } });
  assert.equal(invalid.statusCode, 401);
  assert.equal(app.sessions.size, 0);

  const login = await app.inject({ method: "POST", url: "/login", payload: { password } });
  assert.equal(login.statusCode, 200);
  assert.equal(typeof login.json().csrfToken, "string");
  const setCookie = String(login.headers["set-cookie"]);
  assert.match(setCookie, /^__Host-patchbay_session=[^;]+;/);
  assert.match(setCookie, /(?:^|; )Path=\/(?:;|$)/);
  assert.match(setCookie, /(?:^|; )HttpOnly(?:;|$)/);
  assert.match(setCookie, /(?:^|; )Secure(?:;|$)/);
  assert.match(setCookie, /(?:^|; )SameSite=Strict(?:;|$)/);
  assert.doesNotMatch(setCookie, /(?:^|; )Domain=/i);
  await app.close();
});

test("insecure non-loopback requests cannot establish browser sessions", async () => {
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    passwordVerifier: verifyPassword,
    logger: false,
  });
  const response = await app.inject({
    method: "POST",
    url: "/login",
    remoteAddress: "192.168.10.20",
    payload: { password },
  });
  assert.equal(response.statusCode, 400);
  assert.deepEqual(response.json(), { error: "https_required" });
  await app.close();
});

test("a loopback proxy must be explicitly trusted and attest browser HTTPS", async () => {
  const untrusted = buildApp({
    config,
    coreClient: unusedCoreClient,
    passwordVerifier: verifyPassword,
    logger: false,
  });
  const forwardedHttps = {
    method: "POST" as const,
    url: "/login",
    remoteAddress: "127.0.0.1",
    headers: { "x-forwarded-proto": "https" },
    payload: { password },
  };
  const untrustedResponse = await untrusted.inject(forwardedHttps);
  assert.equal(untrustedResponse.statusCode, 400);
  assert.deepEqual(untrustedResponse.json(), { error: "https_required" });
  await untrusted.close();

  const trusted = buildApp({
    config: { ...config, trustedLoopbackProxy: true },
    coreClient: unusedCoreClient,
    passwordVerifier: verifyPassword,
    logger: false,
  });
  const forwardedHttp = await trusted.inject({
    ...forwardedHttps,
    headers: { "x-forwarded-proto": "http" },
  });
  assert.equal(forwardedHttp.statusCode, 400);
  assert.deepEqual(forwardedHttp.json(), { error: "https_required" });
  const missingForwardedScheme = await trusted.inject({ ...forwardedHttps, headers: {} });
  assert.equal(missingForwardedScheme.statusCode, 400);
  assert.deepEqual(missingForwardedScheme.json(), { error: "https_required" });

  const accepted = await trusted.inject(forwardedHttps);
  assert.equal(accepted.statusCode, 200);

  const session = trusted.sessions.create("operator-from-record");
  const proxiedSessionRead = await trusted.inject({
    method: "GET",
    url: "/csrf-token",
    remoteAddress: "127.0.0.1",
    headers: {
      cookie: `${SESSION_COOKIE_NAME}=${session.sessionId}`,
      "x-forwarded-proto": "http",
    },
  });
  assert.equal(proxiedSessionRead.statusCode, 400);
  assert.deepEqual(proxiedSessionRead.json(), { error: "https_required" });
  await trusted.close();
});

test("state-changing guard enforces auth, live status, and timing-safe CSRF proof", async () => {
  const app = buildApp({ config, coreClient: unusedCoreClient, logger: false });
  app.post(
    "/guarded",
    { preHandler: requireOperatorSession(app.sessions) },
    async (request) => ({
      operator: request.verifiedOperator,
      sessionId: request.verifiedSessionId,
    }),
  );
  app.get(
    "/guarded-read",
    { preHandler: requireOperatorSession(app.sessions, { requireCsrf: false }) },
    async (request) => ({ operator: request.verifiedOperator }),
  );

  const noCookie = await app.inject({ method: "POST", url: "/guarded" });
  assert.equal(noCookie.statusCode, 401);

  const session = app.sessions.create("operator-from-record");
  const cookie = `${SESSION_COOKIE_NAME}=${session.sessionId}`;
  const noProof = await app.inject({ method: "POST", url: "/guarded", headers: { cookie } });
  assert.equal(noProof.statusCode, 403);
  const wrongProof = await app.inject({
    method: "POST",
    url: "/guarded",
    headers: { cookie, [CSRF_HEADER_NAME]: "wrong" },
  });
  assert.equal(wrongProof.statusCode, 403);

  const read = await app.inject({ method: "GET", url: "/guarded-read", headers: { cookie } });
  assert.equal(read.statusCode, 200);

  const valid = await app.inject({
    method: "POST",
    url: "/guarded",
    headers: {
      cookie,
      [CSRF_HEADER_NAME]: session.csrfSecret,
      "x-patchbay-operator-id": "browser-forgery",
    },
  });
  assert.equal(valid.statusCode, 200);
  assert.deepEqual(valid.json(), {
    operator: "operator-from-record",
    sessionId: session.sessionId,
  });

  app.sessions.revoke(session.sessionId);
  const revoked = await app.inject({
    method: "POST",
    url: "/guarded",
    headers: { cookie, [CSRF_HEADER_NAME]: session.csrfSecret },
  });
  assert.equal(revoked.statusCode, 403);
  assert.deepEqual(revoked.json(), { error: "session_revoked" });
  await app.close();
});

test("logout requires CSRF and retains the revoked server record", async () => {
  const app = buildApp({ config, coreClient: unusedCoreClient, logger: false });
  const session = app.sessions.create("operator-from-record");
  const response = await app.inject({
    method: "POST",
    url: "/logout",
    headers: {
      cookie: `${SESSION_COOKIE_NAME}=${session.sessionId}`,
      [CSRF_HEADER_NAME]: session.csrfSecret,
    },
  });

  assert.equal(response.statusCode, 200);
  assert.equal(app.sessions.lookup(session.sessionId)?.status, "revoked");
  assert.equal(app.sessions.size, 1);
  await app.close();
});
