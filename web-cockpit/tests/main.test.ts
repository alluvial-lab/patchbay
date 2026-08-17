import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AdapterCapabilitySummarySchema,
  AdapterIdSchema,
  AdapterStatusPageSchema,
  AdapterStatusSchema,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GenerationSchema,
  LoadSnapshotResponseSchema,
  LogicalTargetIdSchema,
  LsnSchema,
  ManagedSpawnTargetCapabilitySchema,
  OperationKind,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiContinuationMode,
  PiSpawnTargetSpecSchema,
  QueryDiagnosticsResponseSchema,
  ResourceSnapshotSchema,
  SessionActivityState,
  SessionSnapshotSchema,
  SnapshotViewKind,
  SpawnRequestSchema,
  SessionConnectivityState,
  SubmissionOutcome,
  SubmissionResultSchema,
  type LoadSnapshotRequest,
  type QueryDiagnosticsRequest,
} from "@patchbay/contracts";
import { JSDOM } from "jsdom";

import {
  buildFreshSpawnOperation,
  buildInstructOperation,
  buildRestartOperation,
  composeCockpit,
  startCockpit,
} from "../src/main.js";
import type { SessionView } from "../src/domain/model.js";
import type { ProtocolClient } from "../src/domain/protocol-client.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });

test("browser entry composes protocol client, projection, reconciler, and shell", async () => {
  const dom = new JSDOM(`<!doctype html>
    <meta name="patchbay-authority-domain" content="operator-domain">
    <main data-patchbay-cockpit></main>`);
  const fetcher = (async (input: RequestInfo | URL) => {
    assert.equal(String(input), "/csrf-token");
    return new Response(JSON.stringify({ csrfToken: "csrf-proof" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as typeof globalThis.fetch;

  const app = await startCockpit({
    document: dom.window.document,
    fetch: fetcher,
    startSubscription: false,
    isMobile: () => false,
  });

  assert.ok(app.protocol.client);
  assert.ok(app.projection);
  assert.ok(app.reconciler);
  assert.equal(dom.window.document.querySelectorAll(".cockpit").length, 1);
  assert.equal(app.shell.element.isConnected, true);
  app.stop();
});

test("empty-session reconciliation fetches capability and enables declared managed spawn", async () => {
  const dom = new JSDOM("<!doctype html><body><main data-patchbay-cockpit></main></body>");
  const capability = spawnCapability([["uat-logical-target", "uat-project-context"]]);
  let diagnosticsCalls = 0;
  const protocol = protocolStub({
    async queryDiagnostics(_request: QueryDiagnosticsRequest) {
      diagnosticsCalls += 1;
      return adapterCapabilityResponse("pi", capability, BigInt(diagnosticsCalls));
    },
  });
  const app = composeCockpit(
    dom.window.document,
    dom.window.document.querySelector<HTMLElement>("[data-patchbay-cockpit]")!,
    DOMAIN,
    protocol,
    {
      idFactory: () => "empty-session",
      startSubscription: false,
      isMobile: () => false,
      fetcher: globalThis.fetch,
      refreshCsrfToken: async () => "csrf-proof",
    },
  );

  await waitForCondition(() => diagnosticsCalls === 1);
  assert.equal(app.projection.model.sessions.size, 0);
  assert.equal(app.projection.model.adapters.get("pi")?.status?.capability, capability);
  let spawn = app.shell.element.querySelector<HTMLButtonElement>(
    '.sidebar__actions [aria-label="Spawn uat-logical-target on pi"]',
  );
  assert.ok(spawn);
  assert.equal(spawn.disabled, false);
  const resources = [...app.shell.element.querySelectorAll<HTMLButtonElement>(
    '[aria-label="Resources unavailable — no operational-resource adapter is attached"]',
  )];
  assert.equal(resources.length, 2);
  assert.equal(resources.every((button) => button.disabled), true);
  assert.equal(resources.every((button) => button.getAttribute("aria-label") ===
    "Resources unavailable — no operational-resource adapter is attached"), true);

  await app.reconciler.reconcileNow(DOMAIN);
  await waitForCondition(() => diagnosticsCalls === 2);
  assert.ok(diagnosticsCalls <= 2, "startup plus one explicit reconciliation stay bounded");
  assert.equal(app.projection.model.adapters.get("pi")?.status?.capability, capability);
  spawn = app.shell.element.querySelector<HTMLButtonElement>(
    '.sidebar__actions [aria-label="Spawn uat-logical-target on pi"]',
  );
  assert.ok(spawn);
  assert.equal(spawn.disabled, false);
  app.stop();
});

test("empty-session adapter without capability keeps spawn canonically disabled", async () => {
  const dom = new JSDOM("<!doctype html><body><main data-patchbay-cockpit></main></body>");
  let diagnosticsCalls = 0;
  const app = composeCockpit(
    dom.window.document,
    dom.window.document.querySelector<HTMLElement>("[data-patchbay-cockpit]")!,
    DOMAIN,
    protocolStub({
      async queryDiagnostics(_request: QueryDiagnosticsRequest) {
        diagnosticsCalls += 1;
        return adapterCapabilityResponse("pi", undefined, 1n);
      },
    }),
    {
      idFactory: () => "capability-missing",
      startSubscription: false,
      isMobile: () => false,
      fetcher: globalThis.fetch,
      refreshCsrfToken: async () => "csrf-proof",
    },
  );

  await waitForCondition(() => diagnosticsCalls === 1);
  const spawn = app.shell.element.querySelector<HTMLButtonElement>(".sidebar__actions button")!;
  assert.equal(spawn.disabled, true);
  assert.equal(spawn.getAttribute("aria-label"), "Adapter capability is unavailable.");
  app.stop();
});

test("an unauthenticated startup renders login and proceeds after successful authentication", async () => {
  const dom = new JSDOM(`<!doctype html>
    <meta name="patchbay-authority-domain" content="default">
    <main data-patchbay-cockpit></main>`);
  let csrfRequests = 0;
  let loginRequests = 0;
  const fetcher = (async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input) === "/csrf-token") {
      csrfRequests += 1;
      return csrfRequests === 1
        ? new Response(JSON.stringify({ error: "unauthenticated" }), { status: 401 })
        : new Response(JSON.stringify({ csrfToken: "csrf-after-login" }), {
            status: 200,
            headers: { "content-type": "application/json" },
          });
    }
    assert.equal(String(input), "/login");
    assert.equal(init?.method, "POST");
    loginRequests += 1;
    const body = JSON.parse(String(init?.body)) as { password: string };
    assert.deepEqual(body, { password: "correct-password" });
    return loginRequests === 1
      ? new Response(JSON.stringify({ error: "invalid_credentials" }), {
          status: 401,
          headers: { "content-type": "application/json" },
        })
      : new Response(JSON.stringify({ csrfToken: "login-token" }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
  }) as typeof globalThis.fetch;

  const starting = startCockpit({
    document: dom.window.document,
    fetch: fetcher,
    startSubscription: false,
    isMobile: () => false,
  });
  const form = await waitForElement<HTMLFormElement>(dom, ".login-form");
  assert.equal(dom.window.document.querySelector('input[name="actorId"]'), null);
  const password = dom.window.document.querySelector<HTMLInputElement>('input[name="password"]')!;
  password.value = "correct-password";
  form.dispatchEvent(new dom.window.Event("submit", { bubbles: true, cancelable: true }));

  const error = await waitForElement<HTMLElement>(dom, ".login-form__error:not([hidden])");
  assert.match(error.textContent ?? "", /invalid_credentials/);
  assert.equal(dom.window.document.querySelectorAll(".cockpit").length, 0);

  form.dispatchEvent(new dom.window.Event("submit", { bubbles: true, cancelable: true }));
  const app = await starting;
  assert.equal(csrfRequests, 2);
  assert.equal(loginRequests, 2);
  assert.equal(dom.window.document.querySelectorAll(".cockpit").length, 1);
  app.stop();
});

test("composition submission builder emits a boundary-valid instruct Operation", () => {
  const operation = buildInstructOperation(
    DOMAIN,
    session(),
    "Run the verification suite",
    { commandId: "command-browser-1", idempotencyKey: "idem-browser-1" },
  );

  assert.equal(operation.kind, OperationKind.INSTRUCT);
  assert.equal(operation.commandId?.value, "command-browser-1");
  assert.equal(operation.authorityDomainId?.value, DOMAIN.value);
  assert.ok(operation.sender, "the web server replaces this untrusted sender envelope");
  assert.equal(operation.targetScope?.adapterId?.value, "pi");
  assert.equal(operation.targetScope?.runtimeSessionId?.value, "session-1");
  assert.equal(operation.targetScope?.sessionGeneration?.value, 1n);
  assert.equal(operation.idempotencyKey, "idem-browser-1");
  assert.equal(operation.payload?.contentType, PayloadContentType.TEXT_UTF8);
  assert.equal(new TextDecoder().decode(operation.payload?.payload), "Run the verification suite");
});

test("fresh and restart actions derive the declared target shape and exact adapter payloads", () => {
  const capability = spawnCapability([
    ["spawn-fresh", "project-fresh"],
    ["logical-1", "project-restart"],
  ]);
  const fresh = buildFreshSpawnOperation(
    DOMAIN,
    "pi",
    capability,
    { idempotencyKey: "spawn-fresh-key" },
    "spawn-fresh",
  );
  const restart = buildRestartOperation(DOMAIN, session(), capability, {
    commandId: "spawn-restart",
    idempotencyKey: "spawn-restart-key",
  });
  assert.equal(fresh.kind, OperationKind.SPAWN);
  assert.equal(restart.kind, OperationKind.SPAWN);
  assert.equal(fresh.commandId?.value, "spawn-fresh");
  assert.equal(fresh.targetScope?.adapterId?.value, "pi");
  assert.equal(restart.targetScope?.adapterId?.value, "pi");
  const freshRequest = fromBinary(SpawnRequestSchema, fresh.payload!.payload);
  const restartRequest = fromBinary(SpawnRequestSchema, restart.payload!.payload);
  assert.equal(freshRequest.intent.case, "fresh");
  assert.equal(restartRequest.intent.case, "continuation");
  if (restartRequest.intent.case !== "continuation") assert.fail("continuation intent expected");
  assert.equal(restartRequest.intent.value.prior?.logicalTargetId?.value, "logical-1");
  assert.equal(restartRequest.intent.value.prior?.externalRuntime?.generation?.value, 1n);
  assert.equal(freshRequest.targetSpec?.shape, "pi-rpc");
  assert.equal(restartRequest.targetSpec?.shape, "pi-rpc");
  assert.equal(
    fromBinary(PiSpawnTargetSpecSchema, freshRequest.targetSpec!.adapterPayload!.payload).projectContextRef,
    "project-fresh",
  );
  const restartTarget = fromBinary(
    PiSpawnTargetSpecSchema,
    restartRequest.targetSpec!.adapterPayload!.payload,
  );
  assert.equal(restartTarget.projectContextRef, "project-restart");
  assert.equal(restartTarget.continuationMode, PiContinuationMode.REQUIRE_RESUME);
});

test("browser build emits a servable HTML entry and bundled module", async () => {
  const html = await readFile(new URL("../index.html", import.meta.url), "utf8");
  const bundle = await readFile(new URL("../assets/cockpit.js", import.meta.url), "utf8");

  assert.match(html, /data-patchbay-cockpit/);
  assert.match(html, /\/assets\/cockpit\.js/);
  assert.match(bundle, /startCockpit/);
});

async function waitForElement<T extends Element>(dom: JSDOM, selector: string): Promise<T> {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const element = dom.window.document.querySelector<T>(selector);
    if (element) return element;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error(`element did not appear: ${selector}`);
}

async function waitForCondition(condition: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    if (condition()) return;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error("condition did not become true");
}

function protocolStub(overrides: {
  queryDiagnostics(request: QueryDiagnosticsRequest): Promise<ReturnType<typeof adapterCapabilityResponse>>;
}): ProtocolClient {
  const client = {
    ...overrides,
    async loadSnapshot(request: LoadSnapshotRequest) {
      return emptySnapshotResponse(request.viewKind);
    },
    async *subscribe() {
      // Empty operator-visible prefix: adapter registration is intentionally
      // discovered through the diagnostics projection, not fabricated here.
    },
  };
  return { client, transport: {} } as unknown as ProtocolClient;
}

function emptySnapshotResponse(viewKind: SnapshotViewKind) {
  const eventId = create(EventIdSchema, {
    authorityDomainId: DOMAIN,
    lsn: create(LsnSchema, { value: 1n }),
  });
  const coreGeneration = create(GenerationSchema, { value: 1n });
  const snapshotPayload = viewKind === SnapshotViewKind.SESSION
    ? toBinary(SessionSnapshotSchema, create(SessionSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: eventId.lsn,
        coreGeneration,
      }))
    : toBinary(ResourceSnapshotSchema, create(ResourceSnapshotSchema, {
        authorityDomainId: DOMAIN,
        snapshotLsn: eventId.lsn,
        coreGeneration,
      }));
  return create(LoadSnapshotResponseSchema, {
    present: true,
    eventId,
    viewKind,
    snapshotPayload,
  });
}

function adapterCapabilityResponse(
  adapterId: string,
  capability: ReturnType<typeof spawnCapability> | undefined,
  asOfLsn: bigint,
) {
  return create(QueryDiagnosticsResponseSchema, {
    submission: create(SubmissionResultSchema, {
      outcome: SubmissionOutcome.ACCEPTED,
      operationState: OperationState.COMPLETED,
    }),
    resultEventId: create(EventIdSchema, {
      authorityDomainId: DOMAIN,
      lsn: create(LsnSchema, { value: asOfLsn + 1n }),
    }),
    asOfLsn: create(LsnSchema, { value: asOfLsn }),
    result: {
      case: "adapters",
      value: create(AdapterStatusPageSchema, {
        adapters: [create(AdapterStatusSchema, {
          adapterId: create(AdapterIdSchema, { value: adapterId }),
          capability,
        })],
      }),
    },
  });
}

function spawnCapability(targets: readonly (readonly [string, string])[]) {
  return create(AdapterCapabilitySummarySchema, {
    supportedOperationKinds: [OperationKind.SPAWN],
    supportedTargetSpecShapes: ["pi-rpc"],
    sessionReplacementSupport: true,
    managedSpawnTargets: targets.map(([logicalTargetId, projectContextRef]) => {
      const payload = (continuationMode: PiContinuationMode) => create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: "patchbay.PiSpawnTargetSpec.v1",
        payload: toBinary(PiSpawnTargetSpecSchema, create(PiSpawnTargetSpecSchema, {
          projectContextRef,
          continuationMode,
        })),
      });
      return create(ManagedSpawnTargetCapabilitySchema, {
        logicalTargetId: create(LogicalTargetIdSchema, { value: logicalTargetId }),
        targetSpecShape: "pi-rpc",
        freshAdapterPayload: payload(PiContinuationMode.UNSPECIFIED),
        continuationAdapterPayload: payload(PiContinuationMode.REQUIRE_RESUME),
      });
    }),
  });
}

function session(): SessionView {
  return {
    identity: {
      adapterId: "pi",
      deploymentScope: "laptop",
      runtimeSessionId: "session-1",
      generation: 1n,
    },
    logicalTargetId: "logical-1",
    label: { project: "patchbay", name: "core" },
    connectivity: SessionConnectivityState.LIVE,
    activity: SessionActivityState.IDLE,
    needsYou: false,
    lastLsn: 1n,
    tombstoned: false,
    reconciled: true,
  };
}

test("expired session (403 on CSRF token) falls back to login, not startup failure", async () => {
  const dom = new JSDOM(`<!doctype html>
    <meta name="patchbay-authority-domain" content="default">
    <main data-patchbay-cockpit></main>`, { url: "https://localhost/" });
  let csrfRequests = 0;
  let loginRequests = 0;
  const fetcher = (async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input) === "/csrf-token") {
      csrfRequests += 1;
      return csrfRequests === 1
        ? new Response(JSON.stringify({ error: "session_expired" }), { status: 403 })
        : new Response(JSON.stringify({ csrfToken: "csrf-after-login" }), {
            status: 200,
            headers: { "content-type": "application/json" },
          });
    }
    assert.equal(String(input), "/login");
    loginRequests += 1;
    return new Response(JSON.stringify({ csrfToken: "login-token" }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }) as typeof globalThis.fetch;

  const starting = startCockpit({
    document: dom.window.document,
    fetch: fetcher,
    startSubscription: false,
    isMobile: () => false,
  });
  const form = await waitForElement<HTMLFormElement>(dom, ".login-form");
  const password = dom.window.document.querySelector<HTMLInputElement>('input[name="password"]')!;
  password.value = "correct-password";
  form.dispatchEvent(new dom.window.Event("submit", { bubbles: true, cancelable: true }));

  const app = await starting;
  assert.equal(csrfRequests, 2);
  assert.equal(loginRequests, 1);
  assert.equal(dom.window.document.querySelectorAll(".failure-banner").length, 0);
  assert.equal(dom.window.document.querySelectorAll(".cockpit").length, 1);
  app.stop();
});
