import assert from "node:assert/strict";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  OperationKind,
  OperationSchema,
  type EventId,
  type Operation,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SessionActivityState,
  SessionConnectivityState,
} from "@patchbay/contracts";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import { PatchbayCoreClient, type SessionIdentity } from "../src/core_client.js";
import type { SessionReportOrder } from "../src/session_report_sequencer.js";
import type { AdapterDiagnosticInput } from "../src/adapter_diagnostics.js";
import { DeliveryTranslator, UnsupportedCommandError } from "../src/delivery.js";
import { AdapterProcess, type PreprovisionedSession } from "../src/main.js";
import { AgentSessionRuntimeFixture, type PiSession } from "../src/pi_session.js";
import {
  createOfflineFixtureServices,
  createOfflineModelRuntime,
} from "./offline_agent_fixture.js";
import { PiRpcTransportError } from "../src/rpc_client.js";
import { SessionRegistry } from "../src/session_registry.js";

const encoder = new TextEncoder();

test("DeliveryTranslator maps instruct/cancel and rejects adapter-owned generation changes", async () => {
  const calls: string[] = [];
  const session = {
    runtimeSessionId: "runtime-1",
    prompt: async (text: string) => calls.push(`prompt:${text}`),
    cancel: async () => calls.push("cancel"),
  } as unknown as PiSession;
  const translator = new DeliveryTranslator();

  await translator.deliver(operation(OperationKind.INSTRUCT, "hello"), session);
  await translator.deliver(operation(OperationKind.CANCEL), session);
  await assert.rejects(
    translator.deliver(
      operation(OperationKind.SESSION_MANAGEMENT, JSON.stringify({ action: "new" })),
      session,
    ),
    UnsupportedCommandError,
  );
  assert.deepEqual(calls, ["prompt:hello", "cancel"]);
  await assert.rejects(
    translator.deliver(operation(OperationKind.SPAWN), session),
    UnsupportedCommandError,
  );
});

test("DeliveryTranslator preflight defers malformed payload errors to execution", async () => {
  const translator = new DeliveryTranslator();
  const session = {} as PiSession;
  const malformed = [
    ["query malformed JSON", operation(OperationKind.QUERY, "{")],
    ["reconfigure non-object JSON", operation(OperationKind.RECONFIGURE, "[]")],
    [
      "session-management missing action",
      operation(OperationKind.SESSION_MANAGEMENT, "{}"),
    ],
  ] as const;

  for (const [description, candidate] of malformed) {
    assert.doesNotThrow(
      () => translator.validate(candidate),
      `${description} must not escape unsupported preflight`,
    );
    await assert.rejects(
      translator.deliver(candidate, session),
      (error: unknown) => error instanceof Error && !(error instanceof UnsupportedCommandError),
      `${description} must remain an execution failure`,
    );
  }
  assert.throws(
    () => translator.validate(operation(OperationKind.SPAWN)),
    UnsupportedCommandError,
    "semantic unsupported decisions still reject before running",
  );
});

test("DeliveryTranslator resolves committed approval decisions and rejects reserved/question responses", async () => {
  const resolutions: boolean[] = [];
  const session = {
    resolveApproval: async (_operation: Operation, approved: boolean) => {
      resolutions.push(approved);
    },
  } as unknown as PiSession;
  const translator = new DeliveryTranslator();

  await translator.deliver(approvalOperation(ApprovalDecision.APPROVED), session);
  await translator.deliver(approvalOperation(ApprovalDecision.DENIED), session);
  assert.deepEqual(resolutions, [true, false]);

  await assert.rejects(
    translator.deliver(approvalOperation(ApprovalDecision.RESERVED_ALLOW_ONCE), session),
    (error: unknown) =>
      error instanceof UnsupportedCommandError && error.failureCode === "unsupported_command",
  );
  await assert.rejects(
    translator.deliver(operation(OperationKind.ELICITATION_RESPONSE), session),
    UnsupportedCommandError,
  );
});

