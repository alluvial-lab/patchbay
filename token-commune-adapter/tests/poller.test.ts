import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { timestampDate } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError } from "@connectrpc/connect";
import { AdapterSnapshotSupport, EventIdSchema, LsnSchema, type Observation, type ResourceReport } from "@patchbay/contracts";
import type { AdapterDiagnostics } from "../src/adapter_diagnostics.js";
import {
  GatewayClientError,
  MAX_RETRY_AFTER_MS,
  type GatewayEventsPage,
  type TokenCommuneGatewayClient,
} from "../src/gateway_client.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { TokenCommunePoller, type PollClock, type PollerCoreSink, type PollWaiter } from "../src/poller.js";

const emptyFingerprints = {
  anthropic: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
  codex: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
};
const emptyValues = {
  status: { ok: true, anthropicHealth: { state: "fresh" as const }, contributions: [] },
  pool: { contributions: [] },
  me: { displayName: "Ada", reports: [] },
  fingerprints: emptyFingerprints,
  models: { models: [] },
  events: { historyMode: "latest-50-no-cursor" as const, events: [] },
};
const identities = createCompositeLocalIdentitySynthesizer({
  adapterId: "token-commune",
  gatewayBaseUrl: new URL("https://gateway.example/"),
});

function eventId(value: bigint) {
  return create(EventIdSchema, { authorityDomainId: { value: "default" }, lsn: create(LsnSchema, { value }) });
}
function gateway(overrides: Partial<TokenCommuneGatewayClient> = {}): TokenCommuneGatewayClient {
  return {
    getStatus: async () => emptyValues.status,
    getPool: async () => emptyValues.pool,
    getMe: async () => emptyValues.me,
    getFingerprints: async () => emptyValues.fingerprints,
    getModels: async () => emptyValues.models,
    getEvents: async () => emptyValues.events,
    ...overrides,
  };
}
function core(input: {
  reports?: ResourceReport[];
  events?: Observation[];
  onEvent?: (observation: Observation) => Promise<void>;
} = {}): PollerCoreSink {
  let lsn = 0n;
  return {
    async ingestResourceReport(report) {
      input.reports?.push(report);
      return eventId(++lsn);
    },
    async ingestEvent(observation) {
      input.events?.push(observation);
      await input.onEvent?.(observation);
      return eventId(++lsn);
    },
  };
}
function poller(options: {
  gateway: TokenCommuneGatewayClient;
  core: PollerCoreSink;
  clock?: PollClock;
  waiter?: PollWaiter;
  interval?: number;
  diagnostics?: AdapterDiagnostics;
}) {
  return new TokenCommunePoller({
    adapterId: "token-commune",
    adapterGeneration: 3,
    authorityDomainId: "default",
    pollIntervalMs: options.interval ?? 30_000,
    gateway: options.gateway,
    core: options.core,
    identities,
    ...(options.clock ? { clock: options.clock } : {}),
    ...(options.waiter ? { waiter: options.waiter } : {}),
    ...(options.diagnostics ? { diagnostics: options.diagnostics } : {}),
  });
}
function body(observation: Observation): any {
  return JSON.parse(new TextDecoder().decode(observation.payload?.payload));
}

test("run polls immediately, settles all six reads concurrently, and waits only after completion", async () => {
  let release!: () => void;
  const blocked = new Promise<void>((resolve) => { release = resolve; });
  let active = 0;
  let maxActive = 0;
  const calls: string[] = [];
  const read = async <T>(name: string, value: T): Promise<T> => {
    calls.push(name);
    active += 1;
    maxActive = Math.max(maxActive, active);
    await blocked;
    active -= 1;
    return value;
  };
  const fakeGateway = gateway({
    getStatus: () => read("status", emptyValues.status),
    getPool: () => read("pool", emptyValues.pool),
    getMe: () => read("me", emptyValues.me),
    getFingerprints: () => read("fingerprints", emptyValues.fingerprints),
    getModels: () => read("models", emptyValues.models),
    getEvents: () => read("events", emptyValues.events),
  });
  const controller = new AbortController();
  const waits: number[] = [];
  const reports: ResourceReport[] = [];
  const run = poller({
    gateway: fakeGateway,
    core: core({ reports }),
    clock: { now: () => new Date("2026-08-07T12:00:00.000Z") },
    waiter: { async wait(milliseconds) { waits.push(milliseconds); controller.abort(); } },
  }).run(controller.signal);
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.equal(calls.length, 6, "first cycle starts before any wait");
  assert.equal(waits.length, 0, "completion-to-start cadence cannot overlap a running cycle");
  release();
  await run;
  assert.equal(maxActive, 6);
  assert.deepEqual(waits, [30_000]);
  assert.equal(reports.length, 1);
});

