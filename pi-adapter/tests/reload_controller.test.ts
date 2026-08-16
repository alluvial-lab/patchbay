import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  CommandIdSchema,
  ExternalRuntimeRefSchema,
  FailureCode,
  GenerationSchema,
  LogicalTargetIdSchema,
  OperationKind,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiReconfigureOutcome,
  PiReconfigureRequestSchema,
  PiReloadableResourceKind,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  type Operation,
  type RuntimeGenerationRef,
} from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
} from "../extensions/patchbay-control.js";
import type { PiControlHandshake } from "../src/control_handshake.js";
import { FilePiCursorStore } from "../src/cursor_store.js";
import { PiEntryReconciler } from "../src/entry_reconciler.js";
import type {
  ManagedPiRuntimePort,
  PiHandshakeChallenge,
  PiRpcRuntime,
  ProcessExit,
} from "../src/pi_process.js";
import { buildPiRpcArgv, RpcManagedPiRuntimePort } from "../src/pi_process.js";
import { RpcPiSession } from "../src/pi_session.js";
import {
  PiReloadAmbiguousError,
  PiReloadController,
  PiReloadRejectedError,
} from "../src/reload_controller.js";
import { RuntimeActionGate } from "../src/runtime_action_gate.js";

const fixturePath = join(process.cwd(), "tests", "fixtures", "session-valid.jsonl");
const initialEpoch = Buffer.alloc(16, 31).toString("base64url");
const nextEpoch = Buffer.alloc(16, 32).toString("base64url");
const reloadNonce = Buffer.alloc(32, 33).toString("base64url");
const launchNonce = Buffer.alloc(32, 34).toString("base64url");
const challenge = Buffer.alloc(32, 35).toString("base64url");

test("idle materialized reload completes only after both markers, new handshake, rebind, and reconcile", async () => {
  await withHarness(async (harness) => {
    const result = await harness.controller.reloadEnumeratedResources(
      reloadOperation(),
      harness.runtime,
    );
    assert.equal(result.outcome, PiReconfigureOutcome.RELOADED);
    assert.deepEqual(harness.states, ["stale", "live"]);
    assert.equal(harness.rpc.promptCalls, 1);
    assert.equal(harness.runtimePort.handshakeCalls, 1);
    assert.equal(harness.rpc.eventSubscriptionCount, 2, "post-reload hooks were rebound");
    assert.equal(harness.publications.length, 1, "cursor reconciliation reached durable core acknowledgement");
    assert.equal(harness.runtime.pid, 901);
    assert.equal(harness.session.generation, 7);

    const physical = (await readFile(harness.sessionFile, "utf8"))
      .trimEnd()
      .split("\n")
      .map((line) => JSON.parse(line) as Record<string, unknown>);
    const customTypes = physical.map((entry) => entry.customType).filter(Boolean);
    assert.ok(customTypes.includes(PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE));
    assert.ok(customTypes.includes(PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE));
    assert.equal(
      physical.filter((entry) => entry.customType === PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE).length,
      2,
      "the new epoch supplied a second challenged handshake",
    );
  });
});

