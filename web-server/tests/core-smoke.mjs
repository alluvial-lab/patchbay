import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { rm } from "node:fs/promises";
import { once } from "node:events";

import { makeCoreClient } from "../dist/src/core-client.js";

const repo = new URL("../../", import.meta.url).pathname;
const database = new URL("../.core-smoke.sqlite3", import.meta.url).pathname;
const coreAddress = "http://127.0.0.1:50059";
const coreSecret = "web-server-core-smoke-secret";

await removeDatabase();
const core = spawn(`${repo}target/debug/patchbay-core-server`, [], {
  cwd: repo,
  env: {
    ...process.env,
    PATCHBAY_CORE_SECRET: coreSecret,
    PATCHBAY_BIND_ADDR: "127.0.0.1:50059",
    PATCHBAY_DB_PATH: database,
  },
  stdio: ["ignore", "pipe", "pipe"],
});

try {
  await waitForListener(core);
  const client = makeCoreClient(coreAddress, coreSecret);
  const response = await client.loadSnapshot(
    { authorityDomainId: { value: "default" } },
    {
      headers: {
        "x-patchbay-operator-id": "operator-primary",
        "x-patchbay-operator-session-id": "smoke-session",
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

  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error(`core did not start: ${stderr}`)), 10_000);
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`core exited before listening (${code}): ${stderr}`));
    });
    child.stdout.on("data", (chunk) => {
      if (chunk.includes("patchbay-core-server: h2c")) {
        clearTimeout(timeout);
        resolve();
      }
    });
  });
}

async function removeDatabase() {
  await Promise.all(
    [database, `${database}-shm`, `${database}-wal`].map((path) => rm(path, { force: true })),
  );
}
