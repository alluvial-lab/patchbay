import assert from "node:assert/strict";
import { mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import {
  buildPiRpcArgv,
  NodePiProcessLauncher,
  RpcManagedPiRuntimePort,
  type PiRpcRuntime,
} from "../src/pi_process.js";

test("stubborn process group forces bounded SIGKILL escalation and child cleanup", { timeout: 15_000 }, async () => {
  const directory = await mkdtemp(join(process.cwd(), "tmp-stubborn-process-group-"));
  const script = join(directory, "stubborn.cjs");
  await writeFile(script, `
const { spawn } = require("node:child_process");
process.on("SIGTERM", () => undefined);
const child = spawn(process.execPath, ["-e", "process.on('SIGTERM',()=>{});setInterval(()=>{},1000)"], {
  stdio: "ignore",
});
setTimeout(() => {
  process.stdout.write(JSON.stringify({ type: "stubborn_ready", childPid: child.pid }) + "\\n");
}, 25);
setInterval(() => undefined, 1000);
`, { mode: 0o600 });
  const launcher = new NodePiProcessLauncher();
  let runtime: PiRpcRuntime | undefined;
  try {
    runtime = await launcher.launch({
      executable: await realpath(process.execPath),
      argv: [script],
      cwd: await realpath(directory),
      launchNonce: "k".repeat(43),
    });
    const childPid = await new Promise<number>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("stubborn child did not become ready")), 2_000);
      runtime!.rpc.onEvent((event) => {
        if (event.type !== "stubborn_ready" || !Number.isSafeInteger(event["childPid"])) return;
        clearTimeout(timer);
        resolve(event["childPid"] as number);
      });
    });
    const exit = await launcher.terminate(runtime, {
      termTimeoutMs: 50,
      killTimeoutMs: 2_000,
    });
    runtime = undefined;
    assert.equal(exit.signal, "SIGKILL");
    assert.equal(exit.terminatedBySupervisor, true);
    await waitForProcessGone(childPid);
  } finally {
    if (runtime) {
      await launcher.terminate(runtime, {
        termTimeoutMs: 50,
        killTimeoutMs: 2_000,
      }).catch(() => undefined);
      try {
        process.kill(-runtime.pid, "SIGKILL");
      } catch (error) {
        const code = typeof error === "object" && error !== null && "code" in error
          ? error.code
          : undefined;
        if (code !== "ESRCH") throw error;
      }
    }
    await rm(directory, { recursive: true, force: true });
  }
});

// Real process, real extension, real JSONL transport; no model request or
// credential/catalog discovery is needed for the challenged control path.
test("real offline pi --mode rpc child handshakes and exits as one supervised process group", { timeout: 30_000 }, async () => {
  const directory = await mkdtemp(join(process.cwd(), "tmp-real-pi-rpc-"));
  const sessionDirectory = join(directory, "sessions");
  await (await import("node:fs/promises")).mkdir(sessionDirectory);
  const piIndexPath = fileURLToPath(import.meta.resolve("@earendil-works/pi-coding-agent"));
  const cliPath = await realpath(join(dirname(piIndexPath), "cli.js"));
  const extensionPath = await realpath(fileURLToPath(
    new URL("../extensions/patchbay-control.js", import.meta.url),
  ));
  const cwd = await realpath(directory);
  const executable = await realpath(process.execPath);
  const runtimePort = new RpcManagedPiRuntimePort();
  let runtime: PiRpcRuntime | undefined;
  try {
    runtime = await runtimePort.launch({
      executable,
      argv: buildPiRpcArgv({
        cliPath,
        controlExtensionPath: extensionPath,
        sessionDirectory,
      }),
      cwd,
      launchNonce: "n".repeat(43),
      environment: { PI_OFFLINE: "1" },
    });
    const handshake = await runtimePort.handshake(runtime, {
      expectedProjectCwd: cwd,
      expectedExtensionPath: extensionPath,
      rpcTimeoutMs: 10_000,
    });
    assert.equal(handshake.cwd, cwd);
    assert.ok(handshake.sessionId);
    assert.ok(handshake.sessionFile.startsWith(sessionDirectory));
    const exit = await runtimePort.terminate(runtime, {
      termTimeoutMs: 5_000,
      killTimeoutMs: 5_000,
    });
    runtime = undefined;
    assert.equal(exit.expected, true);
    assert.equal(exit.terminatedBySupervisor, true);
    assert.ok(
      exit.code === 0 || exit.code === 143 || exit.signal === "SIGTERM" || exit.signal === "SIGKILL",
      JSON.stringify(exit),
    );
  } finally {
    if (runtime) await runtimePort.terminate(runtime).catch(() => undefined);
    await rm(directory, { recursive: true, force: true });
  }
});

async function waitForProcessGone(pid: number): Promise<void> {
  const deadline = Date.now() + 2_000;
  while (Date.now() < deadline) {
    try {
      process.kill(pid, 0);
    } catch (error) {
      const code = typeof error === "object" && error !== null && "code" in error
        ? error.code
        : undefined;
      if (code === "ESRCH") return;
      throw error;
    }
    await new Promise<void>((resolve) => setTimeout(resolve, 20));
  }
  throw new Error("stubborn process-group child survived SIGKILL escalation");
}
