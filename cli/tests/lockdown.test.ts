import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { timestampFromMs } from "@bufbuild/protobuf/wkt";
import {
  AuthorityDomainIdSchema,
  EnterSecurityLockdownResultSchema,
  EventIdSchema,
  ExitSecurityLockdownResultSchema,
  GenerationSchema,
  SecurityLockdownStateSchema,
} from "@patchbay/contracts";
import { lockdownEnterCommand, lockdownExitCommand } from "../src/commands/lockdown.js";
import { CredentialStore } from "../src/credentials.js";
import { BEARER_SECRET, captureOutput, credentials, DOMAIN } from "./helpers.js";

async function store(): Promise<CredentialStore> {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-lockdown-"));
  const result = new CredentialStore(join(directory, "credentials.json"));
  await result.write(credentials());
  return result;
}

function event(lsn: bigint) {
  return create(EventIdSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
    lsn: { value: lsn },
  });
}

function activeState() {
  return create(SecurityLockdownStateSchema, {
    active: true,
    reasonCode: "operator_requested",
    enteredAt: timestampFromMs(1_700_000_000_000),
    enteredEventId: event(10n),
  });
}

test("lockdown-enter requires the literal confirmation before any RPC", async () => {
  const credentials = await store();
  let calls = 0;
  await assert.rejects(
    lockdownEnterCommand({ enterSecurityLockdown: async () => { calls += 1; throw new Error("must not call"); } }, credentials, DOMAIN, {
      reasonCode: "operator_requested", confirm: "lockdown", json: true,
    }, captureOutput()),
    /LOCKDOWN/,
  );
  assert.equal(calls, 0);
  assert.equal((await credentials.read())?.principal.secret, BEARER_SECRET);
});

test("lockdown-enter clears bearer material only after confirmed active posture", async () => {
  const credentials = await store();
  const output = captureOutput();
  const result = create(EnterSecurityLockdownResultSchema, {
    lockdown: activeState(), lockdownEventId: event(11n), affectedRuntimeSessionCount: 2,
    invalidatedThroughOperatorSessionGeneration: create(GenerationSchema, { value: 7n }),
  });
  assert.equal(await lockdownEnterCommand({ enterSecurityLockdown: async () => result }, credentials, DOMAIN, {
    reasonCode: "operator_requested", confirm: "LOCKDOWN", json: true,
  }, output), 0);
  assert.equal(await credentials.read(), null);
  assert.doesNotMatch(output.out.join("\n") + output.err.join("\n"), /bearer-secret|session-core-issued/);
  assert.equal(JSON.parse(output.out[0]!).active, true);
});

test("lockdown-exit uses the bootstrap channel and emits no credential fields", async () => {
  const output = captureOutput();
  const result = create(ExitSecurityLockdownResultSchema, {
    lockdown: create(SecurityLockdownStateSchema, { active: false, enteredEventId: event(11n) }),
    lockdownEventId: event(12n),
  });
  let request: unknown;
  assert.equal(await lockdownExitCommand({ exitSecurityLockdown: async (value) => { request = value; return result; } }, DOMAIN, { json: true }, output), 0);
  assert.deepEqual(JSON.parse(output.out[0]!), {
    kind: "security_lockdown_exit", active: false, authorityDomainId: DOMAIN,
    bootstrapChannel: "loopback_admin", lockdownEventId: { authorityDomainId: DOMAIN, lsn: "12" },
    alreadyInactive: false, enteredEventId: { authorityDomainId: DOMAIN, lsn: "11" },
  });
  assert.equal((request as { authorityDomainId?: { value?: string } }).authorityDomainId?.value, DOMAIN);
  assert.doesNotMatch(output.out.join("\n") + output.err.join("\n"), /bearer-secret|session-core-issued/);
});
