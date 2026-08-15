import { create, fromBinary } from "@bufbuild/protobuf";
import {
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LsnSchema,
  LogicalTargetIdSchema,
  OperationKind,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnPromotionCommittedSchema,
  StoredEventKind,
  type Session,
} from "@patchbay/contracts";
import {
  continuationContextExplanation,
  continuationSpawnPayload,
  freshSpawnPayload,
  spawnAdapterTarget,
} from "@patchbay/operator-domain";
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
} from "./sessions.js";

export interface SpawnOptions extends OperationIdOptions {
  adapterId: string;
  shape: string;
  deploymentAuthorityRef?: string;
  json: boolean;
}

export interface RestartOptions extends OperationIdOptions {
  target: string;
  shape: string;
  deploymentAuthorityRef?: string;
  json: boolean;
}

export async function spawnCommand(
  client: Pick<ControlClient, "submit">,
  store: CredentialStore,
  authorityDomainId: string,
  options: SpawnOptions,
  output: CliOutput,
): Promise<number> {
  const context = await operationContext(store, authorityDomainId);
  const targetScope = spawnAdapterTarget(options.adapterId);
  const operation = operationBase(
    context,
    targetScope,
    OperationKind.SPAWN,
    operationIds(targetScope, options),
  );
  operation.payload = freshSpawnPayload({
    shape: options.shape,
    deploymentAuthorityRef: options.deploymentAuthorityRef,
  });
  output.stderr(options.json
    ? JSON.stringify({ target: `adapter=${encodeURIComponent(options.adapterId)}`, intent: "fresh" })
    : `Target: adapter=${options.adapterId} intent=fresh`);
  return printSubmissionResult(await client.submit({ operation }), options.json, output);
}

export async function restartCommand(
  client: Pick<ControlClient, "loadSnapshot" | "subscribe" | "submit">,
  store: CredentialStore,
  authorityDomainId: string,
  options: RestartOptions,
  output: CliOutput,
): Promise<number> {
  const context = await operationContext(store, authorityDomainId);
  const session = resolveSession(await loadSessions(client, authorityDomainId), options.target);
  const logicalTargetId = await resolveManagedLogicalTarget(client, authorityDomainId, session);
  const targetScope = spawnAdapterTarget(required(session.adapterId?.value, "session adapter id"));
  const operation = operationBase(
    context,
    targetScope,
    OperationKind.SPAWN,
    operationIds(targetScope, options),
  );
  operation.payload = continuationSpawnPayload(exactPrior(logicalTargetId, session), {
    shape: options.shape,
    deploymentAuthorityRef: options.deploymentAuthorityRef,
  });
  const identity = canonicalSessionIdentity(session);
  output.stderr(options.json
    ? JSON.stringify({ target: identity, intent: "continuation", context: "unknown" })
    : `Target: ${identity} intent=continuation; ${continuationContextExplanation("unknown")}`);
  return printSubmissionResult(await client.submit({ operation }), options.json, output);
}

function exactPrior(logicalTargetId: string, session: Session) {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: logicalTargetId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: session.adapterId,
      deploymentScope: session.deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, {
        value: required(session.runtimeSessionId?.value, "runtime session id"),
      }),
      generation: create(GenerationSchema, {
        value: requiredGeneration(session.sessionGeneration?.value),
      }),
    }),
  });
}

async function resolveManagedLogicalTarget(
  client: Pick<ControlClient, "subscribe">,
  authorityDomainId: string,
  session: Session,
): Promise<string> {
  const expected = externalIdentity(
    session.adapterId?.value,
    session.deploymentScope,
    session.runtimeSessionId?.value,
    session.sessionGeneration?.value,
  );
  let found: string | undefined;
  for await (const event of client.subscribe({
    authorityDomainId: { value: authorityDomainId },
    cursor: create(LsnSchema, { value: 0n }),
  })) {
    if (event.payload?.kind !== StoredEventKind.SPAWN_PROMOTION_COMMITTED) continue;
    const promotion = fromBinary(SpawnPromotionCommittedSchema, event.payload.payload);
    const external = promotion.promotedRuntime?.externalRuntime;
    if (!external || externalIdentity(
      external.adapterId?.value,
      external.deploymentScope,
      external.runtimeSessionId?.value,
      external.generation?.value,
    ) !== expected) continue;
    const candidate = required(promotion.promotedRuntime?.logicalTargetId?.value, "promoted logical target id");
    if (found && found !== candidate) throw new Error("current runtime has conflicting logical-target history");
    found = candidate;
  }
  if (!found) {
    throw new Error("session is not a reconciled managed logical target; restart is unavailable");
  }
  return found;
}

function externalIdentity(
  adapterValue: string | undefined,
  deploymentScope: string,
  runtimeValue: string | undefined,
  generationValue: bigint | undefined,
): string {
  const adapter = required(adapterValue, "external adapter id");
  const runtime = required(runtimeValue, "external runtime id");
  const generation = requiredGeneration(generationValue);
  return `${adapter.length}:${adapter}|${deploymentScope.length}:${deploymentScope}|${runtime.length}:${runtime}|${generation}`;
}

function requiredGeneration(value: bigint | undefined): bigint {
  if (value === undefined || value <= 0n) throw new Error("runtime generation must be positive");
  return value;
}

function required(value: string | undefined, name: string): string {
  if (!value) throw new Error(`${name} is missing`);
  return value;
}
