import assert from "node:assert/strict";
import { mkdtemp, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { authInterceptor, AUTH_HEADERS } from "../src/auth.js";
import { assertLoopbackAdminAddress, loadConfig } from "../src/core-client.js";
import { CredentialStore } from "../src/credentials.js";
import { credentials } from "./helpers.js";

test("credential store is atomically written with owner-only permissions", async () => {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-credentials-"));
  const store = new CredentialStore(join(directory, "credentials.json"));
  const expected = credentials();

  await store.write(expected);

  assert.equal((await stat(store.path)).mode & 0o777, 0o600);
  assert.deepEqual(await store.readRequired(), expected);
  assert.equal((await stat(directory)).mode & 0o777, 0o700);
});

test("auth interceptor reads the store and adds all four verifier headers", async () => {
  const expected = credentials();
  let reads = 0;
  const interceptor = authInterceptor({
    async readRequired() {
      reads += 1;
      return expected;
    },
  });
  const header = new Headers();
  let forwarded = false;
  const handler = interceptor(
    (async (request: { header: Headers }) => {
      forwarded = true;
      assert.equal(request.header.get(AUTH_HEADERS.principalId), expected.principal.principalId);
      assert.equal(request.header.get(AUTH_HEADERS.principalSecret), expected.principal.secret);
      assert.equal(request.header.get(AUTH_HEADERS.operatorId), expected.operatorActorId);
      assert.equal(request.header.get(AUTH_HEADERS.operatorSessionId), expected.sessionId);
      return {};
    }) as never,
  );

  await handler({ header } as never);

  assert.equal(reads, 1);
  assert.equal(forwarded, true);
  assert.equal([...header.keys()].filter((name) => name.startsWith("x-patchbay-")).length, 4);
});

test("config fails closed without a core secret", () => {
  assert.throws(() => loadConfig({}), /PATCHBAY_CORE_SECRET/);
  assert.equal(loadConfig({ PATCHBAY_CORE_SECRET: "configured" }).authorityDomainId, "default");
});

test("admin client address is constrained to the local console", () => {
  assert.doesNotThrow(() => assertLoopbackAdminAddress("http://127.0.0.1:50052"));
  assert.doesNotThrow(() => assertLoopbackAdminAddress("http://localhost:50052"));
  assert.doesNotThrow(() => assertLoopbackAdminAddress("http://[::1]:50052"));
  assert.throws(
    () => assertLoopbackAdminAddress("http://192.168.1.10:50052"),
    /loopback/,
  );
});
