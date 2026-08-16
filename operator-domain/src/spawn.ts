import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  ContinuationContextStatus,
  OperationKind,
  FreshSpawnSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SpawnClaimDisposition,
  SpawnContinuationSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type AdapterCapabilitySummary,
  type ManagedSpawnTargetCapability,
  type PayloadEnvelope,
  type RuntimeGenerationRef,
  type SpawnRequest,
  type SpawnTargetSpec,
  type TargetScope,
} from "@patchbay/contracts";

export const SPAWN_REQUEST_SCHEMA = "patchbay.SpawnRequest";

export const SPAWN_ACTION_UNAVAILABLE = Object.freeze({
  CAPABILITY_UNAVAILABLE: "Adapter capability is unavailable.",
  OPERATION_UNSUPPORTED: "Adapter does not declare spawn support.",
  SHAPE_UNDECLARED: "Adapter does not declare a spawn target-spec shape.",
  SHAPE_AMBIGUOUS: "Adapter declares multiple spawn target-spec shapes; this action requires exactly one.",
  SESSION_REPLACEMENT_UNSUPPORTED: "Adapter does not declare session replacement support.",
  TARGET_UNDECLARED: "Adapter does not declare a configured managed spawn target.",
  TARGET_AMBIGUOUS: "Adapter declares multiple managed spawn targets; select one before spawning.",
  TARGET_NOT_FOUND: "The selected managed spawn target is not declared by the adapter.",
  TARGET_SHAPE_MISMATCH: "The managed spawn target does not match the adapter's declared target-spec shape.",
  TARGET_PAYLOAD_UNAVAILABLE: "The managed spawn target does not declare the required adapter payload.",
} as const);

export type SpawnActionUnavailableReason =
  (typeof SPAWN_ACTION_UNAVAILABLE)[keyof typeof SPAWN_ACTION_UNAVAILABLE];

export type DeclaredManagedSpawnTarget =
  | {
      available: true;
      logicalTargetId: string;
      target: SpawnTargetSpecInput;
    }
  | {
      available: false;
      reason: SpawnActionUnavailableReason;
    };

interface SpawnCapabilityDeclaration {
  readonly supportedOperationKinds: readonly OperationKind[];
  readonly supportedTargetSpecShapes: readonly string[];
  readonly sessionReplacementSupport: boolean;
  readonly managedSpawnTargets: readonly ManagedSpawnTargetCapability[];
}

/** Resolve one adapter-declared managed target without interpreting its
 * adapter-specific payload. Capability remains advisory; the adapter still
 * owns support at delivery time. */
export function declaredManagedSpawnTarget(
  capability: AdapterCapabilitySummary | SpawnCapabilityDeclaration | undefined,
  intent: "fresh" | "continuation",
  logicalTargetId?: string,
): DeclaredManagedSpawnTarget {
  if (!capability) return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.CAPABILITY_UNAVAILABLE };
  if (!capability.supportedOperationKinds.includes(OperationKind.SPAWN)) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.OPERATION_UNSUPPORTED };
  }
  if (intent === "continuation" && !capability.sessionReplacementSupport) {
    return {
      available: false,
      reason: SPAWN_ACTION_UNAVAILABLE.SESSION_REPLACEMENT_UNSUPPORTED,
    };
  }
  if (capability.supportedTargetSpecShapes.length === 0) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.SHAPE_UNDECLARED };
  }
  if (capability.supportedTargetSpecShapes.length !== 1) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.SHAPE_AMBIGUOUS };
  }
  const shape = capability.supportedTargetSpecShapes[0]!;
  const candidates = capability.managedSpawnTargets.filter((target) =>
    logicalTargetId === undefined || target.logicalTargetId?.value === logicalTargetId,
  );
  if (logicalTargetId !== undefined && candidates.length === 0) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_NOT_FOUND };
  }
  if (candidates.length === 0) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_UNDECLARED };
  }
  if (candidates.length !== 1) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_AMBIGUOUS };
  }
  const selected = candidates[0]!;
  const selectedLogicalTargetId = selected.logicalTargetId?.value;
  if (!selectedLogicalTargetId) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_NOT_FOUND };
  }
  if (selected.targetSpecShape !== shape) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_SHAPE_MISMATCH };
  }
  const adapterPayload = intent === "fresh"
    ? selected.freshAdapterPayload
    : selected.continuationAdapterPayload;
  if (!adapterPayload) {
    return { available: false, reason: SPAWN_ACTION_UNAVAILABLE.TARGET_PAYLOAD_UNAVAILABLE };
  }
  return {
    available: true,
    logicalTargetId: selectedLogicalTargetId,
    target: { shape, adapterPayload },
  };
}

/** Adapter-owned target parameters carried without becoming target identity. */
export interface SpawnTargetSpecInput {
  shape: string;
  adapterPayload?: PayloadEnvelope;
  deploymentAuthorityRef?: string;
}

/** Explain the generated adapter-reported context outcome without implying
 * restoration of shells, subprocesses, memory, or arbitrary process state. */
export function continuationContextExplanation(status: ContinuationContextStatus): string {
  switch (status) {
    case ContinuationContextStatus.RESUMED:
      return "Adapter-native logical context resumed; arbitrary process state was not restored.";
    case ContinuationContextStatus.NEW_CONTEXT:
      return "A new adapter-native logical context was created; prior process state was not restored.";
    case ContinuationContextStatus.UNKNOWN:
      return "Adapter-native logical-context continuity is unknown; no process-state continuity is claimed.";
    case ContinuationContextStatus.UNSPECIFIED:
      throw new Error("continuation context status is unspecified");
    default:
      throw new Error(`continuation context status is unknown: ${status}`);
  }
}

export function continuationContextStatusName(status: ContinuationContextStatus): string {
  switch (status) {
    case ContinuationContextStatus.RESUMED:
      return "resumed";
    case ContinuationContextStatus.NEW_CONTEXT:
      return "new_context";
    case ContinuationContextStatus.UNKNOWN:
      return "unknown";
    case ContinuationContextStatus.UNSPECIFIED:
      throw new Error("continuation context status is unspecified");
    default:
      throw new Error(`continuation context status is unknown: ${status}`);
  }
}

export function spawnClaimDispositionName(disposition: SpawnClaimDisposition): string {
  switch (disposition) {
    case SpawnClaimDisposition.ACTIVE: return "active";
    case SpawnClaimDisposition.RELEASED_NO_EXTERNAL_EFFECT: return "released_no_external_effect";
    case SpawnClaimDisposition.POISONED_PENDING_RECONCILIATION: return "poisoned_pending_reconciliation";
    case SpawnClaimDisposition.PROMOTED: return "promoted";
    case SpawnClaimDisposition.TARGET_ABANDONED: return "target_abandoned";
    case SpawnClaimDisposition.UNSPECIFIED:
    default: throw new Error(`unsupported spawn claim disposition ${disposition}`);
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
