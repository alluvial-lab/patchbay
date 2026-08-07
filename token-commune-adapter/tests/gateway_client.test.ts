import assert from "node:assert/strict";
import test from "node:test";
import { createHttpTokenCommuneGatewayClient, GATEWAY_ENDPOINTS, GatewayClientError } from "../src/gateway_client.js";
import type { GatewayCredential } from "../src/credential.js";

const credential: GatewayCredential = {
  apply(headers) { headers.set("Authorization", "Bearer member-key"); },
  redactionSecrets: () => ["member-key"],
  dispose() {},
};
const capacity = [{
  window: "5h", usedFraction: null, usedUnits: null, limitUnits: 100, resetsAt: null,
  source: "usage_endpoint", observedAt: 1_786_060_800_000,
}];
const poolFingerprint = { state: "unknown", templateSource: "compiled", detail: "no capture yet", since: null, diff: null };
const rawFingerprint = { templateSource: "compiled", template: { provider: "anthropic" }, lastCapture: null, lastCaptureAt: null, lastDiff: null, hold: null };
const fixtures: Record<string, unknown> = {
  [GATEWAY_ENDPOINTS.status]: {
    ok: true, anthropicHealth: { state: "fresh" },
    contributions: [{ contributionId: "contribution-1", provider: "anthropic", readings: capacity }],
  },
  [GATEWAY_ENDPOINTS.pool]: { providers: [
    { provider: "anthropic", declaredShare: 0.5, health: { state: "fresh" }, capacity, fingerprint: poolFingerprint },
    { provider: "anthropic", declaredShare: 0.25, health: { state: "exhausted", exhaustedUntil: "2026-08-06T00:00:00Z" }, capacity, fingerprint: poolFingerprint },
    { provider: "anthropic", declaredShare: 0.25, health: { state: "auth_broken", reason: "revoked credential" }, capacity, fingerprint: poolFingerprint },
  ] },
  [GATEWAY_ENDPOINTS.me]: { member: "Ada", draw: [{ provider: "anthropic", limitFraction: 0.5, fromDecree: false, consumedUnits: 5, drawUnits: null, exceeded: false, enforceable: true, resetsAt: null }] },
  [GATEWAY_ENDPOINTS.events]: { events: [{ id: "event-1", at: 1_786_060_800_000, kind: "window_exhausted", provider: "anthropic", contributionId: null, message: "window exhausted" }] },
  [GATEWAY_ENDPOINTS.fingerprints]: { anthropic: rawFingerprint, "openai-codex": { ...rawFingerprint, template: { provider: "openai-codex" } } },
  [GATEWAY_ENDPOINTS.models]: { object: "list", data: [
    "gpt-5.5", "gpt-5.3-codex-spark", "claude-sonnet-4-5", "token-commune/glm-5", "token-commune/kimi-for-coding",
  ].map((id) => ({ id, object: "model", owned_by: "token-commune", provider: id.includes("claude") ? "anthropic" : "openai-codex", surface: "codex", context_window: 200000, max_tokens: 8192, reasoning: true, available: true })) },
};

test("all gateway methods use exact authenticated GET paths and return immutable typed DTOs", async () => {
  const seen: string[] = [];
  const fetcher: typeof fetch = async (input, init) => {
    const url = new URL(typeof input === "string" ? input : input instanceof URL ? input.href : input.url);
    seen.push(url.pathname);
    assert.equal(init?.method, "GET");
    assert.equal(init?.redirect, "error");
    const headers = new Headers(init?.headers);
    assert.equal(headers.get("authorization"), "Bearer member-key");
    assert.equal(headers.get("accept"), "application/json");
    assert.equal(headers.has("x-api-key"), false);
    return Response.json(fixtures[url.pathname]);
  };
  const client = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, fetch: fetcher });
  const status = await client.getStatus();
  const pool = await client.getPool();
  const me = await client.getMe();
  const events = await client.getEvents();
  const fingerprints = await client.getFingerprints();
  const models = await client.getModels();
  assert.deepEqual(seen, Object.values(GATEWAY_ENDPOINTS));
  assert.equal(status.contributions[0]?.readings[0]?.usedFraction, null);
  assert.deepEqual(status.anthropicHealth, { state: "fresh" });
  assert.deepEqual(pool.contributions[1]?.health, {
    state: "exhausted",
    exhaustedUntil: "2026-08-06T00:00:00Z",
  }, "native exhausted-until metadata survives decoding");
  assert.deepEqual(pool.contributions[2]?.health, {
    state: "auth_broken",
    reason: "revoked credential",
  }, "native auth-broken reason survives decoding");
  assert.equal(pool.contributions.length, 3, "duplicate provider contributions remain explicit");
  assert.equal(me.reports[0]?.drawUnits, null);
  assert.equal(events.historyMode, "latest-50-no-cursor");
  assert.equal(fingerprints.codex.capturePresent, false);
  assert.deepEqual(models.models.map((model) => model.id), (fixtures[GATEWAY_ENDPOINTS.models] as any).data.map((model: any) => model.id));
  assert.ok(models.models.every((model) => model.upstreamModel === null), "missing upstream ids remain honestly nullable");
  assert.equal(models.models.some((model) => model.id.includes("gpt-5.6")), false);
  assert.equal(Object.isFrozen(models.models), true);
});

