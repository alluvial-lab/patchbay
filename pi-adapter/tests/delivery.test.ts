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
import {
  AuthStorage,
  ModelRegistry,
  SessionManager,
  SettingsManager,
} from "@earendil-works/pi-coding-agent";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import { PatchbayCoreClient, type SessionIdentity } from "../src/core_client.js";
import type { AdapterDiagnosticInput } from "../src/adapter_diagnostics.js";
import { DeliveryTranslator, UnsupportedCommandError } from "../src/delivery.js";
import { AdapterProcess, type PreprovisionedSession } from "../src/main.js";
import { PiSession } from "../src/pi_session.js";
import { SessionRegistry } from "../src/session_registry.js";

const encoder = new TextEncoder();

test("DeliveryTranslator maps instruct/cancel/session-new and rejects spawn", async () => {
  const calls: string[] = [];
  const session = {
    runtimeSessionId: "runtime-1",
    prompt: async (text: string) => calls.push(`prompt:${text}`),
    cancel: async () => calls.push("cancel"),
    newSession: async () => {
      calls.push("new");
      return 2;
    },
  } as unknown as PiSession;
  const translator = new DeliveryTranslator();

  await translator.deliver(operation(OperationKind.INSTRUCT, "hello"), session);
  await translator.deliver(operation(OperationKind.CANCEL), session);
  const replaced = await translator.deliver(
    operation(OperationKind.SESSION_MANAGEMENT, JSON.stringify({ action: "new" })),
    session,
  );
  assert.deepEqual(calls, ["prompt:hello", "cancel", "new"]);
  assert.equal(replaced.sessionGenerationChanged, true);
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
  const authStorage = AuthStorage.inMemory({
    [provider]: { type: "api_key", key: "test-key" },
  });
  const modelRegistry = ModelRegistry.inMemory(authStorage);
  const baseModel = faux.getModel();
  modelRegistry.registerProvider(provider, {
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
  const pi = await PiSession.create({
    ...configured,
    model: `${provider}/model-a`,
    sessionOptions: {
      modelRegistry,
      sessionManager: SessionManager.inMemory(configured.cwd),
      settingsManager: SettingsManager.inMemory(),
      noTools: "all",
    },
  });

  const reports: Array<{
    model: string;
    activity: SessionActivityState;
    connectivity: SessionConnectivityState;
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
    connectivity = SessionConnectivityState.LIVE,
  ) => {
    reports.push({ model: identity.model, activity, connectivity });
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

    await pi.setModel(provider, "model-b");
    await pi.setModel(provider, "model-c");
    releaseTranscript();
    await prompt;
    await adapter.flushObservations();

    assert.deepEqual(
      pi.getEntries().entries
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
      [SessionActivityState.IDLE, SessionActivityState.WORKING, SessionActivityState.WORKING],
      "model changes preserve the real in-flight activity state",
    );
  } finally {
    releaseTranscript();
    await adapter.dispose();
    await pi.dispose();
    PatchbayCoreClient.prototype.attach = originalAttach;
    PatchbayCoreClient.prototype.ingestTranscript = originalIngestTranscript;
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
