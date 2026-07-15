import { create, toBinary } from "@bufbuild/protobuf";
import { createClient, type CallOptions } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import {
  ControlService,
  LoadSnapshotResponseSchema,
  SubmissionOutcome,
  SubmissionResultSchema,
  SubmitRequestSchema,
  SubscribeEventSchema,
} from "@patchbay/contracts";
import assert from "node:assert/strict";
import type { AddressInfo } from "node:net";
import test from "node:test";

import type { CoreClient } from "../src/core-client.js";
import {
  CSRF_HEADER_NAME,
  SESSION_COOKIE_NAME,
} from "../src/middleware/csrf-auth.js";
import { buildApp, type WebServerConfig } from "../src/main.js";
import { hashPassword, SessionStore } from "../src/sessions.js";

interface CoreCall {
  method: "submit" | "subscribe" | "loadSnapshot";
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
  const session = fixture.sessions.create(operatorActorId);
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
    assert.equal(call.headers.get("x-patchbay-operator-id"), operatorActorId);
    assert.equal(call.headers.get("x-patchbay-operator-session-id"), session.sessionId);
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
  const session = fixture.sessions.create(operatorActorId);
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
  const session = fixture.sessions.create(operatorActorId);
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
  };
  const sessions = new SessionStore();
  return { app: buildApp({ config, coreClient, sessions }), sessions, calls };
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
