import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { scryptSync } from "node:crypto";
import { rm } from "node:fs/promises";
import { once } from "node:events";
import { createClient } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { AdminService } from "@patchbay/contracts";

import { makeCoreClient } from "../dist/src/core-client.js";

const repo = new URL("../../", import.meta.url).pathname;
const database = new URL("../.core-smoke.sqlite3", import.meta.url).pathname;
const coreAddress = "http://127.0.0.1:50059";
const adminAddress = "http://127.0.0.1:50060";
const coreSecret = "web-server-core-smoke-secret";

await removeDatabase();
const core = spawn(`${repo}target/debug/patchbay-core-server`, [], {
  cwd: repo,
  env: {
    ...process.env,
    PATCHBAY_CORE_SECRET: coreSecret,
    PATCHBAY_ADAPTER_ATTACHMENT_SECRET: "smoke-adapter-secret",
    PATCHBAY_BIND_ADDR: "127.0.0.1:50059",
    PATCHBAY_ADMIN_BIND_ADDR: "127.0.0.1:50060",
    PATCHBAY_DB_PATH: database,
  },
  stdio: ["ignore", "pipe", "pipe"],
});

try {
  const setupSecret = await waitForListener(core);
  const admin = createClient(
    AdminService,
    createGrpcTransport({ baseUrl: adminAddress }),
  );
  const salt = Buffer.alloc(16, 9);
  const passwordHash = `scrypt$${salt.toString("base64url")}$${scryptSync("smoke-password", salt, 64).toString("base64url")}`;
  const bootstrap = await admin.bootstrapOperator({
    setupSecret,
    operatorActorId: { value: "operator-primary" },
    passwordHash,
    principal: {
      endpointId: { value: "patchbay-web-server-smoke" },
      deviceId: { value: "web-smoke-device" },
      endpointGeneration: { value: 1n },
    },
  });
  assert.ok(bootstrap.principal);

  const client = makeCoreClient(coreAddress, coreSecret);
  const response = await client.loadSnapshot(
    { authorityDomainId: { value: "default" } },
    {
      headers: {
        "x-patchbay-operator-id": "operator-primary",
        "x-patchbay-operator-session-id": bootstrap.sessionId.value,
        "x-patchbay-principal-id": bootstrap.principal.principalId,
        "x-patchbay-principal-secret": bootstrap.principal.secret,
      },
    },
  );
  assert.equal(response.present, false);
  console.log("core client reached patchbay-core-server with authenticated metadata");
} finally {
  core.kill("SIGTERM");
  await once(core, "exit").catch(() => undefined);
  await removeDatabase();
}

async function waitForListener(child) {
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => {
    stderr += chunk;
  });
  child.stdout.setEncoding("utf8");

  return new Promise((resolve, reject) => {
    let output = "";
    const timeout = setTimeout(() => reject(new Error(`core did not start: ${stderr}`)), 10_000);
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`core exited before listening (${code}): ${stderr}`));
    });
    child.stdout.on("data", (chunk) => {
      output += chunk;
      const match = output.match(/one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/);
      if (output.includes("patchbay-core-server: local admin h2c") && match) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
  });
}

async function removeDatabase() {
  await Promise.all(
    [database, `${database}-shm`, `${database}-wal`].map((path) => rm(path, { force: true })),
  );
}
