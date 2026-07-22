import { create, toBinary } from "@bufbuild/protobuf";
import { Code, ConnectError, createClient, type CallOptions } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import {
  ControlService,
  LoadSnapshotResponseSchema,
  PrincipalCredentialSchema,
  SubmissionOutcome,
  SubmissionResultSchema,
  SubmitRequestSchema,
  SubscribeEventSchema,
  VerifyOperatorPasswordResultSchema,
  EnrollControlSurfacePrincipalResultSchema,
  RevokeOperatorSessionResultSchema,
} from "@patchbay/contracts";
import assert from "node:assert/strict";
import type { AddressInfo } from "node:net";
import test from "node:test";

import { CorePrincipalStore, type CoreClient } from "../src/core-client.js";
import {
  CSRF_HEADER_NAME,
  SESSION_COOKIE_NAME,
} from "../src/middleware/csrf-auth.js";
import { buildApp, type WebServerConfig } from "../src/main.js";
import { hashPassword, SessionStore } from "../src/sessions.js";

interface CoreCall {
  method: "submit" | "subscribe" | "loadSnapshot" | "verifyOperatorPassword" | "revokeOperatorSession";
  request: unknown;
  headers: Headers;
}

const operatorActorId = "operator-from-server-record";
const operatorPasswordHash = await hashPassword("integration-password", Buffer.alloc(16, 3));
const config: WebServerConfig = {
  coreAddr: "http://127.0.0.1:50051",
  coreSecret: "core-principal-secret",
  bindHost: "127.0.0.1",
  bindPort: 3000,
  operatorId: operatorActorId,
  operatorPasswordHash,
};

test("login verifies the core-owned operator record and installs the web principal", async () => {
  const fixture = makeFixture();

  const invalid = await fixture.app.inject({
    method: "POST",
    url: "/login",
    payload: { password: "wrong" },
  });
  assert.equal(invalid.statusCode, 401);

  const login = await fixture.app.inject({
    method: "POST",
    url: "/login",
    payload: { password: "integration-password" },
  });
  assert.equal(login.statusCode, 200);
  assert.equal(fixture.app.corePrincipals.get()?.principalId, "web-principal");
  assert.equal(
    fixture.calls.filter((call) => call.method === "verifyOperatorPassword").length,
    2,
  );
  const setCookie = login.headers["set-cookie"];
  const cookie = (Array.isArray(setCookie) ? setCookie[0] : setCookie)?.split(";", 1)[0];
  assert.ok(cookie);
  const command = await fixture.app.inject({
    method: "POST",
    url: "/patchbay.ControlService/Submit",
    headers: {
      "content-type": "application/grpc-web+proto",
      cookie,
      [CSRF_HEADER_NAME]: login.json<{ csrfToken: string }>().csrfToken,
    },
    payload: submitFrame("browser-claim"),
  });
  assert.equal(command.statusCode, 200);
  const submitted = fixture.calls.find((call) => call.method === "submit");
  assert.equal(
    submitted?.headers.get("x-patchbay-operator-session-id"),
    "core-issued-session",
  );
  await fixture.app.close();
});

test("CsrfRejectsUnauthenticated: no cookie returns 401 before a core call", async () => {
  const fixture = makeFixture();
  const response = await fixture.app.inject({
    method: "POST",
    url: "/patchbay.ControlService/Submit",
    headers: { "content-type": "application/grpc-web+proto" },
    payload: submitFrame("browser-claim"),
  });

  assert.equal(response.statusCode, 401);
  assert.equal(fixture.calls.length, 0);
  await fixture.app.close();
});

test("CsrfRejectsMissingProof: missing or wrong proof returns 403 before a core call", async () => {
  const fixture = makeFixture();
  const session = fixture.sessions.create(operatorActorId);
  const cookie = `${SESSION_COOKIE_NAME}=${session.sessionId}`;
  const request = {
    method: "POST" as const,
    url: "/patchbay.ControlService/Submit",
    headers: { "content-type": "application/grpc-web+proto", cookie },
    payload: submitFrame("browser-claim"),
  };

  const missing = await fixture.app.inject(request);
  const wrong = await fixture.app.inject({
    ...request,
    headers: { ...request.headers, [CSRF_HEADER_NAME]: "wrong-proof" },
  });

  assert.equal(missing.statusCode, 403);
  assert.equal(wrong.statusCode, 403);
  assert.equal(fixture.calls.length, 0);
  await fixture.app.close();
});