test("AdapterProcess preserves real Pi model_change values, activity, and order", async () => {
  const provider = "patchbay-model-switch";
  const faux = createFauxCore({ provider, api: provider, tokensPerSecond: 0 });
  faux.setResponses([
    async () => {
      await new Promise((resolve) => setTimeout(resolve, 200));
      return fauxAssistantMessage("model switch completed");
    },
  ]);
  const modelRuntime = await createOfflineModelRuntime();
  const baseModel = faux.getModel();
  modelRuntime.registerProvider(provider, {
    name: "Patchbay model-switch provider",
    apiKey: "test-key",
    baseUrl: "http://localhost:0",
    api: baseModel.api,
    streamSimple: faux.streamSimple,
    models: ["model-a", "model-b", "model-c"].map((id) => ({
      ...baseModel,
      id,
      name: id,
    })),
  });
  const configured: PreprovisionedSession = {
    cwd: process.cwd(),
    runtimeSessionId: "runtime-model-switch",
    deploymentScope: "machine-a",
    project: "patchbay",
  };
  const pi = await AgentSessionRuntimeFixture.create({
    cwd: configured.cwd,
    runtimeSessionId: configured.runtimeSessionId,
    generation: 1,
    model: `${provider}/model-a`,
    services: await createOfflineFixtureServices(configured.cwd, modelRuntime),
    noTools: "all",
  });

  const reports: Array<{
    model: string;
    activity: SessionActivityState;
    connectivity: SessionConnectivityState;
    sourceOrder: SessionReportOrder;
  }> = [];
  let blockTranscript = false;
  let transcriptStarted!: () => void;
  const transcriptStart = new Promise<void>((resolve) => {
    transcriptStarted = resolve;
  });
  let releaseTranscript!: () => void;
  const transcriptRelease = new Promise<void>((resolve) => {
    releaseTranscript = resolve;
  });
  let blockReport = false;
  let reportStarted!: () => void;
  const reportStart = new Promise<void>((resolve) => {
    reportStarted = resolve;
  });
  let releaseReport!: () => void;
  const reportRelease = new Promise<void>((resolve) => {
    releaseReport = resolve;
  });
  const originalAttach = PatchbayCoreClient.prototype.attach;
  const originalIngestTranscript = PatchbayCoreClient.prototype.ingestTranscript;
  const originalReportSession = PatchbayCoreClient.prototype.reportSession;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  PatchbayCoreClient.prototype.ingestTranscript = async () => {
    if (blockTranscript) {
      transcriptStarted();
      await transcriptRelease;
    }
    return undefined;
  };
  PatchbayCoreClient.prototype.reportSession = async (
    identity: SessionIdentity,
    activity: SessionActivityState,
    connectivity: SessionConnectivityState,
    sourceOrder: SessionReportOrder,
  ) => {
    reports.push({ model: identity.model, activity, connectivity, sourceOrder });
    if (blockReport) {
      reportStarted();
      await reportRelease;
    }
    return undefined;
  };

  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [configured],
    createSession: async () => pi,
  });
  try {
    await adapter.start();
    blockTranscript = true;
    const prompt = pi.prompt("switch models while working");
    await Promise.all([transcriptStart, waitUntil(() => pi.getState().streaming)]);

    blockReport = true;
    await pi.setModel(provider, "model-b");
    releaseTranscript();
    await reportStart;
    await pi.setModel(provider, "model-c");
    releaseReport();
    await prompt;
    await adapter.flushObservations();

    assert.deepEqual(
      (await pi.getEntries()).entries
        .filter((entry) => entry.type === "model_change")
        .map((entry) => `${entry.provider}/${entry.modelId}`)
        .slice(-2),
      [`${provider}/model-b`, `${provider}/model-c`],
      "the real Pi session persists both model_change entries in order",
    );
    assert.deepEqual(
      reports.map(({ model }) => model),
      [`${provider}/model-a`, `${provider}/model-b`, `${provider}/model-c`],
      "registration and each model_change report the event-time model exactly once",
    );
    assert.deepEqual(
      reports.map(({ activity }) => activity),
      [SessionActivityState.IDLE, SessionActivityState.IDLE, SessionActivityState.IDLE],
      "the action gate serializes model changes after the in-flight turn",
    );
    assert.deepEqual(
      reports.map(({ sourceOrder }) => sourceOrder),
      [
        { adapterGeneration: 1, revision: 1n },
        { adapterGeneration: 1, revision: 2n },
        { adapterGeneration: 1, revision: 3n },
      ],
      "enqueue-time sequencing and payload snapshots remain immutable while revision 2 is in flight",
    );
  } finally {
    releaseTranscript();
    releaseReport();
    await adapter.dispose();
    await pi.dispose();
    PatchbayCoreClient.prototype.attach = originalAttach;
    PatchbayCoreClient.prototype.ingestTranscript = originalIngestTranscript;
    PatchbayCoreClient.prototype.reportSession = originalReportSession;
  }
});

