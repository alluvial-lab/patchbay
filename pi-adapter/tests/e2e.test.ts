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
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ControlService,
  FailureCode,
  GenerationSchema,
  GrantProvenanceSchema,
  GrantRevocationPolicy,
  GrantSchema,
  GrantIdSchema,
  LsnSchema,
  ObservationSchema,
  OperationKind,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  RuntimeSessionIdSchema,
  SessionStateEventSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubmitRequestSchema,
  SubscribeRequestSchema,
  TargetScopeKind,
  TargetScopeSchema,
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
const adapterId = "pi";
const runtimeSessionId = "session-e2e";
const deploymentScope = "machine-e2e";

// The test is deliberately serial: it owns one core process and one SQLite fixture.
test("core → adapter → real AgentSession → observation loop, generation bump, and reconnect", { timeout: 60_000 }, async () => {
  const port = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "pi-adapter-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  const core = startCore(port, databasePath);
  let adapter: AdapterProcess | undefined;
  let reconnect: AdapterProcess | undefined;

  try {
    await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
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
      sessions: [configured],
      createSession: sessionFixture.create,
    });
    await adapter.start();

    seedOperatorGrant(databasePath, 1);
    const control = makeControlClient(baseUrl);
    const accepted = await control.submit(
      create(SubmitRequestSchema, {
        operation: operation("command-instruct", OperationKind.INSTRUCT, "hello from Patchbay"),
      }),
    );
    assert.ok(accepted.acceptedLsn);
    assert.equal(await adapter.pollOnce(), 1);

    const outputEvents = await readAfter(control, accepted.acceptedLsn?.value ?? 0n);
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
    seedOperatorGrant(databasePath, 2);

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

    adapter.dispose();
    adapter = undefined;
    const reconnectFixture = createSessionFixture(3);
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
  } finally {
    reconnect?.dispose();
    adapter?.dispose();
    core.kill("SIGTERM");
    rmSync(directory, { recursive: true, force: true });
  }
});

function createSessionFixture(generation: number) {
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
      session = await PiSession.create({
        ...configured,
        model: `${provider}/${model.id}`,
        sessionOptions: {
          modelRegistry: registry,
          sessionManager: SessionManager.inMemory(repoRoot),
          settingsManager: SettingsManager.inMemory(),
          noTools: "all",
        },
      });
      return session;
    },
  };
}

function startCore(port: number, databasePath: string): ChildProcess {
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
      PATCHBAY_DB_PATH: databasePath,
      PATCHBAY_AUTHORITY_DOMAIN_ID: domainId,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function seedOperatorGrant(databasePath: string, generation: number): void {
  const database = new DatabaseSync(databasePath);
  try {
    const grant = create(GrantSchema, {
      grantId: create(GrantIdSchema, { value: `e2e-grant-${generation}` }),
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      subjectActorId: create(ActorIdSchema, { value: operatorId }),
      targetScope: targetScope(generation),
      allowedOperationKinds: [
        OperationKind.INSTRUCT,
        OperationKind.CANCEL,
        OperationKind.SESSION_MANAGEMENT,
        OperationKind.SPAWN,
      ],
      provenance: create(GrantProvenanceSchema, { reason: "Pi adapter e2e fixture" }),
      revocationPolicy: GrantRevocationPolicy.CONTINUE,
    });
    database
      .prepare("INSERT INTO events(authority_domain_id, kind, payload) VALUES (?, ?, ?)")
      .run(
        domainId,
        StoredEventKind.GRANT,
        toBinary(
          StoredEventPayloadSchema,
          create(StoredEventPayloadSchema, {
            kind: StoredEventKind.GRANT,
            payload: toBinary(GrantSchema, grant),
          }),
        ),
      );
  } finally {
    database.close();
  }
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

function makeControlClient(baseUrl: string) {
  const authenticate: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    request.header.set("x-patchbay-operator-session-id", "e2e-session");
    request.header.set("x-patchbay-operator-id", operatorId);
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

function isGenerationTwo(payload: StoredEventPayload): boolean {
  if (payload.kind !== StoredEventKind.SESSION_STATE) return false;
  const event = fromBinary(SessionStateEventSchema, payload.payload);
  if (event.mutation.case !== "generationBumped") return false;
  const bump = event.mutation.value;
  return bump.fromGeneration?.value === 1n && bump.toGeneration?.value === 2n;
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

async function waitForCore(port: number, child: ChildProcess): Promise<void> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`core exited before startup: ${child.exitCode}`);
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
  throw new Error("core did not start before timeout");
}
