import assert from "node:assert/strict";
import { stat, mkdtemp, rm, readdir } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ExternalEffectDisposition,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LogicalTargetIdSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnExecutionPhase,
  SpawnGenerationClaimSchema,
} from "@patchbay/contracts";
import {
  buildPiRpcArgv,
  sanitizedEnvironment,
  type ManagedPiRuntimePort,
  type PiRpcRuntime,
  type ProcessExit,
} from "../src/pi_process.js";
import { RpcPiSession } from "../src/pi_session.js";
import {
  RuntimeActionFencedError,
  RuntimeActionGate,
} from "../src/runtime_action_gate.js";
import { FileSpawnEffectJournal } from "../src/spawn_journal.js";

const cwd = process.cwd();

test("RuntimeActionGate serializes actions and holds a replacement fence until promotion", async () => {
  const gate = new RuntimeActionGate();
  const order: string[] = [];
  let releaseAction!: () => void;
  const held = new Promise<void>((resolve) => { releaseAction = resolve; });
  const first = gate.runAction("delivery", async () => {
    order.push("action-start");
    await held;
    order.push("action-end");
  });
  const leasePromise = gate.acquireReplacement("claim-1").then((lease) => {
    order.push("replacement-owned");
    return lease;
  });
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.deepEqual(order, ["action-start"]);
  releaseAction();
  await first;
  const lease = await leasePromise;
  assert.deepEqual(order, ["action-start", "action-end", "replacement-owned"]);
  await assert.rejects(
    gate.runAction("query", async () => undefined),
    RuntimeActionFencedError,
  );
  lease.promoted();
  await gate.runAction("query", async () => order.push("post-promotion"));
  assert.equal(order.at(-1), "post-promotion");
});

test("RpcPiSession production requests cannot bypass replacement ownership", async () => {
  const gate = new RuntimeActionGate();
  const requests: string[] = [];
  let releaseFirst!: () => void;
  let firstStarted!: () => void;
  const started = new Promise<void>((resolve) => { firstStarted = resolve; });
  const held = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const rpc = {
    async request<T>(command: Record<string, unknown> & { readonly type: string }): Promise<T> {
      requests.push(command.type);
      if (command.type === "get_state") {
        return {
          sessionId: "rpc-gate-runtime",
          sessionFile: join(cwd, "rpc-gate-runtime.jsonl"),
          isStreaming: false,
          isCompacting: false,
          pendingMessageCount: 0,
          model: null,
          thinkingLevel: "off",
        } as T;
      }
      if (command.type === "set_thinking_level") {
        firstStarted();
        await held;
        return {} as T;
      }
      if (command.type === "get_available_models") return { models: [] } as T;
      return {} as T;
    },
    onEvent() { return () => undefined; },
    onFailure() { return () => undefined; },
    close() {},
  };
  const runtime = {
    pid: 41,
    processToken: "rpc-gate-process",
    rpc,
    exit: new Promise<ProcessExit>(() => undefined),
    child: {},
    markExpectedTermination() {},
    onTransportFailure() { return () => undefined; },
  } as unknown as PiRpcRuntime;
  const runtimePort: ManagedPiRuntimePort = {
    async launch() { return runtime; },
    async handshake() { throw new Error("unused handshake"); },
    async terminate(): Promise<ProcessExit> {
      return {
        pid: runtime.pid,
        processToken: runtime.processToken,
        code: 0,
        signal: null,
        expected: true,
        terminatedBySupervisor: true,
      };
    },
  };
  const session = await RpcPiSession.bind({
    runtimeSessionId: "rpc-gate-runtime",
    generation: 1,
    runtime,
    runtimePort,
    actionGate: gate,
    publication: "current",
  });
  try {
    const first = session.setThinkingLevel("high");
    await started;
    const replacement = gate.acquireReplacement("rpc-gate-replacement");
    await new Promise<void>((resolve) => setImmediate(resolve));
    const forbidden = assert.rejects(
      session.getAvailableModels(),
      RuntimeActionFencedError,
    );
    assert.deepEqual(requests, ["get_state", "set_thinking_level"]);
    releaseFirst();
    await first;
    const lease = await replacement;
    await forbidden;
    assert.deepEqual(
      requests,
      ["get_state", "set_thinking_level"],
      "no second stdin action crosses after replacement ownership queues",
    );
    lease.promoted();
  } finally {
    releaseFirst();
    await session.dispose();
  }
});

