import assert from "node:assert/strict";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer, Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { DatabaseSync } from "node:sqlite";
import { createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  AcceptedOperationSchema, ActorEndpointRefSchema, ActorIdSchema, AdapterRegistrationSchema,
  AdapterSnapshotSupport, AdapterTargetCategory, AdminService, AuthorityDomainIdSchema,
  BootstrapRequestSchema, CommandIdSchema, CommandTransitionSchema, ControlService,
  DeviceIdSchema, EndpointIdSchema, FailureCode, GenerationSchema, LsnSchema,
  ObservationSchema, OperationKind, OperationSchema, OperationState,
  PrincipalEnrollmentSchema, StoredEventKind, StoredEventPayloadSchema,
  SubscribeRequestSchema, TargetScopeKind, TargetScopeSchema, TimeWindowSchema,
  VerifyOperatorPasswordRequestSchema, type GrantId, type PrincipalCredential,
  type StoredEventPayload,
} from "@patchbay/contracts";
import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { AdapterProcess } from "../src/main.js";
import type { TokenCommuneGatewayClient } from "../src/gateway_client.js";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const coreSecret = "token-commune-e2e-core-secret";
const adapterEvidence = "token-commune-e2e-attachment-secret";
const gatewayKey = "token-commune-e2e-member-key";
const domainId = "token-commune-e2e";
const operatorId = "operator-e2e";
const operatorPassword = "correct-password";
const operatorPasswordHash = "scrypt$BwcHBwcHBwcHBwcHBwcHBw$fsFQrJSo7EdHnhnfY0xMMJt9qNSBI2P-HkzGsCQBMakmW7BafHsr5ceNfZcDwG0PzpdzBilvkCaPNMMI6BEd3g";

// Serial real-process evidence for the generated registration and delivery seam.
test("real core records the PARTIAL manifest and fails an unexpected operation with unsupported_command", { timeout: 60_000 }, async () => {
  const port = await freePort();
  let adminPort = await freePort();
  while (adminPort === port) adminPort = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "token-commune-adapter-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  const diagnosticPath = join(directory, "adapter.log");
  const core = startCore(port, adminPort, databasePath);
  let host: AdapterProcess | undefined;
  let run: Promise<void> | undefined;
  const controller = new AbortController();
  try {
    const setupSecret = await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
    const auth = await bootstrapAndLogin(baseUrl, `http://127.0.0.1:${adminPort}`, setupSecret);
    const control = makeControlClient(baseUrl, auth);
    const diagnostics = await openAdapterDiagnostics({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 1,
      secrets: [adapterEvidence, gatewayKey, `Bearer ${gatewayKey}`],
    });
    host = new AdapterProcess({
      coreAddress: baseUrl, adapterId: "token-commune", adapterGeneration: 1,
      authorityDomainId: domainId, attachmentEvidence: adapterEvidence,
      gatewayBaseUrl: new URL("https://gateway.invalid/"), gatewayCredentialFile: "/not-read",
      pollIntervalMs: 30_000, diagnosticPath, gateway: {} as TokenCommuneGatewayClient,
      diagnostics, forwardDiagnostics: true,
    });
    await host.start();
    run = host.run(controller.signal);

    const registration = await waitForRegistration(control);
    assert.deepEqual(registration.capability?.targetCategories, [AdapterTargetCategory.OPERATIONAL_RESOURCE]);
    assert.equal(registration.capability?.sessionSnapshotSupport, AdapterSnapshotSupport.UNSPECIFIED);
    assert.deepEqual(registration.capability?.supportedOperationKinds, []);
    assert.equal(registration.capability?.resourceCapabilities.length, 2);
    assert.ok(registration.capability?.resourceCapabilities.every((item) => item.snapshotSupport === AdapterSnapshotSupport.PARTIAL));
    assert.equal(registration.capability?.attachmentMethod?.descriptor.byteLength, 0);

    // The current core does not admit ordinary adapter-scope Submit targets;
    // seed one already-accepted adapter delivery, matching the core's own durable
    // inbox shape, so this adapter-only feature can exercise its delivery seam
    // without fabricating a resource report before snapshot mapping exists.
    appendAcceptedOperation(databasePath, operation("unsupported-query"), auth.grantId);
    let terminal = commandTransitions(await readAfter(control, 0n), "unsupported-query").at(-1);
    await waitFor(async () => {
      terminal = commandTransitions(await readAfter(control, 0n), "unsupported-query").at(-1);
      return terminal !== undefined && terminal.failureCode === FailureCode.UNSUPPORTED_COMMAND;
    }, "unsupported operation terminalization");
    assert.equal(
      terminal?.toState,
      OperationState.FAILED,
      `unexpected terminal transition: ${JSON.stringify(terminal, (_key, value) => typeof value === "bigint" ? value.toString() : value)}; diagnostics=${readFileSync(diagnosticPath, "utf8")}`,
    );

    const visible = JSON.stringify(await readAfter(control, 0n), (_key, value) => typeof value === "bigint" ? value.toString() : value);
    assert.equal(visible.includes(adapterEvidence), false);
    assert.equal(visible.includes(gatewayKey), false);
    controller.abort();
    await run;
    await host.dispose();
    host = undefined;
    run = undefined;
    const local = readFileSync(diagnosticPath, "utf8");
    assert.equal(local.includes(adapterEvidence), false);
    assert.equal(local.includes(gatewayKey), false);
  } finally {
    controller.abort();
    if (host) await host.dispose();
    if (run) await Promise.allSettled([run]);
    core.kill("SIGTERM");
    rmSync(directory, { recursive: true, force: true });
  }
});

