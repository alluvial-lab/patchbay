import { randomBytes, scrypt } from "node:crypto";
import { promisify } from "node:util";
import { create } from "@bufbuild/protobuf";
import { ActorIdSchema } from "@patchbay/contracts";
import type { AdminClient } from "../core-client.js";
import { credentialsFromRpc, type CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { newCliEnrollment, type EnrollmentOptions } from "./enrollment.js";

const scryptAsync = promisify(scrypt);
const PASSWORD_HASH_BYTES = 64;

export interface SetupOptions extends EnrollmentOptions {
  setupSecret: string;
  operatorActorId: string;
  password: string;
}

export async function setupCommand(
  client: Pick<AdminClient, "bootstrapOperator">,
  store: CredentialStore,
  authorityDomainId: string,
  options: SetupOptions,
  output: CliOutput,
): Promise<number> {
  requireValue(options.setupSecret, "setup secret");
  requireValue(options.operatorActorId, "operator actor id");
  requireValue(options.password, "operator password");

  const result = await client.bootstrapOperator({
    setupSecret: options.setupSecret,
    operatorActorId: create(ActorIdSchema, { value: options.operatorActorId }),
    passwordHash: await hashPassword(options.password),
    principal: newCliEnrollment(options),
  });
  const credentials = credentialsFromRpc(
    authorityDomainId,
    result.sessionId?.value,
    result.principal,
  );
  await store.write(credentials);

  output.stdout(
    `Operator ${credentials.operatorActorId} bootstrapped; grant=${result.grantId?.value ?? "unknown"}; credentials=${store.path}`,
  );
  return 0;
}

export async function hashPassword(password: string, salt = randomBytes(16)): Promise<string> {
  requireValue(password, "operator password");
  const derived = (await scryptAsync(password, salt, PASSWORD_HASH_BYTES)) as Buffer;
  return `scrypt$${salt.toString("base64url")}$${derived.toString("base64url")}`;
}

function requireValue(value: string, name: string): void {
  if (!value) throw new Error(`${name} must not be empty`);
}