test("RuntimeActionGate poison is sticky and forbids a different automatic replacement", async () => {
  const gate = new RuntimeActionGate();
  const lease = await gate.acquireReplacement("claim-poisoned");
  lease.poison();
  assert.equal(gate.poisoned, true);
  await assert.rejects(gate.runAction("delivery", async () => undefined), RuntimeActionFencedError);
  await assert.rejects(gate.acquireReplacement("different-claim"), RuntimeActionFencedError);
});

test("Pi launch argv is RPC-only and environment inheritance excludes credentials", () => {
  const argv = buildPiRpcArgv({
    cliPath: process.execPath,
    controlExtensionPath: process.execPath,
    sessionPath: join(cwd, "session.jsonl"),
    model: "provider/model",
  });
  assert.deepEqual(argv.slice(0, 6), [
    process.execPath,
    "--mode",
    "rpc",
    "--no-extensions",
    "--extension",
    process.execPath,
  ]);
  assert.ok(argv.includes("--session"));

  const previous = process.env["OPENAI_API_KEY"];
  process.env["OPENAI_API_KEY"] = "ambient-must-not-cross";
  try {
    const environment = sanitizedEnvironment("n".repeat(43));
    assert.equal(environment["OPENAI_API_KEY"], undefined);
    assert.equal(environment["PATCHBAY_LAUNCH_NONCE"], "n".repeat(43));
    assert.throws(
      () => sanitizedEnvironment("n".repeat(43), { PATCHBAY_SECRET: "forbidden" }),
      /forbidden variable/,
    );
  } finally {
    if (previous === undefined) delete process.env["OPENAI_API_KEY"];
    else process.env["OPENAI_API_KEY"] = previous;
  }
});

test("effect journal is atomic 0600 evidence with monotonic phases and one launch attempt", async () => {
  const directory = await mkdtemp(join(cwd, "tmp-journal-primitives-"));
  const journal = new FileSpawnEffectJournal(directory);
  const claim = create(SpawnGenerationClaimSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority" }),
    claimOperationId: create(CommandIdSchema, { value: "claim-journal" }),
    logicalTargetId: create(LogicalTargetIdSchema, { value: "logical" }),
    claimedGeneration: create(GenerationSchema, { value: 1n }),
  });
  try {
    await journal.beginClaim({
      exactClaim: claim,
      launchNonce: "n".repeat(43),
      targetFingerprint: "a".repeat(64),
      createdAt: new Date().toISOString(),
    });
    const [file] = await readdir(directory);
    assert.ok(file);
    assert.equal((await stat(join(directory, file))).mode & 0o777, 0o600);

    await journal.recordPhase({
      claimOperationId: "claim-journal",
      phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,
      recordedAt: new Date().toISOString(),
      poisoned: true,
    });
    await assert.rejects(journal.recordPhase({
      claimOperationId: "claim-journal",
      phase: SpawnExecutionPhase.OFFERED,
      externalEffectDisposition: ExternalEffectDisposition.PROVED_NONE,
      recordedAt: new Date().toISOString(),
    }), /cannot regress/);
    await assert.rejects(journal.recordPhase({
      claimOperationId: "claim-journal",
      phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      externalEffectDisposition: ExternalEffectDisposition.IDENTIFIED,
      recordedAt: new Date().toISOString(),
    }), /at most one launch attempt/);

    const runtime = create(RuntimeGenerationRefSchema, {
      logicalTargetId: create(LogicalTargetIdSchema, { value: "logical" }),
      externalRuntime: create(ExternalRuntimeRefSchema, {
        adapterId: create(AdapterIdSchema, { value: "pi" }),
        deploymentScope: "machine",
        runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime" }),
        generation: create(GenerationSchema, { value: 1n }),
      }),
    });
    await journal.recordExternalIdentity({
      claimOperationId: "claim-journal",
      runtime,
      processToken: "process-token",
      pid: 42,
      recordedAt: new Date().toISOString(),
    });
    const state = await journal.reconcile("claim-journal");
    assert.equal(state?.exactClaim.claimedGeneration?.value, 1n);
    assert.equal(state?.externalIdentity?.runtime.externalRuntime?.runtimeSessionId?.value, "runtime");
    assert.equal(state?.poisoned, true);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