test("real Pi reload refreshes the entrypoint but leaves transitive and installed-package dist artifacts old", { timeout: 30_000 }, async () => {
  const root = await mkdtemp(join(process.cwd(), "tmp-real-reload-controller-"));
  const sessionFile = join(root, "session.jsonl");
  const dependencyPath = join(root, "reload-dependency.mjs");
  const packageRoot = join(root, "node_modules", "@patchbay", "reload-dist-probe");
  const packageDistPath = join(packageRoot, "dist", "index.mjs");
  const extensionPath = join(root, "reload-entrypoint.ts");
  const source = await readFile(fixturePath, "utf8");
  const sessionObjects = source.trimEnd().split("\n").map(
    (line) => JSON.parse(line) as Record<string, unknown>,
  );
  sessionObjects[0]!["cwd"] = await realpath(root);
  await writeFile(
    sessionFile,
    `${sessionObjects.map((entry) => JSON.stringify(entry)).join("\n")}\n`,
    { mode: 0o600 },
  );
  await writeFile(dependencyPath, "export const dependencyVersion = 'A';\n", { mode: 0o600 });
  await mkdir(join(packageRoot, "dist"), { recursive: true });
  await writeFile(join(packageRoot, "package.json"), JSON.stringify({
    name: "@patchbay/reload-dist-probe",
    type: "module",
    exports: "./dist/index.mjs",
  }), { mode: 0o600 });
  await writeFile(packageDistPath, "export const distVersion = 'A';\n", { mode: 0o600 });
  const controlPath = await realpath(fileURLToPath(
    new URL("../extensions/patchbay-control.js", import.meta.url),
  ));
  const writeWrapper = (entrypointVersion: string) => writeFile(extensionPath, `
import patchbayControl from ${JSON.stringify(pathToFileURL(controlPath).href)};
import { dependencyVersion } from "./reload-dependency.mjs";
import { distVersion } from "@patchbay/reload-dist-probe";
export default function reloadProbe(pi) {
  patchbayControl(pi);
  pi.on("session_start", (event) => {
    if (event.reason === "reload") {
      pi.appendEntry("patchbay.test.reload-probe.v1", { entrypointVersion: ${JSON.stringify(entrypointVersion)}, dependencyVersion, distVersion });
    }
  });
}
`, { mode: 0o600 });
  await writeWrapper("A");

  const piIndexPath = fileURLToPath(import.meta.resolve("@earendil-works/pi-coding-agent"));
  const cliPath = await realpath(join(dirname(piIndexPath), "cli.js"));
  const runtimePort = new RpcManagedPiRuntimePort();
  let runtime: PiRpcRuntime | undefined;
  let session: RpcPiSession | undefined;
  try {
    runtime = await runtimePort.launch({
      executable: await realpath(process.execPath),
      argv: buildPiRpcArgv({
        cliPath,
        controlExtensionPath: await realpath(extensionPath),
        sessionPath: await realpath(sessionFile),
      }),
      cwd: await realpath(root),
      launchNonce,
      environment: { PI_OFFLINE: "1" },
    });
    const initial = await runtimePort.handshake(runtime, {
      expectedProjectCwd: await realpath(root),
      expectedExtensionPath: await realpath(extensionPath),
      rpcTimeoutMs: 10_000,
    });
    const gate = new RuntimeActionGate();
    session = await RpcPiSession.bind({
      runtimeSessionId: "session-fixture",
      generation: 1,
      runtime,
      runtimePort,
      actionGate: gate,
      publication: "current",
      initialControlHandshake: initial,
      controlExtensionPath: await realpath(extensionPath),
    });
    const publications: Uint8Array[] = [];
    const reconciler = new PiEntryReconciler(
      new FilePiCursorStore(join(root, "cursor")),
      { async publish(_runtime, _schemaRef, payload) { publications.push(payload); } },
    );
    await writeWrapper("B");
    await writeFile(dependencyPath, "export const dependencyVersion = 'B';\n", { mode: 0o600 });
    await writeFile(packageDistPath, "export const distVersion = 'B';\n", { mode: 0o600 });
    const controller = new PiReloadController({
      session,
      runtimePort,
      reconciler,
      runtimeReference: runtimeReference(1, "session-fixture"),
      logicalTargetId: "logical-reload",
      configuredSessionRoot: root,
      expectedProjectCwd: await realpath(root),
      hasConflictingDelivery: () => false,
      markRehydrating: async () => undefined,
      markRehydrated: async () => undefined,
    });
    const result = await controller.reloadEnumeratedResources(reloadOperation(), runtime);
    assert.equal(result.outcome, PiReconfigureOutcome.RELOADED);
    const probe = (await session.getEntries()).entries.find(
      (entry) => entry.type === "custom" && entry.customType === "patchbay.test.reload-probe.v1",
    );
    assert.ok(probe && probe.type === "custom");
    assert.deepEqual(probe.data, {
      entrypointVersion: "B",
      dependencyVersion: "A",
      distVersion: "A",
    });
    assert.equal(publications.length, 1);
  } finally {
    if (session) {
      await session.dispose().catch(() => undefined);
      runtime = undefined;
    }
    if (runtime) await runtimePort.terminate(runtime).catch(() => undefined);
    await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 10 });
  }
});