test("client rejects redirects, oversized/malformed bodies, invalid values, and redacts errors", async () => {
  const redirect = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, fetch: async () => new Response(null, { status: 302 }) });
  await assert.rejects(redirect.getStatus(), (error: unknown) => error instanceof GatewayClientError && error.kind === "http" && error.status === 302);

  const oversized = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, maxResponseBytes: 8, fetch: async () => new Response("{\"ok\":true}") });
  await assert.rejects(oversized.getStatus(), (error: unknown) => error instanceof GatewayClientError && error.kind === "invalid-response");

  const invalid = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, fetch: async () => Response.json({ ok: true, anthropicHealth: { state: "mystery" }, contributions: [], secret: "response-body-secret" }) });
  await assert.rejects(invalid.getStatus(), (error: unknown) => {
    const serialized = JSON.stringify(error);
    return error instanceof GatewayClientError && error.kind === "invalid-response" && !serialized.includes("response-body-secret") && !serialized.includes("member-key");
  });
});

test("retryable HTTP failures expose only normalized Retry-After advice", async () => {
  const cases = [
    ["120", { retryAfterMs: 120_000 }],
    ["Fri, 07 Aug 2026 12:02:00 GMT", { retryAt: "2026-08-07T12:02:00.000Z" }],
    ["member-key secret response-body", { invalid: true }],
    ["999999999999999999999999", { invalid: true }],
  ] as const;
  for (const [header, expected] of cases) {
    const client = createHttpTokenCommuneGatewayClient({
      baseUrl: new URL("https://gateway.example/"), credential,
      fetch: async () => new Response("response-body-secret", { status: 429, headers: { "Retry-After": header } }),
    });
    await assert.rejects(client.getStatus(), (error: unknown) => {
      assert.ok(error instanceof GatewayClientError);
      assert.deepEqual(error.backoff, expected);
      const serialized = JSON.stringify(error);
      return !serialized.includes("response-body-secret") && !serialized.includes("member-key secret");
    });
  }
  const notRetryable = createHttpTokenCommuneGatewayClient({
    baseUrl: new URL("https://gateway.example/"), credential,
    fetch: async () => new Response(null, { status: 400, headers: { "Retry-After": "120" } }),
  });
  await assert.rejects(notRetryable.getStatus(), (error: unknown) => error instanceof GatewayClientError && error.backoff === undefined);
});

test("transport errors cannot retain bearer credentials or thrown response-body details", async () => {
  const sentinel = "response-body-transport-sentinel";
  const client = createHttpTokenCommuneGatewayClient({
    baseUrl: new URL("https://gateway.example/"), credential,
    fetch: async () => { throw new Error(`transport failed for member-key with ${sentinel}`); },
  });
  let returned: unknown;
  try { await client.getStatus(); } catch (error) { returned = error; }
  assert.ok(returned instanceof GatewayClientError);
  assert.equal(returned.kind, "transport");
  assert.equal(returned.message, "token-commune gateway /commune/status transport");
  const serialized = `${String(returned)} ${JSON.stringify(returned)}`;
  assert.equal(serialized.includes("member-key"), false);
  assert.equal(serialized.includes(sentinel), false);
});