test("valid Retry-After extends but never shortens the configured minimum", async () => {
  const errors = [
    new GatewayClientError("http", "/commune/status", 429, { retryAfterMs: 60_000 }),
    new GatewayClientError("http", "/commune/status", 503, { retryAt: "2026-08-07T12:00:45.000Z" }),
    new GatewayClientError("http", "/commune/status", 503, { invalid: true }),
    new GatewayClientError("http", "/commune/status", 503, {
      retryAfterMs: Number.MAX_SAFE_INTEGER,
      invalid: true,
    }),
  ];
  const expected = [60_000, 45_000, 30_000, MAX_RETRY_AFTER_MS];
  for (let index = 0; index < errors.length; index += 1) {
    const result = await poller({
      gateway: gateway({ getStatus: async () => { throw errors[index]; } }),
      core: core(),
      clock: { now: () => new Date("2026-08-07T12:00:00.000Z") },
    }).pollOnce(new AbortController().signal);
    assert.equal(result.nextDelayMs, expected[index]);
  }
});

test("oversized Retry-After is diagnosed and the final waiter delay stays bounded", async () => {
  const diagnostics: Parameters<AdapterDiagnostics["record"]>[0][] = [];
  const controller = new AbortController();
  const waits: number[] = [];
  await poller({
    gateway: gateway({
      getStatus: async () => {
        throw new GatewayClientError("http", "/commune/status", 429, {
          retryAfterMs: Number.MAX_SAFE_INTEGER,
          invalid: true,
        });
      },
    }),
    core: core(),
    diagnostics: {
      record(input) { diagnostics.push(input); },
      async flush() {},
      async close() {},
    },
    waiter: {
      async wait(milliseconds) {
        waits.push(milliseconds);
        controller.abort();
      },
    },
  }).run(controller.signal);

  assert.deepEqual(waits, [MAX_RETRY_AFTER_MS]);
  assert.ok(waits[0]! < 2 ** 31 - 1, "Node must never clamp the poll timer to a hot loop");
  assert.equal(diagnostics.filter((item) => item.event === "poll.retry_after.invalid").length, 1);
});

test("failed endpoints become unavailable without reusing prior-cycle source values", async () => {
  const reports: ResourceReport[] = [];
  let statusCalls = 0;
  const readingAt = "2026-08-07T11:55:00.000Z";
  const fakeGateway = gateway({
    getStatus: async () => {
      statusCalls += 1;
      if (statusCalls === 2) throw new GatewayClientError("transport", "/commune/status");
      return {
        ok: true,
        anthropicHealth: { state: "fresh" },
        contributions: [{ contributionId: "one", provider: "zai", readings: [{
          window: "5h", usedFraction: 0.25, usedUnits: 25, limitUnits: 100, resetsAt: null,
          source: "usage_endpoint", observedAt: readingAt,
        }] }],
      };
    },
  });
  const times = [new Date("2026-08-07T12:00:00.000Z"), new Date("2026-08-07T12:01:00.000Z")];
  const runtime = poller({ gateway: fakeGateway, core: core({ reports }), clock: { now: () => times.shift()! } });
  await runtime.pollOnce(new AbortController().signal);
  await runtime.pollOnce(new AbortController().signal);
  assert.equal(reports.length, 2);
  assert.equal(timestampDate(reports[0]!.observedAt!).toISOString(), "2026-08-07T12:00:00.000Z");
  assert.equal(timestampDate(reports[1]!.observedAt!).toISOString(), "2026-08-07T12:01:00.000Z");
  assert.equal(reports[0]!.report.case, "snapshot");
  assert.equal(reports[1]!.report.case, "snapshot");
  if (reports[0]!.report.case !== "snapshot" || reports[1]!.report.case !== "snapshot") assert.fail("snapshot expected");
  const firstProvider = reports[0]!.report.value.views[0]!.mutations.find((row) => row.identity?.resourceId?.value.includes("zai"));
  assert.ok(firstProvider && firstProvider.mutation.case === "upsert");
  if (firstProvider.mutation.case !== "upsert") assert.fail("upsert expected");
  const projected = JSON.parse(new TextDecoder().decode(firstProvider.mutation.value.projectionPayload?.payload));
  assert.equal(projected.statusTelemetry.contributions[0].readings[0].observedAt, readingAt);
  assert.ok(reports[1]!.report.value.views.every((view) => view.completeness === AdapterSnapshotSupport.PARTIAL));
  assert.ok(reports[1]!.report.value.views.every((view) => view.mutations.length === 0), "failed status is unavailable, not cached");
});