test("AdapterProcess resets report revision only for a new runtime or adapter generation", async () => {
  const reports: Array<{
    runtimeSessionId: string;
    sessionGeneration: number;
    sourceOrder: SessionReportOrder;
  }> = [];
  const originalAttach = PatchbayCoreClient.prototype.attach;
  const originalReportSession = PatchbayCoreClient.prototype.reportSession;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  PatchbayCoreClient.prototype.reportSession = async (
    identity: SessionIdentity,
    _activity: SessionActivityState,
    _connectivity: SessionConnectivityState,
    sourceOrder: SessionReportOrder,
  ) => {
    reports.push({
      runtimeSessionId: identity.runtimeSessionId,
      sessionGeneration: identity.generation,
      sourceOrder,
    });
    return undefined;
  };

  const fakeSession = (runtimeSessionId: string, initialGeneration: number) => {
    let generation = initialGeneration;
    let observeModel: ((model: string) => void) | undefined;
    const session = {
      runtimeSessionId,
      get generation() {
        return generation;
      },
      getState() {
        return {
          idle: true,
          model: { provider: "provider", id: "model-a" },
        };
      },
      snapshotTranscript() {
        return [];
      },
      onTranscript() {
        return () => undefined;
      },
      onModelChange(listener: (model: string) => void) {
        observeModel = listener;
        return () => undefined;
      },
      onLifecycle() {
        return () => undefined;
      },
      async dispose() {},
    } as unknown as PiSession;
    return {
      session,
      setGeneration(value: number) {
        generation = value;
      },
      emitModel(model: string) {
        assert.ok(observeModel);
        observeModel(model);
      },
    };
  };

  let first: AdapterProcess | undefined;
  let second: AdapterProcess | undefined;
  try {
    const runtime = fakeSession("runtime-sequencer", 4);
    const configured: PreprovisionedSession = {
      cwd: process.cwd(),
      runtimeSessionId: "runtime-sequencer",
      deploymentScope: "machine-a",
    };
    const independent = fakeSession("runtime-independent", 4);
    const independentConfig: PreprovisionedSession = {
      ...configured,
      runtimeSessionId: "runtime-independent",
    };
    first = new AdapterProcess({
      coreAddress: "http://127.0.0.1:1",
      adapterId: "pi",
      authorityDomainId: "authority-test",
      attachmentEvidence: "adapter-test-secret",
      adapterGeneration: 7,
      sessions: [configured, independentConfig],
      createSession: async (options) =>
        options.runtimeSessionId === configured.runtimeSessionId
          ? runtime.session
          : independent.session,
    });
    await first.start();
    runtime.emitModel("provider/model-b");
    await first.flushObservations();
    runtime.setGeneration(5);
    runtime.emitModel("provider/model-c");
    await first.flushObservations();
    await first.dispose();
    first = undefined;

    const replacement = fakeSession("runtime-sequencer", 5);
    second = new AdapterProcess({
      coreAddress: "http://127.0.0.1:1",
      adapterId: "pi",
      authorityDomainId: "authority-test",
      attachmentEvidence: "adapter-test-secret",
      adapterGeneration: 8,
      sessions: [configured],
      createSession: async () => replacement.session,
    });
    await second.start();

    assert.deepEqual(reports, [
      {
        runtimeSessionId: "runtime-sequencer",
        sessionGeneration: 4,
        sourceOrder: { adapterGeneration: 7, revision: 1n },
      },
      {
        runtimeSessionId: "runtime-independent",
        sessionGeneration: 4,
        sourceOrder: { adapterGeneration: 7, revision: 1n },
      },
      {
        runtimeSessionId: "runtime-sequencer",
        sessionGeneration: 4,
        sourceOrder: { adapterGeneration: 7, revision: 2n },
      },
      {
        runtimeSessionId: "runtime-sequencer",
        sessionGeneration: 5,
        sourceOrder: { adapterGeneration: 7, revision: 1n },
      },
      {
        runtimeSessionId: "runtime-sequencer",
        sessionGeneration: 5,
        sourceOrder: { adapterGeneration: 8, revision: 1n },
      },
    ]);
  } finally {
    await first?.dispose();
    await second?.dispose();
    PatchbayCoreClient.prototype.attach = originalAttach;
    PatchbayCoreClient.prototype.reportSession = originalReportSession;
  }
});