test("streaming, compacting, queued, retry-unsettled, delivery-busy, and unmaterialized reload reject before effect", async () => {
  const cases: Array<{
    readonly name: string;
    readonly configure: (harness: ReloadHarness) => Promise<void> | void;
    readonly reason: string;
  }> = [
    { name: "streaming", configure: (h) => { h.rpc.streaming = true; }, reason: "busy_streaming" },
    { name: "compacting", configure: (h) => { h.rpc.compacting = true; }, reason: "busy_compacting" },
    { name: "queued", configure: (h) => { h.rpc.pendingMessageCount = 1; }, reason: "busy_queued" },
    {
      name: "auto retry without settlement",
      configure: (h) => { h.rpc.emit({ type: "auto_retry_start" }); },
      reason: "busy_unsettled",
    },
    {
      name: "auto retry after a prior activity settled",
      configure: (h) => {
        h.rpc.emit({ type: "agent_start" });
        h.rpc.emit({ type: "agent_settled" });
        h.rpc.emit({ type: "auto_retry_start" });
      },
      reason: "busy_unsettled",
    },
    {
      name: "another delivery",
      configure: (h) => { h.conflictingDelivery = true; },
      reason: "busy_delivery",
    },
    {
      name: "memory-only marker durability",
      configure: async (h) => { await rm(h.sessionFile); },
      reason: "materialization_required",
    },
  ];

  for (const candidate of cases) {
    await withHarness(async (harness) => {
      await candidate.configure(harness);
      await assert.rejects(
        harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime),
        (error: unknown) =>
          error instanceof PiReloadRejectedError && error.reason === candidate.reason,
        candidate.name,
      );
      assert.equal(harness.rpc.promptCalls, 0, `${candidate.name} must not invoke the reload command`);
      assert.equal(harness.rpc.reloadRequestCount(), 0, `${candidate.name} must not append a request marker`);
      assert.deepEqual(harness.states, []);
    });
  }
});

test("an in-flight direct RPC causes immediate busy rejection and the gate closes the later-delivery race", async () => {
  await withHarness(async (harness) => {
    let release!: () => void;
    harness.rpc.modelsBlock = new Promise<void>((resolve) => { release = resolve; });
    const query = harness.session.getAvailableModels();
    await waitUntil(() => harness.rpc.pendingRequestCount === 1);
    await assert.rejects(
      harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime),
      (error: unknown) =>
        error instanceof PiReloadRejectedError && error.reason === "busy_direct_rpc",
    );
    assert.equal(harness.rpc.promptCalls, 0);
    release();
    await query;

    let laterCompleted = false;
    harness.rpc.holdPrompt = true;
    const reload = harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime);
    await waitUntil(() => harness.rpc.releasePrompt !== undefined);
    const later = harness.session.getAvailableModels().then(() => { laterCompleted = true; });
    await new Promise<void>((resolve) => setImmediate(resolve));
    assert.equal(laterCompleted, false, "new stdin actions stay fenced through reload rehydration");
    harness.rpc.releasePrompt?.();
    await reload;
    await later;
  });
});

test("one marker, old-epoch handshake, and forged baseline correlation never report reload success", async () => {
  const cases: Array<{
    readonly name: string;
    readonly configure: (harness: ReloadHarness) => Promise<void> | void;
  }> = [
    { name: "request marker only", configure: (h) => { h.rpc.appendCompletion = false; } },
    { name: "old epoch handshake", configure: (h) => { h.runtimePort.returnOldEpoch = true; } },
    {
      name: "request marker forged before command",
      configure: (h) => h.rpc.appendForgedRequest("reload-command", reloadNonce),
    },
    {
      name: "mismatched completion nonce",
      configure: (h) => { h.rpc.completionNonce = Buffer.alloc(32, 39).toString("base64url"); },
    },
  ];
  for (const candidate of cases) {
    await withHarness(async (harness) => {
      await candidate.configure(harness);
      await assert.rejects(
        harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime),
        PiReloadAmbiguousError,
        candidate.name,
      );
      assert.equal(harness.publications.length, 0, `${candidate.name} cannot reconcile as success`);
      assert.equal(harness.states.at(-1), "stale");
    }, { maxMarkerPolls: 1 });
  }
});

test("a persisted completed reload reconciles without blindly invoking ctx.reload again", async () => {
  await withHarness(async (harness) => {
    await harness.rpc.appendPersistedCompletedReload();
    const result = await harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime);
    assert.equal(result.outcome, PiReconfigureOutcome.RELOADED);
    assert.equal(harness.rpc.promptCalls, 0);
    assert.equal(harness.runtimePort.handshakeCalls, 1);
    assert.equal(harness.publications.length, 1);
    assert.deepEqual(harness.states, ["stale", "live"]);
  });
});

