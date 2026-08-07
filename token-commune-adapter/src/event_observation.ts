import { create } from "@bufbuild/protobuf";
import { timestampDate, timestampFromDate, type Timestamp } from "@bufbuild/protobuf/wkt";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  FailureCode,
  ObservationKind,
  ObservationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type Observation,
} from "@patchbay/contracts";
import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";
import ajvFormatsModule from "ajv-formats";
import eventGapSchema from "../schemas/event-gap-observation.schema.json" with { type: "json" };
import poolEventSchema from "../schemas/pool-event-observation.schema.json" with { type: "json" };
import type { GatewayEvent } from "./gateway_client.js";
import type { ResourceIdentitySynthesizer, SynthesizedResourceIdentity } from "./identity.js";
import { TOKEN_COMMUNE_RESOURCE_KINDS } from "./resource_contract.js";
import type { EventGapEvidence } from "./event_window.js";

export const TOKEN_COMMUNE_EVENT_KINDS = {
  capacity_shift: "production-emitted",
  auth_broken: "production-emitted",
  windfall: "production-emitted",
  fingerprint: "production-emitted",
  member: "production-emitted",
  window_exhausted: "declared-only",
  calibration: "declared-only",
} as const;

export type GatewayEventKind = keyof typeof TOKEN_COMMUNE_EVENT_KINDS;
export type TokenCommuneEmittedEventKind = {
  [K in GatewayEventKind]:
    (typeof TOKEN_COMMUNE_EVENT_KINDS)[K] extends "production-emitted" ? K : never
}[GatewayEventKind];

export const TOKEN_COMMUNE_OBSERVATION_SCHEMAS = {
  poolEvent: "patchbay.token_commune.pool_event.v1",
  eventGap: "patchbay.token_commune.event_gap.v1",
} as const;

export type PoolEventMapResult =
  | { readonly status: "mapped"; readonly observation: Observation }
  | { readonly status: "declared-but-unemitted"; readonly kind: "window_exhausted" | "calibration" };

const ajv = new Ajv2020({ allErrors: true, strict: true });
ajvFormatsModule.default(ajv);
ajv.addSchema(poolEventSchema);
ajv.addSchema(eventGapSchema);
const validatePoolEvent = requiredValidator(TOKEN_COMMUNE_OBSERVATION_SCHEMAS.poolEvent);
const validateEventGap = requiredValidator(TOKEN_COMMUNE_OBSERVATION_SCHEMAS.eventGap);
const encoder = new TextEncoder();

export function parseGatewayEventKind(value: unknown): GatewayEventKind {
  if (typeof value !== "string" || !Object.hasOwn(TOKEN_COMMUNE_EVENT_KINDS, value)) {
    throw new Error("event kind is unknown");
  }
  return value as GatewayEventKind;
}

export function mapPoolEvent(input: {
  authorityDomainId: string;
  adapterId: string;
  identities: ResourceIdentitySynthesizer;
  event: GatewayEvent;
}): PoolEventMapResult {
  validateContext(input.authorityDomainId, input.adapterId);
  const kind = parseGatewayEventKind(input.event.kind);
  if (kind === "window_exhausted" || kind === "calibration") {
    return { status: "declared-but-unemitted", kind };
  }
  validateEvent(input.event);
  const observedAt = timestamp(input.event.occurredAt, "event occurredAt");
  const target = input.identities.providerPool(input.event.provider);
  assertProviderPoolTarget(input.adapterId, target);
  const payload = {
    sourceEventId: input.event.id,
    kind,
    provider: input.event.provider,
    contributionId: input.event.contributionId,
    message: input.event.message,
    occurredAt: input.event.occurredAt,
    deliveryModel: "polling",
    historyMode: "latest-50-no-cursor",
  } as const;
  if (!validatePoolEvent(payload)) throw new Error("token-commune pool event contract validation failed");
  return {
    status: "mapped",
    observation: observation({
      authorityDomainId: input.authorityDomainId,
      adapterId: input.adapterId,
      target,
      observedAt,
      schemaRef: TOKEN_COMMUNE_OBSERVATION_SCHEMAS.poolEvent,
      payload,
    }),
  };
}

