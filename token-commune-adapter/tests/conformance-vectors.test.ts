import assert from "node:assert/strict";
import { create } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  AdapterSnapshotSupport,
  CommandIdSchema,
  EventIdSchema,
  LsnSchema,
  OperationKind,
  OperationSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type Observation,
  type Operation,
  type ResourceReport,
} from "@patchbay/contracts";
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { loadGatewayCredential } from "../src/credential.js";
import { LatestEventWindowTracker } from "../src/event_window.js";
import {
  GatewayClientError,
  createHttpTokenCommuneGatewayClient,
  type GatewayEventsPage,
  type TokenCommuneGatewayClient,
} from "../src/gateway_client.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { CoreDiagnosticsForwarder } from "../src/core_diagnostics_forwarder.js";
import { AdapterProcess } from "../src/main.js";
import { TokenCommunePoller, type PollerCoreSink } from "../src/poller.js";
import { projectTokenCommuneSnapshot } from "../src/snapshot_projection.js";
import type { PatchbayCoreClient } from "../src/core_client.js";
import { withProductionMutant, type ProductionReplacement } from "./production-mutant.js";
import {
  assertPartialSnapshotOracle,
  assertReconnectOracle,
  assertSecretAbsent,
  assertUnsupportedTerminalization,
  type LifecycleFact,
  type ReconnectReferenceStep,
} from "./conformance-oracles.js";

const RUNNER = "token-commune-adapter";
const encoder = new TextEncoder();
const jsonBigint = (_key: string, value: unknown) => typeof value === "bigint" ? value.toString() : value;

interface ImplementationCheck { runner: string; case: string }
interface MutationWitness { mutation_id: string; runner: string; invariant: string }
interface ConformanceVector {
  vector_id: string;
  property_id: string;
  promotion_status: string;
  implementation_checks?: readonly ImplementationCheck[];
  mutation_witnesses?: readonly MutationWitness[];
  input: Record<string, any>;
  expected_outcome: Record<string, any>;
}
interface RequestedCheck { vector_id: string; case: string }
interface RequestedMutation { vector_id: string; mutation_id: string }

function vectors(): ReadonlyMap<string, ConformanceVector> {
  const directory = path.resolve(process.cwd(), "../contracts/vectors");
  const rows = readdirSync(directory).filter((filename) => filename.endsWith(".json")).sort()
    .map((filename) => JSON.parse(readFileSync(path.join(directory, filename), "utf8")) as ConformanceVector);
  return new Map(rows.map((vector) => [vector.vector_id, vector]));
}
function requestedChecks(): readonly RequestedCheck[] {
  return process.env["PATCHBAY_CONFORMANCE_REQUESTS"]
    ? JSON.parse(process.env["PATCHBAY_CONFORMANCE_REQUESTS"]) as RequestedCheck[] : [];
}
function requestedMutations(): readonly RequestedMutation[] {
  return process.env["PATCHBAY_CONFORMANCE_MUTATIONS"]
    ? JSON.parse(process.env["PATCHBAY_CONFORMANCE_MUTATIONS"]) as RequestedMutation[] : [];
}

function identities(vector: ConformanceVector) {
  return createCompositeLocalIdentitySynthesizer({
    adapterId: vector.input.adapter_id ?? "token-commune",
    gatewayBaseUrl: new URL(vector.input.gateway_base_url ?? "https://commune.example/"),
  });
}
function reportObservation(report: ResourceReport) {
  assert.equal(report.report.case, "snapshot");
  if (report.report.case !== "snapshot") assert.fail("snapshot expected");
  return {
    views: report.report.value.views.map((view) => ({
      resourceKind: view.resourceKind?.value ?? "",
      completeness: view.completeness === AdapterSnapshotSupport.PARTIAL ? "partial" : "not-partial",
      mutations: view.mutations.map((mutation) => {
        assert.equal(mutation.mutation.case, "upsert");
        if (mutation.mutation.case !== "upsert") assert.fail("upsert expected");
        return {
          payload: JSON.parse(new TextDecoder().decode(mutation.mutation.value.resourcePayload?.payload)),
          projection: JSON.parse(new TextDecoder().decode(mutation.mutation.value.projectionPayload?.payload)),
        };
      }),
    })),
  };
}
function projectWith(vector: ConformanceVector, projector: typeof projectTokenCommuneSnapshot): ResourceReport {
  return projector({
    adapterId: vector.input.adapter_id,
    adapterGeneration: vector.input.adapter_generation,
    observedAt: timestampFromDate(new Date(vector.input.observed_at)),
    identities: identities(vector),
    gateway: vector.input.gateway,
  });
}
function project(vector: ConformanceVector): ResourceReport {
  return projectWith(vector, projectTokenCommuneSnapshot);
}