test("RevokedSessionCannotCommand: a recognized revoked session returns 403 before a core call", async () => {
  const fixture = makeFixture();
  const session = fixture.sessions.create(operatorActorId);
  fixture.sessions.revoke(session.sessionId);

  const response = await fixture.app.inject({
    method: "POST",
    url: "/patchbay.ControlService/Submit",
    headers: {
      "content-type": "application/grpc-web+proto",
      cookie: `${SESSION_COOKIE_NAME}=${session.sessionId}`,
      [CSRF_HEADER_NAME]: session.csrfSecret,
    },
    payload: submitFrame("browser-claim"),
  });

  assert.equal(response.statusCode, 403);
  assert.equal(fixture.sessions.lookup(session.sessionId)?.status, "revoked");
  assert.equal(fixture.sessions.size, 1);
  assert.equal(fixture.calls.length, 0);
  await fixture.app.close();
});

test("browser_local_state_not_authority: Connect-Web Submit forwards and stamps server identity", async () => {
  const fixture = makeFixture();
  const session = fixture.sessions.create(operatorActorId, "core-issued-session");
  const { client, close } = await listen(fixture);

  try {
    const response = await client.submit(
      { operation: { sender: { actorId: { value: "forged-browser-actor" } } } },
      {
        headers: {
          cookie: `${SESSION_COOKIE_NAME}=${session.sessionId}`,
          [CSRF_HEADER_NAME]: session.csrfSecret,
          "x-patchbay-operator-id": "forged-browser-header",
        },
      },
    );
    assert.equal(response.outcome, SubmissionOutcome.ACCEPTED);
    assert.equal(fixture.calls.length, 1);

    const call = fixture.calls[0];
    assert.equal(call.headers.get("x-patchbay-core-secret"), config.coreSecret);
    assert.equal(call.headers.get("x-patchbay-principal-id"), "web-principal");
    assert.equal(call.headers.get("x-patchbay-principal-secret"), "web-principal-secret");
    assert.equal(call.headers.get("x-patchbay-operator-id"), operatorActorId);
    assert.equal(call.headers.get("x-patchbay-operator-session-id"), "core-issued-session");
    const forwarded = call.request as {
      operation?: { sender?: { actorId?: { value: string } } };
    };
    assert.equal(forwarded.operation?.sender?.actorId?.value, operatorActorId);
  } finally {
    await close();
  }
});

test("Connect-Web Subscribe streams frames and reconnects from the supplied cursor", async () => {
  const fixture = makeFixture();
  const session = fixture.sessions.create(operatorActorId, "core-issued-session");
  const { client, close } = await listen(fixture);
  const headers = { cookie: `${SESSION_COOKIE_NAME}=${session.sessionId}` };

  try {
    const first = [];
    for await (const event of client.subscribe(
      { authorityDomainId: { value: "default" }, cursor: { value: 0n } },
      { headers },
    )) {
      first.push(event.eventId?.lsn?.value);
    }
    const resumed = [];
    for await (const event of client.subscribe(
      { authorityDomainId: { value: "default" }, cursor: { value: 1n } },
      { headers },
    )) {
      resumed.push(event.eventId?.lsn?.value);
    }

    assert.deepEqual(first, [1n]);
    assert.deepEqual(resumed, [2n]);
    const cursors = fixture.calls
      .filter((call) => call.method === "subscribe")
      .map((call) => (call.request as { cursor?: { value: bigint } }).cursor?.value);
    assert.deepEqual(cursors, [0n, 1n]);
  } finally {
    await close();
  }
});

