import {
  PayloadContentType,
  type PayloadEnvelope,
} from "@patchbay/contracts";

export interface ResourceIdentityView {
  adapterId: string;
  resourceKind: string;
  resourceId: string;
}

export interface ProjectionDescriptor {
  schemaRef: string;
  contentType: PayloadContentType;
}

export interface ProviderPoolProjection {
  kind: "pooled-provider-pool";
  displayName: string;
  providerLabel: string;
  health: "serving" | "degraded" | "exhausted" | "paused" | "unknown";
  remainingPercent?: number;
  resetLabel?: string;
  contributionCount?: number;
  serviceLabel?: string;
  controlPosture: "administration-capable";
}

export interface UsageWindowProjection {
  kind: "direct-provider-usage-window";
  displayName: string;
  providerLabel: string;
  health: "ok" | "low" | "exhausted" | "unknown";
  remainingPercent?: number;
  resetLabel?: string;
  accountLabel?: string;
  planLabel?: string;
  windowLabel?: string;
  burnRateLabel?: string;
  activeSessionCount?: number;
  controlPosture: "read-only";
}

export type DecodedResourceProjection = ProviderPoolProjection | UsageWindowProjection;

export type ResourceProjectionResult =
  | { status: "decoded"; value: DecodedResourceProjection }
  | { status: "unsupported"; projection: ProjectionDescriptor }
  | { status: "invalid"; projection: ProjectionDescriptor; reason: "projection_decode_failed" }
  | { status: "unavailable" };

export interface ResourceProjectionDecoder {
  resourceKind: string;
  resourcePayload: ProjectionDescriptor;
  projectionPayload: ProjectionDescriptor;
  decode(payload: Uint8Array): DecodedResourceProjection;
}

const JSON_DESCRIPTOR = PayloadContentType.JSON;
const LABEL_LIMIT = 240;

export const RESOURCE_PROJECTION_DECODERS: readonly ResourceProjectionDecoder[] = registerDecoders([
  {
    resourceKind: "provider_pool",
    resourcePayload: { schemaRef: "provider_pool.payload.v1", contentType: JSON_DESCRIPTOR },
    projectionPayload: { schemaRef: "provider_pool.projection.v1", contentType: JSON_DESCRIPTOR },
    decode: decodeProviderPool,
  },
  {
    resourceKind: "usage_window",
    resourcePayload: { schemaRef: "usage_window.payload.v1", contentType: JSON_DESCRIPTOR },
    projectionPayload: { schemaRef: "usage_window.projection.v1", contentType: JSON_DESCRIPTOR },
    decode: decodeUsageWindow,
  },
]);

export function decodeResourceProjection(
  identity: ResourceIdentityView,
  resourcePayload: PayloadEnvelope | undefined,
  projectionPayload: PayloadEnvelope | undefined,
): ResourceProjectionResult {
  if (!resourcePayload || !projectionPayload) return { status: "unavailable" };

  const projection = descriptor(projectionPayload);
  const decoder = RESOURCE_PROJECTION_DECODERS.find((candidate) =>
    candidate.resourceKind === identity.resourceKind
    && descriptorEquals(candidate.resourcePayload, resourcePayload)
    && descriptorEquals(candidate.projectionPayload, projectionPayload),
  );
  if (!decoder) return { status: "unsupported", projection };

  try {
    return { status: "decoded", value: decoder.decode(projectionPayload.payload) };
  } catch {
    return { status: "invalid", projection, reason: "projection_decode_failed" };
  }
}

function registerDecoders(decoders: readonly ResourceProjectionDecoder[]): readonly ResourceProjectionDecoder[] {
  const keys = new Set<string>();
  for (const decoder of decoders) {
    const key = [
      decoder.resourceKind,
      decoder.resourcePayload.schemaRef,
      decoder.resourcePayload.contentType,
      decoder.projectionPayload.schemaRef,
      decoder.projectionPayload.contentType,
    ].join("\u0000");
    if (keys.has(key)) throw new Error(`duplicate resource projection decoder ${key}`);
    keys.add(key);
  }
  return Object.freeze([...decoders]);
}

function descriptor(envelope: PayloadEnvelope): ProjectionDescriptor {
  return { schemaRef: envelope.schemaRef, contentType: envelope.contentType };
}

function descriptorEquals(expected: ProjectionDescriptor, actual: PayloadEnvelope): boolean {
  return expected.schemaRef === actual.schemaRef && expected.contentType === actual.contentType;
}

function decodeProviderPool(payload: Uint8Array): ProviderPoolProjection {
  const value = decodeObject(payload);
  return {
    kind: "pooled-provider-pool",
    displayName: requiredLabel(value, "displayName"),
    providerLabel: requiredLabel(value, "providerLabel"),
    health: enumValue(value, "health", ["serving", "degraded", "exhausted", "paused", "unknown"]),
    ...optionalPercent(value, "remainingPercent"),
    ...optionalLabel(value, "resetLabel"),
    ...optionalCount(value, "contributionCount"),
    ...optionalLabel(value, "serviceLabel"),
    controlPosture: "administration-capable",
  };
}

function decodeUsageWindow(payload: Uint8Array): UsageWindowProjection {
  const value = decodeObject(payload);
  return {
    kind: "direct-provider-usage-window",
    displayName: requiredLabel(value, "displayName"),
    providerLabel: requiredLabel(value, "providerLabel"),
    health: enumValue(value, "health", ["ok", "low", "exhausted", "unknown"]),
    ...optionalPercent(value, "remainingPercent"),
    ...optionalLabel(value, "resetLabel"),
    ...optionalLabel(value, "accountLabel"),
    ...optionalLabel(value, "planLabel"),
    ...optionalLabel(value, "windowLabel"),
    ...optionalLabel(value, "burnRateLabel"),
    ...optionalCount(value, "activeSessionCount"),
    controlPosture: "read-only",
  };
}

function decodeObject(payload: Uint8Array): Record<string, unknown> {
  const value: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(payload));
  if (!isRecord(value)) throw new Error("projection must be a JSON object");
  return value;
}

function requiredLabel(value: Record<string, unknown>, field: string): string {
  const candidate = value[field];
  if (typeof candidate !== "string" || candidate.length === 0 || candidate.length > LABEL_LIMIT) {
    throw new Error(`${field} must be a bounded non-empty string`);
  }
  return candidate;
}

function optionalLabel(value: Record<string, unknown>, field: string): Record<string, string> {
  if (value[field] === undefined) return {};
  return { [field]: requiredLabel(value, field) };
}

function optionalPercent(value: Record<string, unknown>, field: string): Record<string, number> {
  const candidate = value[field];
  if (candidate === undefined) return {};
  if (typeof candidate !== "number" || !Number.isFinite(candidate) || candidate < 0 || candidate > 100) {
    throw new Error(`${field} must be a finite percentage`);
  }
  return { [field]: candidate };
}

function optionalCount(value: Record<string, unknown>, field: string): Record<string, number> {
  const candidate = value[field];
  if (candidate === undefined) return {};
  if (typeof candidate !== "number" || !Number.isSafeInteger(candidate) || candidate < 0) {
    throw new Error(`${field} must be a non-negative integer`);
  }
  return { [field]: candidate };
}

function enumValue<const T extends string>(
  value: Record<string, unknown>,
  field: string,
  members: readonly T[],
): T {
  const candidate = value[field];
  if (typeof candidate !== "string" || !members.includes(candidate as T)) {
    throw new Error(`${field} has an unknown value`);
  }
  return candidate as T;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
