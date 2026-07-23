import assert from "node:assert/strict";
import test from "node:test";

import { csrfInterceptor } from "../src/domain/protocol-client.js";

// The web-server's CSRF guard requires the proof only on Submit (Subscribe and
// LoadSnapshot are read-only and exempt). The interceptor must match the
// Connect method name as declared in the proto — "Submit", PascalCase — or the
// header is never sent and the guard rejects with 403 (a mock-vs-real gap the
// fetch-mocked tests could not catch).

function fakeRequest(methodName: string) {
  return {
    method: { name: methodName },
    header: new Headers(),
  };
}

test("csrfInterceptor attaches the proof to Submit (proto method name)", async () => {
  const request = fakeRequest("Submit");
  const next = async (req: typeof request) => req;
  const interceptor = csrfInterceptor(() => "proof-token");

  // @ts-expect-error — the fake request is structurally sufficient
  await interceptor(next)(request);

  assert.equal(request.header.get("x-patchbay-csrf"), "proof-token");
});

test("csrfInterceptor does not attach the proof to read-only calls", async () => {
  for (const methodName of ["Subscribe", "LoadSnapshot"]) {
    const request = fakeRequest(methodName);
    const next = async (req: typeof request) => req;
    const interceptor = csrfInterceptor(() => "proof-token");

    // @ts-expect-error — the fake request is structurally sufficient
    await interceptor(next)(request);

    assert.equal(request.header.get("x-patchbay-csrf"), null);
  }
});

test("csrfInterceptor throws when Submit has no token", async () => {
  const request = fakeRequest("Submit");
  const next = async (req: typeof request) => req;
  const interceptor = csrfInterceptor(() => undefined);

  await assert.rejects(
    // @ts-expect-error — the fake request is structurally sufficient
    interceptor(next)(request),
    /Submit requires a session-bound CSRF token/,
  );
});