test("CSRF token issuance and LoadSnapshot are authenticated reads without CSRF", async () => {
  const fixture = makeFixture();
  const session = fixture.sessions.create(operatorActorId, "core-issued-session");
  const cookie = `${SESSION_COOKIE_NAME}=${session.sessionId}`;

  const noSession = await fixture.app.inject({ method: "GET", url: "/csrf-token" });
  assert.equal(noSession.statusCode, 401);
  const token = await fixture.app.inject({ method: "GET", url: "/csrf-token", headers: { cookie } });
  assert.equal(token.statusCode, 200);
  assert.deepEqual(token.json(), { csrfToken: session.csrfSecret });

  const { client, close } = await listen(fixture);
  try {
    const snapshot = await client.loadSnapshot(
      { authorityDomainId: { value: "default" } },
      { headers: { cookie } },
    );
    assert.equal(snapshot.present, false);
  } finally {
    await close();
  }
});

function makeFixture(): {
  app: ReturnType<typeof buildApp>;
  sessions: SessionStore;
  calls: CoreCall[];
} {
  const calls: CoreCall[] = [];
  const coreClient: CoreClient = {
    async submit(request, options) {
      calls.push({ method: "submit", request, headers: callHeaders(options) });
      return create(SubmissionResultSchema, { outcome: SubmissionOutcome.ACCEPTED });
    },
    async *subscribe(request, options) {
      calls.push({ method: "subscribe", request, headers: callHeaders(options) });
      const nextLsn = (request.cursor?.value ?? 0n) + 1n;
      yield create(SubscribeEventSchema, {
        eventId: { authorityDomainId: { value: "default" }, lsn: { value: nextLsn } },
      });
    },
    async loadSnapshot(request, options) {
      calls.push({ method: "loadSnapshot", request, headers: callHeaders(options) });
      return create(LoadSnapshotResponseSchema, {
        present: false,
        snapshotPayload: new Uint8Array(),
      });
    },
    async verifyOperatorPassword(request, options) {
      calls.push({
        method: "verifyOperatorPassword",
        request,
        headers: callHeaders(options),
      });
      if (request.password !== "integration-password") {
        throw new ConnectError("invalid operator credentials", Code.Unauthenticated);
      }
      return create(VerifyOperatorPasswordResultSchema, {
        operatorSessionId: { value: "core-issued-session" },
        principal: {
          principalId: "web-principal",
          secret: "web-principal-secret",
          operatorActorId: { value: operatorActorId },
          endpointId: { value: "patchbay-web-server" },
          deviceId: { value: "web-device" },
          endpointGeneration: { value: 1n },
        },
      });
    },
    async revokeOperatorSession(_request, options) {
      calls.push({
        method: "revokeOperatorSession",
        request: {},
        headers: callHeaders(options),
      });
      return create(RevokeOperatorSessionResultSchema, { revoked: true });
    },
    async enrollControlSurfacePrincipal() {
      return create(EnrollControlSurfacePrincipalResultSchema);
    },
  };
  const sessions = new SessionStore();
  const corePrincipals = new CorePrincipalStore();
  corePrincipals.set(create(PrincipalCredentialSchema, {
    principalId: "web-principal",
    secret: "web-principal-secret",
    operatorActorId: { value: operatorActorId },
    endpointId: { value: "patchbay-web-server" },
    deviceId: { value: "web-device" },
    endpointGeneration: { value: 1n },
  }));
  return {
    app: buildApp({ config, coreClient, corePrincipals, sessions, logger: false }),
    sessions,
    calls,
  };
}

function callHeaders(options: CallOptions | undefined): Headers {
  return new Headers(options?.headers);
}

function submitFrame(browserActorId: string): Buffer {
  const request = create(SubmitRequestSchema, {
    operation: { sender: { actorId: { value: browserActorId } } },
  });
  const payload = toBinary(SubmitRequestSchema, request);
  const header = Buffer.alloc(5);
  header.writeUInt32BE(payload.length, 1);
  return Buffer.concat([header, payload]);
}

async function listen(fixture: ReturnType<typeof makeFixture>): Promise<{
  client: ReturnType<typeof createClient<typeof ControlService>>;
  close: () => Promise<void>;
}> {
  await fixture.app.listen({ host: "127.0.0.1", port: 0 });
  const address = fixture.app.server.address() as AddressInfo;
  const transport = createGrpcWebTransport({ baseUrl: `http://127.0.0.1:${address.port}` });
  return {
    client: createClient(ControlService, transport),
    close: () => fixture.app.close(),
  };
}
