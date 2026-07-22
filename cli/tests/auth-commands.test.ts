import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
  ActorIdSchema,
  BootstrapResultSchema,
  DeviceIdSchema,
  EndpointIdSchema,
  GenerationSchema,
  GrantIdSchema,
  OperatorSessionIdSchema,
  PrincipalCredentialSchema,
  VerifyOperatorPasswordResultSchema,
} from "@patchbay/contracts";
import { loginCommand } from "../src/commands/login.js";
import { logoutCommand } from "../src/commands/logout.js";
import { setupCommand } from "../src/commands/setup.js";
import { CredentialStore } from "../src/credentials.js";
import { BEARER_SECRET, captureOutput, DOMAIN } from "./helpers.js";

const operator = "operator-primary";

function principal(secret = BEARER_SECRET) {
  return create(PrincipalCredentialSchema, {
    principalId: "principal-returned-once",
    secret,
    operatorActorId: create(ActorIdSchema, { value: operator }),
    endpointId: create(EndpointIdSchema, { value: "cli-endpoint" }),
    deviceId: create(DeviceIdSchema, { value: "cli-device" }),
    endpointGeneration: create(GenerationSchema, { value: 1n }),
  });
}

async function temporaryStore(): Promise<CredentialStore> {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-auth-"));
  return new CredentialStore(join(directory, "credentials.json"));
}

test("setup hashes locally, calls bootstrap, and never logs bearer or setup secrets", async () => {
  const store = await temporaryStore();
  const output = captureOutput();
  let request: Record<string, unknown> | undefined;
  const client = {
    async bootstrapOperator(input: Record<string, unknown>) {
      request = input;
      return create(BootstrapResultSchema, {
        grantId: create(GrantIdSchema, { value: "bootstrap-grant" }),
        sessionId: create(OperatorSessionIdSchema, { value: "setup-session" }),
        principal: principal(),
      });
    },
  };

  assert.equal(
    await setupCommand(
      client as never,
      store,
      DOMAIN,
      {
        setupSecret: "one-time-setup-secret",
        operatorActorId: operator,
        password: "correct horse battery staple",
        endpointId: "cli-endpoint",
        deviceId: "cli-device",
      },
      output,
    ),
    0,
  );

  assert.match(String(request?.["passwordHash"]), /^scrypt\$[A-Za-z0-9_-]+\$[A-Za-z0-9_-]+$/);
  assert.notEqual(request?.["passwordHash"], "correct horse battery staple");
  assert.equal((await store.readRequired()).sessionId, "setup-session");
  const logs = [...output.out, ...output.err].join("\n");
  assert.doesNotMatch(logs, /one-time-setup-secret|correct horse battery staple/);
  assert.equal(logs.includes(BEARER_SECRET), false);
});

test("login enrolls a fresh CLI principal and stores the core-issued session", async () => {
  const store = await temporaryStore();
  const output = captureOutput();
  let request: Record<string, unknown> | undefined;
  const client = {
    async verifyOperatorPassword(input: Record<string, unknown>) {
      request = input;
      return create(VerifyOperatorPasswordResultSchema, {
        operatorSessionId: create(OperatorSessionIdSchema, { value: "login-session" }),
        principal: principal("fresh-bearer-secret"),
      });
    },
  };

  assert.equal(
    await loginCommand(
      client as never,
      store,
      DOMAIN,
      {
        operatorActorId: operator,
        password: "password",
        endpointId: "fresh-cli-endpoint",
        deviceId: "cli-device",
      },
      output,
    ),
    0,
  );

  const enrollment = request?.["principal"] as { endpointId?: { value?: string } };
  assert.equal(enrollment.endpointId?.value, "fresh-cli-endpoint");
  assert.equal((await store.readRequired()).sessionId, "login-session");
  assert.equal([...output.out, ...output.err].join("\n").includes("fresh-bearer-secret"), false);
});

test("logout revokes before deleting local credentials", async () => {
  const store = await temporaryStore();
  await store.write({
    version: 1,
    authorityDomainId: DOMAIN,
    operatorActorId: operator,
    sessionId: "session",
    principal: {
      principalId: "principal",
      secret: "secret",
      operatorActorId: operator,
      endpointId: "endpoint",
      deviceId: "device",
      endpointGeneration: "1",
    },
  });
  let revoked = false;
  const output = captureOutput();

  assert.equal(
    await logoutCommand(
      {
        async revokeOperatorSession() {
          revoked = true;
          return { revoked: true };
        },
      } as never,
      store,
      output,
    ),
    0,
  );

  assert.equal(revoked, true);
  assert.equal(await store.read(), null);
});
