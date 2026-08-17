import assert from "node:assert/strict";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer, Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  continuationSpawnPayload,
  declaredManagedSpawnTarget,
  freshSpawnPayload,
} from "@patchbay/operator-domain";
import { Code, ConnectError, createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AcceptedOperationSchema,
  AdapterDiagnosticState,
  AdapterIdSchema,
  AdapterRegistrationSchema,
  AdapterReconciliationStrength,
  AdapterSnapshotSupport,
  AdapterStatusQuerySchema,
  AdapterTargetCategory,
  AdminService,
  AuthorityDomainIdSchema,
  BootstrapRequestSchema,
  CommandIdSchema,
  CommandTransitionSchema,
  ContinuationContextStatus,
  ControlService,
  DeviceIdSchema,
  DiagnosticsQuerySchema,
  EndpointIdSchema,
  ExternalRuntimeRefSchema,
  FailureCode,
  GenerationSchema,
  LoadSnapshotRequestSchema,
  LogicalTargetIdSchema,
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
  PiReconfigureRequestSchema,
  PiReloadableResourceKind,
  PiSpawnTargetSpecSchema,
  PrincipalEnrollmentSchema,
  QueryDiagnosticsRequestSchema,
  QuarantinedRuntimeEvidenceSchema,
  SpawnExecutionPhase,
  SpawnPromotionCommittedSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  RuntimeGenerationRefSchema,
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
  type PayloadEnvelope,
  type PrincipalCredential,
  type RuntimeGenerationRef,
  type Session,
  type StoredEventPayload,
} from "@patchbay/contracts";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import {
  PatchbayCoreClient,
  piCapabilityManifest,
  PI_CAPABILITY_EVIDENCE,
  PI_RPC_TARGET_SHAPE,
  PI_SPAWN_TARGET_SCHEMA_REF,
} from "../src/core_client.js";
import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { AdapterProcess, type PreprovisionedSession } from "../src/main.js";
import {
  RpcManagedPiRuntimePort,
  type ManagedPiRuntimePort,
  type PiLaunchSpec,
  type PiRpcRuntime,
} from "../src/pi_process.js";
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

