import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  FreshSpawnSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SpawnContinuationSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type PayloadEnvelope,
  type RuntimeGenerationRef,
  type SpawnRequest,
  type SpawnTargetSpec,
  type TargetScope,
} from "@patchbay/contracts";

export const SPAWN_REQUEST_SCHEMA = "patchbay.SpawnRequest";

/** Adapter-owned target parameters carried without becoming target identity. */
export interface SpawnTargetSpecInput {
  shape: string;
  adapterPayload?: PayloadEnvelope;
  deploymentAuthorityRef?: string;
}

/** The only generic logical-context outcomes. They never imply restoration of
 * shells, subprocesses, memory, or arbitrary process state. */
export type ContinuationContextStatus = "resumed" | "new_context" | "unknown";

export function continuationContextExplanation(status: ContinuationContextStatus): string {
  switch (status) {
    case "resumed":
      return "Adapter-native logical context resumed; arbitrary process state was not restored.";
    case "new_context":
      return "A new adapter-native logical context was created; prior process state was not restored.";
    case "unknown":
      return "Adapter-native logical-context continuity is unknown; no process-state continuity is claimed.";
  }
}

export function spawnAdapterTarget(adapterId: string): TargetScope {
  requireBoundedText(adapterId, "adapter id", 256);
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.ADAPTER,
    adapterId: create(AdapterIdSchema, { value: adapterId }),
  });
}

export function freshSpawnPayload(target: SpawnTargetSpecInput): PayloadEnvelope {
  return spawnPayload({ case: "fresh", value: create(FreshSpawnSchema) }, target);
}

export function continuationSpawnPayload(
  exactPrior: RuntimeGenerationRef,
  target: SpawnTargetSpecInput,
): PayloadEnvelope {
  validateExactPrior(exactPrior);
  return spawnPayload(
    {
      case: "continuation",
      value: create(SpawnContinuationSchema, { prior: exactPrior }),
    },
    target,
  );
}

function spawnPayload(
  intent: SpawnRequest["intent"],
  target: SpawnTargetSpecInput,
): PayloadEnvelope {
  const targetSpec = targetSpecMessage(target);
  const request = create(SpawnRequestSchema, { intent, targetSpec });
  return create(PayloadEnvelopeSchema, {
    contentType: PayloadContentType.PROTOBUF,
    schemaRef: SPAWN_REQUEST_SCHEMA,
    payload: toBinary(SpawnRequestSchema, request),
  });
}

function targetSpecMessage(target: SpawnTargetSpecInput): SpawnTargetSpec {
  requireBoundedText(target.shape, "spawn target shape", 128);
  if (target.deploymentAuthorityRef !== undefined) {
    requireBoundedText(target.deploymentAuthorityRef, "deployment authority reference", 256, true);
  }
  return create(SpawnTargetSpecSchema, {
    shape: target.shape,
    adapterPayload: target.adapterPayload,
    deploymentAuthorityRef: target.deploymentAuthorityRef ?? "",
  });
}

function validateExactPrior(prior: RuntimeGenerationRef): void {
  requireBoundedText(prior.logicalTargetId?.value, "logical target id", 256);
  const external = prior.externalRuntime;
  if (!external) throw new Error("continuation exact prior is missing external runtime");
  requireBoundedText(external.adapterId?.value, "prior adapter id", 256);
  requireBoundedText(external.deploymentScope, "prior deployment scope", 256);
  requireBoundedText(external.runtimeSessionId?.value, "prior runtime session id", 256);
  if (!external.generation || external.generation.value <= 0n) {
    throw new Error("prior runtime generation must be positive");
  }
  if (external.generation.value === (1n << 64n) - 1n) {
    throw new Error("prior runtime generation cannot advance");
  }
}

function requireBoundedText(
  value: string | undefined,
  name: string,
  maximum: number,
  allowEmpty = false,
): asserts value is string {
  if (value === undefined || (!allowEmpty && value.length === 0) || value.length > maximum) {
    throw new Error(`${name} must be ${allowEmpty ? "0" : "1"}..${maximum} bytes`);
  }
  if ([...value].some((character) => {
    const code = character.charCodeAt(0);
    return code < 0x20 || code > 0x7e;
  })) {
    throw new Error(`${name} must be printable ASCII`);
  }
}