function operation(commandId: string) {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: operatorId }) }),
    kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.ADAPTER, adapterId: { value: "token-commune" } }),
    validityWindow: create(TimeWindowSchema, { startsAt: { seconds: 1n }, expiresAt: { seconds: 2_534_023_007_99n } }),
    submittedAt: { seconds: 1n }, idempotencyKey: `${commandId}-key`,
  });
}

function appendAcceptedOperation(
  databasePath: string,
  acceptedOperation: ReturnType<typeof operation>,
  authorizingGrantId: GrantId,
): void {
  const database = new DatabaseSync(databasePath);
  try {
    database.prepare("INSERT INTO events(authority_domain_id, kind, payload) VALUES (?, ?, ?)").run(
      domainId,
      StoredEventKind.OPERATION,
      toBinary(StoredEventPayloadSchema, create(StoredEventPayloadSchema, {
        kind: StoredEventKind.OPERATION,
        payload: toBinary(AcceptedOperationSchema, create(AcceptedOperationSchema, {
          operation: acceptedOperation,
          authorizingGrantId,
        })),
      })),
    );
  } finally {
    database.close();
  }
}

function startCore(port: number, adminPort: number, databasePath: string): ChildProcess {
  execFileSync("cargo", ["build", "-p", "patchbay-core-server"], {
    cwd: repoRoot,
    env: { ...process.env, CARGO_HOME: join(repoRoot, ".cargo-home"), PATH: `/home/agent/.cargo/bin:${process.env["PATH"] ?? ""}` },
    stdio: "ignore",
  });
  return spawn(join(repoRoot, "target/debug/patchbay-core-server"), [], {
    cwd: repoRoot,
    env: {
      ...process.env, PATCHBAY_CORE_SECRET: coreSecret, PATCHBAY_ADAPTER_ATTACHMENT_SECRET: adapterEvidence,
      PATCHBAY_BIND_ADDR: `127.0.0.1:${port}`, PATCHBAY_ADMIN_BIND_ADDR: `127.0.0.1:${adminPort}`,
      PATCHBAY_DB_PATH: databasePath, PATCHBAY_AUTHORITY_DOMAIN_ID: domainId,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
}

interface Auth { principal: PrincipalCredential; operatorSessionId: string; grantId: GrantId }
async function bootstrapAndLogin(baseUrl: string, adminUrl: string, setupSecret: string): Promise<Auth> {
  const enrollment = (endpoint: string) => create(PrincipalEnrollmentSchema, {
    endpointId: create(EndpointIdSchema, { value: endpoint }), deviceId: create(DeviceIdSchema, { value: "token-commune-e2e-device" }),
    endpointGeneration: create(GenerationSchema, { value: 1n }),
  });
  const admin = createClient(AdminService, createGrpcTransport({ baseUrl: adminUrl }));
  const bootstrap = await admin.bootstrapOperator(create(BootstrapRequestSchema, {
    setupSecret, operatorActorId: create(ActorIdSchema, { value: operatorId }), passwordHash: operatorPasswordHash,
    principal: enrollment("token-commune-e2e-bootstrap"),
  }));
  assert.ok(bootstrap.grantId);
  const authenticate: Interceptor = (next) => async (request) => { request.header.set("x-patchbay-core-secret", coreSecret); return next(request); };
  const control = createClient(ControlService, createGrpcTransport({ baseUrl, interceptors: [authenticate] }));
  const login = await control.verifyOperatorPassword(create(VerifyOperatorPasswordRequestSchema, {
    operatorActorId: create(ActorIdSchema, { value: operatorId }), password: operatorPassword,
    principal: enrollment("token-commune-e2e-control"),
  }));
  assert.ok(login.principal && login.operatorSessionId?.value);
  return { principal: login.principal, operatorSessionId: login.operatorSessionId.value, grantId: bootstrap.grantId };
}
function makeControlClient(baseUrl: string, auth: Auth) {
  const interceptor: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    request.header.set("x-patchbay-principal-id", auth.principal.principalId);
    request.header.set("x-patchbay-principal-secret", auth.principal.secret);
    request.header.set("x-patchbay-operator-id", operatorId);
    request.header.set("x-patchbay-operator-session-id", auth.operatorSessionId);
    return next(request);
  };
  return createClient(ControlService, createGrpcTransport({ baseUrl, interceptors: [interceptor] }));
}
async function readAfter(control: ReturnType<typeof makeControlClient>, cursor: bigint): Promise<StoredEventPayload[]> {
  const values: StoredEventPayload[] = [];
  for await (const item of control.subscribe(create(SubscribeRequestSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }), cursor: create(LsnSchema, { value: cursor }),
  }))) if (item.payload) values.push(item.payload);
  return values;
}
async function waitForRegistration(control: ReturnType<typeof makeControlClient>) {
  let found: ReturnType<typeof fromBinary<typeof AdapterRegistrationSchema>> | undefined;
  await waitFor(async () => {
    for (const payload of await readAfter(control, 0n)) {
      if (payload.kind !== StoredEventKind.OBSERVATION) continue;
      const observation = fromBinary(ObservationSchema, payload.payload);
      if (observation.payload?.schemaRef === "patchbay.AdapterRegistration") {
        found = fromBinary(AdapterRegistrationSchema, observation.payload.payload); return true;
      }
    }
    return false;
  }, "adapter registration");
  return found!;
}
function commandTransitions(payloads: readonly StoredEventPayload[], commandId: string) {
  return payloads.filter((item) => item.kind === StoredEventKind.COMMAND_TRANSITION)
    .map((item) => fromBinary(CommandTransitionSchema, item.payload))
    .filter((item) => item.commandId?.value === commandId);
}
async function waitFor(predicate: () => boolean | Promise<boolean>, message: string, timeoutMs = 10_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) { if (await predicate()) return; await new Promise((resolve) => setTimeout(resolve, 25)); }
  throw new Error(`timed out waiting for ${message}`);
}
async function freePort(): Promise<number> {
  const server = createServer();
  await new Promise<void>((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
  const address = server.address(); assert.ok(address && typeof address === "object");
  await new Promise<void>((resolveClose) => server.close(() => resolveClose()));
  return address.port;
}
async function waitForCore(port: number, child: ChildProcess): Promise<string> {
  let stdout = ""; child.stdout?.setEncoding("utf8"); child.stdout?.on("data", (chunk: string) => { stdout += chunk; });
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`core exited: ${child.exitCode}`);
    const secret = stdout.match(/one-time setup secret \(expires in \d+s\): ([A-Za-z0-9_-]+)/)?.[1];
    if (secret) try {
      await new Promise<void>((resolveConnect, rejectConnect) => {
        const socket = new Socket(); socket.once("error", rejectConnect); socket.connect(port, "127.0.0.1", () => { socket.destroy(); resolveConnect(); });
      });
      return secret;
    } catch { /* listener not ready */ }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 50));
  }
  throw new Error("core did not start");
}
