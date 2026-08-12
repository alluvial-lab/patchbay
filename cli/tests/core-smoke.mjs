import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { once } from "node:events";
import { fileURLToPath } from "node:url";

const cliRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const repo = resolve(cliRoot, "..");
const stateDirectory = await mkdtemp(join(tmpdir(), "patchbay-cli-smoke-"));
const database = join(stateDirectory, "core.sqlite3");
const credentialPath = join(stateDirectory, "credentials.json");
const coreAddress = "http://127.0.0.1:50159";
const adminAddress = "http://127.0.0.1:50160";
const coreSecret = "cli-core-smoke-secret";

const core = spawn(join(repo, "target/debug/patchbay-core-server"), [], {
  cwd: repo,
  env: {
    ...process.env,
    PATCHBAY_CORE_SECRET: coreSecret,
    PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS: '{"pi":"smoke-adapter-secret"}',
    PATCHBAY_BIND_ADDR: "127.0.0.1:50159",
    PATCHBAY_ADMIN_BIND_ADDR: "127.0.0.1:50160",
    PATCHBAY_DB_PATH: database,
  },
  stdio: ["ignore", "pipe", "pipe"],
});

const cliEnv = {
  ...process.env,
  PATCHBAY_CORE_SECRET: coreSecret,
  PATCHBAY_CORE_ADDR: coreAddress,
  PATCHBAY_CORE_ADMIN_ADDR: adminAddress,
  PATCHBAY_CREDENTIALS_PATH: credentialPath,
};

try {
  const setupSecret = await waitForCore(core);
  const setup = await runCli(
    [
      "setup",
      "--operator-id",
      "operator-primary",
      "--endpoint-id",
      "cli-setup",
      "--device-id",
      "smoke-device",
    ],
    {
      PATCHBAY_SETUP_SECRET: setupSecret,
      PATCHBAY_OPERATOR_PASSWORD: "smoke-password",
    },
  );
  assert.equal(setup.code, 0, setup.stderr);
  assert.equal(setup.stdout.includes(setupSecret), false);
  assert.equal((await stat(credentialPath)).mode & 0o777, 0o600);

  const login = await runCli(
    [
      "login",
      "--operator-id",
      "operator-primary",
      "--endpoint-id",
      "cli-login",
      "--device-id",
      "smoke-device",
    ],
    { PATCHBAY_OPERATOR_PASSWORD: "smoke-password" },
  );
  assert.equal(login.code, 0, login.stderr);

  const health = await runCli(["session-health", "--json"]);
  assert.equal(health.code, 1);
  assert.match(health.stderr, /No sessions found/);
  assert.doesNotMatch(health.stderr, /unauthenticated|invalid transport principal/i);

  const logout = await runCli(["logout"]);
  assert.equal(logout.code, 0, logout.stderr);
  const afterLogout = await runCli(["session-health"]);
  assert.equal(afterLogout.code, 1);
  assert.match(afterLogout.stderr, /run patchbay-cli login/);
  console.log("CLI smoke: setup → login → authenticated LoadSnapshot → logout/rejection passed");
} finally {
  core.kill("SIGTERM");
  await once(core, "exit").catch(() => undefined);
  await rm(stateDirectory, { recursive: true, force: true });
}

function runCli(args, secretEnv = {}) {
  return new Promise((resolveResult, reject) => {
    const child = spawn(process.execPath, [join(cliRoot, "dist/src/main.js"), ...args], {
      cwd: cliRoot,
      env: { ...cliEnv, ...secretEnv },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.once("error", reject);
    child.once("exit", (code) => resolveResult({ code, stdout, stderr }));
  });
}

function waitForCore(child) {
  let stderr = "";
  let stdout = "";
  child.stderr.setEncoding("utf8");
  child.stdout.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });

  return new Promise((resolveSecret, reject) => {
    const timeout = setTimeout(() => reject(new Error(`core did not start: ${stderr}`)), 10_000);
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`core exited before listening (${code}): ${stderr}`));
    });
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      const match = stdout.match(/one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/);
      if (stdout.includes("patchbay-core-server: local admin h2c") && match) {
        clearTimeout(timeout);
        resolveSecret(match[1]);
      }
    });
  });
}
