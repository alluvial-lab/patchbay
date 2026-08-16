import assert from "node:assert/strict";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import {
  buildPiRpcArgv,
  RpcManagedPiRuntimePort,
  type PiRpcRuntime,
} from "../src/pi_process.js";

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