function executePartial(vector: ConformanceVector, projector = projectTokenCommuneSnapshot): void {
  assertPartialSnapshotOracle(vector.input, reportObservation(projectWith(vector, projector)));
}

function eventPage(ids: readonly string[]): GatewayEventsPage {
  return {
    historyMode: "latest-50-no-cursor",
    events: ids.map((id, index) => ({
      id,
      occurredAt: new Date(Date.UTC(2026, 7, 8, 12, 0, index)).toISOString(),
      kind: "member",
      provider: "openai-codex",
      contributionId: null,
      message: id,
    })),
  };
}
async function executeReconnect(vector: ConformanceVector, Poller: typeof TokenCommunePoller = TokenCommunePoller): Promise<void> {
  const steps = vector.input.steps as ReconnectReferenceStep[];
  const safeStatus = { ok: true, anthropicHealth: { state: "fresh" as const }, contributions: [] };
  const safeFingerprints = {
    anthropic: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
    codex: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
  };
  let currentPage = eventPage([]);
  const gateway: TokenCommuneGatewayClient = {
    getStatus: async () => safeStatus, getPool: async () => ({ contributions: [] }),
    getMe: async () => ({ displayName: "Ada", reports: [] }), getFingerprints: async () => safeFingerprints,
    getModels: async () => ({ models: [] }), getEvents: async () => currentPage,
  };
  let lsn = 0n;
  let cycle = 0;
  const traces = steps.map(() => [] as Array<{ kind: "report" | "gap" | "event"; lsn?: bigint; id?: string; reason?: string; missedCount?: number }>);
  const core: PollerCoreSink = {
    async ingestResourceReport() {
      const committed = ++lsn; traces[cycle]!.push({ kind: "report", lsn: committed }); return eventId(committed);
    },
    async ingestEvent(observation) {
      const payload = JSON.parse(new TextDecoder().decode(observation.payload?.payload)) as Record<string, any>;
      const gap = observation.payload?.schemaRef === "patchbay.token_commune.event_gap.v1";
      const row: { kind: "gap" | "event"; lsn?: bigint; id?: string; reason?: string; missedCount?: number } = gap
        ? { kind: "gap", reason: payload.reason, ...(payload.missedCount === undefined ? {} : { missedCount: payload.missedCount }) }
        : { kind: "event", id: payload.sourceEventId };
      traces[cycle]!.push(row);
      if (cycle === 1 && row.kind === "event") throw new ConnectError("injected response loss before acknowledgement", Code.Unavailable);
      const committed = ++lsn; row.lsn = committed; return eventId(committed);
    },
  };
  const poller = new Poller({
    adapterId: "token-commune", adapterGeneration: 1, authorityDomainId: "token-conformance",
    pollIntervalMs: 1000, gateway, core,
    identities: createCompositeLocalIdentitySynthesizer({ adapterId: "token-commune", gatewayBaseUrl: new URL("https://commune.example/") }),
    clock: { now: () => new Date("2026-08-08T12:00:00.000Z") },
  });
  for (cycle = 0; cycle < steps.length; cycle += 1) {
    currentPage = eventPage(steps[cycle]!.page_ids);
    await poller.pollOnce(new AbortController().signal);
  }
  const observed = traces.map((trace, index) => ({
    baselineOnly: index === 0 && trace.every((entry) => entry.kind !== "event"),
    emittedIds: trace.filter((entry) => entry.kind === "event").map((entry) => entry.id!),
    gap: trace.find((entry) => entry.kind === "gap")?.reason ?? null,
    order: trace.map((entry) => entry.kind),
    missedCount: trace.find((entry) => entry.kind === "gap")?.missedCount,
  }));
  assertReconnectOracle(vector.input as { steps: readonly ReconnectReferenceStep[] }, observed as any);
}

