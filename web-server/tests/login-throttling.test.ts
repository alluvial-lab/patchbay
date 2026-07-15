import assert from "node:assert/strict";
import test from "node:test";

import { LoginLimiter } from "../src/login-limiter.js";
import { buildApp, type WebServerConfig } from "../src/main.js";
import { hashPassword } from "../src/sessions.js";

const operatorPasswordHash = await hashPassword("real-password", Buffer.alloc(16, 4));
const config: WebServerConfig = {
  coreAddr: "http://127.0.0.1:50051",
  coreSecret: "test-core-secret",
  bindHost: "127.0.0.1",
  bindPort: 3000,
  operatorId: "operator-primary",
  operatorPasswordHash,
};
const unusedCoreClient = {} as never;

function failedLogin(app: ReturnType<typeof buildApp>, remoteAddress: string) {
  return app.inject({
    method: "POST",
    url: "/login",
    remoteAddress,
    payload: { password: "wrong-password" },
  });
}

test("repeated failures against the configured operator account are throttled across addresses", async () => {
  let verificationCalls = 0;
  const limiter = new LoginLimiter({
    accountMaxFailures: 2,
    networkMaxFailures: 100,
  });
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    loginLimiter: limiter,
    passwordVerifier: async () => {
      verificationCalls += 1;
      return false;
    },
    logger: false,
  });

  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 401);
  assert.equal((await failedLogin(app, "::1")).statusCode, 401);
  const throttled = await failedLogin(app, "::ffff:127.0.0.1");

  assert.equal(throttled.statusCode, 429);
  assert.deepEqual(throttled.json(), { error: "login_throttled" });
  assert.equal(verificationCalls, 2);
  await app.close();
});

test("repeated failures from one direct socket address are throttled by the network dimension", async () => {
  let verificationCalls = 0;
  const limiter = new LoginLimiter({
    accountMaxFailures: 100,
    networkMaxFailures: 2,
  });
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    loginLimiter: limiter,
    passwordVerifier: async () => {
      verificationCalls += 1;
      return false;
    },
    logger: false,
  });

  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 401);
  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 401);
  const throttled = await failedLogin(app, "127.0.0.1");

  assert.equal(throttled.statusCode, 429);
  assert.equal(verificationCalls, 2);
  await app.close();
});

test("a legitimate login succeeds after the bounded throttle window decays", async () => {
  let now = 1_000;
  let passwordIsValid = false;
  let verificationCalls = 0;
  const limiter = new LoginLimiter({
    now: () => now,
    windowMs: 100,
    accountMaxFailures: 2,
    networkMaxFailures: 2,
  });
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    loginLimiter: limiter,
    passwordVerifier: async () => {
      verificationCalls += 1;
      return passwordIsValid;
    },
    logger: false,
  });

  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 401);
  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 401);
  assert.equal((await failedLogin(app, "127.0.0.1")).statusCode, 429);

  now += 101;
  passwordIsValid = true;
  const recovered = await app.inject({
    method: "POST",
    url: "/login",
    remoteAddress: "127.0.0.1",
    payload: { password: "real-password" },
  });

  assert.equal(recovered.statusCode, 200);
  assert.equal(typeof recovered.json().csrfToken, "string");
  assert.equal(verificationCalls, 3);
  await app.close();
});

test("a throttled request is rejected before password verification reaches scrypt", async () => {
  const limiter = new LoginLimiter({ accountMaxFailures: 1, networkMaxFailures: 1 });
  const address = "127.0.0.1";
  assert.deepEqual(limiter.beginAttempt(address), { allowed: true });
  limiter.recordFailure(address);

  let verificationCalls = 0;
  const app = buildApp({
    config,
    coreClient: unusedCoreClient,
    loginLimiter: limiter,
    passwordVerifier: async () => {
      verificationCalls += 1;
      return true;
    },
    logger: false,
  });

  const throttled = await app.inject({
    method: "POST",
    url: "/login",
    remoteAddress: address,
    payload: { password: "real-password" },
  });

  assert.equal(throttled.statusCode, 429);
  assert.equal(verificationCalls, 0, "throttling must happen before password verification");
  await app.close();
});
