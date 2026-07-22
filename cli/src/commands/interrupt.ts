import { OperationKind } from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { printSubmissionResult } from "../output.js";
import {
  commandCorrelation,
  operationBase,
  operationContext,
  operationIds,
  resolveCommandTarget,
  targetIdentity,
  type OperationIdOptions,
} from "./operations.js";

export interface InterruptOptions extends OperationIdOptions {
  targetCommandId: string;
  json: boolean;
}

export async function interruptCommand(
  client: Pick<ControlClient, "subscribe" | "submit">,
  store: CredentialStore,
  authorityDomainId: string,
  options: InterruptOptions,
  output: CliOutput,
): Promise<number> {
  const context = await operationContext(store, authorityDomainId);
  const target = await resolveCommandTarget(client, authorityDomainId, options.targetCommandId);
  const operation = operationBase(
    context,
    target,
    OperationKind.INTERRUPT,
    operationIds(target, options),
  );
  operation.correlations = [commandCorrelation(options.targetCommandId)];
  output.stderr(
    options.json
      ? JSON.stringify({ target: targetIdentity(target), commandId: options.targetCommandId })
      : `Target: ${targetIdentity(target)} command=${options.targetCommandId}`,
  );
  return printSubmissionResult(await client.submit({ operation }), options.json, output);
}
