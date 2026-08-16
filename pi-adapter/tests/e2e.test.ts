import assert from "node:assert/strict";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer, Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { Code, ConnectError, createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AcceptedOperationSchema,
  AdapterDiagnosticState,
  AdapterIdSchema,
  AdapterRegistrationSchema,
  AdapterSnapshotSupport,
  AdapterStatusQuerySchema,
  AdapterTargetCategory,
  AdminService,
  AuthorityDomainIdSchema,
  BootstrapRequestSchema,
  CommandIdSchema,
  CommandTransitionSchema,
  ControlService,
  DeviceIdSchema,
  DiagnosticsQuerySchema,
  EndpointIdSchema,
  FailureCode,
  GenerationSchema,
  LoadSnapshotRequestSchema,
  LsnSchema,
  SnapshotViewKind,
  ObservationKind,
  ObservationSchema,
  OperationKind,
  OperationState,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  FreshSpawnSchema,
  PrincipalEnrollmentSchema,
  QueryDiagnosticsRequestSchema,
  QuarantinedRuntimeEvidenceSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionSnapshotSchema,
  SessionStateEventSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubmissionOutcome,
  SubmitRequestSchema,
  SubscribeRequestSchema,
  TargetScopeKind,
  TargetScopeSchema,
  TimeWindowSchema,
  VerifyOperatorPasswordRequestSchema,
  type AdapterStatus,
  type GrantId,
  type PrincipalCredential,
  type StoredEventPayload,
} from "@patchbay/contracts";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import { PatchbayCoreClient } from "../src/core_client.js";
import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { AdapterProcess, type PreprovisionedSession } from "../src/main.js";
import { AgentSessionRuntimeFixture, type PiSession } from "../src/pi_session.js";
import {
  createOfflineFixtureServices,
  createOfflineModelRuntime,
} from "./offline_agent_fixture.js";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const coreSecret = "e2e-core-secret";
const adapterEvidence = "e2e-adapter-secret";
const domainId = "authority-e2e";
const operatorId = "operator-e2e";
const operatorPassword = "correct-password";
const operatorPasswordHash =
  "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g";
const adapterId = "pi";
const runtimeSessionId = "session-e2e";
const deploymentScope = "machine-e2e";