test("persisted post-effect reporting failures remain redacted execution ambiguity", async () => {
  const rawFailure = "secret session report failed";
  const cases: Array<{
    readonly name: string;
    readonly arrange: (harness: ReloadHarness) => Promise<void>;
  }> = [
    {
      name: "completed marker pair",
      arrange: (harness) => harness.rpc.appendPersistedCompletedReload(),
    },
    {
      name: "conflicting marker set",
      arrange: (harness) => harness.rpc.appendForgedRequest("reload-command", reloadNonce),
    },
  ];

  for (const candidate of cases) {
    await withHarness(async (harness) => {
      await candidate.arrange(harness);
      await assert.rejects(
        harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime),
        (error: unknown) => {
          assert.ok(error instanceof PiReloadAmbiguousError, candidate.name);
          assert.equal(error.failureCode, FailureCode.EXECUTION_OUTCOME_UNKNOWN);
          assert.equal(error.message.includes(rawFailure), false, "raw reporting error is redacted");
          return true;
        },
      );
      assert.equal(harness.rpc.promptCalls, 0, "recovery must not invoke ctx.reload again");
    }, {
      markRehydrating: async () => { throw new Error(rawFailure); },
    });
  }
});

test("completion without an earlier request is invalid durable state and rejects before command", async () => {
  await withHarness(async (harness) => {
    harness.rpc.appendForgedCompletion("missing-request");
    await assert.rejects(
      harness.controller.reloadEnumeratedResources(reloadOperation(), harness.runtime),
      (error: unknown) =>
        error instanceof PiReloadRejectedError && error.reason === "materialization_required",
    );
    assert.equal(harness.rpc.promptCalls, 0);
  });
});

test("unknown enum scope requires process replacement without reload effect", async () => {
  await withHarness(async (harness) => {
    const result = await harness.controller.reloadEnumeratedResources(
      reloadOperation([999 as PiReloadableResourceKind]),
      harness.runtime,
    );
    assert.equal(result.outcome, PiReconfigureOutcome.PROCESS_REPLACEMENT_REQUIRED);
    assert.equal(harness.rpc.promptCalls, 0);
    assert.equal(harness.rpc.reloadRequestCount(), 0);
  });
});

interface ReloadHarness {
  readonly root: string;
  readonly sessionFile: string;
  readonly rpc: FakeReloadRpc;
  readonly runtime: PiRpcRuntime;
  readonly runtimePort: FakeRuntimePort;
  readonly session: RpcPiSession;
  readonly controller: PiReloadController;
  readonly publications: Uint8Array[];
  readonly states: string[];
  conflictingDelivery: boolean;
}

async function withHarness(
  action: (harness: ReloadHarness) => Promise<void>,
  controllerOptions: {
    readonly maxMarkerPolls?: number;
    readonly markRehydrating?: () => Promise<void>;
  } = {},
): Promise<void> {
  const root = await mkdtemp(join(process.cwd(), "tmp-reload-controller-"));
  const cursorRoot = join(root, "cursor");
  const sessionFile = join(root, "session.jsonl");
  const source = await readFile(fixturePath, "utf8");
  const objects = source.trimEnd().split("\n").map((line) => JSON.parse(line) as Record<string, unknown>);
  const header = objects[0]!;
  const entries = objects.slice(1);
  const initialHandshake = handshakeMarker(
    "initial-handshake",
    entries.at(-1)?.id as string,
    initialEpoch,
    sessionFile,
  );
  entries.push(initialHandshake.entry);
  const rpc = new FakeReloadRpc(sessionFile, header, entries);
  await rpc.persist();
  const runtime = fakeRuntime(rpc);
  const runtimePort = new FakeRuntimePort(rpc, runtime);
  const gate = new RuntimeActionGate();
  const session = await RpcPiSession.bind({
    runtimeSessionId: "runtime-reload",
    generation: 7,
    runtime,
    runtimePort,
    actionGate: gate,
    publication: "current",
    initialControlHandshake: initialHandshake.handshake,
    controlExtensionPath: process.execPath,
  });
  const publications: Uint8Array[] = [];
  const reconciler = new PiEntryReconciler(new FilePiCursorStore(cursorRoot), {
    async publish(_runtime, _schemaRef, payload) { publications.push(payload); },
  });
  const states: string[] = [];
  const harness = {
    root,
    sessionFile,
    rpc,
    runtime,
    runtimePort,
    session,
    publications,
    states,
    conflictingDelivery: false,
  } as ReloadHarness;
  const controller = new PiReloadController({
    session,
    runtimePort,
    reconciler,
    runtimeReference: runtimeReference(),
    logicalTargetId: "logical-reload",
    configuredSessionRoot: root,
    expectedProjectCwd: process.cwd(),
    hasConflictingDelivery: () => harness.conflictingDelivery,
    markRehydrating: controllerOptions.markRehydrating
      ?? (async () => { states.push("stale"); }),
    markRehydrated: async () => { states.push("live"); },
    randomBytes: () => Buffer.alloc(32, 33),
    sleep: async () => undefined,
    ...(controllerOptions.maxMarkerPolls
      ? { maxMarkerPolls: controllerOptions.maxMarkerPolls }
      : {}),
  });
  Object.assign(harness, { controller });
  try {
    await action(harness);
  } finally {
    rpc.releasePrompt?.();
    await session.dispose().catch(() => undefined);
    await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 10 });
  }
}

