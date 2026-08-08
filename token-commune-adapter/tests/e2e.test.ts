import assert from "node:assert/strict";
import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer, Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { DatabaseSync } from "node:sqlite";
import { Code, ConnectError, createClient, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  AcceptedOperationSchema, ActorEndpointRefSchema, ActorIdSchema, AdapterIdSchema, AdapterRegistrationSchema,
  AdapterSnapshotSupport, AdapterTargetCategory, AdminService, AuthorityDomainIdSchema,
  BootstrapRequestSchema, CommandIdSchema, CommandTransitionSchema, ControlService,
  DeviceIdSchema, DiagnosticsQuerySchema, EndpointIdSchema, FailureCode, GenerationSchema,
  LoadSnapshotRequestSchema, LsnSchema, ObservationKind, ObservationSchema, OperationKind,
  OperationSchema, OperationState, PayloadContentType, PayloadEnvelopeSchema,
  PrincipalEnrollmentSchema, QueryDiagnosticsRequestSchema, ResourceFreshnessState,
  ResourceSnapshotSchema, SnapshotViewKind, StoredEventKind, StoredEventPayloadSchema,
  SubscribeRequestSchema, TargetScopeKind, TargetScopeSchema, TimeWindowSchema,
  VerifyOperatorPasswordRequestSchema, AdapterStatusQuerySchema, type GrantId,
  type PrincipalCredential, type ResourceSnapshot, type StoredEventPayload,
} from "@patchbay/contracts";
import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { AdapterProcess } from "../src/main.js";
import { PatchbayCoreClient } from "../src/core_client.js";
import { loadGatewayCredential } from "../src/credential.js";
import { createHttpTokenCommuneGatewayClient, type TokenCommuneGatewayClient } from "../src/gateway_client.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { projectTokenCommuneSnapshot } from "../src/snapshot_projection.js";
import { assertSecretAbsent } from "./conformance-oracles.js";
import { ScriptedTokenCommuneGateway, type ScriptedGatewayStep } from "./fixtures/conformance-gateway.js";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const encoder = new TextEncoder();
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
    const coreClient = new PatchbayCoreClient({
      coreAddress: baseUrl, adapterId: "token-commune", authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
    }, diagnostics);
    const actualFailUnsupported = coreClient.failUnsupported.bind(coreClient);
    let failRetryOnce = true;
    let crashNextTerminal = false;
    coreClient.failUnsupported = async (operation) => {
      if (failRetryOnce) {
        failRetryOnce = false;
        throw new ConnectError("injected retryable terminal report loss", Code.Unavailable);
      }
      if (crashNextTerminal) {
        crashNextTerminal = false;
        throw new Error("injected hard process loss after delivery acknowledgement");
      }
      return actualFailUnsupported(operation);
    };
    host = new AdapterProcess({
      coreAddress: baseUrl, adapterId: "token-commune", adapterGeneration: 1,
      authorityDomainId: domainId, attachmentEvidence: adapterEvidence,
      gatewayBaseUrl: new URL("https://gateway.invalid/"), gatewayCredentialFile: "/not-read",
      pollIntervalMs: 30_000, diagnosticPath, gateway: emptyGateway(),
      diagnostics, forwardDiagnostics: true, coreClient,
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
    // inbox shape, so this adapter-only feature can exercise its delivery seam.
    // The polling runtime independently emits its honest empty PARTIAL report.
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
    const transitions = commandTransitions(await readAfter(control, 0n), "unsupported-query");
    const deliveredIndex = transitions.findIndex((item) => item.toState === OperationState.DELIVERED);
    const failedIndex = transitions.findIndex((item) => item.toState === OperationState.FAILED && item.failureCode === FailureCode.UNSUPPORTED_COMMAND);
    assert.equal(transitions.filter((item) => item.toState === OperationState.DELIVERED).length, 1, "delivery must be acknowledged exactly once");
    assert.notEqual(deliveredIndex, -1, "transition sequence must contain DELIVERED");
    assert.notEqual(failedIndex, -1, "transition sequence must contain FAILED + UNSUPPORTED_COMMAND");
    assert.ok(deliveredIndex < failedIndex, "unsupported delivery must transition DELIVERED before FAILED");

    appendAcceptedOperation(databasePath, operation("unsupported-replacement"), auth.grantId);
    crashNextTerminal = true;
    await assert.rejects(run, /injected hard process loss after delivery acknowledgement/);
    await host.dispose();
    host = undefined;
    run = undefined;
    const replacementDiagnostics = await openAdapterDiagnostics({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 1,
      secrets: [adapterEvidence, gatewayKey, `Bearer ${gatewayKey}`],
    });
    host = new AdapterProcess({
      coreAddress: baseUrl, adapterId: "token-commune", adapterGeneration: 1,
      authorityDomainId: domainId, attachmentEvidence: adapterEvidence,
      gatewayBaseUrl: new URL("https://gateway.invalid/"), gatewayCredentialFile: "/not-read",
      pollIntervalMs: 30_000, diagnosticPath, gateway: emptyGateway(),
      diagnostics: replacementDiagnostics, forwardDiagnostics: true,
    });
    await host.start();
    run = host.run(controller.signal);
    await waitFor(async () => commandTransitions(await readAfter(control, 0n), "unsupported-replacement")
      .some((item) => item.toState === OperationState.FAILED && item.failureCode === FailureCode.UNSUPPORTED_COMMAND),
    "replacement process unsupported terminalization");
    const replacementTransitions = commandTransitions(await readAfter(control, 0n), "unsupported-replacement");
    assert.equal(replacementTransitions.filter((item) => item.toState === OperationState.DELIVERED).length, 1);
    assert.equal(replacementTransitions.filter((item) => item.toState === OperationState.FAILED && item.failureCode === FailureCode.UNSUPPORTED_COMMAND).length, 1);
    assert.equal(replacementTransitions.some((item) => item.toState === OperationState.COMPLETED), false);

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

// Serial full-boundary evidence: local HTTP gateway + 0600 credential + real
// adapter process + generated RPCs + Rust core/SQLite + resource cockpit payload.
test("real gateway/core flow preserves PARTIAL, reconnect, source fencing, and member-key redaction", { timeout: 90_000 }, async () => {
  const port = await freePort();
  let adminPort = await freePort();
  while (adminPort === port) adminPort = await freePort();
  mkdirSync(join(repoRoot, "tmp"), { recursive: true });
  const directory = mkdtempSync(join(repoRoot, "tmp", "token-commune-conformance-e2e-"));
  const databasePath = join(directory, "core.sqlite3");
  const diagnosticPath = join(directory, "adapter.log");
  const credentialPath = join(directory, "member.key");
  const sentinel = "tc_member_9Wv3mK8qR6xN2pH7dF4sT1zC";
  writeFileSync(credentialPath, `${sentinel}\n`, { mode: 0o600 });
  chmodSync(credentialPath, 0o600);
  const gateway = new ScriptedTokenCommuneGateway();
  const gatewayUrl = await gateway.start(conformanceGatewaySteps(sentinel));
  const credential = await loadGatewayCredential(credentialPath);
  const httpGateway = createHttpTokenCommuneGatewayClient({ baseUrl: gatewayUrl, credential, requestTimeoutMs: 2_000 });
  const core = startCore(port, adminPort, databasePath);
  const controller1 = new AbortController();
  const controller2 = new AbortController();
  let process1: AdapterProcess | undefined;
  let process2: AdapterProcess | undefined;
  let run1: Promise<void> | undefined;
  let run2: Promise<void> | undefined;
  try {
    const setupSecret = await waitForCore(port, core);
    const baseUrl = `http://127.0.0.1:${port}`;
    const auth = await bootstrapAndLogin(baseUrl, `http://127.0.0.1:${adminPort}`, setupSecret);
    const control = makeControlClient(baseUrl, auth);
    const identities = createCompositeLocalIdentitySynthesizer({ adapterId: "token-commune", gatewayBaseUrl: gatewayUrl });
    const openaiId = identities.providerPool("openai-codex").resourceId;
    const anthropicId = identities.providerPool("anthropic").resourceId;
    const diagnostics1 = await openAdapterDiagnostics({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 1,
      secrets: [adapterEvidence, sentinel, `Bearer ${sentinel}`, credentialPath],
    });
    const oldClient = new PatchbayCoreClient({
      coreAddress: baseUrl, adapterId: "token-commune", authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
    }, diagnostics1);
    process1 = new AdapterProcess({
      coreAddress: baseUrl, adapterId: "token-commune", adapterGeneration: 1,
      authorityDomainId: domainId, attachmentEvidence: adapterEvidence,
      gatewayBaseUrl: gatewayUrl, gatewayCredentialFile: credentialPath,
      pollIntervalMs: 100, diagnosticPath, gateway: httpGateway, diagnostics: diagnostics1,
      forwardDiagnostics: true, coreClient: oldClient, retryDelayMs: 10,
    });
    await process1.start();
    run1 = process1.run(controller1.signal);

    let snapshot = await waitForResourceSnapshot(control, (candidate) =>
      resource(candidate, openaiId)?.freshness === ResourceFreshnessState.CURRENT,
    );
    assert.equal(snapshot.viewRevisions.length, 2, "both exact token-commune views have revisions");
    assert.ok(snapshot.viewRevisions.every((view) => view.completeness === AdapterSnapshotSupport.PARTIAL));
    assert.ok(resource(snapshot, openaiId)?.projectionPayload, "mixed-success report installs listed provider evidence");

    gateway.advance();
    await waitFor(async () => observations(await readAfter(control, 0n), "patchbay.token_commune.pool_event.v1")
      .some((observation) => JSON.parse(new TextDecoder().decode(observation.payload?.payload)).sourceEventId === "new"),
    "overlapping latest-50 event to append exactly once");
    assert.equal(observations(await readAfter(control, 0n), "patchbay.token_commune.pool_event.v1")
      .filter((observation) => JSON.parse(new TextDecoder().decode(observation.payload?.payload)).sourceEventId === "new").length, 1);

    gateway.advance();
    snapshot = await waitForResourceSnapshot(control, (candidate) =>
      resource(candidate, openaiId)?.freshness === ResourceFreshnessState.STALE,
    );
    assert.equal(resource(snapshot, openaiId)?.freshness, ResourceFreshnessState.STALE, "empty PARTIAL poll preserves the row as stale");
    controller1.abort();
    await run1;
    await process1.dispose();
    process1 = undefined;
    run1 = undefined;

    gateway.advance();
    const diagnostics2 = await openAdapterDiagnostics({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 2,
      secrets: [adapterEvidence, sentinel, `Bearer ${sentinel}`, credentialPath],
    });
    process2 = new AdapterProcess({
      coreAddress: baseUrl, adapterId: "token-commune", adapterGeneration: 2,
      authorityDomainId: domainId, attachmentEvidence: adapterEvidence,
      gatewayBaseUrl: gatewayUrl, gatewayCredentialFile: credentialPath,
      pollIntervalMs: 100, diagnosticPath, gateway: httpGateway, diagnostics: diagnostics2,
      forwardDiagnostics: true, retryDelayMs: 10,
    });
    await process2.start();
    run2 = process2.run(controller2.signal);
    snapshot = await waitForResourceSnapshot(control, (candidate) =>
      resource(candidate, anthropicId)?.freshness === ResourceFreshnessState.CURRENT
        && resource(candidate, openaiId)?.freshness === ResourceFreshnessState.STALE,
    );
    assert.equal(resource(snapshot, anthropicId)?.sourceAdapterGeneration?.value, 2n);
    assert.equal(resource(snapshot, openaiId)?.freshness, ResourceFreshnessState.STALE, "generation-2 PARTIAL omission cannot promote prior identities");

    const gap = observations(await readAfter(control, 0n), "patchbay.token_commune.event_gap.v1")
      .map((observation) => JSON.parse(new TextDecoder().decode(observation.payload?.payload)) as Record<string, unknown>)
      .find((payload) => payload["visibleWindowSize"] === 50);
    assert.ok(gap, "the latest-50 rollover is externally visible as gap evidence");
    assert.equal(gap["continuity"], "unknown-before-visible-window");
    assert.equal(Object.hasOwn(gap, "missedCount"), false, "the adapter cannot fabricate a missed count");

    const otherClient = new PatchbayCoreClient({
      coreAddress: baseUrl, adapterId: "other-token-observer", authorityDomainId: domainId,
      attachmentEvidence: adapterEvidence,
    });
    await otherClient.attach(1);
    const beforeFence = await readAfter(control, 0n);
    const staleReport = projectTokenCommuneSnapshot({
      adapterId: "token-commune", adapterGeneration: 1,
      observedAt: timestampFromDate(new Date("2026-08-08T12:00:00.000Z")), identities,
      gateway: { status: { status: "unavailable" }, pool: { status: "unavailable" }, me: { status: "unavailable" }, fingerprints: { status: "unavailable" }, models: { status: "unavailable" } },
    });
    await assert.rejects(oldClient.ingestResourceReport(staleReport), "generation-1 client remains fenced after generation 2 attaches");
    await assert.rejects(otherClient.ingestEvent(create(ObservationSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
      sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: "other-token-observer" }) }),
      kind: ObservationKind.STATUS,
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RESOURCE,
        resource: { adapterId: create(AdapterIdSchema, { value: "token-commune" }), resourceKind: { value: "token-commune.provider-pool" }, resourceId: { value: openaiId } },
      }),
      payload: create(PayloadEnvelopeSchema, { contentType: PayloadContentType.JSON, schemaRef: "patchbay.token_commune.cross_owner.v1", payload: encoder.encode("{}") }),
    })), "cross-owner evidence remains inert");
    const afterFence = await readAfter(control, 0n);
    assert.equal(countKinds(afterFence, [StoredEventKind.RESOURCE_STATE]), countKinds(beforeFence, [StoredEventKind.RESOURCE_STATE]));
    assert.equal(nonRegistrationObservationCount(afterFence), nonRegistrationObservationCount(beforeFence));

    const diagnosticsQuery = await control.queryDiagnostics(create(QueryDiagnosticsRequestSchema, {
      operation: adapterStatusOperation("redaction-adapter-status"),
    }));
    const finalEvents = await readAfter(control, 0n);
    const finalSnapshot = await loadResourceSnapshot(control);
    assertSecretAbsent(sentinel, [
      { name: "subscription-events", bytes: encoder.encode(JSON.stringify(finalEvents, bigintJson)) },
      { name: "resource-snapshot", bytes: toBinary(ResourceSnapshotSchema, finalSnapshot) },
      { name: "diagnostic-query", bytes: encoder.encode(JSON.stringify(diagnosticsQuery, bigintJson)) },
      { name: "adapter-diagnostics", bytes: readFileSync(diagnosticPath) },
      { name: "sqlite-bytes", bytes: readFileSync(databasePath) },
    ]);
    assert.ok(gateway.requests.length > 0);
    assert.ok(gateway.requests.every((request) => request.authorization === `Bearer ${sentinel}`));
  } finally {
    controller2.abort();
    controller1.abort();
    if (process2) await process2.dispose();
    if (process1) await process1.dispose();
    await Promise.allSettled([run2, run1].filter((run): run is Promise<void> => run !== undefined));
    credential.dispose();
    await gateway.close();
    core.kill("SIGTERM");
    rmSync(directory, { recursive: true, force: true });
  }
});