// The test is deliberately serial: it owns one core process and one SQLite fixture.
test("core → adapter → offline AgentSession fixture → observation loop, fenced generation, reconnect, and core restart", { timeout: 180_000 }, async () => {
  const port = await freePort();
  let adminPort = await freePort();
  while (adminPort === port) adminPort = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "pi-adapter-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  const diagnosticsPath = join(directory, "adapter.log");
  let core = startCore(port, adminPort, databasePath);
  let adapter: AdapterProcess | undefined;
  let adapterController: AbortController | undefined;
  let adapterRun: Promise<void> | undefined;
  let reconnect: AdapterProcess | undefined;
  let reconnectController: AbortController | undefined;
  let reconnectRun: Promise<void> | undefined;

  try {
    const setupSecret = await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
    const auth = await bootstrapAndLogin(
      baseUrl,
      `http://127.0.0.1:${adminPort}`,
      setupSecret,
    );
    let control = makeControlClient(baseUrl, auth);
    const sessionFixture = createSessionFixture(1);
    const configured: PreprovisionedSession = {
      cwd: repoRoot,
      runtimeSessionId,
      deploymentScope,
      project: "patchbay",
      generation: 1,
    };
    const diagnostics = await openAdapterDiagnostics({
      path: diagnosticsPath,
      adapterId,
      adapterGeneration: 1,
      secrets: [adapterEvidence],
    });
    adapter = new AdapterProcess({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
      adapterGeneration: 1,
      sessions: [],
      createSession: sessionFixture.create,
      diagnostics,
      forwardDiagnostics: true,
    });
    await adapter.start();
    await waitForAdapterDiagnostic(control, "pi_adapter_started", AdapterDiagnosticState.ATTACHED);
    // Future spawn uses this same complete runtime-entry path; delivery routing
    // has no separate immutable pre-provisioned configuration dependency.
    await adapter.registerSession(configured);
    adapterController = new AbortController();
    adapterRun = adapter.run(adapterController.signal);

    const loadedSnapshot = await control.loadSnapshot(
      create(LoadSnapshotRequestSchema, {
        authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
        viewKind: SnapshotViewKind.SESSION,
      }),
    );
    assert.equal(loadedSnapshot.present, true);
    assert.equal(loadedSnapshot.viewKind, SnapshotViewKind.SESSION);
    const snapshot = fromBinary(SessionSnapshotSchema, loadedSnapshot.snapshotPayload);
    assert.equal(snapshot.authorityDomainId?.value, domainId);
    assert.equal(snapshot.snapshotLsn?.value, loadedSnapshot.eventId?.lsn?.value);
    assert.equal(snapshot.sessions.length, 1);
    assert.equal(snapshot.sessions[0]?.runtimeSessionId?.value, runtimeSessionId);
    assert.equal(snapshot.sessions[0]?.model, `${sessionFixture.faux.getModel().provider}/${sessionFixture.faux.getModel().id}`);

    const attachedEvents = await readAfter(control, 0n);
    const manifest = attachedEvents
      .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
      .map((payload) => fromBinary(ObservationSchema, payload.payload))
      .find((observation) => observation.payload?.schemaRef === "patchbay.AdapterRegistration")
      ?.payload;
    assert.ok(manifest);
    const registration = fromBinary(AdapterRegistrationSchema, manifest.payload);
    assert.equal(
      registration.capability?.supportedOperationKinds.includes(
        OperationKind.APPROVAL_RESPONSE,
      ),
      false,
    );
    assert.equal(
      registration.capability?.supportedOperationKinds.includes(
        OperationKind.ELICITATION_RESPONSE,
      ),
      false,
    );
    assert.deepEqual(registration.capability?.targetCategories, [
      AdapterTargetCategory.RUNTIME_SESSION,
    ]);
    assert.equal(
      registration.capability?.sessionSnapshotSupport,
      AdapterSnapshotSupport.PARTIAL,
    );
    assert.deepEqual(registration.capability?.resourceCapabilities, []);

    const accepted = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-instruct", OperationKind.INSTRUCT, "hello from Patchbay"),
      }),
    );
    assert.ok(accepted.acceptedLsn);
    await waitForCommandState(control, "command-instruct", OperationState.COMPLETED);

    const outputEvents = await readAfter(control, accepted.acceptedLsn?.value ?? 0n);
    assert.deepEqual(
      commandStates(outputEvents, "command-instruct"),
      [OperationState.DELIVERED, OperationState.RUNNING, OperationState.COMPLETED],
    );
    const transcriptPayloads = outputEvents
      .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
      .map((payload) => fromBinary(ObservationSchema, payload.payload))
      .filter((observation) => observation.payload?.schemaRef === "patchbay.pi.TranscriptEvent.v1")
      .map((observation) => JSON.parse(new TextDecoder().decode(observation.payload?.payload)) as { kind?: string; text?: string });
    assert.ok(
      transcriptPayloads.some(
        (event) => event.kind === "assistant_committed" && event.text === "Pi received the operation",
      ),
    );

    // The first command advances the live adapter's private stream cursor beyond
    // zero. A replacement stream then ends the old subscription; the adapter
    // must reconnect from that cursor rather than re-executing terminal history.
    let terminalExecutions = 0;
    sessionFixture.faux.appendResponses([
      async () => {
        terminalExecutions += 1;
        return fauxAssistantMessage("terminal command must not replay");
      },
    ]);
    const terminal = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-reconnect-terminal",
          OperationKind.INSTRUCT,
          "finish before reconnect",
        ),
      }),
    );
    assert.ok(terminal.acceptedLsn && terminal.acceptedLsn.value > 0n);
    await waitForCommandState(
      control,
      "command-reconnect-terminal",
      OperationState.COMPLETED,
    );
    assert.equal(terminalExecutions, 1);

    const streamInterrupter = new PatchbayCoreClient({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
    });
    await streamInterrupter.attach(1);
    const interrupterController = new AbortController();
    const interruptedStream = (async () => {
      for await (const _delivery of streamInterrupter.receiveDeliveries(
        0n,
        interrupterController.signal,
      )) {
        // Opening this replacement stream fences and closes the adapter's idle stream.
      }
    })();

    // This acceptance is the unseen tail the reconnect path must catch up;
    // the durable terminal-state wait below synchronizes on that outcome
    // instead of assuming a fixed retry delay.
    const caughtUp = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-reconnect-catch-up",
          OperationKind.INSTRUCT,
          "run exactly once after reconnect",
        ),
      }),
    );
    assert.ok(caughtUp.acceptedLsn && caughtUp.acceptedLsn.value > terminal.acceptedLsn.value);
    await waitForCommandState(
      control,
      "command-reconnect-catch-up",
      OperationState.COMPLETED,
    );
    interrupterController.abort();
    assert.equal(interrupterController.signal.aborted, true);
    let interruptedStreamOutcome: "fenced" | "aborted" = "fenced";
    try {
      await interruptedStream;
    } catch (error: unknown) {
      // The explicit abort is expected when the replacement stream is still
      // live. If the adapter has already reconnected, the core's epoch fence
      // ends this observer normally before the local abort reaches the RPC.
      assert.ok(error instanceof ConnectError);
      assert.equal(error.code, Code.Canceled);
      interruptedStreamOutcome = "aborted";
    }
    assert.ok(
      interruptedStreamOutcome === "aborted" || interruptedStreamOutcome === "fenced",
      "replacement stream must end by the explicit abort or the core epoch fence",
    );

    const reconnectCatchUpEvents = await readAfter(control, 0n);
    assert.deepEqual(
      commandStates(reconnectCatchUpEvents, "command-reconnect-terminal"),
      [OperationState.DELIVERED, OperationState.RUNNING, OperationState.COMPLETED],
      "the terminal command is filtered from reconnect delivery",
    );
    assert.equal(terminalExecutions, 1, "the terminal command never executes again");
    assert.deepEqual(
      commandStates(reconnectCatchUpEvents, "command-reconnect-catch-up"),
      [OperationState.DELIVERED, OperationState.RUNNING, OperationState.COMPLETED],
      "the later accepted operation is caught up exactly once",
    );

    const sessionNew = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-session-new",
          OperationKind.SESSION_MANAGEMENT,
          JSON.stringify({ action: "new" }),
        ),
      }),
    );
    await waitForCommandState(control, "command-session-new", OperationState.REJECTED);
    assert.equal(
      commandTransitions(await readAfter(control, 0n), "command-session-new").at(-1)?.failureCode,
      FailureCode.UNSUPPORTED_COMMAND,
    );
    const generationEvents = await readAfter(control, sessionNew.acceptedLsn?.value ?? 0n);
    assert.equal(generationEvents.some(isGenerationTwo), false);

    const query = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-query",
          OperationKind.QUERY,
          JSON.stringify({ action: "state" }),
          1,
        ),
      }),
    );
    await waitForCommandState(control, "command-query", OperationState.COMPLETED);
    const queryEvents = await readAfter(control, query.acceptedLsn?.value ?? 0n);
    const queryResult = observationsFor(queryEvents, "command-query").find(
      (observation) => observation.kind === ObservationKind.RESULT,
    );
    assert.equal(queryResult?.payload?.schemaRef, "patchbay.pi.DeliveryResult.v1");
    const queryValue = JSON.parse(
      new TextDecoder().decode(queryResult?.payload?.payload),
    ) as { value?: { generation?: number } };
    assert.equal(queryValue.value?.generation, 1);

    const malformedPayloadCases = [
      {
        slug: "query-json",
        kind: OperationKind.QUERY,
        payload: "{",
      },
      {
        slug: "reconfigure-shape",
        kind: OperationKind.RECONFIGURE,
        payload: "[]",
      },
      {
        slug: "session-management-action",
        kind: OperationKind.SESSION_MANAGEMENT,
        payload: "{}",
      },
    ] as const;
    for (const malformed of malformedPayloadCases) {
      const malformedCommandId = `command-malformed-${malformed.slug}`;
      const malformedSubmission = await control.submit(
        create(SubmitRequestSchema, {
          operation: operation(
            malformedCommandId,
            malformed.kind,
            malformed.payload,
            1,
          ),
        }),
      );
      await waitForCommandFailure(
        control,
        malformedCommandId,
        FailureCode.EXECUTION_FAILED,
      );
      const malformedEvents = await readAfter(
        control,
        malformedSubmission.acceptedLsn?.value ?? 0n,
      );
      assert.deepEqual(
        commandStates(malformedEvents, malformedCommandId),
        [OperationState.DELIVERED, OperationState.RUNNING, OperationState.FAILED],
        `${malformed.slug} terminalizes through the execution-failure path`,
      );

      const validCommandId = `command-after-malformed-${malformed.slug}`;
      const validSubmission = await control.submit(
        create(SubmitRequestSchema, {
          operation: operation(
            validCommandId,
            OperationKind.QUERY,
            JSON.stringify({ action: "state" }),
            1,
          ),
        }),
      );
      await waitForCommandState(control, validCommandId, OperationState.COMPLETED);
      const validEvents = await readAfter(
        control,
        validSubmission.acceptedLsn?.value ?? 0n,
      );
      assert.deepEqual(
        commandStates(validEvents, validCommandId),
        [OperationState.DELIVERED, OperationState.RUNNING, OperationState.COMPLETED],
        `adapter continues after ${malformed.slug}`,
      );
    }

    const cancelFixture = sessionFixture.faux;
    cancelFixture.appendResponses([
      async () => {
        await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
        return fauxAssistantMessage("late response that cancellation must settle");
      },
    ]);
    await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-long", OperationKind.INSTRUCT, "start a cancellable turn", 1),
      }),
    );
    await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-cancel", OperationKind.CANCEL, "", 1),
      }),
    );
    await waitForCommandState(control, "command-cancel", OperationState.COMPLETED);
    await waitForPiIdle(sessionFixture);

    // A future-generation acceptance cannot enter the current Pi context.
    appendAcceptedOperation(
      databasePath,
      operation("command-old-generation", OperationKind.INSTRUCT, "must not execute", 2),
      auth.grantId,
    );
    await new Promise((resolve) => setTimeout(resolve, 300));
    const staleDeliveryEvents = await readAfter(control, 0n);
    assert.deepEqual(
      commandStates(staleDeliveryEvents, "command-old-generation"),
      [],
      "the server does not deliver an operation for a non-current generation",
    );
    assert.equal(
      observationsFor(staleDeliveryEvents, "command-old-generation").some(
        (observation) => observation.payload?.schemaRef === "patchbay.pi.TranscriptEvent.v1",
      ),
      false,
      "the adapter never executes in the replacement Pi context",
    );

    const spawn = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-spawn", OperationKind.SPAWN, "", 1),
      }),
    );
    assert.equal(spawn.outcome, SubmissionOutcome.REJECTED);
    assert.equal(spawn.failureCode, FailureCode.TARGET_NOT_FOUND);
    assert.equal(spawn.operationState, OperationState.UNSPECIFIED);
    assert.equal(
      spawn.acceptedLsn,
      undefined,
      "a runtime-session target is incompatible with spawn and must reject before durability",
    );

    sessionFixture.faux.appendResponses([
      async () => {
        await new Promise((resolveDelay) => setTimeout(resolveDelay, 750));
        return fauxAssistantMessage("completion after adapter loss");
      },
    ]);
    await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-restart-mid-turn",
          OperationKind.INSTRUCT,
          "lose the adapter after running",
          1,
        ),
      }),
    );
    await waitForCommandState(control, "command-restart-mid-turn", OperationState.RUNNING);
    adapterController.abort();
    await Promise.all([
      waitForCommandFailure(
        control,
        "command-restart-mid-turn",
        FailureCode.EXECUTION_OUTCOME_UNKNOWN,
      ),
      waitForSessionConnectivity(control, SessionConnectivityState.STALE),
    ]);
    await adapter.dispose();
    await adapterRun;
    adapter = undefined;
    adapterController = undefined;
    adapterRun = undefined;

    const reconnectFixture = createSessionFixture(1, true);
    const reconnectDiagnostics = await openAdapterDiagnostics({
      path: diagnosticsPath,
      adapterId,
      adapterGeneration: 2,
      secrets: [adapterEvidence],
    });
    reconnect = new AdapterProcess({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
      adapterGeneration: 2,
      sessions: [{ ...configured, generation: 1 }],
      createSession: reconnectFixture.create,
      diagnostics: reconnectDiagnostics,
      forwardDiagnostics: true,
    });
    await reconnect.start();
    await waitForAdapterDiagnostic(control, "pi_adapter_started", AdapterDiagnosticState.ATTACHED);
    reconnectController = new AbortController();
    reconnectRun = reconnect.run(reconnectController.signal);
    assert.equal(reconnectFixture.session?.getState().generation, 1);

    const reconnectEvents = await readAfter(control, 0n);
    assert.ok(
      reconnectEvents.some((payload) => {
        if (payload.kind !== StoredEventKind.OBSERVATION) return false;
        const observation = fromBinary(ObservationSchema, payload.payload);
        return observation.payload?.schemaRef === "patchbay.pi.TranscriptEvent.v1" &&
          new TextDecoder().decode(observation.payload.payload).includes("replayed snapshot entry");
      }),
      "same-generation reconnect replays snapshot evidence into the current runtime",
    );
    assert.equal(
      reconnectEvents.some((payload) => isGenerationBumpBeyondOne(payload)),
      false,
      "adapter restart never invents a successor generation",
    );
    const attachCountBeforeRestart = reconnectEvents.filter(isAdapterRegistration).length;

    // Stop the adapter's delivery loop before restarting core so the query can
    // observe the durable diagnostic record while current attachment evidence
    // is intentionally absent.
    reconnectController.abort();
    await reconnectRun;
    reconnectController = undefined;
    core.kill("SIGTERM");
    await waitForExit(core);
    core = startCore(port, adminPort, databasePath);
    await waitForCoreListener(port, core);
    control = makeControlClient(baseUrl, await loginAfterRestart(baseUrl, auth.grantId));

    const afterRestart = await waitForAdapterDiagnostic(
      control,
      "pi_adapter_started",
      AdapterDiagnosticState.UNKNOWN,
    );
    assert.equal(afterRestart.recentDiagnostics.some((record) => record.reasonCode === "pi_adapter_started"), true);

    reconnectController = new AbortController();
    reconnectRun = reconnect.run(reconnectController.signal);
    const afterReattach = await waitForAdapterDiagnostic(
      control,
      "pi_adapter_started",
      AdapterDiagnosticState.ATTACHED,
    );
    assert.equal(afterReattach.recentDiagnostics.some((record) => record.reasonCode === "pi_adapter_started"), true);
    await waitFor(
      () => countAdapterRegistrations(databasePath) === attachCountBeforeRestart + 1,
      "the retry path durably records exactly one fresh attachment",
    );

    reconnectController.abort();
    await reconnect.dispose();
    await reconnectRun;
    reconnect = undefined;
    reconnectRun = undefined;
    const diagnosticLines = readFileSync(diagnosticsPath, "utf8")
      .trimEnd()
      .split("\n")
      .map((line) => JSON.parse(line) as Record<string, unknown>);
    const commandRecords = diagnosticLines.filter(
      (line) => line["command_id"] === "command-instruct",
    );
    assert.ok(commandRecords.some((line) => line["event"] === "delivery.received"));
    assert.ok(commandRecords.some((line) => line["event"] === "delivery.completed"));
    assert.equal(
      diagnosticLines.some((line) => line["event"] === "session.generation.changed"),
      false,
      "no adapter-local generation bump is published",
    );
    assert.ok(diagnosticLines.some((line) => line["event"] === "adapter.stopped"));
    assert.equal(diagnosticLines.some((line) => JSON.stringify(line).includes(adapterEvidence)), false);
    assert.equal(diagnosticLines.some((line) => JSON.stringify(line).includes("hello from Patchbay")), false);
  } finally {
    reconnectController?.abort();
    adapterController?.abort();
    if (reconnect) await reconnect.dispose();
    if (adapter) await adapter.dispose();
    const runs = [reconnectRun, adapterRun].filter(
      (run): run is Promise<void> => run !== undefined,
    );
    await Promise.allSettled(runs);
    core.kill("SIGTERM");
    rmSync(directory, { recursive: true, force: true });
  }
});