test("fingerprint presence projections require explicit object-or-null source fields", async () => {
  const invalid: Array<["pool" | "fingerprints", unknown]> = [];
  for (const [field, value] of [["diff", undefined], ["diff", "raw-diff"]] as const) {
    const body = structuredClone(fixtures[GATEWAY_ENDPOINTS.pool]) as any;
    if (value === undefined) delete body.providers[0].fingerprint[field];
    else body.providers[0].fingerprint[field] = value;
    invalid.push(["pool", body]);
  }
  for (const [field, value] of [
    ["lastCapture", undefined], ["lastCapture", "raw-capture"],
    ["lastDiff", undefined], ["lastDiff", 1],
  ] as const) {
    const body = structuredClone(fixtures[GATEWAY_ENDPOINTS.fingerprints]) as any;
    if (value === undefined) delete body.anthropic[field];
    else body.anthropic[field] = value;
    invalid.push(["fingerprints", body]);
  }
  for (const value of [undefined, false] as const) {
    const body = structuredClone(fixtures[GATEWAY_ENDPOINTS.fingerprints]) as any;
    body.anthropic.hold = { reason: "held", since: 1_786_060_800_000, diff: null };
    if (value === undefined) delete body.anthropic.hold.diff;
    else body.anthropic.hold.diff = value;
    invalid.push(["fingerprints", body]);
  }
  for (const [method, body] of invalid) {
    const client = createHttpTokenCommuneGatewayClient({
      baseUrl: new URL("https://gateway.example/"), credential,
      fetch: async () => Response.json(body),
    });
    await assert.rejects(
      method === "pool" ? client.getPool() : client.getFingerprints(),
      (error: unknown) => error instanceof GatewayClientError && error.kind === "invalid-response",
    );
  }
});

test("runtime decoders reject malformed arrays, health details, timestamps, fractions, and non-finite fields", async () => {
  const missingExhaustedUntil = structuredClone(fixtures[GATEWAY_ENDPOINTS.pool]) as any;
  delete missingExhaustedUntil.providers[1].health.exhaustedUntil;
  const missingAuthReason = structuredClone(fixtures[GATEWAY_ENDPOINTS.pool]) as any;
  missingAuthReason.providers[0].health = { state: "auth_broken" };
  const cases: Array<[keyof typeof GATEWAY_ENDPOINTS, unknown]> = [
    ["pool", missingExhaustedUntil],
    ["pool", missingAuthReason],
    ["status", { ok: true, anthropicHealth: { state: "fresh" }, contributions: {} }],
    ["pool", { providers: [{ provider: "anthropic", declaredShare: 2, health: { state: "fresh" }, capacity: [], fingerprint: poolFingerprint }] }],
    ["me", { member: "Ada", draw: [{ provider: "anthropic", limitFraction: 0.5, fromDecree: false, consumedUnits: 0, drawUnits: null, exceeded: false, enforceable: false, resetsAt: "tomorrow" }] }],
    ["events", { events: [{ id: "x", at: -1, kind: "member", provider: "anthropic", contributionId: null, message: "x" }] }],
    ["models", { data: [{ id: "gpt-5.5", provider: "openai-codex", surface: "codex", context_window: null, max_tokens: 1, reasoning: true, available: true }] }],
  ];
  for (const [method, body] of cases) {
    const client = createHttpTokenCommuneGatewayClient({
      baseUrl: new URL("https://gateway.example/"), credential,
      fetch: async () => Response.json(body),
    });
    const call = method === "status" ? client.getStatus()
      : method === "pool" ? client.getPool()
        : method === "me" ? client.getMe()
          : method === "events" ? client.getEvents()
            : client.getModels();
    await assert.rejects(call, (error: unknown) => error instanceof GatewayClientError && error.kind === "invalid-response");
  }
});

test("error taxonomy and abort propagation are explicit", async () => {
  for (const [status, kind] of [[401, "unauthorized"], [403, "forbidden"], [500, "http"]] as const) {
    const client = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, fetch: async () => new Response(null, { status }) });
    await assert.rejects(client.getPool(), (error: unknown) => error instanceof GatewayClientError && error.kind === kind);
  }
  let receivedSignal: AbortSignal | null | undefined;
  const client = createHttpTokenCommuneGatewayClient({ baseUrl: new URL("https://gateway.example/"), credential, fetch: async (_input, init) => {
    receivedSignal = init?.signal;
    throw new DOMException("aborted", "AbortError");
  } });
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(client.getModels(controller.signal), (error: unknown) => error instanceof GatewayClientError && error.kind === "timeout");
  assert.equal(receivedSignal?.aborted, true);

  const deadline = createHttpTokenCommuneGatewayClient({
    baseUrl: new URL("https://gateway.example/"), credential, requestTimeoutMs: 5,
    fetch: async (_input, init) => new Promise<Response>((_resolve, reject) => {
      const requestSignal = init?.signal;
      assert.ok(requestSignal);
      requestSignal.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")), { once: true });
    }),
  });
  await assert.rejects(deadline.getStatus(), (error: unknown) => error instanceof GatewayClientError && error.kind === "timeout");
});