// This activation-gate test owns real core, Pi process, session-file, extension,
// journal, cursor-store, reconnect, and process-group boundaries. The only
// injected materialization input is an offline extension command that appends
// the prebuilt current-v3 assistant shape without a model or credential lookup.
test("real core + real Pi managed fresh/continuation/reload/reconnect lifecycle", { timeout: 180_000 }, async () => {
  const freshCommandId = "real-pi-managed-fresh";
  const continuationCommandId = "real-pi-managed-continuation";
  const projectContextRef = "real-pi-project-context";
  const managedDeploymentScope = "machine-real-pi";
  const port = await freePort();
  let adminPort = await freePort();
  while (adminPort === port) adminPort = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "pi-managed-lifecycle-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  const sessionDirectory = join(directory, "sessions");
  const journalDirectory = join(directory, "journal");
  const cursorDirectory = join(directory, "cursor");
  mkdirSync(sessionDirectory, { recursive: true });
  const materializationExtension = join(directory, "offline-materialization.mjs");
  writeFileSync(materializationExtension, `
export default function offlineMaterialization(pi) {
  pi.registerCommand("patchbay-test-materialize", {
    description: "Append a deterministic offline current-v3 assistant fixture",
    handler: async (_args, ctx) => {
      const timestamp = 1776124801000;
      ctx.sessionManager.appendMessage({ role: "user", content: "offline fixture prompt", timestamp });
      ctx.sessionManager.appendMessage({
        role: "assistant",
        content: [{ type: "text", text: "offline fixture response" }],
        api: "offline-fixture",
        provider: "offline-fixture",
        model: "offline",
        usage: {
          input: 1,
          output: 1,
          cacheRead: 0,
          cacheWrite: 0,
          totalTokens: 2,
          cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
        },
        stopReason: "stop",
        timestamp: timestamp + 1,
      });
    },
  });
}
`, { mode: 0o600 });

  const piIndexPath = fileURLToPath(import.meta.resolve("@earendil-works/pi-coding-agent"));
  const cliPath = realpathSync(join(dirname(piIndexPath), "cli.js"));
  const controlExtensionPath = realpathSync(fileURLToPath(
    new URL("../extensions/patchbay-control.js", import.meta.url),
  ));
  const managedCwd = realpathSync(directory);
  let core = startCore(port, adminPort, databasePath);
  let adapter: AdapterProcess | undefined;
  let adapterController: AbortController | undefined;
  let adapterRun: Promise<void> | undefined;
  let interrupterController: AbortController | undefined;
  let interruptedStream: Promise<void> | undefined;

  try {
    const setupSecret = await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
    const auth = await bootstrapAndLogin(
      baseUrl,
      `http://127.0.0.1:${adminPort}`,
      setupSecret,
    );
    const control = makeControlClient(baseUrl, auth);
    const productionRuntimePort = new RpcManagedPiRuntimePort();
    const realLaunches: Array<{ readonly spec: PiLaunchSpec; readonly runtime: PiRpcRuntime }> = [];
    const managedRuntimePort: ManagedPiRuntimePort = {
      async launch(spec) {
        const runtime = await productionRuntimePort.launch(spec);
        realLaunches.push({ spec, runtime });
        return runtime;
      },
      handshake(runtime, challenge) {
        return productionRuntimePort.handshake(runtime, challenge);
      },
      terminate(runtime, policy) {
        return productionRuntimePort.terminate(runtime, policy);
      },
    };
    adapter = new AdapterProcess({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
      adapterGeneration: 1,
      sessions: [],
      managedTargets: [{
        projectContextRef,
        deploymentTarget: {
          credentialPolicy: "credential-free",
          adapterId,
          deploymentScope: managedDeploymentScope,
          logicalTargetId: freshCommandId,
        },
        cwd: managedCwd,
        sessionRoot: realpathSync(sessionDirectory),
        executable: realpathSync(process.execPath),
        cliPath,
        controlExtensionPath,
        environment: { PI_OFFLINE: "1" },
        additionalArguments: ["--extension", realpathSync(materializationExtension)],
      }],
      spawnJournalDirectory: journalDirectory,
      cursorStoreDirectory: cursorDirectory,
      managedRuntimePort,
    });
    await adapter.start();
    adapterController = new AbortController();
    adapterRun = adapter.run(adapterController.signal);

    const registrationPayload = (await readAfter(control, 0n))
      .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
      .map((payload) => fromBinary(ObservationSchema, payload.payload))
      .find((observation) => observation.payload?.schemaRef === "patchbay.AdapterRegistration")
      ?.payload;
    assert.ok(registrationPayload, "the activated Pi manifest is durably registered");
    const registration = fromBinary(AdapterRegistrationSchema, registrationPayload.payload);
    const assurance = registration.capability?.assurance?.contract;
    assert.equal(registration.capability?.supportedOperationKinds.includes(OperationKind.SPAWN), true);
    assert.deepEqual(registration.capability?.supportedTargetSpecShapes, [PI_RPC_TARGET_SHAPE]);
    assert.equal(registration.capability?.managedSpawnTargets.length, 1);
    const declaredTarget = registration.capability!.managedSpawnTargets[0]!;
    assert.equal(declaredTarget.logicalTargetId?.value, freshCommandId);
    assert.equal(declaredTarget.targetSpecShape, PI_RPC_TARGET_SHAPE);
    assert.equal(declaredTarget.freshAdapterPayload?.schemaRef, PI_SPAWN_TARGET_SCHEMA_REF);
    assert.equal(
      fromBinary(PiSpawnTargetSpecSchema, declaredTarget.freshAdapterPayload!.payload).projectContextRef,
      projectContextRef,
    );
    assert.equal(registration.capability?.sessionReplacementSupport, true);
    assert.equal(assurance?.case, "v1");
    if (assurance?.case !== "v1") assert.fail("activated Pi assurance V1 is absent");
    assert.equal(assurance.value.continuationProofSupport, true);
    assert.equal(assurance.value.cursorSupport, true);
    assert.equal(assurance.value.generationFenceSupport, true);
    assert.equal(
      assurance.value.reconciliationStrength,
      AdapterReconciliationStrength.BOUNDED,
    );

    const adapterStatus = await control.queryDiagnostics(
      create(QueryDiagnosticsRequestSchema, {
        operation: adapterStatusOperation("managed-target-capability-status"),
      }),
    );
    assert.equal(adapterStatus.submission?.outcome, SubmissionOutcome.ACCEPTED);
    assert.equal(adapterStatus.submission?.operationState, OperationState.COMPLETED);
    assert.equal(adapterStatus.result.case, "adapters");
    if (adapterStatus.result.case !== "adapters") assert.fail("adapter diagnostics expected");
    const projectedTarget = adapterStatus.result.value.adapters[0]?.capability?.managedSpawnTargets[0];
    assert.equal(projectedTarget?.logicalTargetId?.value, freshCommandId);
    assert.equal(projectedTarget?.targetSpecShape, PI_RPC_TARGET_SHAPE);
    assert.equal(
      fromBinary(PiSpawnTargetSpecSchema, projectedTarget!.freshAdapterPayload!.payload).projectContextRef,
      projectContextRef,
    );

    const fresh = await control.submit(create(SubmitRequestSchema, {
      operation: managedSpawnOperation({
        commandId: freshCommandId,
        projectContextRef,
      }),
    }));
    assert.equal(fresh.outcome, SubmissionOutcome.ACCEPTED);
    await waitForSpawnPromotion(control, freshCommandId, 45_000);
    const freshSession = await waitForManagedSession(
      control,
      managedDeploymentScope,
      1n,
    );
    assert.equal(freshSession.state?.connectivity, SessionConnectivityState.LIVE);
    assert.equal(freshSession.state?.activity, SessionActivityState.IDLE);
    assert.deepEqual(
      commandStates(await readAfter(control, 0n), freshCommandId),
      [OperationState.DELIVERED, OperationState.RUNNING],
      "the real delivery loop records delivery/running before atomic promotion owns completion",
    );
    assert.equal(realLaunches.length, 1, "the spawn delivery launches exactly one real child");
    const modeIndex = realLaunches[0]!.spec.argv.indexOf("--mode");
    assert.deepEqual(
      realLaunches[0]!.spec.argv.slice(modeIndex, modeIndex + 2),
      ["--mode", "rpc"],
      "the delivery-loop supervisor launches Pi in RPC mode",
    );
    const sessionDirectoryIndex = realLaunches[0]!.spec.argv.indexOf("--session-dir");
    assert.notEqual(sessionDirectoryIndex, -1, "sessionRoot supplies the default Pi session directory");
    assert.equal(
      realLaunches[0]!.spec.argv[sessionDirectoryIndex + 1],
      realpathSync(sessionDirectory),
    );
    assert.doesNotThrow(
      () => process.kill(realLaunches[0]!.runtime.pid, 0),
      "the supervised Pi child is live after promotion",
    );
    const freshJournalFiles = readdirSync(journalDirectory)
      .filter((name) => name.endsWith(".json"));
    assert.equal(freshJournalFiles.length, 1, "spawn writes one exact-claim journal receipt");
    const freshJournal = JSON.parse(
      readFileSync(join(journalDirectory, freshJournalFiles[0]!), "utf8"),
    ) as {
      claim?: { claimOperationId?: string };
      phases?: readonly { phase?: number }[];
      promotionObserved?: boolean;
      publicationCommitted?: boolean;
    };
    assert.equal(freshJournal.claim?.claimOperationId, freshCommandId);
    assert.equal(freshJournal.promotionObserved, true);
    assert.equal(freshJournal.publicationCommitted, true);
    assert.ok(
      freshJournal.phases?.some((phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED),
      "launch_attempted remains in the durable receipt after promotion",
    );

    const materializeCommandId = "real-pi-materialize";
    await control.submit(create(SubmitRequestSchema, {
      operation: managedRuntimeOperation({
        commandId: materializeCommandId,
        kind: OperationKind.INSTRUCT,
        deploymentScope: managedDeploymentScope,
        runtimeSessionId: freshSession.runtimeSessionId!.value,
        generation: 1n,
        payload: create(PayloadEnvelopeSchema, {
          contentType: PayloadContentType.TEXT_UTF8,
          payload: new TextEncoder().encode("/patchbay-test-materialize"),
        }),
      }),
    }));
    await waitForCommandState(control, materializeCommandId, OperationState.COMPLETED, 30_000);
    let materializedPath = "";
    await waitFor(() => {
      materializedPath = findSessionJsonl(sessionDirectory) ?? "";
      return materializedPath.length > 0
        && readFileSync(materializedPath, "utf8").includes("offline fixture response");
    }, "the real Pi child to materialize its offline assistant fixture", 15_000);

    const prior = create(RuntimeGenerationRefSchema, {
      logicalTargetId: create(LogicalTargetIdSchema, { value: freshCommandId }),
      externalRuntime: create(ExternalRuntimeRefSchema, {
        adapterId: create(AdapterIdSchema, { value: adapterId }),
        deploymentScope: managedDeploymentScope,
        runtimeSessionId: freshSession.runtimeSessionId,
        generation: create(GenerationSchema, { value: 1n }),
      }),
    });
    const continuation = await control.submit(create(SubmitRequestSchema, {
      operation: managedSpawnOperation({
        commandId: continuationCommandId,
        projectContextRef,
        prior,
      }),
    }));
    assert.equal(continuation.outcome, SubmissionOutcome.ACCEPTED);
    await waitForSpawnPromotion(control, continuationCommandId, 60_000);
    const resumed = await waitForManagedSession(
      control,
      managedDeploymentScope,
      2n,
    );
    assert.equal(resumed.runtimeSessionId?.value, freshSession.runtimeSessionId?.value);
    assert.equal(resumed.state?.connectivity, SessionConnectivityState.LIVE);
    assert.equal(resumed.state?.activity, SessionActivityState.IDLE);
    assert.equal(realLaunches.length, 2, "continuation launches one replacement child");

    const afterContinuation = await readAfter(control, 0n);
    const continuationPromotion = afterContinuation
      .filter((payload) => payload.kind === StoredEventKind.SPAWN_PROMOTION_COMMITTED)
      .map((payload) => fromBinary(SpawnPromotionCommittedSchema, payload.payload))
      .find((promotion) =>
        promotion.acceptedClaim?.claim?.claimOperationId?.value === continuationCommandId
      );
    assert.ok(continuationPromotion, "continuation has one authority-bearing promotion");
    assert.equal(
      continuationPromotion.stagedSuccessor?.staged?.continuationContextStatus,
      ContinuationContextStatus.RESUMED,
    );
    assert.equal(
      afterContinuation.filter((payload) => payload.kind === StoredEventKind.SPAWN_PROMOTION_COMMITTED)
        .filter((payload) =>
          fromBinary(SpawnPromotionCommittedSchema, payload.payload)
            .acceptedClaim?.claim?.claimOperationId?.value === continuationCommandId
        ).length,
      1,
    );
    const promotions = afterContinuation
      .filter((payload) => payload.kind === StoredEventKind.SPAWN_PROMOTION_COMMITTED)
      .map((payload) => fromBinary(SpawnPromotionCommittedSchema, payload.payload));
    for (const commandId of [freshCommandId, continuationCommandId]) {
      const promotion = promotions.find((candidate) =>
        candidate.acceptedClaim?.claim?.claimOperationId?.value === commandId
      );
      assert.equal(
        promotion?.stagedSuccessor?.staged?.exactClaim?.claimOperationId?.value,
        commandId,
      );
      assert.ok(
        (promotion?.stagedSuccessor?.eventId?.lsn?.value ?? 0n)
          < (promotion?.promotionEventId?.lsn?.value ?? 0n),
        `${commandId} remains staged and non-current before its atomic promotion`,
      );
    }

    const reloadCommandId = "real-pi-live-reload";
    await control.submit(create(SubmitRequestSchema, {
      operation: managedRuntimeOperation({
        commandId: reloadCommandId,
        kind: OperationKind.RECONFIGURE,
        deploymentScope: managedDeploymentScope,
        runtimeSessionId: resumed.runtimeSessionId!.value,
        generation: 2n,
        payload: create(PayloadEnvelopeSchema, {
          contentType: PayloadContentType.PROTOBUF,
          schemaRef: "patchbay.PiReconfigureRequest",
          payload: toBinary(PiReconfigureRequestSchema, create(PiReconfigureRequestSchema, {
            reloadResources: [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
          })),
        }),
      }),
    }));
    await waitForCommandState(control, reloadCommandId, OperationState.COMPLETED, 30_000);
    const persistedAfterReload = readFileSync(materializedPath, "utf8");
    assert.ok(persistedAfterReload.includes("patchbay.control.reload-request.v1"));
    assert.ok(persistedAfterReload.includes("patchbay.control.reload-completion.v1"));

    const streamInterrupter = new PatchbayCoreClient({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
    });
    await streamInterrupter.attach(1);
    interrupterController = new AbortController();
    interruptedStream = (async () => {
      for await (const _delivery of streamInterrupter.receiveDeliveries(
        0n,
        interrupterController!.signal,
      )) {
        // Holding the replacement stream forces the production adapter to
        // reattach and replay from its durable core cursor after this aborts.
      }
    })();
    const reconnectCommandId = "real-pi-reconnect-query";
    await control.submit(create(SubmitRequestSchema, {
      operation: managedRuntimeOperation({
        commandId: reconnectCommandId,
        kind: OperationKind.QUERY,
        deploymentScope: managedDeploymentScope,
        runtimeSessionId: resumed.runtimeSessionId!.value,
        generation: 2n,
        payload: create(PayloadEnvelopeSchema, {
          contentType: PayloadContentType.TEXT_UTF8,
          payload: new TextEncoder().encode(JSON.stringify({ action: "state" })),
        }),
      }),
    }));
    interrupterController.abort();
    await interruptedStream.catch((error: unknown) => {
      assert.ok(error instanceof ConnectError && error.code === Code.Canceled);
    });
    interruptedStream = undefined;
    interrupterController = undefined;
    await waitForCommandState(control, reconnectCommandId, OperationState.COMPLETED, 30_000);
    assert.deepEqual(
      commandStates(await readAfter(control, 0n), reconnectCommandId),
      [OperationState.DELIVERED, OperationState.RUNNING, OperationState.COMPLETED],
      "reconnect converges without duplicate execution or lifecycle transitions",
    );

    const journalFiles = readdirSync(journalDirectory).filter((name) => name.endsWith(".json"));
    assert.equal(journalFiles.length, 2, "the bounded duplicate-replay window retains two receipts");
    for (const journalFile of journalFiles) {
      const journal = JSON.parse(readFileSync(join(journalDirectory, journalFile), "utf8")) as {
        claim?: { claimOperationId?: string };
        phases?: readonly { phase?: number }[];
        promotionObserved?: boolean;
        publicationCommitted?: boolean;
        stagedPublication?: unknown;
        committedPublication?: {
          continuationContextStatus?: number;
          entryCount?: number;
          committedAt?: string;
        };
      };
      assert.equal(journal.promotionObserved, true, journalFile);
      assert.equal(journal.publicationCommitted, true, journalFile);
      assert.equal(journal.stagedPublication, undefined, journalFile);
      assert.equal(journal.committedPublication?.entryCount, 1, journalFile);
      assert.ok(journal.committedPublication?.committedAt, journalFile);
      assert.ok((journal.phases?.length ?? 0) >= 4, journalFile);
      if (journal.claim?.claimOperationId === freshCommandId) {
        assert.equal(
          journal.committedPublication?.continuationContextStatus,
          ContinuationContextStatus.UNSPECIFIED,
        );
      } else if (journal.claim?.claimOperationId === continuationCommandId) {
        assert.equal(
          journal.committedPublication?.continuationContextStatus,
          ContinuationContextStatus.RESUMED,
        );
      } else {
        assert.fail(`unexpected managed spawn journal ${journalFile}`);
      }
    }
    assert.ok(readdirSync(cursorDirectory).length > 0, "cursor state is durable after core acknowledgement");

    adapterController.abort();
    await adapterRun;
    adapterRun = undefined;
    adapterController = undefined;
    await adapter.dispose();
    adapter = undefined;
    const exits = await Promise.all(realLaunches.map(({ runtime }) => runtime.exit));
    assert.equal(exits.length, 2);
    assert.equal(
      exits.every((exit) => exit.expected && exit.terminatedBySupervisor),
      true,
      "both real Pi process groups are reaped by the supervisor",
    );
  } finally {
    interrupterController?.abort();
    await interruptedStream?.catch(() => undefined);
    adapterController?.abort();
    if (adapter) await adapter.dispose().catch(() => undefined);
    await adapterRun?.catch(() => undefined);
    core.kill("SIGTERM");
    await waitForExit(core);
    rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 10 });
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

function managedSpawnOperation(options: {
  readonly commandId: string;
  readonly projectContextRef: string;
  readonly prior?: RuntimeGenerationRef;
}) {
  const logicalTargetId = options.prior?.logicalTargetId?.value ?? options.commandId;
  const capability = piCapabilityManifest(PI_CAPABILITY_EVIDENCE, [{
    projectContextRef: options.projectContextRef,
    logicalTargetId,
  }]);
  const selected = declaredManagedSpawnTarget(
    capability,
    options.prior ? "continuation" : "fresh",
    logicalTargetId,
  );
  if (!selected.available) assert.fail(selected.reason);
  assert.equal(selected.available, true);
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: options.commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: operatorId }),
    }),
    kind: OperationKind.SPAWN,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.ADAPTER,
      adapterId: create(AdapterIdSchema, { value: adapterId }),
    }),
    validityWindow: create(TimeWindowSchema, {
      startsAt: { seconds: 1n },
      expiresAt: { seconds: 2_534_023_007_99n },
    }),
    submittedAt: { seconds: 1n },
    idempotencyKey: `${options.commandId}-key`,
    payload: options.prior
      ? continuationSpawnPayload(options.prior, selected.target)
      : freshSpawnPayload(selected.target),
  });
}