function createSessionFixture(generation: number, seedSnapshot = false) {
  const provider = `patchbay-e2e-${generation}`;
  const faux = createFauxCore({ provider, api: provider, tokensPerSecond: 0 });
  faux.setResponses([fauxAssistantMessage("Pi received the operation")]);
  let session: PiSession | undefined;
  return {
    faux,
    get session() {
      return session;
    },
    create: async (configured: PreprovisionedSession) => {
      const modelRuntime = await createOfflineModelRuntime();
      const model = faux.getModel();
      modelRuntime.registerProvider(provider, {
        name: provider,
        apiKey: "test",
        baseUrl: "http://localhost:0",
        api: model.api,
        streamSimple: faux.streamSimple,
        models: [
          {
            id: model.id,
            name: model.name,
            api: model.api,
            baseUrl: "http://localhost:0",
            reasoning: model.reasoning,
            input: model.input,
            cost: model.cost,
            contextWindow: model.contextWindow,
            maxTokens: model.maxTokens,
          },
        ],
      });
      const services = await createOfflineFixtureServices(repoRoot, modelRuntime);
      if (seedSnapshot) {
        services.sessionManager.appendMessage({
          role: "user",
          content: "replayed snapshot entry",
          timestamp: Date.now(),
        });
      }
      session = await AgentSessionRuntimeFixture.create({
        cwd: configured.cwd,
        runtimeSessionId: configured.runtimeSessionId,
        generation,
        model: `${provider}/${model.id}`,
        services,
        noTools: "all",
      });
      return session!;
    },
  };
}

