import { create, fromBinary } from "@bufbuild/protobuf";
import { createHash, randomUUID } from "node:crypto";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  DeviceIdSchema,
  EndpointIdSchema,
  GenerationSchema,
  LsnSchema,
  OperationKind,
  OperationSchema,
  StoredEventKind,
  TargetScopeKind,
  TypedCorrelationSchema,
  type Operation,
  type TargetScope,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CliCredentials, CredentialStore } from "../credentials.js";

export interface OperationIdOptions {
  idempotencyKey?: string;
  commandId?: string;
}

export interface OperationContext {
  authorityDomainId: ReturnType<typeof create<typeof AuthorityDomainIdSchema>>;
  sender: ReturnType<typeof create<typeof ActorEndpointRefSchema>>;
  credentials: CliCredentials;
}

export async function operationContext(
  store: CredentialStore,
  authorityDomainId: string,
): Promise<OperationContext> {
  const credentials = await store.readRequired();
  if (credentials.authorityDomainId !== authorityDomainId) {
    throw new Error(
      `credential store belongs to authority domain ${credentials.authorityDomainId}, not ${authorityDomainId}`,
    );
  }
  return {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: credentials.operatorActorId }),
      endpointId: create(EndpointIdSchema, { value: credentials.principal.endpointId }),
      deviceId: create(DeviceIdSchema, { value: credentials.principal.deviceId }),
      endpointGeneration: create(GenerationSchema, {
        value: BigInt(credentials.principal.endpointGeneration),
      }),
    }),
    credentials,
  };
}

export function operationIds(
  targetScope: TargetScope,
  options: OperationIdOptions,
): { commandId: string; idempotencyKey: string } {
  if (options.commandId === "" || options.idempotencyKey === "") {
    throw new Error("command id and idempotency key must not be empty");
  }
  const idempotencyKey = options.idempotencyKey ?? `cli-${randomUUID()}`;
  const commandId =
    options.commandId ??
    (options.idempotencyKey
      ? `cli-${createHash("sha256")
          .update(idempotencyKey)
          .update("\0")
          .update(targetIdentity(targetScope))
          .digest("hex")
          .slice(0, 32)}`
      : `cli-${randomUUID()}`);
  return { commandId, idempotencyKey };
}

export function commandCorrelation(commandId: string) {
  if (!commandId) throw new Error("target command id must not be empty");
  return create(TypedCorrelationSchema, {
    ref: {
      case: "commandId",
      value: create(CommandIdSchema, { value: commandId }),
    },
  });
}

export function operationBase(
  context: OperationContext,
  targetScope: TargetScope,
  kind: OperationKind,
  ids: { commandId: string; idempotencyKey: string },
): Operation {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: ids.commandId }),
    authorityDomainId: context.authorityDomainId,
    sender: context.sender,
    kind,
    targetScope,
    idempotencyKey: ids.idempotencyKey,
  });
}

export async function resolveCommandTarget(
  client: Pick<ControlClient, "subscribe">,
  authorityDomainId: string,
  commandId: string,
): Promise<TargetScope> {
  if (!commandId) throw new Error("target command id must not be empty");
  const events = client.subscribe({
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
    cursor: create(LsnSchema, { value: 0n }),
  });
  for await (const event of events) {
    if (event.payload?.kind !== StoredEventKind.OPERATION) continue;
    const operation = fromBinary(OperationSchema, event.payload.payload);
    if (operation.commandId?.value !== commandId) continue;
    if (!operation.targetScope) throw new Error(`command ${commandId} has no target scope`);
    if (operation.targetScope.kind !== TargetScopeKind.RUNTIME_SESSION) {
      throw new Error(`command ${commandId} does not target a runtime session`);
    }
    return operation.targetScope;
  }
  throw new Error(`command not found in the core command records: ${commandId}`);
}

export function targetIdentity(target: TargetScope): string {
  const adapter = target.adapterId?.value;
  const runtime = target.runtimeSessionId?.value;
  const generation = target.sessionGeneration?.value;
  if (!adapter || !runtime || generation === undefined) {
    throw new Error("runtime-session target identity is incomplete");
  }
  return `adapter=${encodeURIComponent(adapter)};scope=${encodeURIComponent(target.deploymentScope)};runtime=${encodeURIComponent(runtime)};generation=${generation}`;
}