function managedRuntimeOperation(options: {
  readonly commandId: string;
  readonly kind: OperationKind;
  readonly deploymentScope: string;
  readonly runtimeSessionId: string;
  readonly generation: bigint;
  readonly payload: PayloadEnvelope;
}) {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: options.commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: operatorId }),
    }),
    kind: options.kind,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: create(AdapterIdSchema, { value: adapterId }),
      deploymentScope: options.deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: options.runtimeSessionId }),
      sessionGeneration: create(GenerationSchema, { value: options.generation }),
    }),
    validityWindow: create(TimeWindowSchema, {
      startsAt: { seconds: 1n },
      expiresAt: { seconds: 2_534_023_007_99n },
    }),
    submittedAt: { seconds: 1n },
    idempotencyKey: `${options.commandId}-key`,
    payload: options.payload,
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
  timeoutMs = 10_000,
): Promise<void> {
  await waitForStoredEvent(
    control,
    (payload) => {
      if (payload.kind !== StoredEventKind.COMMAND_TRANSITION) return false;
      const transition = fromBinary(CommandTransitionSchema, payload.payload);
      return transition.commandId?.value === commandId && transition.toState === expected;
    },
    `${commandId} to reach ${OperationState[expected] ?? expected}`,
    timeoutMs,
  );
}