function startCore(port: number, adminPort: number, databasePath: string): ChildProcess {
  execFileSync("cargo", ["build", "-p", "patchbay-core-server"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      CARGO_HOME: join(repoRoot, ".cargo-home"),
      PATH: `/home/agent/.cargo/bin:${process.env["PATH"] ?? ""}`,
    },
    stdio: "ignore",
  });
  return spawn(join(repoRoot, "target/debug/patchbay-core-server"), [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      PATCHBAY_CORE_SECRET: coreSecret,
      PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS: JSON.stringify({ [adapterId]: adapterEvidence }),
      PATCHBAY_BIND_ADDR: `127.0.0.1:${port}`,
      PATCHBAY_ADMIN_BIND_ADDR: `127.0.0.1:${adminPort}`,
      PATCHBAY_DB_PATH: databasePath,
      PATCHBAY_AUTHORITY_DOMAIN_ID: domainId,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function operation(
  commandId: string,
  kind: OperationKind,
  payload: string,
  generation = 1,
) {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: operatorId }),
    }),
    kind,
    targetScope: targetScope(generation),
    validityWindow: create(TimeWindowSchema, {
      startsAt: { seconds: 1n },
      expiresAt: { seconds: 2_534_023_007_99n },
    }),
    submittedAt: { seconds: 1n },
    idempotencyKey: `${commandId}-key`,
    payload: payloadEnvelope(kind, payload),
  });
}

