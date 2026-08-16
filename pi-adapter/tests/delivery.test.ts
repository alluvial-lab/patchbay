import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, realpath, rm } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AcceptedOperationSchema,
  AdapterIdSchema,
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  DeliverySchema,
  EventIdSchema,
  ExternalRuntimeRefSchema,
  FailureCode,
  FreshSpawnSchema,
  GenerationSchema,
  GrantIdSchema,
  LogicalTargetIdSchema,
  LsnSchema,
  OperationKind,
  OperationSchema,
  type EventId,
  type Operation,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiReconfigureRequestSchema,
  PiReloadableResourceKind,
  PiSpawnTargetSpecSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SpawnClaimAcceptedSchema,
  SpawnExecutionPhase,
  SpawnGenerationClaimSchema,
  SpawnPromotionCommittedSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type Delivery,
  type SpawnClaimAccepted,
} from "@patchbay/contracts";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import {
  PatchbayCoreClient,
  PI_RPC_TARGET_SHAPE,
  type SessionIdentity,
} from "../src/core_client.js";
import type { SessionReportOrder } from "../src/session_report_sequencer.js";
import type { AdapterDiagnosticInput } from "../src/adapter_diagnostics.js";
import { DeliveryTranslator, UnsupportedCommandError } from "../src/delivery.js";
import {
  AdapterProcess,
  classifyDeliveryFailure,
  type PreprovisionedSession,
} from "../src/main.js";
import type { PiControlHandshake } from "../src/control_handshake.js";
import {
  type ManagedPiRuntimePort,
  type PiHandshakeChallenge,
  type PiLaunchSpec,
  type PiRpcRuntime,
  type ProcessExit,
} from "../src/pi_process.js";
import { AgentSessionRuntimeFixture, type PiSession } from "../src/pi_session.js";
import {
  createOfflineFixtureServices,
  createOfflineModelRuntime,
} from "./offline_agent_fixture.js";
import { PiRpcTransportError } from "../src/rpc_client.js";
import { PiReloadAmbiguousError, PiReloadRejectedError } from "../src/reload_controller.js";
import { SessionRegistry } from "../src/session_registry.js";
import { PI_SPAWN_TARGET_SCHEMA_REF } from "../src/spawn_supervisor.js";

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
    (error: unknown) =>
      error instanceof UnsupportedCommandError &&
      error.message === "Pi spawn requires a managed Delivery.accepted_spawn envelope",
  );
});

test("DeliveryTranslator routes typed Pi reload through the bounded controller seam", async () => {
  const seen: Operation[] = [];
  const expected = { outcome: "rehydrated" };
  const session = {} as PiSession;
  const translator = new DeliveryTranslator(async (candidate, receivedSession) => {
    seen.push(candidate);
    assert.equal(receivedSession, session);
    return expected;
  });
  const candidate = create(OperationSchema, {
    kind: OperationKind.RECONFIGURE,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.PiReconfigureRequest",
      payload: toBinary(PiReconfigureRequestSchema, create(PiReconfigureRequestSchema, {
        reloadResources: [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
      })),
    }),
  });

  assert.doesNotThrow(() => translator.validate(candidate));
  assert.deepEqual(await translator.deliver(candidate, session), { value: expected });
  assert.deepEqual(seen, [candidate]);
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
    /Pi spawn requires a managed Delivery\.accepted_spawn envelope/u,
    "a bare session-target spawn still rejects before running",
  );
});

