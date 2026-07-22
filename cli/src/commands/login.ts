import { create } from "@bufbuild/protobuf";
import { ActorIdSchema } from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import { credentialsFromRpc, type CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { newCliEnrollment, type EnrollmentOptions } from "./enrollment.js";

export interface LoginOptions extends EnrollmentOptions {
  operatorActorId: string;
  password: string;
}

export async function loginCommand(
  client: Pick<ControlClient, "verifyOperatorPassword">,
  store: CredentialStore,
  authorityDomainId: string,
  options: LoginOptions,
  output: CliOutput,
): Promise<number> {
  if (!options.operatorActorId) throw new Error("operator actor id must not be empty");
  if (!options.password) throw new Error("operator password must not be empty");

  const result = await client.verifyOperatorPassword({
    operatorActorId: create(ActorIdSchema, { value: options.operatorActorId }),
    password: options.password,
    principal: newCliEnrollment(options),
  });
  const credentials = credentialsFromRpc(
    authorityDomainId,
    result.operatorSessionId?.value,
    result.principal,
  );
  await store.write(credentials);

  output.stdout(`Authenticated operator ${credentials.operatorActorId}; credentials=${store.path}`);
  return 0;
}