function payloadEnvelope(kind: OperationKind, payload: string) {
  if (kind !== OperationKind.SPAWN) {
    return create(PayloadEnvelopeSchema, {
      payload: new TextEncoder().encode(payload),
      contentType: PayloadContentType.TEXT_UTF8,
    });
  }
  const request = create(SpawnRequestSchema, {
    intent: { case: "fresh", value: create(FreshSpawnSchema, {}) },
    targetSpec: create(SpawnTargetSpecSchema, { shape: "session" }),
  });
  return create(PayloadEnvelopeSchema, {
    payload: toBinary(SpawnRequestSchema, request),
    contentType: PayloadContentType.PROTOBUF,
    schemaRef: "patchbay.SpawnRequest",
  });
}

function targetScope(generation: number) {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: create(AdapterIdSchema, { value: adapterId }),
    deploymentScope,
    runtimeSessionId: create(RuntimeSessionIdSchema, { value: runtimeSessionId }),
    sessionGeneration: create(GenerationSchema, { value: BigInt(generation) }),
  });
}

interface ControlAuth {
  principal: PrincipalCredential;
  operatorSessionId: string;
  grantId: GrantId;
}

async function bootstrapAndLogin(
  baseUrl: string,
  adminBaseUrl: string,
  setupSecret: string,
): Promise<ControlAuth> {
  const enrollment = (endpoint: string) =>
    create(PrincipalEnrollmentSchema, {
      endpointId: create(EndpointIdSchema, { value: endpoint }),
      deviceId: create(DeviceIdSchema, { value: "pi-adapter-e2e-device" }),
      endpointGeneration: create(GenerationSchema, { value: 1n }),
    });
  const admin = createClient(
    AdminService,
    createGrpcTransport({ baseUrl: adminBaseUrl }),
  );
  const bootstrap = await admin.bootstrapOperator(
    create(BootstrapRequestSchema, {
      setupSecret,
      operatorActorId: create(ActorIdSchema, { value: operatorId }),
      passwordHash: operatorPasswordHash,
      principal: enrollment("pi-adapter-e2e-bootstrap"),
    }),
  );
  assert.ok(bootstrap.grantId?.value, "bootstrap creates the authority grant");

  const coreAuthenticate: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };
  const control = createClient(
    ControlService,
    createGrpcTransport({ baseUrl, interceptors: [coreAuthenticate] }),
  );
  const login = await control.verifyOperatorPassword(
    create(VerifyOperatorPasswordRequestSchema, {
      operatorActorId: create(ActorIdSchema, { value: operatorId }),
      password: operatorPassword,
      principal: enrollment("pi-adapter-e2e-control"),
    }),
  );
  assert.ok(login.principal, "password verification enrolls a transport principal");
  assert.ok(login.operatorSessionId?.value, "password verification issues an operator session");
  return {
    principal: login.principal,
    operatorSessionId: login.operatorSessionId.value,
    grantId: bootstrap.grantId,
  };
}

