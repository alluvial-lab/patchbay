import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { buildApp, loadConfig, type WebServerConfig } from "../src/main.js";
import { hashPassword } from "../src/sessions.js";

const operatorPasswordHash = await hashPassword("test-password", Buffer.alloc(16, 1));
const config: WebServerConfig = {
  coreAddr: "http://127.0.0.1:50051",
  coreSecret: "test-core-secret",
  bindHost: "127.0.0.1",
  bindPort: 3000,
  authorityDomainId: "default",
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
  assert.equal(sharedRecordConfig.authorityDomainId, "default");

  const configuredDomain = loadConfig({
    PATCHBAY_CORE_SECRET: "secret",
    PATCHBAY_OPERATOR_ID: "operator-primary",
    PATCHBAY_AUTHORITY_DOMAIN_ID: "operator-fleet",
  });
  assert.equal(configuredDomain.authorityDomainId, "operator-fleet");

  assert.equal(
    loadConfig({
      PATCHBAY_CORE_SECRET: "secret",
      PATCHBAY_OPERATOR_ID: "operator-primary",
      PATCHBAY_TRUST_LOOPBACK_PROXY: "true",
    }).trustedLoopbackProxy,
    true,
  );
  assert.throws(
    () => loadConfig({
      PATCHBAY_CORE_SECRET: "secret",
      PATCHBAY_OPERATOR_ID: "operator-primary",
      PATCHBAY_TRUST_LOOPBACK_PROXY: "yes",
    }),
    /PATCHBAY_TRUST_LOOPBACK_PROXY must be true or false/,
  );
});

test("health check is unauthenticated", async () => {
  const app = buildApp({ config, coreClient: unusedCoreClient, logger: false });
  const response = await app.inject({ method: "GET", url: "/healthz" });

  assert.equal(response.statusCode, 200);
  assert.deepEqual(response.json(), { status: "ok" });
  await app.close();
});

test("served cockpit entry carries the configured authority domain", async () => {
  const assets = await mkdtemp(join(tmpdir(), "patchbay-cockpit-assets-"));
  try {
    await writeFile(
      join(assets, "index.html"),
      '<meta name="patchbay-authority-domain" content="__PATCHBAY_AUTHORITY_DOMAIN_ID__">',
    );
    const app = buildApp({
      config: { ...config, authorityDomainId: "operator-fleet" },
      coreClient: unusedCoreClient,
      cockpitAssetsDir: assets,
      logger: false,
    });
    const response = await app.inject({ method: "GET", url: "/" });

    assert.equal(response.statusCode, 200);
    assert.match(
      response.body,
      /<meta name="patchbay-authority-domain" content="operator-fleet">/,
    );
    assert.doesNotMatch(response.body, /__PATCHBAY_AUTHORITY_DOMAIN_ID__/);
    await app.close();
  } finally {
    await rm(assets, { recursive: true, force: true });
  }
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
