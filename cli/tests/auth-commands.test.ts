import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
  ActorIdSchema,
  AuthorityDomainIdSchema,
  FailureCode,
  BootstrapResultSchema,
  DeviceIdSchema,
  EndpointIdSchema,
  GenerationSchema,
  EventIdSchema,
  GrantIdSchema,
  GrantRevocationEffectSchema,
  GrantRevocationPolicy,
  RevokeGrantResultSchema,
  RevokeAllOperatorSessionsResultSchema,
  RevokeControlSurfaceResultSchema,
  OperatorSessionIdSchema,
  PrincipalCredentialSchema,
  VerifyOperatorPasswordResultSchema,
} from "@patchbay/contracts";
import { loginCommand } from "../src/commands/login.js";
import { logoutCommand } from "../src/commands/logout.js";
import { grantRevokeCommand } from "../src/commands/grant-revoke.js";
import {
  revokeAllSessionsCommand,
  revokeEndpointCommand,
  revokePrincipalCommand,
} from "../src/commands/revocation.js";
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

test("grant-revoke reports changed and idempotent outcomes without clearing credentials", async () => {
  const output = captureOutput();
  let calls = 0;
  const client = {
    async revokeGrant() {
      calls += 1;
      return create(RevokeGrantResultSchema, calls === 1 ? {
        changed: true,
        appliedPolicy: GrantRevocationPolicy.CANCEL,
        revocationEventId: create(EventIdSchema, { authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }), lsn: { value: 42n } }),
        commandEffects: [create(GrantRevocationEffectSchema, { failureCode: FailureCode.CANCELLED })],
      } : {
        alreadyRevoked: true,
        appliedPolicy: GrantRevocationPolicy.CANCEL,
      });
    },
  };
  assert.equal(await grantRevokeCommand(client as never, DOMAIN, { grantId: "grant-1", json: true }, output), 0);
  assert.deepEqual(JSON.parse(output.out[0]!), {
    grantId: "grant-1",
    changed: true,
    alreadyRevoked: false,
    revocationEventId: { authorityDomainId: DOMAIN, lsn: "42" },
    appliedPolicy: "cancel",
    affectedCommandCount: 1,
    commandEffects: [{ commandId: null, fromState: "unspecified", toState: "unspecified", failureCode: "cancelled" }],
  });
  const repeat = captureOutput();
  assert.equal(await grantRevokeCommand(client as never, DOMAIN, { grantId: "grant-1", json: true }, repeat), 0);
  assert.equal(JSON.parse(repeat.out[0]!).alreadyRevoked, true);
});

test("grant-revoke rejects malformed protocol responses instead of rendering success", async () => {
  const malformed = [
    create(RevokeGrantResultSchema, { appliedPolicy: GrantRevocationPolicy.CANCEL }),
    create(RevokeGrantResultSchema, { changed: true, alreadyRevoked: true, appliedPolicy: GrantRevocationPolicy.CANCEL }),
    create(RevokeGrantResultSchema, { changed: true, appliedPolicy: GrantRevocationPolicy.CANCEL }),
    create(RevokeGrantResultSchema, { changed: true, revocationEventId: create(EventIdSchema, { authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }), lsn: { value: 42n } }) }),
    create(RevokeGrantResultSchema, { changed: true, appliedPolicy: 99 as never, revocationEventId: create(EventIdSchema, { authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }), lsn: { value: 42n } }) }),
  ];
  for (const result of malformed) {
    const output = captureOutput();
    await assert.rejects(
      grantRevokeCommand({ revokeGrant: async () => result } as never, DOMAIN, { grantId: "grant-1", json: true }, output),
      /invalid grant revocation response/,
    );
    assert.equal(output.out.length, 0);
  }
});

test("revoke-all clears local credentials and renders only safe generation/event fields", async () => {
  const store = await temporaryStore();
  const stored = {
    version: 1 as const,
    authorityDomainId: DOMAIN,
    operatorActorId: operator,
    sessionId: "opaque-session-secret",
    principal: {
      principalId: "principal",
      secret: BEARER_SECRET,
      operatorActorId: operator,
      endpointId: "endpoint",
      deviceId: "device",
      endpointGeneration: "1",
    },
  };
  await store.write(stored);
  const output = captureOutput();
  const code = await revokeAllSessionsCommand(
    {
      async revokeAllOperatorSessions() {
        return create(RevokeAllOperatorSessionsResultSchema, {
          revokedSessionCount: 3,
          invalidatedThroughGeneration: create(GenerationSchema, { value: 4n }),
          revocationEventId: create(EventIdSchema, {
            authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
            lsn: { value: 42n },
          }),
        });
      },
    } as never,
    store,
    { reasonCode: "operator_requested", json: true },
    output,
  );
  assert.equal(code, 0);
  assert.equal(await store.read(), null);
  assert.deepEqual(JSON.parse(output.out[0]!), {
    kind: "operator_sessions",
    revokedSessionCount: 3,
    generation: "4",
    revocationEventId: { authorityDomainId: DOMAIN, lsn: "42" },
  });
  assert.doesNotMatch([...output.out, ...output.err].join("\n"), /opaque-session-secret|bearer-secret/);
});

test("self endpoint/principal revocation clears only matching credentials", async () => {
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
  const output = captureOutput();
  const result = create(RevokeControlSurfaceResultSchema, {
    newlyRevoked: true,
    revokedPrincipalCount: 1,
    revokedSessionCount: 2,
    revocationEventId: create(EventIdSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
      lsn: { value: 9n },
    }),
  });
  assert.equal(
    await revokePrincipalCommand({ revokeControlSurfacePrincipal: async () => result } as never, store, {
      principalId: "other-principal",
      reasonCode: "operator_requested",
      json: false,
    }, output),
    0,
  );
  assert.notEqual(await store.read(), null);
  assert.equal(
    await revokeEndpointCommand({ revokeControlSurfaceEndpoint: async () => result } as never, store, {
      endpointId: "endpoint",
      reasonCode: "operator_requested",
      json: false,
    }, output),
    0,
  );
  assert.equal(await store.read(), null);
});

test("revocation transport failure is honest and retains credentials", async () => {
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
  const output = captureOutput();
  const code = await revokeAllSessionsCommand({
    revokeAllOperatorSessions: async () => { throw new Error("connection refused"); },
  } as never, store, { reasonCode: "operator_requested", json: false }, output);
  assert.equal(code, 1);
  assert.notEqual(await store.read(), null);
  assert.match(output.err.join("\n"), /may already be invalid.*patchbay-cli login/);
  assert.doesNotMatch(output.err.join("\n"), /session|secret/i);
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
