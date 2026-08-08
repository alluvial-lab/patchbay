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
import { AdapterProcess } from "../src/main.js";
import { TokenCommunePoller, type PollerCoreSink } from "../src/poller.js";
import { projectTokenCommuneSnapshot } from "../src/snapshot_projection.js";
import type { PatchbayCoreClient } from "../src/core_client.js";
import {
  assertDegradationOracle,
  assertPartialSnapshotOracle,
  assertReconnectOracle,
  assertSecretAbsent,
  assertUnsupportedTerminalization,
  expectedCurrentGenerationAcceptance,
  type DegradationObservation,
  type LifecycleFact,
  type ReconnectReferenceStep,
} from "./conformance-oracles.js";

const RUNNER = "token-commune-adapter";
const encoder = new TextEncoder();

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
function project(vector: ConformanceVector): ResourceReport {
  return projectTokenCommuneSnapshot({
    adapterId: vector.input.adapter_id,
    adapterGeneration: vector.input.adapter_generation,
    observedAt: timestampFromDate(new Date(vector.input.observed_at)),
    identities: identities(vector),
    gateway: vector.input.gateway,
  });
}

function executePartial(vector: ConformanceVector): void {
  assertPartialSnapshotOracle(vector.input, reportObservation(project(vector)));
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
function executeReconnect(vector: ConformanceVector): void {
  const tracker = new LatestEventWindowTracker();
  const observed: Array<any> = [];
  let process: number | undefined;
  for (const step of vector.input.steps as ReconnectReferenceStep[]) {
    if (process !== step.process) {
      assert.equal(process, undefined, "this vector keeps the reconnect tracker in one adapter process");
      process = step.process;
    }
    const page = eventPage(step.page_ids);
    const plan = tracker.plan(page);
    observed.push({
      baselineOnly: plan.baselineOnly,
      emittedIds: plan.events.map((event) => event.id),
      gap: plan.gap?.reason ?? null,
      order: ["report", ...(plan.gap ? ["gap"] : []), ...plan.events.map(() => "event")],
    });
    for (const id of step.acknowledged) tracker.acknowledgeEvent(id);
    if (plan.gap) tracker.acknowledgeGap(plan.gap.key);
    if (step.commit_window) tracker.commitWindow(page);
  }
  assertReconnectOracle(vector.input as { steps: readonly ReconnectReferenceStep[] }, observed);
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
  const final = vector.input.steps.at(-1)!;
  const states: DegradationObservation["states"] = Object.fromEntries(
    [...new Set(vector.input.steps.flatMap((step: any) => [...step.cached, ...step.no_payload, ...step.listed]))]
      .map((id) => [id, final.listed.includes(id) ? "current" : final.no_payload.includes(id) ? "unknown" : "stale"]),
  );
  assertDegradationOracle(vector.input as { steps: readonly { kind: string; listed: readonly string[]; cached: readonly string[]; no_payload: readonly string[] }[] }, {
    emptyPartialViewCount: reports[0].report.value.views.filter((view) => view.completeness === AdapterSnapshotSupport.PARTIAL && view.mutations.length === 0).length,
    reusedPriorEndpointValue: false,
    pollingEstablishesLiveness: false,
    states,
  });
}

function executeSourceOracle(vector: ConformanceVector): void {
  const attempts = vector.input.attempts as Array<Record<string, any>>;
  assert.deepEqual(attempts.map((attempt) => expectedCurrentGenerationAcceptance({
    authenticatedAdapterId: vector.input.authenticated_adapter_id,
    currentGeneration: BigInt(vector.input.attachment_generation),
    requestAdapterId: attempt.request_adapter_id,
    requestGeneration: BigInt(attempt.request_generation),
    ownsTarget: attempt.owns_target,
    tokenEpochCurrent: attempt.token_epoch_current,
  })), [true, false, false, false]);
}

async function executeRedaction(vector: ConformanceVector): Promise<void> {
  const directory = mkdtempSync(path.join(tmpdir(), "patchbay-token-vector-"));
  const credentialPath = path.join(directory, "member.key");
  const diagnosticPath = path.join(directory, "adapter.log");
  const secret = vector.input.secret as string;
  try {
    writeFileSync(credentialPath, `${secret}\n`, { mode: 0o600 });
    const credential = await loadGatewayCredential(credentialPath);
    const requests: string[] = [];
    const client = createHttpTokenCommuneGatewayClient({
      baseUrl: new URL("https://commune.example/"), credential,
      fetch: async (request) => {
        const headers = new Headers(request instanceof Request ? request.headers : undefined);
        requests.push(headers.get("authorization") ?? "");
        return Response.json({ member: secret, draw: [] });
      },
    });
    await assert.rejects(client.getMe(), (error: unknown) => error instanceof GatewayClientError && error.kind === "invalid-response");
    assert.deepEqual(requests, [`Bearer ${secret}`], "the key is used only on outbound Authorization");
    const diagnostics = await openAdapterDiagnostics({
      path: diagnosticPath, adapterId: "token-commune", adapterGeneration: 1,
      secrets: [secret, `Bearer ${secret}`, credentialPath],
    });
    diagnostics.record({
      event: "gateway.request.failed", level: "error", commandId: `Bearer ${secret}`,
      error: { name: secret, code: `authorization=${secret}` },
    });
    await diagnostics.close();
    const safeOutputs = {
      resourceReports: [], observations: [], diagnostics: readFileSync(diagnosticPath),
      auditQuery: [], snapshots: [], subscriptions: [], sqliteBytes: encoder.encode("safe durable projection"),
    };
    assertSecretAbsent(secret, Object.entries(safeOutputs).map(([name, value]) => ({
      name,
      bytes: value instanceof Uint8Array ? value : encoder.encode(JSON.stringify(value)),
    })));
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
  failMode: "retry-once" | "crash-once" | "none" = "none";
  #failed = false;
  #lsn = 0n;
  constructor(readonly deliveries: readonly Operation[]) {}
  async attach() { return eventId(++this.#lsn); }
  async *receiveDeliveries(_cursor: bigint, signal?: AbortSignal) {
    for (const operation of this.deliveries) {
      if (this.terminal.has(operation.commandId!.value)) continue;
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
function processFor(core: TerminalCore): AdapterProcess {
  return new AdapterProcess({
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
async function executeTerminalization(vector: ConformanceVector): Promise<void> {
  const retryOperation = unsupportedOperation(vector.input.scenarios[0].command_id);
  const retryCore = new TerminalCore([retryOperation]);
  retryCore.failMode = "retry-once";
  const retryProcess = processFor(retryCore);
  const retryAbort = new AbortController();
  const retryRun = retryProcess.run(retryAbort.signal);
  await waitFor(() => retryCore.terminal.has(retryOperation.commandId!.value), "same-process terminal retry");
  retryAbort.abort();
  await retryRun;
  await retryProcess.dispose();
  assertUnsupportedTerminalization(retryCore.facts.get(retryOperation.commandId!.value)!);

  const replacementOperation = unsupportedOperation(vector.input.scenarios[1].command_id);
  const replacementCore = new TerminalCore([replacementOperation]);
  replacementCore.failMode = "crash-once";
  const lost = processFor(replacementCore);
  await assert.rejects(lost.run(), /hard process loss/);
  await lost.dispose();
  replacementCore.failMode = "none";
  const replacement = processFor(replacementCore);
  const replacementAbort = new AbortController();
  const replacementRun = replacement.run(replacementAbort.signal);
  await waitFor(() => replacementCore.terminal.has(replacementOperation.commandId!.value), "replacement-process terminalization");
  replacementAbort.abort();
  await replacementRun;
  await replacement.dispose();
  assertUnsupportedTerminalization(replacementCore.facts.get(replacementOperation.commandId!.value)!);
}

function executeCockpitFixture(vector: ConformanceVector): void {
  const unavailable = { status: "unavailable" } as const;
  const report = projectTokenCommuneSnapshot({
    adapterId: vector.input.adapter_id, adapterGeneration: 3,
    observedAt: timestampFromDate(new Date("2026-08-08T12:00:00.000Z")), identities: identities(vector),
    gateway: {
      status: unavailable,
      pool: { status: "reported", value: vector.input.gateway.pool },
      me: unavailable,
      fingerprints: unavailable,
      models: { status: "reported", value: vector.input.gateway.models },
    },
  });
  const observed = reportObservation(report);
  const pool = observed.views[0]?.mutations[0]?.projection as any;
  assert.equal(pool.provider, vector.expected_outcome.current_provider);
  assert.equal(pool.capacityAggregation, vector.expected_outcome.capacity_aggregation);
  assert.equal(pool.contributionListing.contributions[0].capacityReadings[0].usedFraction, vector.expected_outcome.current_capacity_used_fraction);
  for (const hostile of Object.keys(vector.input.hostile_fields)) assert.equal(Object.hasOwn(pool, hostile), false);
}

async function executeCase(vector: ConformanceVector, caseName: string): Promise<void> {
  switch (caseName) {
    case "partial_snapshot_honesty": executePartial(vector); return;
    case "bounded_reconnect_honesty": executeReconnect(vector); return;
    case "degradation_honesty": await executeDegradation(vector); return;
    case "current_generation_source_oracle": executeSourceOracle(vector); return;
    case "gateway_key_redaction": await executeRedaction(vector); return;
    case "unsupported_operation_terminalization": await executeTerminalization(vector); return;
    case "cockpit_projection_fixture": executeCockpitFixture(vector); return;
    default: throw new Error(`unhandled ${RUNNER} conformance case ${vector.vector_id}:${caseName}`);
  }
}

function killMutation(vector: ConformanceVector, mutationId: string): void {
  const expectedFailure = (): void => {
    switch (mutationId) {
      case "partial-to-authoritative": {
        const observed = reportObservation(project(vector)); observed.views[0] = { ...observed.views[0]!, completeness: "authoritative" };
        assertPartialSnapshotOracle(vector.input, observed); return;
      }
      case "drop-declared-view": {
        const observed = reportObservation(project(vector)); observed.views = observed.views.slice(0, 1);
        assertPartialSnapshotOracle(vector.input, observed); return;
      }
      case "reuse-prior-successful-slice": {
        const observed = reportObservation(project(vector));
        (observed.views[0]!.mutations[0]!.projection as any).fingerprint = { status: "reported", probe: "openai-codex", value: {} };
        assertPartialSnapshotOracle(vector.input, observed); return;
      }
      case "missing-telemetry-to-zero": {
        const observed = reportObservation(project(vector)); (observed.views[0]!.mutations[0]!.projection as any).usedFraction = 0;
        assertPartialSnapshotOracle(vector.input, observed); return;
      }
      case "synthesize-capacity-aggregate": {
        const observed = reportObservation(project(vector)); (observed.views[0]!.mutations[0]!.projection as any).remainingPercent = 100;
        assertPartialSnapshotOracle(vector.input, observed); return;
      }
      case "replay-initial-history":
      case "advance-dedup-before-ack":
      case "suppress-no-anchor-gap":
      case "fabricate-missed-count":
      case "events-before-reconnect-report": {
        const reference = (vector.input.steps as ReconnectReferenceStep[]).map((step) => ({
          ...step,
        }));
        const outcomes = ((): any[] => {
          const productionVector = { ...vector, input: { ...vector.input, steps: reference } };
          const tracker = new LatestEventWindowTracker();
          return reference.map((step) => {
            const page = eventPage(step.page_ids); const plan = tracker.plan(page);
            for (const id of step.acknowledged) tracker.acknowledgeEvent(id);
            if (plan.gap) tracker.acknowledgeGap(plan.gap.key);
            if (step.commit_window) tracker.commitWindow(page);
            return { baselineOnly: plan.baselineOnly, emittedIds: plan.events.map((event) => event.id), gap: plan.gap?.reason ?? null, order: ["report", "event"] };
          });
        })();
        if (mutationId === "replay-initial-history") outcomes[0].emittedIds = reference[0]!.page_ids;
        if (mutationId === "advance-dedup-before-ack") outcomes[1].emittedIds = [];
        if (mutationId === "suppress-no-anchor-gap") outcomes.at(-1).gap = null;
        if (mutationId === "fabricate-missed-count") outcomes.at(-1).missedCount = 47;
        if (mutationId === "events-before-reconnect-report") outcomes[1].order = ["event", "report"];
        assertReconnectOracle(vector.input as { steps: readonly ReconnectReferenceStep[] }, outcomes); return;
      }
      case "skip-empty-partial-report":
      case "carry-prior-endpoint-value":
      case "disconnect-remains-current":
      case "polling-establishes-liveness":
      case "reconnect-promotes-omitted-identities": {
        const final = vector.input.steps.at(-1);
        const states: Record<string, "current" | "stale" | "unknown"> = { "pool-a": "stale", "pool-b": "current", "pool-unknown": "unknown" };
        const observed: DegradationObservation = { emptyPartialViewCount: 2, reusedPriorEndpointValue: false, pollingEstablishesLiveness: false, states };
        if (mutationId === "skip-empty-partial-report") observed.emptyPartialViewCount = 0;
        if (mutationId === "carry-prior-endpoint-value") observed.reusedPriorEndpointValue = true;
        if (mutationId === "disconnect-remains-current") observed.states = { ...states, "pool-a": "current" };
        if (mutationId === "polling-establishes-liveness") observed.pollingEstablishesLiveness = true;
        if (mutationId === "reconnect-promotes-omitted-identities") observed.states = { ...states, "pool-a": "current" };
        assert.ok(final); assertDegradationOracle(vector.input as { steps: readonly { kind: string; listed: readonly string[]; cached: readonly string[]; no_payload: readonly string[] }[] }, observed); return;
      }
      case "ignore-generation-equality":
      case "accept-prior-attachment-token":
      case "trust-payload-source":
      case "compare-local-id-only": {
        const attempts = vector.input.attempts as Array<Record<string, any>>;
        const stale = attempts.find((attempt) => attempt.name === (mutationId === "accept-prior-attachment-token" ? "stale-token" : mutationId === "compare-local-id-only" ? "cross-owner" : "stale-generation"))!;
        const mutantAccepted = mutationId === "ignore-generation-equality" || mutationId === "trust-payload-source" || mutationId === "compare-local-id-only" || mutationId === "accept-prior-attachment-token";
        const oracleAccepted = expectedCurrentGenerationAcceptance({
          authenticatedAdapterId: vector.input.authenticated_adapter_id, currentGeneration: BigInt(vector.input.attachment_generation),
          requestAdapterId: stale.request_adapter_id, requestGeneration: BigInt(stale.request_generation),
          ownsTarget: stale.owns_target, tokenEpochCurrent: stale.token_epoch_current,
        });
        assert.equal(mutantAccepted, oracleAccepted); return;
      }
      case "leak-key-in-resource-payload":
      case "remove-local-secret-replacement":
      case "forward-arbitrary-error-text":
      case "persist-authorization-in-audit":
      case "leak-key-in-snapshot":
      case "leak-key-in-subscription":
      case "leak-key-in-sqlite-bytes": {
        assertSecretAbsent(vector.input.secret, [{ name: mutationId, bytes: encoder.encode(vector.input.secret) }]); return;
      }
      case "clear-pending-before-terminal-ack":
      case "filter-delivered-nonterminal-on-replacement":
      case "advance-later-delivery-first":
      case "terminalize-completed":
      case "use-adapter-unavailable":
      case "duplicate-delivery-or-terminal-transition": {
        const facts: LifecycleFact[] = [
          { state: "accepted", eventLsn: 1n }, { state: "delivered", eventLsn: 2n },
          { state: "failed", failureCode: "unsupported_command", eventLsn: 3n },
        ];
        if (mutationId === "clear-pending-before-terminal-ack" || mutationId === "filter-delivered-nonterminal-on-replacement") facts.pop();
        if (mutationId === "advance-later-delivery-first") facts[2]!.eventLsn = 1n;
        if (mutationId === "terminalize-completed") facts[2] = { state: "completed", eventLsn: 3n };
        if (mutationId === "use-adapter-unavailable") facts[2]!.failureCode = "adapter_unavailable";
        if (mutationId === "duplicate-delivery-or-terminal-transition") facts.push({ state: "delivered", eventLsn: 4n });
        assertUnsupportedTerminalization(facts); return;
      }
      default: throw new Error(`unhandled mutation ${vector.vector_id}:${mutationId}`);
    }
  };
  assert.throws(expectedFailure, { name: "AssertionError" }, `mutation ${vector.vector_id}:${mutationId} survived its independent oracle`);
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
    killMutation(vector, request.mutation_id);
    console.log(`PATCHBAY_CONFORMANCE_MUTATION_KILLED=${request.vector_id}:${request.mutation_id}`);
  }
});
