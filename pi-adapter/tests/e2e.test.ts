import assert from "node:assert/strict";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { createServer, Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  AdapterRegistrationSchema,
  AdminService,
  AuthorityDomainIdSchema,
  BootstrapRequestSchema,
  CommandIdSchema,
  CommandTransitionSchema,
  ControlService,
  DeviceIdSchema,
  EndpointIdSchema,
  FailureCode,
  GenerationSchema,
  LoadSnapshotRequestSchema,
  LsnSchema,
  ObservationKind,
  ObservationSchema,
  OperationKind,
  OperationState,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PrincipalEnrollmentSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionSnapshotSchema,
  SessionStateEventSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubmitRequestSchema,
  SubscribeRequestSchema,
  TargetScopeKind,
  TargetScopeSchema,
  VerifyOperatorPasswordRequestSchema,
  type PrincipalCredential,
  type StoredEventPayload,
} from "@patchbay/contracts";
import {
  AuthStorage,
  ModelRegistry,
  SessionManager,
  SettingsManager,
} from "@earendil-works/pi-coding-agent";
import { createFauxCore, fauxAssistantMessage } from "@earendil-works/pi-ai/providers/faux";
import { AdapterProcess, type PreprovisionedSession } from "../src/main.js";
import { PiSession } from "../src/pi_session.js";

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
test("core → adapter → real AgentSession → observation loop, generation bump, reconnect, and core restart", { timeout: 60_000 }, async () => {
  const port = await freePort();
  let adminPort = await freePort();
  while (adminPort === port) adminPort = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "pi-adapter-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  let core = startCore(port, adminPort, databasePath);
  let adapter: AdapterProcess | undefined;
  let reconnect: AdapterProcess | undefined;

  try {
    const setupSecret = await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
    const auth = await bootstrapAndLogin(
      baseUrl,
      `http://127.0.0.1:${adminPort}`,
      setupSecret,
    );
    const control = makeControlClient(baseUrl, auth);
    const sessionFixture = createSessionFixture(1);
    const configured: PreprovisionedSession = {
      cwd: repoRoot,
      runtimeSessionId,
      deploymentScope,
      project: "patchbay",
      generation: 1,
    };
    adapter = new AdapterProcess({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
      adapterGeneration: 1,
      sessions: [],
      createSession: sessionFixture.create,
    });
    await adapter.start();
    // Future spawn uses this same complete runtime-entry path; delivery routing
    // has no separate immutable pre-provisioned configuration dependency.
    await adapter.registerSession(configured);

    const loadedSnapshot = await control.loadSnapshot(
      create(LoadSnapshotRequestSchema, {
        authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      }),
    );
    assert.equal(loadedSnapshot.present, true);
    const snapshot = fromBinary(SessionSnapshotSchema, loadedSnapshot.snapshotPayload);
    assert.equal(snapshot.authorityDomainId?.value, domainId);
    assert.equal(snapshot.snapshotLsn?.value, loadedSnapshot.eventId?.lsn?.value);
    assert.equal(snapshot.sessions.length, 1);
    assert.equal(snapshot.sessions[0]?.runtimeSessionId?.value, runtimeSessionId);

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

    const accepted = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-instruct", OperationKind.INSTRUCT, "hello from Patchbay"),
      }),
    );
    assert.ok(accepted.acceptedLsn);
    assert.equal(await adapter.pollOnce(), 1);

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

    const sessionNew = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-session-new",
          OperationKind.SESSION_MANAGEMENT,
          JSON.stringify({ action: "new" }),
        ),
      }),
    );
    assert.equal(await adapter.pollOnce(), 1);
    const generationEvents = await readAfter(control, sessionNew.acceptedLsn?.value ?? 0n);
    assert.ok(generationEvents.some(isGenerationTwo));

    const query = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation(
          "command-query",
          OperationKind.QUERY,
          JSON.stringify({ action: "state" }),
          2,
        ),
      }),
    );
    assert.equal(await adapter.pollOnce(), 1);
    const queryEvents = await readAfter(control, query.acceptedLsn?.value ?? 0n);
    const queryResult = observationsFor(queryEvents, "command-query").find(
      (observation) => observation.kind === ObservationKind.RESULT,
    );
    assert.equal(queryResult?.payload?.schemaRef, "patchbay.pi.DeliveryResult.v1");
    const queryValue = JSON.parse(
      new TextDecoder().decode(queryResult?.payload?.payload),
    ) as { value?: { generation?: number } };
    assert.equal(queryValue.value?.generation, 2);

    const cancelFixture = sessionFixture.faux;
    cancelFixture.appendResponses([
      async () => {
        await new Promise((resolveDelay) => setTimeout(resolveDelay, 250));
        return fauxAssistantMessage("late response that cancellation must settle");
      },
    ]);
    await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-long", OperationKind.INSTRUCT, "start a cancellable turn", 2),
      }),
    );
    await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-cancel", OperationKind.CANCEL, "", 2),
      }),
    );
    assert.equal(await adapter.pollOnce(), 2);
    assert.equal(sessionFixture.session?.getState().idle, true);

    // An accepted old-generation delivery may remain after replacement, but it
    // must be acknowledged-and-rejected without entering the new Pi context.
    appendAcceptedOperation(
      databasePath,
      operation("command-old-generation", OperationKind.INSTRUCT, "must not execute", 1),
    );
    assert.equal(await adapter.pollOnce(), 1);
    const staleDeliveryEvents = await readAfter(control, 0n);
    const staleFailure = observationsFor(
      staleDeliveryEvents,
      "command-old-generation",
    ).find((observation) => observation.kind === ObservationKind.RESULT);
    assert.equal(staleFailure?.failureCode, FailureCode.DELIVERY_REJECTED);
    assert.equal(
      observationsFor(staleDeliveryEvents, "command-old-generation").some(
        (observation) => observation.payload?.schemaRef === "patchbay.pi.TranscriptEvent.v1",
      ),
      false,
    );

    const spawn = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-spawn", OperationKind.SPAWN, "", 2),
      }),
    );
    assert.equal(await adapter.pollOnce(), 1);
    const spawnEvents = await readAfter(control, spawn.acceptedLsn?.value ?? 0n);
    assert.ok(
      spawnEvents
        .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
        .map((payload) => fromBinary(ObservationSchema, payload.payload))
        .some((observation) => observation.failureCode === FailureCode.UNSUPPORTED_COMMAND),
    );

    await adapter.dispose();
    adapter = undefined;
    const reconnectFixture = createSessionFixture(3, true);
    reconnect = new AdapterProcess({
      coreAddress: baseUrl,
      adapterId,
      authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
      adapterGeneration: 2,
      sessions: [{ ...configured, generation: 3 }],
      createSession: reconnectFixture.create,
    });
    await reconnect.start();
    assert.equal(reconnectFixture.session?.getState().generation, 3);
    assert.equal(await reconnect.pollOnce(), 0, "durably acknowledged history is not re-offered");

    const reconnectEvents = await readAfter(control, 0n);
    assert.ok(
      reconnectEvents
        .filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
        .map((payload) => fromBinary(ObservationSchema, payload.payload))
        .some(
          (observation) =>
            observation.payload?.schemaRef === "patchbay.pi.TranscriptEvent.v1" &&
            new TextDecoder().decode(observation.payload.payload).includes("replayed snapshot entry"),
        ),
      "reconnect explicitly replays Pi getEntries()/TranscriptEventLog snapshot",
    );
    assert.ok(reconnectEvents.some(isGenerationThreeUnknown));
    const attachCountBeforeRestart = reconnectEvents.filter(isAdapterRegistration).length;

    core.kill("SIGTERM");
    await waitForExit(core);
    core = startCore(port, adminPort, databasePath);
    await waitForCoreListener(port, core);
    assert.equal(
      await reconnect.pollOnce(),
      0,
      "an unauthenticated post-restart poll reattaches once and retries",
    );
    assert.equal(
      countAdapterRegistrations(databasePath),
      attachCountBeforeRestart + 1,
      "the retry path durably records exactly one fresh attachment",
    );
  } finally {
    if (reconnect) await reconnect.dispose();
    if (adapter) await adapter.dispose();
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
      const auth = AuthStorage.inMemory({ [provider]: { type: "api_key", key: "test" } });
      const registry = ModelRegistry.inMemory(auth);
      const model = faux.getModel();
      registry.registerProvider(provider, {
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
      const sessionManager = SessionManager.inMemory(repoRoot);
      if (seedSnapshot) {
        sessionManager.appendMessage({
          role: "user",
          content: "replayed snapshot entry",
          timestamp: Date.now(),
        });
      }
      session = await PiSession.create({
        ...configured,
        model: `${provider}/${model.id}`,
        sessionOptions: {
          modelRegistry: registry,
          sessionManager,
          settingsManager: SettingsManager.inMemory(),
          noTools: "all",
        },
      });
      return session;
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
      PATCHBAY_ADAPTER_ATTACHMENT_SECRET: adapterEvidence,
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
    idempotencyKey: `${commandId}-key`,
    payload: create(PayloadEnvelopeSchema, {
      payload: new TextEncoder().encode(payload),
      contentType: PayloadContentType.TEXT_UTF8,
    }),
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

function appendAcceptedOperation(
  databasePath: string,
  acceptedOperation: ReturnType<typeof operation>,
): void {
  const database = new DatabaseSync(databasePath);
  try {
    database
      .prepare("INSERT INTO events(authority_domain_id, kind, payload) VALUES (?, ?, ?)")
      .run(
        domainId,
        StoredEventKind.OPERATION,
        toBinary(
          StoredEventPayloadSchema,
          create(StoredEventPayloadSchema, {
            kind: StoredEventKind.OPERATION,
            payload: toBinary(OperationSchema, acceptedOperation),
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

function isGenerationThreeUnknown(payload: StoredEventPayload): boolean {
  if (payload.kind !== StoredEventKind.SESSION_STATE) return false;
  const event = fromBinary(SessionStateEventSchema, payload.payload);
  if (event.mutation.case !== "generationBumped") return false;
  const bump = event.mutation.value;
  return (
    bump.toGeneration?.value === 3n &&
    bump.initialState?.activity === SessionActivityState.UNKNOWN
  );
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