class FakeReloadRpc {
  readonly #eventListeners = new Set<(event: Record<string, unknown>) => void>();
  readonly #failureListeners = new Set<() => void>();
  readonly #header: Record<string, unknown>;
  readonly entries: Record<string, unknown>[];
  pendingRequestCount = 0;
  promptCalls = 0;
  eventSubscriptionCount = 0;
  streaming = false;
  compacting = false;
  pendingMessageCount = 0;
  appendCompletion = true;
  completionNonce = reloadNonce;
  holdPrompt = false;
  releasePrompt: (() => void) | undefined;
  modelsBlock: Promise<void> | undefined;
  #sequence = 100;

  constructor(
    readonly sessionFile: string,
    header: Record<string, unknown>,
    entries: Record<string, unknown>[],
  ) {
    this.#header = header;
    this.entries = entries;
  }

  async request<T>(command: Record<string, unknown> & { readonly type: string }): Promise<T> {
    this.pendingRequestCount += 1;
    try {
      switch (command.type) {
        case "get_state":
          return {
            sessionId: "session-fixture",
            sessionFile: this.sessionFile,
            isStreaming: this.streaming,
            isCompacting: this.compacting,
            pendingMessageCount: this.pendingMessageCount,
            model: null,
            thinkingLevel: "off",
          } as T;
        case "get_entries":
          return { entries: structuredClone(this.entries), leafId: this.entries.at(-1)?.id ?? null } as T;
        case "get_available_models":
          await this.modelsBlock;
          return { models: [] } as T;
        case "prompt":
          this.promptCalls += 1;
          await this.appendReloadFromPrompt(String(command["message"]));
          if (this.holdPrompt) {
            await new Promise<void>((resolve) => { this.releasePrompt = resolve; });
          }
          return {} as T;
        default:
          return {} as T;
      }
    } finally {
      this.pendingRequestCount -= 1;
    }
  }

  onEvent(listener: (event: Record<string, unknown>) => void): () => void {
    this.eventSubscriptionCount += 1;
    this.#eventListeners.add(listener);
    return () => this.#eventListeners.delete(listener);
  }

  onFailure(listener: () => void): () => void {
    this.#failureListeners.add(listener);
    return () => this.#failureListeners.delete(listener);
  }