export function mapEventGap(input: {
  authorityDomainId: string;
  adapterId: string;
  targets: readonly SynthesizedResourceIdentity[];
  detectedAt: Timestamp;
  gap: EventGapEvidence;
}): readonly Observation[] {
  validateContext(input.authorityDomainId, input.adapterId);
  const detectedAt = timestampDate(input.detectedAt);
  if (!Number.isFinite(detectedAt.getTime())) throw new Error("gap detection timestamp is invalid");
  const payload = {
    reason: input.gap.reason,
    previousWindowSize: input.gap.previousWindowSize,
    visibleWindowSize: input.gap.visibleWindowSize,
    overlapCount: input.gap.overlapCount,
    detectedAt: detectedAt.toISOString(),
    deliveryModel: "polling",
    historyMode: "latest-50-no-cursor",
    reconstruction: input.gap.reconstruction,
    continuity: input.gap.continuity,
  } as const;
  if (!validateEventGap(payload)) throw new Error("token-commune event gap contract validation failed");
  const unique = new Map<string, SynthesizedResourceIdentity>();
  for (const target of input.targets) {
    assertProviderPoolTarget(input.adapterId, target);
    const key = JSON.stringify([target.adapterId, target.resourceKind, target.resourceId]);
    if (unique.has(key)) throw new Error("duplicate event gap target");
    unique.set(key, target);
  }
  return [...unique.values()]
    .sort((left, right) => left.resourceId.localeCompare(right.resourceId))
    .map((target) => observation({
      authorityDomainId: input.authorityDomainId,
      adapterId: input.adapterId,
      target,
      observedAt: input.detectedAt,
      schemaRef: TOKEN_COMMUNE_OBSERVATION_SCHEMAS.eventGap,
      payload,
    }));
}

function observation(input: {
  authorityDomainId: string;
  adapterId: string;
  target: SynthesizedResourceIdentity;
  observedAt: Timestamp;
  schemaRef: string;
  payload: unknown;
}): Observation {
  return create(ObservationSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: input.authorityDomainId }),
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: input.adapterId }),
    }),
    kind: ObservationKind.STATUS,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RESOURCE,
      resource: create(ResourceIdentitySchema, {
        adapterId: create(AdapterIdSchema, { value: input.target.adapterId }),
        resourceKind: create(ResourceKindSchema, { value: input.target.resourceKind }),
        resourceId: create(ResourceIdSchema, { value: input.target.resourceId }),
      }),
    }),
    payload: create(PayloadEnvelopeSchema, {
      payload: encoder.encode(JSON.stringify(input.payload)),
      contentType: PayloadContentType.JSON,
      schemaRef: input.schemaRef,
    }),
    observedAt: input.observedAt,
    failureCode: FailureCode.UNSPECIFIED,
  });
}

function validateContext(authorityDomainId: string, adapterId: string): void {
  if (!authorityDomainId.trim() || !adapterId.trim()) throw new Error("authority domain and adapter id are required");
}

function validateEvent(event: GatewayEvent): void {
  if (!event.id.trim() || event.id.length > 512) throw new Error("event id must be bounded");
  if (!event.provider.trim() || event.provider.length > 512) throw new Error("event provider must be bounded");
  if (!event.message.trim() || event.message.length > 1024) throw new Error("event message must be bounded");
  if (event.contributionId !== null && (!event.contributionId.trim() || event.contributionId.length > 512)) {
    throw new Error("event contribution id must be bounded or null");
  }
}

function timestamp(value: string, name: string): Timestamp {
  const milliseconds = Date.parse(value);
  if (!Number.isFinite(milliseconds)) throw new Error(`${name} must be a timestamp`);
  return timestampFromDate(new Date(milliseconds));
}

function assertProviderPoolTarget(adapterId: string, target: SynthesizedResourceIdentity): void {
  if (
    target.adapterId !== adapterId
    || target.resourceKind !== TOKEN_COMMUNE_RESOURCE_KINDS.providerPool
    || !target.resourceId.trim()
  ) throw new Error("event target must be the adapter-owned provider pool resource");
}

function requiredValidator(schemaRef: string): ValidateFunction {
  const validator = ajv.getSchema(schemaRef);
  if (!validator) throw new Error(`missing token-commune observation schema: ${schemaRef}`);
  return validator;
}