function emptyGateway(): TokenCommuneGatewayClient {
  const fingerprints = {
    anthropic: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
    codex: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
  };
  return {
    getStatus: async () => ({ ok: true, anthropicHealth: { state: "fresh" }, contributions: [] }),
    getPool: async () => ({ contributions: [] }),
    getMe: async () => ({ displayName: "Ada", reports: [] }),
    getEvents: async () => ({ historyMode: "latest-50-no-cursor", events: [] }),
    getFingerprints: async () => fingerprints,
    getModels: async () => ({ models: [] }),
  };
}

function conformanceGatewaySteps(secret: string): readonly ScriptedGatewayStep[] {
  const authorization = `Bearer ${secret}`;
  const fingerprint = { state: "unknown", templateSource: "compiled", since: null, diff: null };
  const fingerprints = {
    anthropic: { templateSource: "compiled", lastCapture: null, lastCaptureAt: null, lastDiff: null, hold: null },
    "openai-codex": { templateSource: "compiled", lastCapture: null, lastCaptureAt: null, lastDiff: null, hold: null },
  };
  const contribution = (provider: string, usedFraction: number) => ({
    provider, declaredShare: 1, health: { state: "fresh" },
    capacity: [{ window: "5h", usedFraction, usedUnits: usedFraction * 100, limitUnits: 100, resetsAt: null, source: "usage_endpoint", observedAt: 1_786_190_100_000 }],
    fingerprint,
  });
  const model = (provider: string, id: string) => ({
    id, provider, surface: "chat", upstream_model: null,
    context_window: 200000, max_tokens: 8192, reasoning: true, available: true,
  });
  const event = (id: string, at: number, provider: string) => ({
    id, at, kind: "member", provider, contributionId: null, message: `visible-${id}`,
  });
  const response = (body: unknown, status = 200) => ({ status, body });
  const step = (provider: string, usedFraction: number, events: unknown[], statusBody: unknown): ScriptedGatewayStep => ({
    expectedAuthorization: authorization,
    responses: {
      "/commune/status": response(statusBody),
      "/commune/pool": response({ providers: [contribution(provider, usedFraction)] }),
      "/commune/me": response({ member: "Ada", draw: [{ provider, limitFraction: 0.25, fromDecree: false, consumedUnits: 5, drawUnits: null, exceeded: false, enforceable: false, resetsAt: null }] }),
      "/commune/events": response({ events }),
      "/commune/fingerprint": response(fingerprints),
      "/v1/models": response({ data: [model(provider, provider === "openai-codex" ? "gpt-5.5" : "claude-sonnet-4-5")] }),
    },
  });
  const initial = [event("old", 1_786_190_000_000, "openai-codex")];
  const overlap = [...initial, event("new", 1_786_190_060_000, "openai-codex")];
  const failed = Object.fromEntries(Object.values({
    status: "/commune/status", pool: "/commune/pool", me: "/commune/me", events: "/commune/events",
    fingerprints: "/commune/fingerprint", models: "/v1/models",
  }).map((endpoint) => [endpoint, response({ error: "unavailable" }, 503)])) as ScriptedGatewayStep["responses"];
  const latestFifty = Array.from({ length: 50 }, (_, index) => event(`g2-${String(index + 1).padStart(2, "0")}`, 1_786_190_200_000 + index, "anthropic"));
  return [
    step("openai-codex", 0.35, initial, { ok: false, anthropicHealth: { state: "auth_broken", reason: secret }, contributions: [] }),
    step("openai-codex", 0.40, overlap, { ok: true, anthropicHealth: { state: "fresh" }, contributions: [] }),
    { expectedAuthorization: authorization, responses: failed },
    step("anthropic", 0.20, latestFifty, { ok: true, anthropicHealth: { state: "fresh" }, contributions: [] }),
  ];
}

