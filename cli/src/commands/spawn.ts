import { create, fromBinary } from "@bufbuild/protobuf";
import {
  ContinuationContextStatus,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LsnSchema,
  LogicalTargetIdSchema,
  OperationKind,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnClaimDisposition,
  SpawnPromotionCommittedSchema,
  StoredEventKind,
  type Session,
} from "@patchbay/contracts";
import {
  continuationContextExplanation,
  continuationContextStatusName,
  continuationSpawnPayload,
  freshSpawnPayload,
  spawnAdapterTarget,
  spawnClaimDispositionName,
} from "@patchbay/operator-domain";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { printSubmissionResult } from "../output.js";
import { capabilityForUnknownSubmission } from "./adapter-status.js";
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

export interface AbandonSpawnTargetOptions {
  claimOperationId: string;
  logicalTargetId: string;
  reasonCode: string;
  json: boolean;
}

export async function abandonSpawnTargetCommand(
  client: Pick<ControlClient, "abandonSpawnTarget">,
  store: CredentialStore,
  authorityDomainId: string,
  options: AbandonSpawnTargetOptions,
  output: CliOutput,
): Promise<number> {
  if (!options.claimOperationId || !options.logicalTargetId) {
    throw new Error("claim operation id and logical target id are required");
  }
  if (!/^[a-z0-9_]{1,64}$/.test(options.reasonCode)) {
    throw new Error("reason code must match [a-z0-9_]{1,64}");
  }
  await operationContext(store, authorityDomainId);
  const result = await client.abandonSpawnTarget({
    authorityDomainId: { value: authorityDomainId },
    claimOperationId: { value: options.claimOperationId },
    logicalTargetId: { value: options.logicalTargetId },
    reasonCode: options.reasonCode,
  });
  if (result.disposition !== SpawnClaimDisposition.TARGET_ABANDONED) {
    throw new Error("core returned a non-abandoned spawn claim disposition");
  }
  const view = {
    changed: result.changed,
    alreadyAbandoned: result.alreadyAbandoned,
    disposition: spawnClaimDispositionName(result.disposition),
    claimOperationId: options.claimOperationId,
    logicalTargetId: result.logicalTargetId?.value ?? options.logicalTargetId,
    abandonmentEventLsn: result.abandonmentEventId?.lsn?.value.toString(),
    auditEventLsn: result.auditEventId?.lsn?.value.toString(),
    authorizingGrantId: result.authorizingGrantId?.value,
  };
  output.stdout(options.json
    ? JSON.stringify(view)
    : `${view.disposition}: ${view.logicalTargetId} (claim ${view.claimOperationId})${view.alreadyAbandoned ? " already committed" : ""}`);
  return 0;
}

export async function spawnCommand(
  client: Pick<ControlClient, "submit">
    & Partial<Pick<ControlClient, "queryDiagnostics">>,
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
  const result = await client.submit({ operation });
  const capability = await capabilityForUnknownSubmission(
    client,
    store,
    authorityDomainId,
    targetScope,
    result,
  );
  return printSubmissionResult(result, options.json, output, capability);
}

export async function restartCommand(
  client: Pick<ControlClient, "loadSnapshot" | "subscribe" | "submit">
    & Partial<Pick<ControlClient, "queryDiagnostics">>,
  store: CredentialStore,
  authorityDomainId: string,
  options: RestartOptions,
  output: CliOutput,
): Promise<number> {
  const context = await operationContext(store, authorityDomainId);
  const session = resolveSession(await loadSessions(client, authorityDomainId), options.target);
  const managed = await resolveManagedLogicalTarget(client, authorityDomainId, session);
  const targetScope = spawnAdapterTarget(required(session.adapterId?.value, "session adapter id"));
  const operation = operationBase(
    context,
    targetScope,
    OperationKind.SPAWN,
    operationIds(targetScope, options),
  );
  operation.payload = continuationSpawnPayload(exactPrior(managed.logicalTargetId, session), {
    shape: options.shape,
    deploymentAuthorityRef: options.deploymentAuthorityRef,
  });
  const identity = canonicalSessionIdentity(session);
  output.stderr(options.json
    ? JSON.stringify({
        target: identity,
        intent: "continuation",
        ...(managed.contextStatus === undefined
          ? {}
          : { currentContext: continuationContextStatusName(managed.contextStatus) }),
      })
    : `Target: ${identity} intent=continuation${managed.contextStatus === undefined
      ? ""
      : `; current ${continuationContextExplanation(managed.contextStatus)}`}`);
  const result = await client.submit({ operation });
  const capability = await capabilityForUnknownSubmission(
    client,
    store,
    authorityDomainId,
    targetScope,
    result,
  );
  return printSubmissionResult(result, options.json, output, capability);
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
): Promise<{ logicalTargetId: string; contextStatus?: ContinuationContextStatus }> {
  const expected = externalIdentity(
    session.adapterId?.value,
    session.deploymentScope,
    session.runtimeSessionId?.value,
    session.sessionGeneration?.value,
  );
  let found: { logicalTargetId: string; contextStatus?: ContinuationContextStatus } | undefined;
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
    const logicalTargetId = required(
      promotion.promotedRuntime?.logicalTargetId?.value,
      "promoted logical target id",
    );
    const reportedContext = promotion.stagedSuccessor?.staged?.continuationContextStatus;
    const contextStatus = reportedContext === undefined
      || reportedContext === ContinuationContextStatus.UNSPECIFIED
      ? undefined
      : reportedContinuationContext(reportedContext);
    const candidate = { logicalTargetId, contextStatus };
    if (found && (found.logicalTargetId !== candidate.logicalTargetId
        || found.contextStatus !== candidate.contextStatus)) {
      throw new Error("current runtime has conflicting managed promotion history");
    }
    found = candidate;
  }
  if (!found) {
    throw new Error("session is not a reconciled managed logical target; restart is unavailable");
  }
  return found;
}

function reportedContinuationContext(status: ContinuationContextStatus): ContinuationContextStatus {
  continuationContextStatusName(status);
  return status;
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