  emit(event: Record<string, unknown>): void {
    for (const listener of this.#eventListeners) listener(event);
  }

  close(): void {}

  reloadRequestCount(): number {
    return this.entries.filter(
      (entry) => entry.customType === PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
    ).length;
  }

  async appendForgedRequest(commandId: string, nonce: string): Promise<void> {
    this.appendEntry(PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE, {
      commandId,
      nonce,
      priorExtensionEpoch: initialEpoch,
      resources: [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
    });
    await this.persist();
  }

  async appendPersistedCompletedReload(): Promise<void> {
    const request = this.appendEntry(PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE, {
      commandId: "reload-command",
      nonce: reloadNonce,
      priorExtensionEpoch: initialEpoch,
      resources: [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
    });
    this.appendEntry(PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE, {
      commandId: "reload-command",
      nonce: reloadNonce,
      requestEntryId: request.id,
      priorExtensionEpoch: initialEpoch,
      extensionEpoch: nextEpoch,
    });
    await this.persist();
  }

  async appendForgedCompletion(requestEntryId: string): Promise<void> {
    this.appendEntry(PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE, {
      commandId: "reload-command",
      nonce: reloadNonce,
      requestEntryId,
      priorExtensionEpoch: initialEpoch,
      extensionEpoch: nextEpoch,
    });
    await this.persist();
  }

  appendHandshake(epoch: string): PiControlHandshake {
    const marker = handshakeMarker(
      `reload-handshake-${this.#sequence}`,
      this.entries.at(-1)?.id as string,
      epoch,
      this.sessionFile,
    );
    this.entries.push(marker.entry);
    return marker.handshake;
  }

  async persist(): Promise<void> {
    await writeFile(
      this.sessionFile,
      `${[this.#header, ...this.entries].map((entry) => JSON.stringify(entry)).join("\n")}\n`,
      { mode: 0o600 },
    );
  }

  private async appendReloadFromPrompt(message: string): Promise<void> {
    const encoded = message.split(" ")[1];
    assert.ok(encoded);
    const request = JSON.parse(Buffer.from(encoded, "base64url").toString("utf8")) as Record<string, unknown>;
    const requestEntry = this.appendEntry(PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE, request);
    if (this.appendCompletion) {
      this.appendEntry(PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE, {
        commandId: request.commandId,
        nonce: this.completionNonce,
        requestEntryId: requestEntry.id,
        priorExtensionEpoch: request.priorExtensionEpoch,
        extensionEpoch: nextEpoch,
      });
    }
    await this.persist();
  }

  private appendEntry(customType: string, data: unknown): Record<string, unknown> {
    this.#sequence += 1;
    const entry = {
      type: "custom",
      id: `reload${this.#sequence}`,
      parentId: this.entries.at(-1)?.id ?? null,
      timestamp: new Date(Date.UTC(2026, 7, 12, 0, 0, this.#sequence % 60)).toISOString(),
      customType,
      data,
    };
    this.entries.push(entry);
    return entry;
  }
}

class FakeRuntimePort implements ManagedPiRuntimePort {
  handshakeCalls = 0;
  returnOldEpoch = false;

  constructor(
    readonly rpc: FakeReloadRpc,
    readonly runtime: PiRpcRuntime,
  ) {}

  async launch(): Promise<PiRpcRuntime> { return this.runtime; }

  async handshake(_runtime: PiRpcRuntime, challengeOptions: PiHandshakeChallenge): Promise<PiControlHandshake> {
    this.handshakeCalls += 1;
    const epoch = this.returnOldEpoch ? initialEpoch : nextEpoch;
    const handshake = this.rpc.appendHandshake(epoch);
    await this.rpc.persist();
    assert.equal(challengeOptions.previousExtensionEpoch, initialEpoch);
    return handshake;
  }

  async terminate(): Promise<ProcessExit> {
    return {
      pid: this.runtime.pid,
      processToken: this.runtime.processToken,
      code: 0,
      signal: null,
      expected: true,
      terminatedBySupervisor: true,
    };
  }
}

function fakeRuntime(rpc: FakeReloadRpc): PiRpcRuntime {
  return {
    pid: 901,
    processToken: "reload-process-token",
    rpc,
    exit: new Promise<ProcessExit>(() => undefined),
    child: {},
    markExpectedTermination() {},
    onTransportFailure(listener: () => void) {
      return rpc.onFailure(listener);
    },
  } as unknown as PiRpcRuntime;
}

function handshakeMarker(
  id: string,
  parentId: string,
  extensionEpoch: string,
  sessionFile: string,
): { readonly entry: Record<string, unknown>; readonly handshake: PiControlHandshake } {
  const data = {
    challenge,
    launchNonce,
    extensionEpoch,
    cwd: process.cwd(),
    sessionId: "session-fixture",
    sessionFile,
  };
  return {
    entry: {
      type: "custom",
      id,
      parentId,
      timestamp: "2026-08-12T00:00:03.000Z",
      customType: PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
      data,
    },
    handshake: { ...data, markerEntryId: id },
  };
}

function runtimeReference(generation = 7, runtimeSessionId = "runtime-reload"): RuntimeGenerationRef {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: "logical-reload" }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: "pi" }),
      deploymentScope: "machine-reload",
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: runtimeSessionId }),
      generation: create(GenerationSchema, { value: BigInt(generation) }),
    }),
  });
}

function reloadOperation(
  resources: readonly PiReloadableResourceKind[] = [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
): Operation {
  return create(OperationSchema, {
    kind: OperationKind.RECONFIGURE,
    commandId: create(CommandIdSchema, { value: "reload-command" }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.PiReconfigureRequest",
      payload: toBinary(
        PiReconfigureRequestSchema,
        create(PiReconfigureRequestSchema, { reloadResources: [...resources] }),
      ),
    }),
  });
}

async function waitUntil(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 1_000; attempt += 1) {
    if (predicate()) return;
    await new Promise<void>((resolve) => setTimeout(resolve, 1));
  }
  throw new Error("condition was not observed");
}