test("AdapterProcess maps transport loss and exact process exits without mutating generation", async () => {
  const reports: Array<{ connectivity: SessionConnectivityState; activity: SessionActivityState; generation: number }> = [];
  let lifecycle: ((event: Parameters<Parameters<PiSession["onLifecycle"]>[0]>[0]) => void) | undefined;
  const session = {
    runtimeSessionId: "runtime-lifecycle",
    generation: 5,
    getState: () => ({
      idle: true,
      model: { provider: "provider", id: "model" },
    }),
    snapshotTranscript: async () => [],
    onTranscript: () => () => undefined,
    onModelChange: () => () => undefined,
    onLifecycle(listener: NonNullable<typeof lifecycle>) {
      lifecycle = listener;
      return () => undefined;
    },
    dispose: async () => undefined,
  } as unknown as PiSession;
  const originalAttach = PatchbayCoreClient.prototype.attach;
  const originalReportSession = PatchbayCoreClient.prototype.reportSession;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  PatchbayCoreClient.prototype.reportSession = async (identity, activity, connectivity) => {
    reports.push({ connectivity, activity, generation: identity.generation });
    return undefined;
  };
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [{
      cwd: process.cwd(),
      runtimeSessionId: "runtime-lifecycle",
      deploymentScope: "machine-a",
      generation: 5,
    }],
    createSession: async () => session,
  });
  try {
    await adapter.start();
    assert.ok(lifecycle);
    lifecycle({
      kind: "transport_loss",
      error: new PiRpcTransportError("pipe", "test transport loss"),
    });
    lifecycle({
      kind: "process_exit",
      exit: {
        pid: 10,
        processToken: "process",
        code: 0,
        signal: null,
        expected: true,
        terminatedBySupervisor: true,
      },
    });
    lifecycle({
      kind: "process_exit",
      exit: {
        pid: 10,
        processToken: "process",
        code: 9,
        signal: null,
        expected: false,
        terminatedBySupervisor: false,
      },
    });
    await adapter.flushObservations();
    assert.deepEqual(
      reports.slice(-3),
      [
        { connectivity: SessionConnectivityState.STALE, activity: SessionActivityState.UNKNOWN, generation: 5 },
        { connectivity: SessionConnectivityState.OFFLINE, activity: SessionActivityState.UNKNOWN, generation: 5 },
        { connectivity: SessionConnectivityState.FAILED, activity: SessionActivityState.UNKNOWN, generation: 5 },
      ],
    );
  } finally {
    await adapter.dispose();
    PatchbayCoreClient.prototype.attach = originalAttach;
    PatchbayCoreClient.prototype.reportSession = originalReportSession;
  }
});

test("AdapterProcess isolates broken diagnostics from lifecycle operations", async () => {
  const diagnostics = {
    record() {
      throw new Error("diagnostics record failed");
    },
    flush: async () => {
      throw new Error("diagnostics flush failed");
    },
    close: async () => {
      throw new Error("diagnostics close failed");
    },
  };
  const originalAttach = PatchbayCoreClient.prototype.attach;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [],
    diagnostics,
  });
  try {
    await adapter.start();
    await adapter.dispose();
    await adapter.dispose();
  } finally {
    PatchbayCoreClient.prototype.attach = originalAttach;
  }
});