async function loadResourceSnapshot(control: ReturnType<typeof makeControlClient>): Promise<ResourceSnapshot> {
  const loaded = await control.loadSnapshot(create(LoadSnapshotRequestSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    viewKind: SnapshotViewKind.RESOURCE,
  }));
  assert.equal(loaded.present, true);
  assert.equal(loaded.viewKind, SnapshotViewKind.RESOURCE);
  return fromBinary(ResourceSnapshotSchema, loaded.snapshotPayload);
}

async function waitForResourceSnapshot(
  control: ReturnType<typeof makeControlClient>,
  predicate: (snapshot: ResourceSnapshot) => boolean,
): Promise<ResourceSnapshot> {
  let current: ResourceSnapshot | undefined;
  await waitFor(async () => {
    current = await loadResourceSnapshot(control);
    return predicate(current);
  }, "resource snapshot predicate", 15_000);
  return current!;
}

function resource(snapshot: ResourceSnapshot, resourceId: string) {
  return snapshot.resources.find((candidate) => candidate.identity?.resourceId?.value === resourceId);
}

function observations(payloads: readonly StoredEventPayload[], schemaRef: string) {
  return payloads.filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
    .map((payload) => fromBinary(ObservationSchema, payload.payload))
    .filter((observation) => observation.payload?.schemaRef === schemaRef);
}