test("production delivery loop routes the exact accepted spawn envelope to the supervisor", { timeout: 15_000 }, async () => {
  const commandId = "delivery-loop-managed-spawn";
  const projectContextRef = "delivery-loop-project";
  const deploymentScope = "delivery-loop-machine";
  const runtimeSessionId = "delivery-loop-successor";
  const directory = await mkdtemp(join(process.cwd(), "tmp-delivery-loop-spawn-"));
  const canonicalDirectory = await realpath(directory);
  const journalDirectory = join(directory, "journal");
  const acceptedSpawn = managedAcceptedSpawn({
    commandId,
    projectContextRef,
    logicalTargetId: commandId,
  });
  const compatibilityOperation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.ADAPTER,
      adapterId: create(AdapterIdSchema, { value: "pi" }),
    }),
  });
  const promotedRuntime = create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: commandId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: "pi" }),
      deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: runtimeSessionId }),
      generation: create(GenerationSchema, { value: 1n }),
    }),
  });
  let reportResult!: () => void;
  const resultReported = new Promise<void>((resolve) => {
    reportResult = resolve;
  });
  const transitions: string[] = [];
  const runtimePort = new DeliveryLoopRuntimePort(
    journalDirectory,
    canonicalDirectory,
    runtimeSessionId,
  );
  const deliveryStream = async function* (signal?: AbortSignal): AsyncGenerator<Delivery> {
    yield create(DeliverySchema, {
      // Deliberately non-authoritative compatibility carriage: if the adapter
      // reconstructs from this bare Operation instead of accepted_spawn, it
      // routes as a query and the supervisor is never reached.
      operation: compatibilityOperation,
      deliveryEventId: testEventId(10n),
      acceptedSpawn,
    });
    await resultReported;
    yield create(DeliverySchema, {
      deliveryEventId: testEventId(20n),
      promotionCommitted: create(SpawnPromotionCommittedSchema, {
        acceptedClaim: acceptedSpawn,
        promotedRuntime,
      }),
    });
    await waitForAbort(signal);
  };

  const originals = {
    attach: PatchbayCoreClient.prototype.attach,
    receiveDeliveries: PatchbayCoreClient.prototype.receiveDeliveries,
    acknowledgeDelivery: PatchbayCoreClient.prototype.acknowledgeDelivery,
    reportRunning: PatchbayCoreClient.prototype.reportRunning,
    reportSpawnEvidence: PatchbayCoreClient.prototype.reportSpawnEvidence,
    reportSession: PatchbayCoreClient.prototype.reportSession,
    ingestPiProjection: PatchbayCoreClient.prototype.ingestPiProjection,
    reportSpawnResult: PatchbayCoreClient.prototype.reportSpawnResult,
    ingestFailure: PatchbayCoreClient.prototype.ingestFailure,
  };
  PatchbayCoreClient.prototype.attach = async () => testEventId(1n);
  PatchbayCoreClient.prototype.receiveDeliveries = (_cursor, signal) => deliveryStream(signal);
  PatchbayCoreClient.prototype.acknowledgeDelivery = async (candidate) => {
    assert.equal(candidate.kind, OperationKind.SPAWN);
    transitions.push("delivered");
    return testEventId(11n);
  };
  PatchbayCoreClient.prototype.reportRunning = async (candidate) => {
    assert.equal(candidate.kind, OperationKind.SPAWN);
    transitions.push("running");
    return testEventId(12n);
  };
  PatchbayCoreClient.prototype.reportSpawnEvidence = async (input) => {
    assert.equal(input.exactClaim.claimOperationId?.value, commandId);
    transitions.push(`evidence:${input.phase}`);
    return testEventId(13n);
  };
  PatchbayCoreClient.prototype.reportSession = async (
    identity,
    activity,
    connectivity,
  ) => {
    assert.equal(identity.runtimeSessionId, runtimeSessionId);
    transitions.push(`session:${connectivity}:${activity}`);
    return testEventId(14n);
  };
  PatchbayCoreClient.prototype.ingestPiProjection = async (runtime) => {
    assert.equal(runtime.externalRuntime?.runtimeSessionId?.value, runtimeSessionId);
    transitions.push("projection-published");
    return testEventId(15n);
  };
  PatchbayCoreClient.prototype.reportSpawnResult = async (candidate) => {
    assert.equal(candidate.commandId?.value, commandId);
    transitions.push("result-reported");
    reportResult();
    return testEventId(16n);
  };
  PatchbayCoreClient.prototype.ingestFailure = async (_candidate, failureCode) => {
    transitions.push(`failure:${failureCode}`);
    return testEventId(17n);
  };

  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [],
    managedTargets: [{
      projectContextRef,
      deploymentTarget: {
        credentialPolicy: "credential-free",
        adapterId: "pi",
        deploymentScope,
        logicalTargetId: commandId,
      },
      cwd: canonicalDirectory,
      sessionRoot: canonicalDirectory,
      executable: await realpath(process.execPath),
      cliPath: await realpath(process.execPath),
      controlExtensionPath: await realpath(process.execPath),
    }],
    spawnJournalDirectory: journalDirectory,
    cursorStoreDirectory: join(directory, "cursor"),
    managedRuntimePort: runtimePort,
  });
  const controller = new AbortController();
  let run: Promise<void> | undefined;
  try {
    await adapter.start();
    run = adapter.run(controller.signal);
    void run.catch(() => undefined);
    await waitUntil(
      () => transitions.includes(
        `session:${SessionConnectivityState.LIVE}:${SessionActivityState.IDLE}`,
      ),
      5_000,
    );

    assert.equal(runtimePort.launchCalls, 1, "the production loop engages the supervisor once");
    assert.equal(transitions[0], "delivered");
    assert.equal(transitions[1], "running");
    assert.ok(transitions.includes(`evidence:${SpawnExecutionPhase.LAUNCH_ATTEMPTED}`));
    assert.ok(transitions.includes(`evidence:${SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN}`));
    assert.ok(transitions.includes(`evidence:${SpawnExecutionPhase.HANDSHAKE_RECONCILING}`));
    assert.ok(transitions.includes(`evidence:${SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED}`));
    assert.ok(transitions.includes("projection-published"));
    assert.equal(transitions.some((transition) => transition.startsWith("failure:")), false);

    const journalFiles = (await readdir(journalDirectory)).filter((name) => name.endsWith(".json"));
    assert.equal(journalFiles.length, 1);
    const journal = JSON.parse(
      await readFile(join(journalDirectory, journalFiles[0]!), "utf8"),
    ) as {
      claim?: { claimOperationId?: string; claimedGeneration?: string };
      phases?: readonly { phase?: number }[];
      promotionObserved?: boolean;
      publicationCommitted?: boolean;
    };
    assert.equal(journal.claim?.claimOperationId, commandId);
    assert.equal(journal.claim?.claimedGeneration, "1");
    assert.equal(journal.promotionObserved, true);
    assert.equal(journal.publicationCommitted, true);
    assert.ok(
      journal.phases?.some((phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED),
    );

    controller.abort();
    await run;
    run = undefined;
    await adapter.dispose();
    assert.equal(runtimePort.terminateCalls, 1, "the injected child is reaped after loop shutdown");
  } finally {
    controller.abort();
    await run?.catch(() => undefined);
    await adapter.dispose().catch(() => undefined);
    PatchbayCoreClient.prototype.attach = originals.attach;
    PatchbayCoreClient.prototype.receiveDeliveries = originals.receiveDeliveries;
    PatchbayCoreClient.prototype.acknowledgeDelivery = originals.acknowledgeDelivery;
    PatchbayCoreClient.prototype.reportRunning = originals.reportRunning;
    PatchbayCoreClient.prototype.reportSpawnEvidence = originals.reportSpawnEvidence;
    PatchbayCoreClient.prototype.reportSession = originals.reportSession;
    PatchbayCoreClient.prototype.ingestPiProjection = originals.ingestPiProjection;
    PatchbayCoreClient.prototype.reportSpawnResult = originals.reportSpawnResult;
    PatchbayCoreClient.prototype.ingestFailure = originals.ingestFailure;
    await rm(directory, { recursive: true, force: true });
  }
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

test("delivery classification preserves post-write RPC ambiguity and session axes", () => {
  const busyReload = classifyDeliveryFailure(new PiReloadRejectedError("busy_streaming"));
  assert.equal(busyReload.failureCode, FailureCode.DELIVERY_REJECTED);
  assert.equal(busyReload.rejected, true);
  assert.equal(busyReload.diagnostic, "pi_reload_busy_streaming");

  const ambiguousReload = classifyDeliveryFailure(new PiReloadAmbiguousError());
  assert.equal(ambiguousReload.failureCode, FailureCode.EXECUTION_OUTCOME_UNKNOWN);
  assert.equal(ambiguousReload.connectivity, SessionConnectivityState.STALE);
  assert.equal(ambiguousReload.rejected, false);
  assert.equal(ambiguousReload.diagnostic, "pi_reload_rehydration_outcome_unknown");
  assert.equal(ambiguousReload.diagnostic.includes("session report failed"), false);

  const timeout = classifyDeliveryFailure(new PiRpcTransportError(
    "timeout",
    "secret raw timeout",
    undefined,
    "possibly_written",
  ));
  assert.equal(timeout.failureCode, FailureCode.EXECUTION_OUTCOME_UNKNOWN);
  assert.equal(timeout.connectivity, SessionConnectivityState.STALE);
  assert.equal(timeout.diagnostic, "rpc_execution_outcome_unknown");
  assert.equal(timeout.diagnostic.includes("secret"), false);

  const uncleanExit = classifyDeliveryFailure(new PiRpcTransportError(
    "process_exit",
    "secret raw exit",
    { code: 9, signal: null, expected: false },
    "possibly_written",
  ));
  assert.equal(uncleanExit.failureCode, FailureCode.EXECUTION_OUTCOME_UNKNOWN);
  assert.equal(uncleanExit.connectivity, SessionConnectivityState.FAILED);

  const bareEof = classifyDeliveryFailure(new PiRpcTransportError(
    "eof",
    "secret raw eof",
    undefined,
    "possibly_written",
  ));
  assert.equal(bareEof.failureCode, FailureCode.EXECUTION_OUTCOME_UNKNOWN);
  assert.equal(bareEof.connectivity, SessionConnectivityState.STALE);

  const preWrite = classifyDeliveryFailure(new PiRpcTransportError(
    "pipe",
    "secret raw prewrite",
    undefined,
    "proved_not_written",
  ));
  assert.equal(preWrite.failureCode, FailureCode.EXECUTION_FAILED);
  assert.equal(preWrite.connectivity, SessionConnectivityState.STALE);
  assert.equal(preWrite.diagnostic, "rpc_request_not_written");
});

test("AdapterProcess never preprovisions a managed logical target outside the journal", async () => {
  const directory = await mkdtemp(join(process.cwd(), "tmp-managed-startup-"));
  const originalAttach = PatchbayCoreClient.prototype.attach;
  PatchbayCoreClient.prototype.attach = async () => ({}) as EventId;
  let launchCalls = 0;
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
    adapterGeneration: 1,
    sessions: [{
      cwd: process.cwd(),
      runtimeSessionId: "managed-must-not-launch",
      deploymentScope: "machine-a",
      logicalTargetId: "logical-managed",
    } as unknown as PreprovisionedSession],
    spawnJournalDirectory: directory,
    managedRuntimePort: {
      async launch() {
        launchCalls += 1;
        throw new Error("managed auto-launch mutant crossed the process port");
      },
      async handshake() {
        throw new Error("unexpected handshake");
      },
      async terminate() {
        throw new Error("unexpected termination");
      },
    },
  });
  try {
    await assert.rejects(
      adapter.start(),
      /managed logical targets recover only from the spawn journal/,
    );
    assert.equal(launchCalls, 0);
  } finally {
    PatchbayCoreClient.prototype.attach = originalAttach;
    await adapter.dispose().catch(() => undefined);
    await rm(directory, { recursive: true, force: true });
  }
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
      onPersistedEntry() {
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
    onPersistedEntry: () => () => undefined,
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

test("SessionRegistry suppresses current-runtime callbacks while replacement authority is fenced", async () => {
  const registry = new SessionRegistry();
  let transcript: (() => void) | undefined;
  let model: (() => void) | undefined;
  let lifecycle: (() => void) | undefined;
  let persisted: (() => void) | undefined;
  let observations = 0;
  const session = {
    runtimeSessionId: "runtime-fenced",
    onTranscript(listener: () => void) {
      transcript = listener;
      return () => undefined;
    },
    onModelChange(listener: () => void) {
      model = listener;
      return () => undefined;
    },
    onLifecycle(listener: () => void) {
      lifecycle = listener;
      return () => undefined;
    },
    onPersistedEntry(listener: () => void) {
      persisted = listener;
      return () => undefined;
    },
    async dispose() {},
  } as unknown as PiSession;
  registry.register(
    {
      runtimeSessionId: "runtime-fenced",
      deploymentScope: "machine-a",
      cwd: "/work/patchbay",
      logicalTargetId: "logical-target",
    },
    session,
    () => { observations += 1; },
    () => { observations += 1; },
    () => { observations += 1; },
    () => { observations += 1; },
  );

  const lease = await registry.gateFor("logical-target").acquireReplacement("claim-next");
  transcript?.();
  model?.();
  lifecycle?.();
  persisted?.();
  assert.equal(observations, 0, "prior callbacks cannot escape an accepted replacement fence");

  lease.release();
  transcript?.();
  model?.();
  lifecycle?.();
  persisted?.();
  assert.equal(observations, 4, "released fences restore current-runtime observation ownership");

  const runtime = {
    pid: 1,
    processToken: "process-token",
    rpc: {
      pendingRequestCount: 0,
      async request() {
        return {
          sessionId: "runtime-fenced",
          sessionFile: "/work/patchbay/session.jsonl",
          isStreaming: false,
          isCompacting: false,
          pendingMessageCount: 0,
        };
      },
    },
  } as unknown as PiRpcRuntime;
  await registry.gateFor("logical-target").withExclusiveCurrent(runtime, async () => {
    transcript?.();
    model?.();
    lifecycle?.();
    persisted?.();
    assert.equal(observations, 4, "reload-owned callbacks cannot queue behind their own action fence");
  });
  transcript?.();
  model?.();
  lifecycle?.();
  persisted?.();
  assert.equal(observations, 8, "reload observation fencing is released with exclusive ownership");
  await registry.dispose();
});

test("SessionRegistry owns complete runtime entries and observation wiring", async () => {
  const registry = new SessionRegistry();
  let transcriptListener: ((event: never) => void) | undefined;
  let modelChangeListener: ((model: string) => void) | undefined;
  let persistedEntryListener: (() => void) | undefined;
  let unsubscribed = false;
  let modelUnsubscribed = false;
  let lifecycleUnsubscribed = false;
  let persistedEntryUnsubscribed = false;
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
    onPersistedEntry(listener: () => void) {
      persistedEntryListener = listener;
      return () => {
        persistedEntryUnsubscribed = true;
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
  let observedPersistedEntry = false;
  registry.register(
    config,
    session,
    (entry) => {
      observedEntryName = entry.name ?? "";
    },
    (_entry, model) => {
      observedModel = model;
    },
    () => undefined,
    () => {
      observedPersistedEntry = true;
    },
  );
  const entry = registry.resolve("runtime-1");
  assert.equal(entry?.session, session);
  assert.equal(entry?.deploymentScope, "machine-a");
  transcriptListener?.({} as never);
  assert.equal(observedEntryName, "dynamic");
  modelChangeListener?.("provider/model-2");
  assert.equal(observedModel, "provider/model-2");
  persistedEntryListener?.();
  assert.equal(observedPersistedEntry, true);
  assert.throws(
    () => registry.register(config, session, () => undefined, () => undefined),
    /already registered/,
  );
  await registry.dispose();
  assert.equal(unsubscribed, true);
  assert.equal(modelUnsubscribed, true);
  assert.equal(lifecycleUnsubscribed, true);
  assert.equal(persistedEntryUnsubscribed, true);
});

class DeliveryLoopRpc {
  readonly #events = new Set<(event: Record<string, unknown>) => void>();
  readonly #failures = new Set<(error: PiRpcTransportError) => void>();
  readonly pendingRequestCount = 0;

  constructor(
    readonly sessionId: string,
    readonly sessionFile: string,
  ) {}

  async request<T>(command: Record<string, unknown> & { readonly type: string }): Promise<T> {
    switch (command.type) {
      case "get_state":
        return {
          sessionId: this.sessionId,
          sessionFile: this.sessionFile,
          isStreaming: false,
          isCompacting: false,
          pendingMessageCount: 0,
          model: null,
          thinkingLevel: "off",
        } as T;
      case "get_entries":
        return { entries: [], leafId: null } as T;
      default:
        return {} as T;
    }
  }

  onEvent(listener: (event: Record<string, unknown>) => void): () => void {
    this.#events.add(listener);
    return () => this.#events.delete(listener);
  }

  onFailure(listener: (error: PiRpcTransportError) => void): () => void {
    this.#failures.add(listener);
    return () => this.#failures.delete(listener);
  }

  close(): void {
    this.#events.clear();
    this.#failures.clear();
  }
}

class DeliveryLoopRuntimePort implements ManagedPiRuntimePort {
  launchCalls = 0;
  terminateCalls = 0;

  constructor(
    readonly journalDirectory: string,
    readonly cwd: string,
    readonly runtimeSessionId: string,
  ) {}

  async launch(_spec: PiLaunchSpec): Promise<PiRpcRuntime> {
    const journalFiles = (await readdir(this.journalDirectory))
      .filter((name) => name.endsWith(".json"));
    assert.equal(journalFiles.length, 1, "the effect journal exists before process launch");
    const journal = JSON.parse(
      await readFile(join(this.journalDirectory, journalFiles[0]!), "utf8"),
    ) as { phases?: readonly { phase?: number }[] };
    assert.equal(
      journal.phases?.at(-1)?.phase,
      SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      "launch_attempted is durable before the external effect",
    );
    this.launchCalls += 1;
    const rpc = new DeliveryLoopRpc(
      this.runtimeSessionId,
      join(this.cwd, "memory-only-session.jsonl"),
    );
    return {
      pid: 20_001,
      processToken: "delivery-loop-process",
      rpc: rpc as unknown as PiRpcRuntime["rpc"],
      exit: new Promise<ProcessExit>(() => undefined),
      child: {} as PiRpcRuntime["child"],
      markExpectedTermination() {},
      onTransportFailure(listener) {
        return rpc.onFailure(listener);
      },
    };
  }

  async handshake(
    runtime: PiRpcRuntime,
    _challenge: PiHandshakeChallenge,
  ): Promise<PiControlHandshake> {
    const rpc = runtime.rpc as unknown as DeliveryLoopRpc;
    return {
      challenge: "c".repeat(43),
      launchNonce: "n".repeat(43),
      extensionEpoch: "e".repeat(43),
      cwd: this.cwd,
      sessionId: rpc.sessionId,
      sessionFile: rpc.sessionFile,
      markerEntryId: "delivery-loop-marker",
    };
  }

  async terminate(runtime: PiRpcRuntime): Promise<ProcessExit> {
    this.terminateCalls += 1;
    runtime.rpc.close();
    return {
      pid: runtime.pid,
      processToken: runtime.processToken,
      code: 0,
      signal: null,
      expected: true,
      terminatedBySupervisor: true,
    };
  }
}

function managedAcceptedSpawn(options: {
  readonly commandId: string;
  readonly projectContextRef: string;
  readonly logicalTargetId: string;
}): SpawnClaimAccepted {
  const piTarget = create(PiSpawnTargetSpecSchema, {
    projectContextRef: options.projectContextRef,
  });
  const request = create(SpawnRequestSchema, {
    intent: { case: "fresh", value: create(FreshSpawnSchema) },
    targetSpec: create(SpawnTargetSpecSchema, {
      shape: PI_RPC_TARGET_SHAPE,
      adapterPayload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: PI_SPAWN_TARGET_SCHEMA_REF,
        payload: toBinary(PiSpawnTargetSpecSchema, piTarget),
      }),
    }),
  });
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: options.commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
    kind: OperationKind.SPAWN,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.ADAPTER,
      adapterId: create(AdapterIdSchema, { value: "pi" }),
    }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.SpawnRequest",
      payload: toBinary(SpawnRequestSchema, request),
    }),
  });
  return create(SpawnClaimAcceptedSchema, {
    acceptedOperation: create(AcceptedOperationSchema, {
      operation,
      authorizingGrantId: create(GrantIdSchema, { value: "spawn-grant" }),
    }),
    claim: create(SpawnGenerationClaimSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
      claimOperationId: create(CommandIdSchema, { value: options.commandId }),
      logicalTargetId: create(LogicalTargetIdSchema, { value: options.logicalTargetId }),
      claimedGeneration: create(GenerationSchema, { value: 1n }),
    }),
  });
}

function testEventId(lsn: bigint): EventId {
  return create(EventIdSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
    lsn: create(LsnSchema, { value: lsn }),
  });
}

async function waitForAbort(signal?: AbortSignal): Promise<void> {
  if (!signal || signal.aborted) return;
  await new Promise<void>((resolve) => {
    signal.addEventListener("abort", () => resolve(), { once: true });
  });
}

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

async function waitUntil(predicate: () => boolean, timeoutMs = 2_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("condition was not reached");
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