function makeControlClient(baseUrl: string, auth: ControlAuth) {
  const authenticate: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    request.header.set("x-patchbay-principal-id", auth.principal.principalId);
    request.header.set("x-patchbay-principal-secret", auth.principal.secret);
    request.header.set("x-patchbay-operator-id", operatorId);
    request.header.set("x-patchbay-operator-session-id", auth.operatorSessionId);
    return next(request);
  };
  return createClient(
    ControlService,
    createGrpcTransport({ baseUrl, interceptors: [authenticate] }),
  );
}

async function waitForAdapterDiagnostic(
  control: ReturnType<typeof makeControlClient>,
  code: string,
  state: AdapterDiagnosticState,
) {
  let result: AdapterStatus | undefined;
  let sequence = 0;
  await waitFor(async () => {
    const response = await control.queryDiagnostics(
      create(QueryDiagnosticsRequestSchema, {
        operation: adapterStatusOperation(`adapter-status-${code}-${state}-${sequence++}`),
      }),
    );
    if (
      response.submission?.outcome !== SubmissionOutcome.ACCEPTED ||
      response.submission.operationState !== OperationState.COMPLETED ||
      response.result.case !== "adapters"
    ) return false;
    const status = response.result.value.adapters.find(
      (candidate) => candidate.adapterId?.value === adapterId,
    );
    if (!status || status.state !== state) return false;
    if (!status.recentDiagnostics.some((record) => record.reasonCode === code)) return false;
    result = status;
    return true;
  }, `adapter diagnostic ${code} with state ${AdapterDiagnosticState[state] ?? state}`);
  return result!;
}

function adapterStatusOperation(commandId: string) {
  const query = create(DiagnosticsQuerySchema, {
    query: {
      case: "adapters",
      value: create(AdapterStatusQuerySchema, {
        adapterIds: [create(AdapterIdSchema, { value: adapterId })],
        limit: 1,
        recentDiagnosticLimit: 20,
      }),
    },
  });
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: operatorId }),
    }),
    kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.AUTHORITY_DOMAIN }),
    validityWindow: create(TimeWindowSchema, {
      startsAt: { seconds: 1n },
      expiresAt: { seconds: 2_534_023_007_99n },
    }),
    submittedAt: { seconds: 1n },
    idempotencyKey: `${commandId}-key`,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.DiagnosticsQuery",
      payload: toBinary(DiagnosticsQuerySchema, query),
    }),
  });
}

