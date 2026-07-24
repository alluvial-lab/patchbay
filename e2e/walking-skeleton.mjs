import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer, Socket } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const e2eRoot = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(e2eRoot, "..");
const cliRoot = join(repoRoot, "cli");
const adapterRoot = join(repoRoot, "pi-adapter");
const stateDirectory = await mkdtemp(join(tmpdir(), "patchbay-walking-skeleton-"));
const databasePath = join(stateDirectory, "core.sqlite3");
const credentialPath = join(stateDirectory, "credentials.json");
const authorityDomainId = "walking-skeleton-domain";
const coreSecret = "walking-skeleton-core-secret";
const adapterSecret = "walking-skeleton-adapter-secret";
const operatorId = "walking-operator";
const operatorPassword = "walking-password";
const runtimeSessionId = "walking-session";
const corePort = await freePort();
let adminPort = await freePort();
while (adminPort === corePort) adminPort = await freePort();
const coreAddress = `http://127.0.0.1:${corePort}`;
const adminAddress = `http://127.0.0.1:${adminPort}`;
const children = [];

try {
  await runChecked(
    "cargo",
    ["build", "-p", "patchbay-core-server"],
    repoRoot,
    {
      ...process.env,
      CARGO_HOME: join(repoRoot, ".cargo-home"),
      PATH: `/home/agent/.cargo/bin:${process.env.PATH ?? ""}`,
    },
  );
  await runChecked("npm", ["run", "build"], adapterRoot, process.env);
  await runChecked("npm", ["run", "build"], cliRoot, process.env);

  const core = spawn(join(repoRoot, "target/debug/patchbay-core-server"), [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      PATCHBAY_CORE_SECRET: coreSecret,
      PATCHBAY_ADAPTER_ATTACHMENT_SECRET: adapterSecret,
      PATCHBAY_AUTHORITY_DOMAIN_ID: authorityDomainId,
      PATCHBAY_BIND_ADDR: `127.0.0.1:${corePort}`,
      PATCHBAY_ADMIN_BIND_ADDR: `127.0.0.1:${adminPort}`,
      PATCHBAY_DB_PATH: databasePath,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  children.push(core);
  const coreOutput = capture(core);
  const setupSecret = await waitForMatch(
    coreOutput,
    /one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/,
    15_000,
    "core setup secret",
  );
  await waitForPort(corePort, core, 15_000);

  const adapter = spawn(process.execPath, [join(e2eRoot, "pi-adapter-fixture.mjs")], {
    cwd: repoRoot,
    env: {
      ...process.env,
      PATCHBAY_CORE_ADDR: coreAddress,
      PATCHBAY_AUTHORITY_DOMAIN_ID: authorityDomainId,
      PATCHBAY_ADAPTER_ATTACHMENT_SECRET: adapterSecret,
      WALKING_SESSION_ID: runtimeSessionId,
      WALKING_DEPLOYMENT_SCOPE: "walking-machine",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  children.push(adapter);
  const adapterOutput = capture(adapter);
  await waitForMatch(
    adapterOutput,
    new RegExp(`PI_ADAPTER_READY ${runtimeSessionId}`),
    15_000,
    "Pi adapter startup",
  );

  const cliEnv = {
    ...process.env,
    PATCHBAY_CORE_SECRET: coreSecret,
    PATCHBAY_AUTHORITY_DOMAIN_ID: authorityDomainId,
    PATCHBAY_CORE_ADDR: coreAddress,
    PATCHBAY_CORE_ADMIN_ADDR: adminAddress,
    PATCHBAY_CREDENTIALS_PATH: credentialPath,
  };
  const setup = await runCli(
    [
      "setup",
      "--operator-id",
      operatorId,
      "--endpoint-id",
      "walking-setup",
      "--device-id",
      "walking-device",
    ],
    {
      ...cliEnv,
      PATCHBAY_SETUP_SECRET: setupSecret,
      PATCHBAY_OPERATOR_PASSWORD: operatorPassword,
    },
  );
  assert.equal(setup.code, 0, setup.stderr);

  const login = await runCli(
    [
      "login",
      "--operator-id",
      operatorId,
      "--endpoint-id",
      "walking-login",
      "--device-id",
      "walking-device",
    ],
    { ...cliEnv, PATCHBAY_OPERATOR_PASSWORD: operatorPassword },
  );
  assert.equal(login.code, 0, login.stderr);

  const initialHealth = await runCli(["session-health", "--json"], cliEnv);
  assert.equal(initialHealth.code, 0, initialHealth.stderr);
  const initialSession = oneSession(initialHealth.stdout);
  assert.equal(initialSession.runtimeSessionId, runtimeSessionId);
  assert.equal(initialSession.connectivity, "live");
  assert.equal(initialSession.activity, "idle");

  const instruct = await runCli(
    [
      "instruct",
      runtimeSessionId,
      "Walk the Patchbay skeleton",
      "--command-id",
      "walking-command",
      "--idempotency-key",
      "walking-command-key",
      "--json",
    ],
    cliEnv,
  );
  assert.equal(instruct.code, 0, `${instruct.stderr}\n${instruct.stdout}`);
  const submission = JSON.parse(instruct.stdout);
  assert.equal(submission.outcome, "accepted");
  assert.equal(submission.commandId, "walking-command");

  await waitForCommandCompletion(
    cliEnv,
    submission.acceptedLsn,
    submission.commandId,
    10_000,
  );
  const settled = await waitForSessionActivity(cliEnv, "idle", 5_000);
  assert.equal(settled.connectivity, "live");

  console.log(
    "Walking skeleton: core → Pi adapter/AgentSession → CLI login/instruct → durable completed/idle passed",
  );
} finally {
  for (const child of children.reverse()) await terminate(child);
  await rm(stateDirectory, { recursive: true, force: true });
}

function runCli(args, env) {
  return runProcess(process.execPath, [join(cliRoot, "dist/src/main.js"), ...args], cliRoot, env);
}

async function waitForSessionActivity(env, expected, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let last = "";
  while (Date.now() < deadline) {
    const health = await runCli(["session-health", "--json"], env);
    last = `${health.stderr}\n${health.stdout}`;
    if (health.code === 0) {
      const session = oneSession(health.stdout);
      if (session.activity === expected) return session;
    }
    await delay(25);
  }
  throw new Error(`session never reached ${expected}; last response: ${last}`);
}

async function waitForCommandCompletion(env, acceptedLsn, commandId, timeoutMs) {
  assert.ok(acceptedLsn, "accepted submission must carry an LSN");
  const [{ makeControlClient }, { CredentialStore }, protobuf, contracts] = await Promise.all([
    import(pathToFileURL(join(cliRoot, "dist/src/core-client.js")).href),
    import(pathToFileURL(join(cliRoot, "dist/src/credentials.js")).href),
    import(pathToFileURL(join(cliRoot, "node_modules/@bufbuild/protobuf/dist/esm/index.js")).href),
    import(pathToFileURL(join(cliRoot, "node_modules/@patchbay/contracts/dist/index.js")).href),
  ]);
  const client = makeControlClient(
    env.PATCHBAY_CORE_ADDR,
    env.PATCHBAY_CORE_SECRET,
    new CredentialStore(env.PATCHBAY_CREDENTIALS_PATH),
  );
  const terminalStates = new Set([
    contracts.OperationState.COMPLETED,
    contracts.OperationState.REJECTED,
    contracts.OperationState.FAILED,
    contracts.OperationState.EXPIRED,
    contracts.OperationState.CANCELLED,
    contracts.OperationState.SUPERSEDED,
  ]);
  let cursor = BigInt(acceptedLsn);
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    for await (const event of client.subscribe({
      authorityDomainId: { value: env.PATCHBAY_AUTHORITY_DOMAIN_ID },
      cursor: { value: cursor },
    })) {
      cursor = event.eventId?.lsn?.value ?? cursor;
      if (event.payload?.kind !== contracts.StoredEventKind.COMMAND_TRANSITION) continue;
      const transition = protobuf.fromBinary(
        contracts.CommandTransitionSchema,
        event.payload.payload,
      );
      if (transition.commandId?.value !== commandId) continue;
      if (transition.toState === contracts.OperationState.COMPLETED) return;
      if (terminalStates.has(transition.toState)) {
        throw new Error(`command ${commandId} terminalized as ${transition.toState}`);
      }
    }
    await delay(25);
  }
  throw new Error(`command ${commandId} never reached durable completion after LSN ${cursor}`);
}

function oneSession(stdout) {
  const sessions = JSON.parse(stdout);
  assert.equal(sessions.length, 1, stdout);
  return sessions[0];
}

async function runChecked(command, args, cwd, env) {
  const result = await runProcess(command, args, cwd, env);
  assert.equal(result.code, 0, `${command} ${args.join(" ")} failed:\n${result.stderr}`);
}

function runProcess(command, args, cwd, env) {
  return new Promise((resolveResult, reject) => {
    const child = spawn(command, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.once("error", reject);
    child.once("exit", (code) => resolveResult({ code, stdout, stderr }));
  });
}

function capture(child) {
  const output = { stdout: "", stderr: "" };
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    output.stdout += chunk;
  });
  child.stderr.on("data", (chunk) => {
    output.stderr += chunk;
  });
  return output;
}

async function waitForMatch(output, pattern, timeoutMs, description) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const match = output.stdout.match(pattern);
    if (match) return match[1] ?? match[0];
    await delay(25);
  }
  throw new Error(`${description} timed out:\nstdout=${output.stdout}\nstderr=${output.stderr}`);
}

async function waitForPort(port, child, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`process exited before binding: ${child.exitCode}`);
    try {
      await new Promise((resolveConnect, rejectConnect) => {
        const socket = new Socket();
        socket.once("error", rejectConnect);
        socket.connect(port, "127.0.0.1", () => {
          socket.destroy();
          resolveConnect();
        });
      });
      return;
    } catch {
      await delay(25);
    }
  }
  throw new Error(`port ${port} did not open`);
}

async function freePort() {
  const server = createServer();
  await new Promise((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
  const address = server.address();
  assert.ok(address && typeof address === "object");
  const port = address.port;
  await new Promise((resolveClose) => server.close(resolveClose));
  return port;
}

async function terminate(child) {
  if (child.exitCode !== null) return;
  const exited = once(child, "exit").then(() => true, () => true);
  child.kill("SIGTERM");
  if (await Promise.race([exited, delay(3_000).then(() => false)])) return;
  child.kill("SIGKILL");
  await Promise.race([exited, delay(3_000)]);
}

function delay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}