async function waitForSpawnPromotion(
  control: ReturnType<typeof makeControlClient>,
  commandId: string,
  timeoutMs: number,
): Promise<void> {
  await waitForStoredEvent(
    control,
    (payload) => payload.kind === StoredEventKind.SPAWN_PROMOTION_COMMITTED
      && fromBinary(SpawnPromotionCommittedSchema, payload.payload)
        .acceptedClaim?.claim?.claimOperationId?.value === commandId,
    `${commandId} to reach atomic spawn promotion`,
    timeoutMs,
  );
}

async function waitForStoredEvent(
  control: ReturnType<typeof makeControlClient>,
  predicate: (payload: StoredEventPayload) => boolean,
  message: string,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), Math.max(1, deadline - Date.now()));
    try {
      for await (const event of control.subscribe(
        create(SubscribeRequestSchema, {
          authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
          cursor: create(LsnSchema, { value: 0n }),
        }),
        { signal: controller.signal },
      )) {
        if (event.payload && predicate(event.payload)) return;
      }
    } catch (error) {
      if (!(error instanceof ConnectError && error.code === Code.Canceled && controller.signal.aborted)) {
        throw error;
      }
    } finally {
      clearTimeout(timer);
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 25));
  }
  throw new Error(`timed out waiting for ${message}`);
}

async function waitForManagedSession(
  control: ReturnType<typeof makeControlClient>,
  expectedDeploymentScope: string,
  expectedGeneration: bigint,
): Promise<Session> {
  let matched: Session | undefined;
  await waitFor(async () => {
    const loaded = await control.loadSnapshot(create(LoadSnapshotRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      viewKind: SnapshotViewKind.SESSION,
    }));
    if (!loaded.present) return false;
    const snapshot = fromBinary(SessionSnapshotSchema, loaded.snapshotPayload);
    matched = snapshot.sessions.find((session) =>
      session.deploymentScope === expectedDeploymentScope
      && session.sessionGeneration?.value === expectedGeneration
      && !session.tombstoned
      && session.state?.connectivity === SessionConnectivityState.LIVE
    );
    return matched !== undefined;
  }, `managed generation ${expectedGeneration} to become current`, 30_000);
  return matched!;
}

function findSessionJsonl(root: string): string | undefined {
  for (const relative of readdirSync(root, { recursive: true, encoding: "utf8" })) {
    if (relative.endsWith(".jsonl")) return join(root, relative);
  }
  return undefined;
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
  if (child.exitCode !== null || child.signalCode !== null) return;
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
