import { create } from "@bufbuild/protobuf";
import {
  OperationKind,
  PayloadContentType,
  PayloadEnvelopeSchema,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { printSubmissionResult } from "../output.js";
import {
  operationBase,
  operationContext,
  operationIds,
  type OperationIdOptions,
} from "./operations.js";
import {
  canonicalSessionIdentity,
  loadSessions,
  resolveSession,
  sessionTargetScope,
} from "./sessions.js";

export interface InstructOptions extends OperationIdOptions {
  target: string;
  prompt: string;
  json: boolean;
}

export async function instructCommand(
  client: Pick<ControlClient, "loadSnapshot" | "submit">,
  store: CredentialStore,
  authorityDomainId: string,
  options: InstructOptions,
  output: CliOutput,
): Promise<number> {
  if (!options.prompt.trim()) throw new Error("instruction prompt must not be empty");
  const context = await operationContext(store, authorityDomainId);
  const session = resolveSession(await loadSessions(client, authorityDomainId), options.target);
  const targetScope = sessionTargetScope(session);
  const identity = canonicalSessionIdentity(session);
  const ids = operationIds(targetScope, options);
  const operation = operationBase(context, targetScope, OperationKind.INSTRUCT, ids);
  operation.payload = create(PayloadEnvelopeSchema, {
    contentType: PayloadContentType.TEXT_UTF8,
    payload: new TextEncoder().encode(options.prompt),
  });

  printTargetBeforeIntent(identity, options.json, output);
  const result = await client.submit({ operation });
  return printSubmissionResult(result, options.json, output);
}

export function printTargetBeforeIntent(
  identity: string,
  json: boolean,
  output: CliOutput,
): void {
  output.stderr(json ? JSON.stringify({ target: identity }) : `Target: ${identity}`);
}