test("AdapterProcess preserves a hostile registration failure while logging a safe fallback", async () => {
  const records: AdapterDiagnosticInput[] = [];
  const diagnostics = {
    record(input: AdapterDiagnosticInput) {
      records.push(input);
    },
    flush: async () => undefined,
    close: async () => undefined,
  };
  const hostile = new Proxy({}, {
    get(_target, property) {
      if (property === "name" || property === "code" || property === "constructor") {
        throw new Error(`hostile getter: ${String(property)}`);
      }
      return undefined;
    },
  });
  const configured: PreprovisionedSession = {
    cwd: process.cwd(),
    runtimeSessionId: "runtime-hostile-error",
    deploymentScope: "machine-a",
    project: "patchbay",
  };
  const originalAttach = PatchbayCoreClient.prototype.attach;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [],
    createSession: async () => {
      throw hostile;
    },
    diagnostics,
  });
  try {
    await adapter.start();
    await assert.rejects(
      adapter.registerSession(configured),
      (error: unknown) => error === hostile,
    );
    const failure = records.find((record) => record.event === "session.register.failed");
    assert.deepEqual(failure?.error, { name: "Error", code: "DIAGNOSTIC_ERROR" });
  } finally {
    await adapter.dispose();
    PatchbayCoreClient.prototype.attach = originalAttach;
  }
});

test("SessionRegistry owns complete runtime entries and observation wiring", async () => {
  const registry = new SessionRegistry();
  let transcriptListener: ((event: never) => void) | undefined;
  let modelChangeListener: ((model: string) => void) | undefined;
  let unsubscribed = false;
  let modelUnsubscribed = false;
  let lifecycleUnsubscribed = false;
  const session = {
    runtimeSessionId: "runtime-1",
    onTranscript(listener: (event: never) => void) {
      transcriptListener = listener;
      return () => {
        unsubscribed = true;
      };
    },
    onModelChange(listener: (model: string) => void) {
      modelChangeListener = listener;
      return () => {
        modelUnsubscribed = true;
      };
    },
    onLifecycle() {
      return () => {
        lifecycleUnsubscribed = true;
      };
    },
    async dispose() {},
  } as unknown as PiSession;
  const config = {
    runtimeSessionId: "runtime-1",
    deploymentScope: "machine-a",
    project: "patchbay",
    cwd: "/work/patchbay",
    name: "dynamic",
  };
  let observedEntryName = "";
  let observedModel = "";
  registry.register(
    config,
    session,
    (entry) => {
      observedEntryName = entry.name ?? "";
    },
    (_entry, model) => {
      observedModel = model;
    },
  );
  const entry = registry.resolve("runtime-1");
  assert.equal(entry?.session, session);
  assert.equal(entry?.deploymentScope, "machine-a");
  transcriptListener?.({} as never);
  assert.equal(observedEntryName, "dynamic");
  modelChangeListener?.("provider/model-2");
  assert.equal(observedModel, "provider/model-2");
  assert.throws(
    () => registry.register(config, session, () => undefined, () => undefined),
    /already registered/,
  );
  await registry.dispose();
  assert.equal(unsubscribed, true);
  assert.equal(modelUnsubscribed, true);
  assert.equal(lifecycleUnsubscribed, true);
});

function operation(kind: OperationKind, payload = ""): Operation {
  return create(OperationSchema, {
    kind,
    payload: create(PayloadEnvelopeSchema, {
      payload: encoder.encode(payload),
      contentType: PayloadContentType.TEXT_UTF8,
    }),
  });
}

function approvalOperation(decision: ApprovalDecision): Operation {
  return create(OperationSchema, {
    kind: OperationKind.APPROVAL_RESPONSE,
    payload: create(PayloadEnvelopeSchema, {
      payload: toBinary(
        ApprovalResponsePayloadSchema,
        create(ApprovalResponsePayloadSchema, { decision }),
      ),
      contentType: PayloadContentType.PROTOBUF,
    }),
  });
}

async function waitUntil(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 2_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("condition was not reached");
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