async function loginAfterRestart(baseUrl: string, grantId: GrantId): Promise<ControlAuth> {
  const coreAuthenticate: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };
  const control = createClient(
    ControlService,
    createGrpcTransport({ baseUrl, interceptors: [coreAuthenticate] }),
  );
  const login = await control.verifyOperatorPassword(
    create(VerifyOperatorPasswordRequestSchema, {
      operatorActorId: create(ActorIdSchema, { value: operatorId }),
      password: operatorPassword,
      principal: create(PrincipalEnrollmentSchema, {
        endpointId: create(EndpointIdSchema, { value: "pi-adapter-e2e-restart" }),
        deviceId: create(DeviceIdSchema, { value: "pi-adapter-e2e-device" }),
        endpointGeneration: create(GenerationSchema, { value: 2n }),
      }),
    }),
  );
  assert.ok(login.principal, "restart login enrolls a transport principal");
  assert.ok(login.operatorSessionId?.value, "restart login issues an operator session");
  return {
    principal: login.principal,
    operatorSessionId: login.operatorSessionId.value,
    grantId,
  };
}

async function readAfter(
  control: ReturnType<typeof makeControlClient>,
  cursor: bigint,
): Promise<StoredEventPayload[]> {
  const payloads: StoredEventPayload[] = [];
  for await (const event of control.subscribe(
    create(SubscribeRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      cursor: create(LsnSchema, { value: cursor }),
    }),
  )) {
    if (event.payload) payloads.push(event.payload);
  }
  return payloads;
}

async function waitFor(
  predicate: () => boolean | Promise<boolean>,
  message: string,
  timeoutMs = 10_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await predicate()) return;
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 25));
  }
  throw new Error(`timed out waiting for ${message}`);
}

async function waitForCommandState(
  control: ReturnType<typeof makeControlClient>,
  commandId: string,
  expected: OperationState,
): Promise<void> {
  await waitFor(async () => {
    const states = commandStates(await readAfter(control, 0n), commandId);
    return states.at(-1) === expected;
  }, `${commandId} to reach ${OperationState[expected] ?? expected}`);
}

async function waitForCommandFailure(
  control: ReturnType<typeof makeControlClient>,
  commandId: string,
  failureCode: FailureCode,
): Promise<void> {
  await waitFor(async () => {
    const transitions = commandTransitions(await readAfter(control, 0n), commandId);
    const terminal = transitions.at(-1);
    return terminal?.toState === OperationState.FAILED && terminal.failureCode === failureCode;
  }, `${commandId} to fail with ${FailureCode[failureCode] ?? failureCode}`);
}

async function waitForSessionConnectivity(
  control: ReturnType<typeof makeControlClient>,
  expected: SessionConnectivityState,
): Promise<void> {
  await waitFor(async () => {
    const loaded = await control.loadSnapshot(
      create(LoadSnapshotRequestSchema, {
        authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
        viewKind: SnapshotViewKind.SESSION,
      }),
    );
    if (!loaded.present) return false;
    const snapshot = fromBinary(SessionSnapshotSchema, loaded.snapshotPayload);
    return snapshot.sessions.some(
      (session) =>
        session.runtimeSessionId?.value === runtimeSessionId &&
        session.state?.connectivity === expected,
    );
  }, `session connectivity ${SessionConnectivityState[expected] ?? expected}`);
}

async function waitForPiIdle(fixture: { readonly session: PiSession | undefined }): Promise<void> {
  await waitFor(() => fixture.session?.getState().idle === true, "Pi session to become idle");
}

function commandTransitions(
  payloads: readonly StoredEventPayload[],
  commandId: string,
) {
  return payloads
    .filter((payload) => payload.kind === StoredEventKind.COMMAND_TRANSITION)
    .map((payload) => fromBinary(CommandTransitionSchema, payload.payload))
    .filter((transition) => transition.commandId?.value === commandId);
}

function commandStates(
  payloads: readonly StoredEventPayload[],
  commandId: string,
): OperationState[] {
  return payloads
    .filter((payload) => payload.kind === StoredEventKind.COMMAND_TRANSITION)
    .map((payload) => fromBinary(CommandTransitionSchema, payload.payload))
    .filter((transition) => transition.commandId?.value === commandId)
    .map((transition) => transition.toState);
}

function observationsFor(
  payloads: readonly StoredEventPayload[],
  commandId: string,
) {
  return payloads
    .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
    .map((payload) => fromBinary(ObservationSchema, payload.payload))
    .filter((observation) =>
      observation.correlations.some(
        (correlation) =>
          correlation.ref.case === "commandId" && correlation.ref.value.value === commandId,
      ),
    );
}