function countKinds(payloads: readonly StoredEventPayload[], kinds: readonly StoredEventKind[]): number {
  return payloads.filter((payload) => kinds.includes(payload.kind)).length;
}

function nonRegistrationObservationCount(payloads: readonly StoredEventPayload[]): number {
  return payloads.filter((payload) => payload.kind === StoredEventKind.OBSERVATION)
    .map((payload) => fromBinary(ObservationSchema, payload.payload))
    .filter((observation) => observation.payload?.schemaRef !== "patchbay.AdapterRegistration").length;
}

function adapterStatusOperation(commandId: string) {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: domainId }),
    sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: operatorId }) }),
    kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.AUTHORITY_DOMAIN }),
    validityWindow: create(TimeWindowSchema, { startsAt: { seconds: 1n }, expiresAt: { seconds: 2_534_023_007_99n } }),
    submittedAt: { seconds: 1n }, idempotencyKey: `${commandId}-key`,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.DiagnosticsQuery",
      payload: toBinary(DiagnosticsQuerySchema, create(DiagnosticsQuerySchema, {
        query: { case: "adapters", value: create(AdapterStatusQuerySchema, { adapterIds: [create(AdapterIdSchema, { value: "token-commune" })], limit: 10, recentDiagnosticLimit: 20 }) },
      })),
    }),
  });
}

function bigintJson(_key: string, value: unknown): unknown {
  return typeof value === "bigint" ? value.toString() : value;
}

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