function failedGateway(): TokenCommuneGatewayClient {
  const failure = (endpoint: ConstructorParameters<typeof GatewayClientError>[1]) => async () => {
    throw new GatewayClientError("transport", endpoint);
  };
  return {
    getStatus: failure("/commune/status"),
    getPool: failure("/commune/pool"),
    getMe: failure("/commune/me"),
    getEvents: failure("/commune/events"),
    getFingerprints: failure("/commune/fingerprint"),
    getModels: failure("/v1/models"),
  };
}
function eventId(lsn: bigint) {
  return create(EventIdSchema, { authorityDomainId: { value: "token-conformance" }, lsn: create(LsnSchema, { value: lsn }) });
}
async function executeDegradation(vector: ConformanceVector): Promise<void> {
  const reports: ResourceReport[] = [];
  const core: PollerCoreSink = {
    async ingestResourceReport(report) { reports.push(report); return eventId(BigInt(reports.length)); },
    async ingestEvent() { assert.fail("failed events endpoint must emit no event"); },
  };
  const poller = new TokenCommunePoller({
    adapterId: "token-commune", adapterGeneration: 1, authorityDomainId: "token-conformance",
    pollIntervalMs: 1000, gateway: failedGateway(), core,
    identities: createCompositeLocalIdentitySynthesizer({ adapterId: "token-commune", gatewayBaseUrl: new URL("https://commune.example/") }),
    clock: { now: () => new Date("2026-08-08T12:00:00.000Z") },
  });
  await poller.pollOnce(new AbortController().signal);
  assert.equal(reports[0]?.report.case, "snapshot");
  if (reports[0]?.report.case !== "snapshot") assert.fail("snapshot expected");
  assert.equal(reports.length, 1, "the real failed poll must commit one resource report");
  assert.equal(reports[0].report.value.views.length, 2, "the real projector must retain both declared views");
  assert.ok(reports[0].report.value.views.every((view) =>
    view.completeness === AdapterSnapshotSupport.PARTIAL && view.mutations.length === 0),
  "every failed endpoint must produce an empty PARTIAL view without prior-cycle reuse");
}