function quarantinedObservationsFromDatabase(databasePath: string) {
  const database = new DatabaseSync(databasePath);
  try {
    const rows = database
      .prepare("SELECT payload FROM events WHERE authority_domain_id = ? AND kind = ?")
      .all(domainId, StoredEventKind.QUARANTINED_RUNTIME_EVIDENCE) as { payload: Uint8Array }[];
    return rows
      .map((row) => fromBinary(StoredEventPayloadSchema, row.payload))
      .map((payload) => fromBinary(QuarantinedRuntimeEvidenceSchema, payload.payload))
      .flatMap((quarantined) => {
        // Runtime-targeted Event/Status/Delta observations quarantine inside the
        // typed RuntimeTranscriptStatusEvidence wrapper (Unit 5); Results and
        // SessionReports stay directly nested. Flatten every admitted shape so
        // this reader keeps observing ALL quarantined observations.
        switch (quarantined.candidate.case) {
          case "observation":
            return [quarantined.candidate.value];
          case "transcriptStatus":
            return quarantined.candidate.value.observation
              ? [quarantined.candidate.value.observation]
              : [];
          default:
            return [];
        }
      });
  } finally {
    database.close();
  }
}

function appendAcceptedOperation(
  databasePath: string,
  acceptedOperation: ReturnType<typeof operation>,
  authorizingGrantId: GrantId,
): void {
  const database = new DatabaseSync(databasePath);
  try {
    database.exec("PRAGMA busy_timeout = 5000");
    database
      .prepare("INSERT INTO events(authority_domain_id, kind, payload) VALUES (?, ?, ?)")
      .run(
        domainId,
        StoredEventKind.OPERATION,
        toBinary(
          StoredEventPayloadSchema,
          create(StoredEventPayloadSchema, {
            kind: StoredEventKind.OPERATION,
            payload: toBinary(
              AcceptedOperationSchema,
              create(AcceptedOperationSchema, {
                operation: acceptedOperation,
                authorizingGrantId,
              }),
            ),
          }),
        ),
      );
  } finally {
    database.close();
  }
}

function countAdapterRegistrations(databasePath: string): number {
  const database = new DatabaseSync(databasePath);
  try {
    const rows = database
      .prepare("SELECT payload FROM events WHERE authority_domain_id = ? AND kind = ?")
      .all(domainId, StoredEventKind.OBSERVATION) as { payload: Uint8Array }[];
    return rows
      .map((row) => fromBinary(StoredEventPayloadSchema, row.payload))
      .filter(isAdapterRegistration).length;
  } finally {
    database.close();
  }
}

function isAdapterRegistration(payload: StoredEventPayload): boolean {
  return (
    payload.kind === StoredEventKind.OBSERVATION &&
    fromBinary(ObservationSchema, payload.payload).payload?.schemaRef ===
      "patchbay.AdapterRegistration"
  );
}

function isGenerationTwo(payload: StoredEventPayload): boolean {
  if (payload.kind !== StoredEventKind.SESSION_STATE) return false;
  const event = fromBinary(SessionStateEventSchema, payload.payload);
  if (event.mutation.case !== "generationBumped") return false;
  const bump = event.mutation.value;
  return bump.fromGeneration?.value === 1n && bump.toGeneration?.value === 2n;
}

function isGenerationBumpBeyondOne(payload: StoredEventPayload): boolean {
  if (payload.kind !== StoredEventKind.SESSION_STATE) return false;
  const event = fromBinary(SessionStateEventSchema, payload.payload);
  return event.mutation.case === "generationBumped" &&
    (event.mutation.value.toGeneration?.value ?? 0n) > 1n;
}

async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
  const address = server.address();
  assert.ok(address && typeof address === "object");
  const port = address.port;
  await new Promise<void>((resolveClose) => server.close(() => resolveClose()));
  return port;
}

async function waitForExit(child: ChildProcess): Promise<void> {
  if (child.exitCode !== null) return;
  await new Promise<void>((resolveExit) => child.once("exit", () => resolveExit()));
}

async function waitForCoreListener(port: number, child: ChildProcess): Promise<void> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`core exited before restart: ${child.exitCode}`);
    try {
      await new Promise<void>((resolveConnect, rejectConnect) => {
        const socket = new Socket();
        socket.once("error", rejectConnect);
        socket.connect(port, "127.0.0.1", () => {
          socket.destroy();
          resolveConnect();
        });
      });
      return;
    } catch {
      await new Promise((resolveDelay) => setTimeout(resolveDelay, 50));
    }
  }
  throw new Error("core did not restart before timeout");
}

async function waitForCore(port: number, child: ChildProcess): Promise<string> {
  let stdout = "";
  child.stdout?.setEncoding("utf8");
  child.stdout?.on("data", (chunk: string) => {
    stdout += chunk;
  });
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`core exited before startup: ${child.exitCode}`);
    const setupSecret = stdout.match(
      /one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/,
    )?.[1];
    if (setupSecret) {
      try {
        await new Promise<void>((resolveConnect, rejectConnect) => {
          const socket = new Socket();
          socket.once("error", rejectConnect);
          socket.connect(port, "127.0.0.1", () => {
            socket.destroy();
            resolveConnect();
          });
        });
        return setupSecret;
      } catch {
        // The process has printed startup metadata but has not bound yet.
      }
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 50));
  }
  throw new Error(`core did not start before timeout: ${stdout}`);
}