test("all snapshot failures still emit an empty two-view PARTIAL report", async () => {
  const reports: ResourceReport[] = [];
  const failed = (endpoint: ConstructorParameters<typeof GatewayClientError>[1]) => async () => {
    throw new GatewayClientError("transport", endpoint);
  };
  await poller({
    gateway: gateway({
      getStatus: failed("/commune/status"), getPool: failed("/commune/pool"), getMe: failed("/commune/me"),
      getFingerprints: failed("/commune/fingerprint"), getModels: failed("/v1/models"), getEvents: failed("/commune/events"),
    }),
    core: core({ reports }),
  }).pollOnce(new AbortController().signal);
  assert.equal(reports[0]?.report.case, "snapshot");
  if (reports[0]?.report.case !== "snapshot") assert.fail("snapshot expected");
  assert.equal(reports[0].report.value.views.length, 2);
  assert.ok(reports[0].report.value.views.every((view) => view.completeness === AdapterSnapshotSupport.PARTIAL && view.mutations.length === 0));
});

test("event ids advance only after acknowledgement and recovery always reports before reconciliation", async () => {
  const pages: GatewayEventsPage[] = [
    { historyMode: "latest-50-no-cursor", events: [{ id: "old", occurredAt: "2026-08-07T12:00:00.000Z", kind: "member", provider: "zai", contributionId: null, message: "old" }] },
    { historyMode: "latest-50-no-cursor", events: [
      { id: "old", occurredAt: "2026-08-07T12:00:00.000Z", kind: "member", provider: "zai", contributionId: null, message: "old" },
      { id: "new", occurredAt: "2026-08-07T12:01:00.000Z", kind: "windfall", provider: "zai", contributionId: null, message: "new" },
    ] },
  ];
  let pageIndex = 0;
  const calls: string[] = [];
  const observations: Observation[] = [];
  let failPoolEvent = true;
  let lsn = 0n;
  const sink: PollerCoreSink = {
    async ingestResourceReport() { calls.push("report"); return eventId(++lsn); },
    async ingestEvent(observation) {
      observations.push(observation);
      const schema = observation.payload?.schemaRef ?? "missing";
      calls.push(schema);
      if (schema.endsWith("pool_event.v1") && failPoolEvent) {
        failPoolEvent = false;
        throw new ConnectError("core unavailable", Code.Unavailable);
      }
      return eventId(++lsn);
    },
  };
  const runtime = poller({
    gateway: gateway({ getEvents: async () => pages[Math.min(pageIndex, 1)]! }),
    core: sink,
  });
  await runtime.pollOnce(new AbortController().signal); // baseline, no replay
  pageIndex = 1;
  await runtime.pollOnce(new AbortController().signal); // event ingress fails
  await runtime.pollOnce(new AbortController().signal); // retry succeeds
  await runtime.pollOnce(new AbortController().signal); // acknowledged id is deduped
  assert.equal(observations.filter((item) => item.payload?.schemaRef.endsWith("pool_event.v1")).length, 2, "failed pre-ack id retries exactly once");
  assert.equal(observations.filter((item) => body(item).sourceEventId === "old").length, 0, "baseline history is never replayed");
  const poolIndexes = calls.flatMap((value, index) => value.endsWith("pool_event.v1") ? [index] : []);
  for (const index of poolIndexes) assert.equal(calls[index - 1], "report", "each recovery cycle reports before event reconciliation");
  for (const observation of observations) {
    const serialized = JSON.stringify(body(observation));
    assert.equal(/heartbeat|sessionConnectivity|liveness|"current"|"stale"/i.test(serialized), false, "poller emits no fabricated liveness evidence");
  }
});