async function executeRedaction(vector: ConformanceVector, seams: {
  createGatewayClient?: typeof createHttpTokenCommuneGatewayClient;
  openDiagnostics?: typeof openAdapterDiagnostics;
  Forwarder?: typeof CoreDiagnosticsForwarder;
  expectReflectionRejected?: boolean;
} = {}): Promise<void> {
  const directory = mkdtempSync(path.join(tmpdir(), "patchbay-token-vector-"));
  const credentialPath = path.join(directory, "member.key");
  const diagnosticPath = path.join(directory, "adapter.log");
  const sqlitePath = path.join(directory, "evidence.sqlite3");
  const secret = vector.input.secret as string;
  try {
    writeFileSync(credentialPath, `${secret}\n`, { mode: 0o600 });
    const credential = await loadGatewayCredential(credentialPath);
    const forms = [
      secret,
      `Bearer ${secret}`,
      `https://gateway.invalid/?token=${encodeURIComponent(secret)}`,
      Buffer.from(secret).toString("base64"),
      JSON.stringify({ authorization: secret }),
      credentialPath,
    ];
    assert.deepEqual(vector.input.hostile_forms, ["raw", "bearer", "url-encoded", "base64", "json-string", "credential-file-path"]);
    const requests: string[] = [];
    const reflectedMe: any[] = [];
    for (const hostile of forms) {
      const client = (seams.createGatewayClient ?? createHttpTokenCommuneGatewayClient)({
        baseUrl: new URL("https://commune.example/"), credential, redactionSecrets: [credentialPath],
        fetch: async (request, init) => {
          const headers = new Headers(request instanceof Request ? request.headers : init?.headers);
          requests.push(headers.get("authorization") ?? "");
          return Response.json({ member: hostile, draw: [{
            provider: "openai-codex", limitFraction: 0.25, fromDecree: false, consumedUnits: 1,
            drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
          }] });
        },
      });
      if (seams.expectReflectionRejected === false) reflectedMe.push(await client.getMe());
      else await assert.rejects(client.getMe(), (error: unknown) => error instanceof GatewayClientError && error.kind === "invalid-response");
    }
    assert.ok(requests.every((authorization) => authorization === `Bearer ${secret}`), "the key is used only on outbound Authorization");

    const diagnostics = await (seams.openDiagnostics ?? openAdapterDiagnostics)({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 1,
      secrets: [...credential.redactionSecrets(), credentialPath],
    });
    for (const hostile of forms) diagnostics.record({
      event: "gateway.request.failed", level: "error", commandId: hostile,
      error: { name: hostile, code: `authorization=${hostile}` },
    });
    await diagnostics.close();

    const forwarded: unknown[] = [];
    const forwarder = new (seams.Forwarder ?? CoreDiagnosticsForwarder)(
      async (report) => { forwarded.push(report); return { accepted: true } as any; },
      { authorityDomainId: "token-conformance", adapterId: "token-commune", adapterGeneration: 1 },
      { reportsPerSecond: 1000 },
    );
    forwarder.record({ event: "gateway.request.failed", level: "error", error: { name: secret, code: `authorization=${secret}` } });
    await forwarder.flush();
    await forwarder.close();
    assert.ok(forwarded.length > 0, "a real forwarded diagnostic must be scanned");

    let page = eventPage(["baseline"]);
    const resourceReports: ResourceReport[] = [];
    const observations: Observation[] = [];
    let lsn = 0n;
    const safeGateway: TokenCommuneGatewayClient = {
      getStatus: async () => ({ ok: true, anthropicHealth: { state: "fresh" }, contributions: [] }),
      getPool: async () => ({ contributions: [] }), getMe: async () => reflectedMe[0] ?? ({ displayName: "Ada", reports: [] }),
      getFingerprints: async () => ({
        anthropic: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
        codex: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
      }),
      getModels: async () => ({ models: [] }), getEvents: async () => page,
    };
    const poller = new TokenCommunePoller({
      adapterId: "token-commune", adapterGeneration: 1, authorityDomainId: "token-conformance", pollIntervalMs: 1000,
      gateway: safeGateway, identities: createCompositeLocalIdentitySynthesizer({ adapterId: "token-commune", gatewayBaseUrl: new URL("https://commune.example/") }),
      core: {
        async ingestResourceReport(report) { resourceReports.push(report); return eventId(++lsn); },
        async ingestEvent(observation) { observations.push(observation); return eventId(++lsn); },
      },
    });
    await poller.pollOnce(new AbortController().signal);
    page = eventPage(["baseline", "new"]);
    await poller.pollOnce(new AbortController().signal);
    assert.ok(resourceReports.length >= 2 && observations.length >= 1, "actual poller outputs must populate every scan family");

    const database = new DatabaseSync(sqlitePath);
    database.exec("PRAGMA journal_mode=WAL; CREATE TABLE evidence(kind TEXT, payload BLOB)");
    const insert = database.prepare("INSERT INTO evidence(kind,payload) VALUES (?,?)");
    for (const [kind, value] of [["report", resourceReports], ["observation", observations], ["diagnostic", forwarded]] as const) {
      insert.run(kind, encoder.encode(JSON.stringify(value, (_key, item) => typeof item === "bigint" ? item.toString() : item)));
    }
    const queryOutput = database.prepare("SELECT kind,payload FROM evidence ORDER BY rowid").all();
    const scanTargets = [
      { name: "resource-reports", bytes: encoder.encode(JSON.stringify(resourceReports, jsonBigint)) },
      { name: "observations", bytes: encoder.encode(JSON.stringify(observations, jsonBigint)) },
      { name: "diagnostics", bytes: readFileSync(diagnosticPath) },
      { name: "forwarded-diagnostics", bytes: encoder.encode(JSON.stringify(forwarded, jsonBigint)) },
      { name: "audit-query", bytes: encoder.encode(JSON.stringify(queryOutput, jsonBigint)) },
      { name: "snapshots", bytes: encoder.encode(JSON.stringify(resourceReports.at(-1), jsonBigint)) },
      { name: "subscriptions", bytes: encoder.encode(JSON.stringify({ resourceReports, observations }, jsonBigint)) },
      ...[sqlitePath, `${sqlitePath}-wal`, `${sqlitePath}-shm`].map((file) => ({ name: path.basename(file), bytes: readFileSync(file) })),
    ];
    assert.deepEqual(vector.input.required_sinks, ["resource-reports", "observations", "diagnostics", "audit-query", "snapshots", "subscriptions", "sqlite-bytes"]);
    assertSecretAbsent(secret, scanTargets);
    database.exec("PRAGMA wal_checkpoint(TRUNCATE)");
    database.close();
    assertSecretAbsent(secret, [
      { name: "sqlite-closed", bytes: readFileSync(sqlitePath) },
      ...[`${sqlitePath}-wal`, `${sqlitePath}-shm`].flatMap((file) => {
        try { return [{ name: path.basename(file), bytes: readFileSync(file) }]; } catch { return []; }
      }),
    ]);
    credential.dispose();
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function unsupportedOperation(id: string): Operation {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: id }), kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.ADAPTER, adapterId: { value: "token-commune" } }),
  });
}
class TerminalCore {
  readonly facts = new Map<string, LifecycleFact[]>();
  readonly terminal = new Set<string>();
  readonly deliveryAttempts = new Map<string, number>();
  failMode: "retry-once" | "crash-once" | "none" = "none";
  #failed = false;
  #lsn = 0n;
  constructor(readonly deliveries: readonly Operation[]) {}
  async attach() { return eventId(++this.#lsn); }
  async *receiveDeliveries(_cursor: bigint, signal?: AbortSignal) {
    for (const operation of this.deliveries) {
      const id = operation.commandId!.value;
      if (this.terminal.has(id)) continue;
      const attempt = (this.deliveryAttempts.get(id) ?? 0) + 1;
      this.deliveryAttempts.set(id, attempt);
      (operation as any).__conformanceRedelivery = attempt > 1;
      yield { operation, deliveryEventId: eventId(++this.#lsn) } as any;
    }
    await new Promise<void>((resolve) => {
      const finish = () => resolve();
      signal?.addEventListener("abort", finish, { once: true });
      if (signal?.aborted) resolve();
    });
  }
  async acknowledgeDelivery(operation: Operation) {
    const facts = this.for(operation);
    if (!facts.some((fact) => fact.state === "delivered")) facts.push({ state: "delivered", eventLsn: ++this.#lsn });
    return eventId(this.#lsn);
  }
  async failUnsupported(operation: Operation) {
    if (!this.#failed && this.failMode !== "none") {
      this.#failed = true;
      if (this.failMode === "retry-once") throw new ConnectError("retry", Code.Unavailable);
      throw new Error("hard process loss");
    }
    const facts = this.for(operation);
    if (!facts.some((fact) => fact.state === "failed")) facts.push({ state: "failed", failureCode: "unsupported_command", eventLsn: ++this.#lsn });
    this.terminal.add(operation.commandId!.value);
    return eventId(this.#lsn);
  }
  async completeUnsupported(operation: Operation) {
    const facts = this.for(operation);
    facts.push({ state: "completed", eventLsn: ++this.#lsn });
    this.terminal.add(operation.commandId!.value);
    return eventId(this.#lsn);
  }
  async failUnavailable(operation: Operation) {
    const facts = this.for(operation);
    facts.push({ state: "failed", failureCode: "adapter_unavailable", eventLsn: ++this.#lsn });
    this.terminal.add(operation.commandId!.value);
    return eventId(this.#lsn);
  }
  async appendDuplicateTransitions(operation: Operation) {
    const facts = this.for(operation);
    facts.push({ state: "delivered", eventLsn: ++this.#lsn });
    facts.push({ state: "failed", failureCode: "unsupported_command", eventLsn: ++this.#lsn });
    this.terminal.add(operation.commandId!.value);
    return eventId(this.#lsn);
  }
  reportDiagnostic() { return Promise.resolve({ accepted: true } as any); }
  ingestResourceReport() { return Promise.resolve(eventId(++this.#lsn)); }
  ingestEvent() { return Promise.resolve(eventId(++this.#lsn)); }
  private for(operation: Operation): LifecycleFact[] {
    const id = operation.commandId!.value;
    const facts = this.facts.get(id) ?? [{ state: "accepted", eventLsn: ++this.#lsn } satisfies LifecycleFact];
    this.facts.set(id, facts);
    return facts;
  }
}
function idlePoller() {
  return { async run(signal: AbortSignal) {
    await new Promise<void>((resolve) => {
      const finish = () => resolve();
      signal.addEventListener("abort", finish, { once: true });
      if (signal.aborted) resolve();
    });
  } } as TokenCommunePoller;
}
function processFor(core: TerminalCore, Process: typeof AdapterProcess = AdapterProcess): AdapterProcess {
  return new Process({
    coreAddress: "http://unused", adapterId: "token-commune", adapterGeneration: 1,
    authorityDomainId: "token-conformance", attachmentEvidence: "unused",
    gatewayBaseUrl: new URL("https://commune.example/"), gatewayCredentialFile: "/unused",
    pollIntervalMs: 1000, diagnosticPath: "/unused", gateway: failedGateway(),
    coreClient: core as unknown as PatchbayCoreClient, poller: idlePoller(), retryDelayMs: 1,
  });
}
async function waitFor(predicate: () => boolean, message: string): Promise<void> {
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) { if (predicate()) return; await new Promise((resolve) => setTimeout(resolve, 5)); }
  throw new Error(`timed out waiting for ${message}`);
}
async function executeTerminalization(vector: ConformanceVector, Process: typeof AdapterProcess = AdapterProcess): Promise<void> {
  const retryOperation = unsupportedOperation(vector.input.scenarios[0].command_id);
  const retryLater = unsupportedOperation(`${vector.input.later_command_id}-retry`);
  const retryCore = new TerminalCore([retryOperation, retryLater]);
  retryCore.failMode = "retry-once";
  const retryProcess = processFor(retryCore, Process);
  const retryAbort = new AbortController();
  const retryRun = retryProcess.run(retryAbort.signal);
  await waitFor(() => retryCore.terminal.has(retryOperation.commandId!.value) && retryCore.terminal.has(retryLater.commandId!.value), "same-process terminal retry and later delivery");
  retryAbort.abort();
  await retryRun;
  await retryProcess.dispose();
  assertUnsupportedTerminalization(
    retryCore.facts.get(retryOperation.commandId!.value)!,
    retryCore.facts.get(retryLater.commandId!.value)!,
  );

  const replacementOperation = unsupportedOperation(vector.input.scenarios[1].command_id);
  const replacementLater = unsupportedOperation(`${vector.input.later_command_id}-replacement`);
  const replacementCore = new TerminalCore([replacementOperation, replacementLater]);
  replacementCore.failMode = "crash-once";
  const lost = processFor(replacementCore, Process);
  await assert.rejects(lost.run(), /hard process loss/);
  await lost.dispose();
  replacementCore.failMode = "none";
  const replacement = processFor(replacementCore, Process);
  const replacementAbort = new AbortController();
  const replacementRun = replacement.run(replacementAbort.signal);
  await waitFor(() => replacementCore.terminal.has(replacementOperation.commandId!.value) && replacementCore.terminal.has(replacementLater.commandId!.value), "replacement-process terminalization and later delivery");
  replacementAbort.abort();
  await replacementRun;
  await replacement.dispose();
  assertUnsupportedTerminalization(
    replacementCore.facts.get(replacementOperation.commandId!.value)!,
    replacementCore.facts.get(replacementLater.commandId!.value)!,
  );
}

function executeCockpitFixture(vector: ConformanceVector): void {
  const unavailable = { status: "unavailable" } as const;
  const report = projectTokenCommuneSnapshot({
    adapterId: vector.input.adapter_id, adapterGeneration: 3,
    observedAt: timestampFromDate(new Date("2026-08-08T12:00:00.000Z")), identities: identities(vector),
    gateway: {
      status: unavailable,
      pool: { status: "reported", value: vector.input.gateway.pool },
      me: { status: "reported", value: vector.input.gateway.me },
      fingerprints: unavailable,
      models: { status: "reported", value: vector.input.gateway.models },
    },
  });
  const observed = reportObservation(report);
  const pool = observed.views[0]?.mutations[0]?.projection as any;
  const draw = observed.views[1]?.mutations[0]?.projection as any;
  assert.deepEqual(pool, vector.input.expected_projection, "adapter projection bytes must equal the cross-package fixture");
  assert.deepEqual(draw, vector.input.expected_draw_projection, "adapter member-draw projection must equal the cross-package fixture");
  assert.equal(pool.provider, vector.expected_outcome.current_provider);
  assert.equal(pool.capacityAggregation, vector.expected_outcome.capacity_aggregation);
  assert.equal(Math.max(...pool.contributionListing.contributions.map((contribution: any) => contribution.capacityReadings[0].usedFraction)), vector.expected_outcome.current_capacity_used_fraction);
  for (const hostile of Object.keys(vector.input.hostile_fields)) assert.equal(Object.hasOwn(pool, hostile), false);
}

async function executeCase(vector: ConformanceVector, caseName: string): Promise<void> {
  switch (caseName) {
    case "partial_snapshot_honesty": executePartial(vector); return;
    case "bounded_reconnect_honesty": await executeReconnect(vector); return;
    case "degradation_failed_poll_report": await executeDegradation(vector); return;
    case "gateway_key_redaction": await executeRedaction(vector); return;
    case "unsupported_operation_terminalization": await executeTerminalization(vector); return;
    case "cockpit_projection_fixture": executeCockpitFixture(vector); return;
    default: throw new Error(`unhandled ${RUNNER} conformance case ${vector.vector_id}:${caseName}`);
  }
}

async function expectProductionMutationKilled(
  baseline: () => Promise<void> | void,
  replacements: readonly ProductionReplacement[],
  entry: string,
  mutant: (module: Record<string, any>) => Promise<void> | void,
  label: string,
): Promise<void> {
  await baseline();
  let killed = false;
  try { await withProductionMutant(replacements, entry, mutant); }
  catch (error) {
    assert.notEqual((error as Error).name, "SyntaxError", `production mutation ${label} did not load`);
    assert.doesNotMatch(String((error as Error).message), /production mutation anchor/, `production mutation ${label} did not reach its oracle`);
    killed = true;
  }
  assert.equal(killed, true, `production mutation ${label} survived the baseline oracle`);
}

async function killMutation(vector: ConformanceVector, mutationId: string): Promise<void> {
  const projectorMutations: Record<string, ProductionReplacement> = {
    "partial-to-authoritative": {
      file: "snapshot_projection.js", from: "completeness: AdapterSnapshotSupport.PARTIAL,", to: "completeness: AdapterSnapshotSupport.AUTHORITATIVE,",
    },
    "drop-declared-view": {
      file: "snapshot_projection.js", from: "const views = Object.keys(TOKEN_COMMUNE_RESOURCES).map((name) =>", to: "const views = Object.keys(TOKEN_COMMUNE_RESOURCES).slice(0, 1).map((name) =>",
    },
    "reuse-prior-successful-slice": {
      file: "snapshot_projection.js",
      from: "return { status: \"unknown\", probe, reason: \"probe-unavailable\" };",
      to: "return { status: \"reported\", probe, value: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false } };",
    },
    "missing-telemetry-to-zero": {
      file: "snapshot_projection.js",
      from: "? { ...common, telemetryState: \"no-readings\", capacityReadings: [] }",
      to: "? { ...common, telemetryState: \"readings\", capacityReadings: [{ window: \"5h\", usedFraction: 0, usedUnits: 0, limitUnits: null, resetsAt: null, source: \"declared\", observedAt: \"2026-08-08T12:00:00.000Z\" }] }",
    },
    "synthesize-capacity-aggregate": {
      file: "snapshot_projection.js",
      from: "capacityAggregation: \"none\",\n        };\n        return upsertMutation",
      to: "capacityAggregation: \"average\",\n        };\n        return upsertMutation",
    },
  };
  if (projectorMutations[mutationId]) {
    await expectProductionMutationKilled(
      () => executePartial(vector), [projectorMutations[mutationId]!], "snapshot_projection.js",
      (module) => executePartial(vector, module.projectTokenCommuneSnapshot), mutationId,
    );
    return;
  }

  const reconnectMutations: Record<string, readonly ProductionReplacement[]> = {
    "replay-initial-history": [{
      file: "event_window.js", from: "events: [],", to: "events: page.events.slice().sort(compareEvent),",
    }],
    "advance-dedup-before-ack": [{
      file: "poller.js",
      from: "requireAcknowledgement(await this.options.core.ingestEvent(mapped.observation), \"pool event\", this.options.authorityDomainId);\n            this.#tracker.acknowledgeEvent(event.id);",
      to: "this.#tracker.acknowledgeEvent(event.id);\n            requireAcknowledgement(await this.options.core.ingestEvent(mapped.observation), \"pool event\", this.options.authorityDomainId);",
    }],
    "suppress-no-anchor-gap": [{
      file: "event_window.js",
      from: "if (visibleWindowSize === MAX_PAGE_SIZE)\n            return \"window-saturated-without-anchor\";",
      to: "if (visibleWindowSize === MAX_PAGE_SIZE)\n            return undefined;",
    }],
    "fabricate-missed-count": [{
      file: "event_observation.js", from: "continuity: input.gap.continuity,", to: "continuity: input.gap.continuity,\n        missedCount: 47,",
    }],
    "events-before-reconnect-report": [{
      file: "poller.js",
      from: `        try {
            requireAcknowledgement(await this.options.core.ingestResourceReport(report), "resource report", this.options.authorityDomainId);
        }
        catch (error) {
            if (isRetryableCoreFailure(error))
                return { nextDelayMs };
            throw error;
        }
        if (cycle.events.status === "fulfilled") {
            try {
                await this.#emitEventPlan(cycle.events.value, report, observedAt);
            }
            catch (error) {
                if (isRetryableCoreFailure(error))
                    return { nextDelayMs };
                throw error;
            }
        }`,
      to: `        if (cycle.events.status === "fulfilled") {
            try {
                await this.#emitEventPlan(cycle.events.value, report, observedAt);
            }
            catch (error) {
                if (isRetryableCoreFailure(error))
                    return { nextDelayMs };
                throw error;
            }
        }
        try {
            requireAcknowledgement(await this.options.core.ingestResourceReport(report), "resource report", this.options.authorityDomainId);
        }
        catch (error) {
            if (isRetryableCoreFailure(error))
                return { nextDelayMs };
            throw error;
        }`,
    }],
  };
  if (reconnectMutations[mutationId]) {
    await expectProductionMutationKilled(
      () => executeReconnect(vector), reconnectMutations[mutationId]!, "poller.js",
      (module) => executeReconnect(vector, module.TokenCommunePoller), mutationId,
    );
    return;
  }

  const terminalMutations: Record<string, ProductionReplacement> = {
    "clear-pending-before-terminal-ack": {
      file: "main.js",
      from: "await this.#core.failUnsupported(operation);\n        this.#pendingTerminalization = undefined;",
      to: "this.#pendingTerminalization = undefined;\n        await this.#core.failUnsupported(operation);",
    },
    "filter-delivered-nonterminal-on-replacement": {
      file: "main.js",
      from: "const operation = requiredOperation(delivery);\n            const commandId",
      to: "const operation = requiredOperation(delivery);\n            if (operation.__conformanceRedelivery) continue;\n            const commandId",
    },
    "advance-later-delivery-first": {
      file: "main.js",
      from: "this.#record({\n                event: \"delivery.acknowledged\", level: \"info\", ...(commandId ? { commandId } : {}), operationKind: operation.kind,\n            });\n            await this.#finishPendingTerminalization();",
      to: "this.#record({\n                event: \"delivery.acknowledged\", level: \"info\", ...(commandId ? { commandId } : {}), operationKind: operation.kind,\n            });\n            if (!operation.commandId?.value.includes(\"later\")) continue;\n            await this.#finishPendingTerminalization();",
    },
    "terminalize-completed": {
      file: "main.js", from: "await this.#core.failUnsupported(operation);", to: "await this.#core.completeUnsupported(operation);",
    },
    "use-adapter-unavailable": {
      file: "main.js", from: "await this.#core.failUnsupported(operation);", to: "await this.#core.failUnavailable(operation);",
    },
    "duplicate-delivery-or-terminal-transition": {
      file: "main.js", from: "await this.#core.failUnsupported(operation);", to: "await this.#core.appendDuplicateTransitions(operation);",
    },
  };
  if (terminalMutations[mutationId]) {
    await expectProductionMutationKilled(
      () => executeTerminalization(vector), [terminalMutations[mutationId]!], "main.js",
      (module) => executeTerminalization(vector, module.AdapterProcess), mutationId,
    );
    return;
  }

  if (["leak-key-in-resource-payload", "persist-authorization-in-audit", "leak-key-in-snapshot", "leak-key-in-subscription", "leak-key-in-sqlite-bytes"].includes(mutationId)) {
    await expectProductionMutationKilled(
      () => executeRedaction(vector), [{
        file: "gateway_client.js",
        from: "if (forms.some((form) => text.includes(form))) {",
        to: "if (false && forms.some((form) => text.includes(form))) {",
      }], "gateway_client.js",
      (module) => executeRedaction(vector, { createGatewayClient: module.createHttpTokenCommuneGatewayClient, expectReflectionRejected: false }), mutationId,
    );
    return;
  }
  if (mutationId === "remove-local-secret-replacement") {
    await expectProductionMutationKilled(
      () => executeRedaction(vector), [{
        file: "adapter_diagnostics.js", from: "for (const secret of this.#secrets)\n            result = result.split(secret).join(\"[REDACTED]\");",
        to: "for (const secret of [])\n            result = result.split(secret).join(\"[REDACTED]\");",
      }], "adapter_diagnostics.js",
      (module) => executeRedaction(vector, { openDiagnostics: module.openAdapterDiagnostics }), mutationId,
    );
    return;
  }
  if (mutationId === "forward-arbitrary-error-text") {
    await expectProductionMutationKilled(
      () => executeRedaction(vector), [{
        file: "core_diagnostics_forwarder.js", from: "const code = TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES[input.event];",
        to: "const code = input.error?.name ?? TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES[input.event];",
      }], "core_diagnostics_forwarder.js",
      (module) => executeRedaction(vector, { Forwarder: module.CoreDiagnosticsForwarder }), mutationId,
    );
    return;
  }
  throw new Error(`unhandled mutation ${vector.vector_id}:${mutationId}`);
}

test("conformance vector runner", async () => {
  const corpus = vectors();
  for (const request of requestedChecks()) {
    const vector = corpus.get(request.vector_id);
    assert.ok(vector, `unknown vector id ${request.vector_id}`);
    assert.ok(vector.implementation_checks?.some((check) => check.runner === RUNNER && check.case === request.case), `unregistered requested check ${request.vector_id}:${request.case}`);
    await executeCase(vector, request.case);
    console.log(`PATCHBAY_CONFORMANCE_EXECUTED=${request.vector_id}:${request.case}`);
  }
  for (const request of requestedMutations()) {
    const vector = corpus.get(request.vector_id);
    assert.ok(vector, `unknown mutation vector id ${request.vector_id}`);
    assert.ok(vector.mutation_witnesses?.some((witness) => witness.runner === RUNNER && witness.mutation_id === request.mutation_id), `unregistered requested mutation ${request.vector_id}:${request.mutation_id}`);
    await killMutation(vector, request.mutation_id);
    console.log(`PATCHBAY_CONFORMANCE_MUTATION_KILLED=${request.vector_id}:${request.mutation_id}`);
  }
});
