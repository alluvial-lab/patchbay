import assert from "node:assert/strict";
import test from "node:test";

import { buildApp, loadConfig, type WebServerConfig } from "../src/main.js";
import { hashPassword } from "../src/sessions.js";

const operatorPasswordHash = await hashPassword("test-password", Buffer.alloc(16, 1));
const config: WebServerConfig = {
  coreAddr: "http://127.0.0.1:50051",
  coreSecret: "test-core-secret",
  bindHost: "127.0.0.1",
  bindPort: 3000,
  operatorId: "operator-primary",
  operatorPasswordHash,
};

const unusedCoreClient = {} as never;

test("startup fails closed without the core trust root or operator identity", () => {
  assert.throws(
    () => loadConfig({ PATCHBAY_OPERATOR_ID: "operator-primary" }),
    /PATCHBAY_CORE_SECRET is required/,
  );
  assert.throws(
    () => loadConfig({ PATCHBAY_CORE_SECRET: "secret" }),
    /PATCHBAY_OPERATOR_ID is required/,
  );

  const sharedRecordConfig = loadConfig({
    PATCHBAY_CORE_SECRET: "secret",
    PATCHBAY_OPERATOR_ID: "operator-primary",
  });
  assert.equal(sharedRecordConfig.operatorPasswordHash, undefined);
});

test("health check is unauthenticated", async () => {
  const app = buildApp({ config, coreClient: unusedCoreClient, logger: false });
  const response = await app.inject({ method: "GET", url: "/healthz" });

  assert.equal(response.statusCode, 200);
  assert.deepEqual(response.json(), { status: "ok" });
  await app.close();
});

test("TLS certificate and key must be configured as a pair", () => {
  assert.throws(
    () =>
      loadConfig({
        PATCHBAY_CORE_SECRET: "secret",
        PATCHBAY_OPERATOR_ID: "operator-primary",
        PATCHBAY_OPERATOR_PASSWORD_HASH: operatorPasswordHash,
        PATCHBAY_TLS_CERT: "cert.pem",
      }),
    /must be configured together/,
  );
});