test("core disconnect accepts no event and reconnect emits a fresh report before overlap reconciliation", async () => {
  const baseline: GatewayEventsPage = { historyMode: "latest-50-no-cursor", events: [
    { id: "old", occurredAt: "2026-08-07T12:00:00.000Z", kind: "member", provider: "zai", contributionId: null, message: "old" },
  ] };
  const changed: GatewayEventsPage = { historyMode: "latest-50-no-cursor", events: [...baseline.events,
    { id: "new", occurredAt: "2026-08-07T12:01:00.000Z", kind: "member", provider: "zai", contributionId: null, message: "new" },
  ] };
  let page = baseline;
  let reportCalls = 0;
  let eventCalls = 0;
  const order: string[] = [];
  let lsn = 0n;
  const sink: PollerCoreSink = {
    async ingestResourceReport() {
      reportCalls += 1;
      order.push(`report:${reportCalls}`);
      if (reportCalls === 2) throw new ConnectError("disconnected", Code.Unavailable);
      return eventId(++lsn);
    },
    async ingestEvent(observation) {
      eventCalls += 1;
      order.push(observation.payload?.schemaRef ?? "event");
      return eventId(++lsn);
    },
  };
  const runtime = poller({ gateway: gateway({ getEvents: async () => page }), core: sink });
  await runtime.pollOnce(new AbortController().signal);
  const afterBaseline = eventCalls;
  page = changed;
  await runtime.pollOnce(new AbortController().signal);
  assert.equal(eventCalls, afterBaseline, "disconnected report ingress cannot advance event delivery");
  await runtime.pollOnce(new AbortController().signal);
  assert.equal(eventCalls, afterBaseline + 1);
  assert.deepEqual(order.slice(-2), ["report:3", "patchbay.token_commune.pool_event.v1"]);
});

test("wrong-domain and nonpositive event acknowledgements leave the event retryable", async () => {
  let page: GatewayEventsPage = emptyValues.events;
  let eventAttempts = 0;
  let invalidAcknowledgement = 0;
  let lsn = 0n;
  const sink: PollerCoreSink = {
    async ingestResourceReport() { return eventId(++lsn); },
    async ingestEvent(observation) {
      const acknowledged = eventId(++lsn);
      if (observation.payload?.schemaRef.endsWith("pool_event.v1")) {
        eventAttempts += 1;
        invalidAcknowledgement += 1;
        if (invalidAcknowledgement === 1) acknowledged.authorityDomainId!.value = "other-domain";
        if (invalidAcknowledgement === 2) acknowledged.lsn!.value = 0n;
      }
      return acknowledged;
    },
  };
  const runtime = poller({ gateway: gateway({ getEvents: async () => page }), core: sink });
  await runtime.pollOnce(new AbortController().signal);
  page = { historyMode: "latest-50-no-cursor", events: [{
    id: "new", occurredAt: "2026-08-07T12:01:00.000Z", kind: "member",
    provider: "zai", contributionId: null, message: "new",
  }] };

  await assert.rejects(
    runtime.pollOnce(new AbortController().signal),
    /authority domain/,
  );
  await assert.rejects(
    runtime.pollOnce(new AbortController().signal),
    /authority domain or LSN/,
  );
  await runtime.pollOnce(new AbortController().signal);
  await runtime.pollOnce(new AbortController().signal);

  assert.equal(eventAttempts, 3, "invalid acknowledgements must not consume the source event");
});

test("partially accepted multi-target gap retry does not duplicate accepted targets", async () => {
  const page: GatewayEventsPage = { historyMode: "latest-50-no-cursor", events: [
    { id: "a", occurredAt: "2026-08-07T12:00:00.000Z", kind: "member", provider: "anthropic", contributionId: null, message: "a" },
    { id: "z", occurredAt: "2026-08-07T12:00:01.000Z", kind: "member", provider: "zai", contributionId: null, message: "z" },
  ] };
  const counts = new Map<string, number>();
  let failSecond = true;
  const runtime = poller({
    gateway: gateway({ getEvents: async () => page }),
    core: core({ onEvent: async (observation) => {
      const id = observation.targetScope?.resource?.resourceId?.value ?? "missing";
      counts.set(id, (counts.get(id) ?? 0) + 1);
      if (id === identities.providerPool("zai").resourceId && failSecond) {
        failSecond = false;
        throw new ConnectError("outage", Code.Unavailable);
      }
    } }),
  });
  await runtime.pollOnce(new AbortController().signal);
  await runtime.pollOnce(new AbortController().signal);
  assert.equal(counts.get(identities.providerPool("anthropic").resourceId), 1);
  assert.equal(counts.get(identities.providerPool("zai").resourceId), 2);
});
