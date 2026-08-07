import { create } from "@bufbuild/protobuf";
import type { Timestamp } from "@bufbuild/protobuf/wkt";
import { createHash } from "node:crypto";
import {
  AdapterIdSchema,
  AdapterSnapshotSupport,
  GenerationSchema,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  ResourceReportMutationSchema,
  ResourceReportSchema,
  ResourceSnapshotReportSchema,
  ResourceStateUpsertSchema,
  ResourceViewReportSchema,
  type ResourceReport,
  type ResourceReportMutation,
} from "@patchbay/contracts";
import type {
  GatewayCapacityReading,
  GatewayDrawReport,
  GatewayFingerprints,
  GatewayMe,
  GatewayModel,
  GatewayModels,
  GatewayPool,
  GatewayPoolContribution,
  GatewayStatus,
  GatewayStatusContribution,
} from "./gateway_client.js";
import type { ResourceIdentitySynthesizer, SynthesizedResourceIdentity } from "./identity.js";
import {
  encodeResourceEnvelope,
  ResourceEnvelopeValidationError,
  type TokenCommuneResourceName,
} from "./resource_envelope.js";
import {
  TOKEN_COMMUNE_RESOURCE_KINDS,
  TOKEN_COMMUNE_RESOURCES,
  type AnonymousPoolContribution,
  type ContributionListing,
  type MemberDrawPayload,
  type MemberDrawProjection,
  type ProviderFingerprint,
  type ProviderModelCatalog,
  type ProviderPoolPayload,
  type ProviderPoolProjection,
  type ProviderStatusTelemetry,
} from "./resource_contract.js";

export type EndpointSnapshot<T> =
  | { readonly status: "reported"; readonly value: T }
  | { readonly status: "unavailable" };

export interface TokenCommuneGatewaySnapshot {
  readonly status: EndpointSnapshot<GatewayStatus>;
  readonly pool: EndpointSnapshot<GatewayPool>;
  readonly me: EndpointSnapshot<GatewayMe>;
  readonly fingerprints: EndpointSnapshot<GatewayFingerprints>;
  readonly models: EndpointSnapshot<GatewayModels>;
}

export interface TokenCommuneSnapshotProjectionInput {
  readonly adapterId: string;
  readonly adapterGeneration: number;
  readonly observedAt: Timestamp;
  readonly identities: ResourceIdentitySynthesizer;
  readonly gateway: TokenCommuneGatewaySnapshot;
}

export class SnapshotProjectionError extends Error {
  readonly name = "SnapshotProjectionError";
  constructor(
    readonly code:
      | "invalid-context"
      | "identity-mismatch"
      | "contract-validation-failed",
  ) {
    super(`token-commune snapshot projection ${code}`);
  }
}

export function projectTokenCommuneSnapshot(
  input: TokenCommuneSnapshotProjectionInput,
): ResourceReport {
  validateContext(input);
  try {
    const mutations: Record<TokenCommuneResourceName, ResourceReportMutation[]> = {
      providerPool: projectProviderPools(input),
      memberDraw: projectMemberDraws(input),
    };
    const views = (Object.keys(TOKEN_COMMUNE_RESOURCES) as TokenCommuneResourceName[]).map((name) =>
      create(ResourceViewReportSchema, {
        resourceKind: create(ResourceKindSchema, { value: TOKEN_COMMUNE_RESOURCES[name].kind }),
        completeness: AdapterSnapshotSupport.PARTIAL,
        mutations: mutations[name],
      }),
    );
    return create(ResourceReportSchema, {
      adapterId: create(AdapterIdSchema, { value: input.adapterId }),
      adapterGeneration: create(GenerationSchema, { value: BigInt(input.adapterGeneration) }),
      report: {
        case: "snapshot",
        value: create(ResourceSnapshotReportSchema, { views }),
      },
      observedAt: input.observedAt,
    });
  } catch (error) {
    if (error instanceof SnapshotProjectionError) throw error;
    if (error instanceof ResourceEnvelopeValidationError) {
      throw new SnapshotProjectionError("contract-validation-failed");
    }
    throw error;
  }
}

function projectProviderPools(input: TokenCommuneSnapshotProjectionInput): ResourceReportMutation[] {
  const providers = new Set<string>();
  if (input.gateway.pool.status === "reported") {
    for (const contribution of input.gateway.pool.value.contributions) providers.add(contribution.provider);
  }
  if (input.gateway.status.status === "reported") {
    providers.add("anthropic");
    for (const contribution of input.gateway.status.value.contributions) providers.add(contribution.provider);
  }
  if (input.gateway.models.status === "reported") {
    for (const model of input.gateway.models.value.models) providers.add(model.provider);
  }

  return [...providers].sort(compareText).map((provider) => {
    const contributionListing = contributionListingFor(input, provider);
    const statusTelemetry = statusTelemetryFor(input, provider);
    const modelCatalog = modelCatalogFor(input, provider);
    const fingerprint = fingerprintFor(input, provider);
    const contributions = contributionListing.contributions;
    const payload: ProviderPoolPayload = {
      identityStrategy: "composite-local",
      gatewayDeploymentKey: input.identities.gatewayDeploymentKey,
      provider,
      contributionListing,
      statusTelemetry,
      modelCatalog,
      fingerprint,
      limitations: {
        snapshotCompleteness: "partial",
        contributorAttribution: "unavailable",
        contributionIdentity: "snapshot-local-synthesized",
        statusPoolJoin: "unavailable",
        capacityAggregation: "none",
      },
    };
    const projection: ProviderPoolProjection = {
      provider,
      contributionListing,
      credentialHealthCounts: {
        fresh: contributions.filter(({ health }) => health.state === "fresh").length,
        exhausted: contributions.filter(({ health }) => health.state === "exhausted").length,
        authBroken: contributions.filter(({ health }) => health.state === "auth_broken").length,
      },
      totalDeclaredShare: contributions.reduce((sum, contribution) => sum + contribution.declaredShare, 0),
      statusTelemetry,
      modelCatalog,
      fingerprint,
      capacityAggregation: "none",
    };
    return upsertMutation(input, "providerPool", input.identities.providerPool(provider), payload, projection);
  });
}

function projectMemberDraws(input: TokenCommuneSnapshotProjectionInput): ResourceReportMutation[] {
  if (input.gateway.me.status === "unavailable" || input.gateway.me.value.reports.length === 0) return [];
  const memberDisplayName = input.gateway.me.value.displayName;
  const providers = new Set(input.gateway.me.value.reports.map(({ provider }) => provider));
  return [...providers].sort(compareText).map((provider) => {
    const reports = input.gateway.me.status === "reported"
      ? input.gateway.me.value.reports
        .filter((report) => report.provider === provider)
        .map(copyDrawReport)
        .sort((left, right) => compareText(stableJson(left), stableJson(right)))
      : [];
    const payload: MemberDrawPayload = {
      identityStrategy: "composite-local",
      gatewayDeploymentKey: input.identities.gatewayDeploymentKey,
      memberDisplayName,
      provider,
      reports,
      limitations: {
        snapshotCompleteness: "partial",
        stableMemberIdentity: "unavailable",
      },
    };
    const projection: MemberDrawProjection = { memberDisplayName, provider, reports };
    return upsertMutation(
      input,
      "memberDraw",
      input.identities.memberDraw(memberDisplayName, provider),
      payload,
      projection,
    );
  });
}

function contributionListingFor(
  input: TokenCommuneSnapshotProjectionInput,
  provider: string,
): ContributionListing {
  if (input.gateway.pool.status === "unavailable") return { status: "unavailable", contributions: [] };
  const rows = input.gateway.pool.value.contributions.filter((row) => row.provider === provider);
  if (rows.length === 0) return { status: "not-reported", contributions: [] };
  const canonicalRows = rows.map(canonicalPoolRow).sort((left, right) => compareText(left.key, right.key));
  const occurrences = new Map<string, number>();
  return {
    status: "reported",
    contributions: canonicalRows.map(({ key, row }) => {
      const occurrence = (occurrences.get(key) ?? 0) + 1;
      occurrences.set(key, occurrence);
      return {
        subKey: `local:anonymous-contribution:${digest(stableJson({
          gatewayDeploymentKey: input.identities.gatewayDeploymentKey,
          provider,
          row,
        }))}:${occurrence}`,
        subKeySource: "synthesized-content-hash",
        subKeyStability: "snapshot-local",
        attribution: "unavailable",
        declaredShare: row.declaredShare,
        health: row.health,
        telemetryState: row.capacityReadings.length === 0 ? "no-readings" : "readings",
        capacityReadings: row.capacityReadings,
        fingerprint: row.fingerprint,
      } satisfies AnonymousPoolContribution;
    }),
  };
}

function canonicalPoolRow(row: GatewayPoolContribution): {
  readonly key: string;
  readonly row: Omit<AnonymousPoolContribution, "subKey" | "subKeySource" | "subKeyStability" | "attribution" | "telemetryState">;
} {
  const canonical = {
    declaredShare: row.declaredShare,
    health: row.health,
    capacityReadings: sortCapacityReadings(row.capacity),
    fingerprint: row.fingerprint,
  };
  return { key: stableJson(canonical), row: canonical };
}

function statusTelemetryFor(
  input: TokenCommuneSnapshotProjectionInput,
  provider: string,
): ProviderStatusTelemetry {
  if (input.gateway.status.status === "unavailable") return { status: "unavailable", contributions: [] };
  const contributions = sortStatusContributions(
    input.gateway.status.value.contributions.filter((row) => row.provider === provider),
  );
  if (provider !== "anthropic" && contributions.length === 0) {
    return { status: "not-reported", contributions: [] };
  }
  return {
    status: "reported",
    gatewayOk: input.gateway.status.value.ok,
    anthropicHealth: provider === "anthropic" ? input.gateway.status.value.anthropicHealth : null,
    joinability: "unjoinable-with-pool-rows",
    contributions,
  };
}

function modelCatalogFor(
  input: TokenCommuneSnapshotProjectionInput,
  provider: string,
): ProviderModelCatalog {
  if (input.gateway.models.status === "unavailable") return { status: "unavailable", models: [] };
  return {
    status: "reported",
    models: input.gateway.models.value.models
      .filter((model) => model.provider === provider)
      .map(copyModel)
      .sort((left, right) => compareText(left.id, right.id) || compareText(left.provider, right.provider)),
  };
}

function fingerprintFor(
  input: TokenCommuneSnapshotProjectionInput,
  provider: string,
): ProviderFingerprint {
  const probe = provider === "anthropic" ? "anthropic"
    : provider === "openai-codex" ? "openai-codex"
      : null;
  if (probe === null) return { status: "unknown", probe: null, reason: "not-probed" };
  if (input.gateway.fingerprints.status === "unavailable") {
    return { status: "unknown", probe, reason: "probe-unavailable" };
  }
  return {
    status: "reported",
    probe,
    value: probe === "anthropic"
      ? input.gateway.fingerprints.value.anthropic
      : input.gateway.fingerprints.value.codex,
  };
}

function upsertMutation(
  input: TokenCommuneSnapshotProjectionInput,
  resource: TokenCommuneResourceName,
  identity: SynthesizedResourceIdentity,
  payload: unknown,
  projection: unknown,
): ResourceReportMutation {
  const expectedKind = TOKEN_COMMUNE_RESOURCES[resource].kind;
  if (
    identity.adapterId !== input.adapterId
    || identity.resourceKind !== expectedKind
    || !identity.resourceId.trim()
  ) {
    throw new SnapshotProjectionError("identity-mismatch");
  }
  const resourcePayload = encodeResourceEnvelope(resource, "payload", payload);
  const projectionPayload = encodeResourceEnvelope(resource, "projection", projection);
  return create(ResourceReportMutationSchema, {
    identity: create(ResourceIdentitySchema, {
      adapterId: create(AdapterIdSchema, { value: identity.adapterId }),
      resourceId: create(ResourceIdSchema, { value: identity.resourceId }),
      resourceKind: create(ResourceKindSchema, { value: identity.resourceKind }),
    }),
    mutation: {
      case: "upsert",
      value: create(ResourceStateUpsertSchema, { resourcePayload, projectionPayload }),
    },
  });
}

function sortCapacityReadings(readings: readonly GatewayCapacityReading[]): GatewayCapacityReading[] {
  return readings.map((reading) => ({ ...reading })).sort((left, right) => compareText(stableJson(left), stableJson(right)));
}

function sortStatusContributions(rows: readonly GatewayStatusContribution[]): GatewayStatusContribution[] {
  return rows.map((row) => ({ ...row, readings: sortCapacityReadings(row.readings) }))
    .sort((left, right) => compareText(left.contributionId, right.contributionId) || compareText(left.provider, right.provider));
}

function copyModel(model: GatewayModel): GatewayModel {
  return { ...model };
}

function copyDrawReport(report: GatewayDrawReport): GatewayDrawReport {
  return { ...report };
}

function stableJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const row = value as Record<string, unknown>;
    return `{${Object.keys(row).sort(compareText).map((key) => `${JSON.stringify(key)}:${stableJson(row[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function digest(value: string): string {
  return createHash("sha256").update(value).digest("hex").slice(0, 24);
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function validateContext(input: TokenCommuneSnapshotProjectionInput): void {
  if (
    !input.adapterId.trim()
    || !Number.isSafeInteger(input.adapterGeneration)
    || input.adapterGeneration <= 0
    || !input.identities.gatewayDeploymentKey.trim()
    || typeof input.observedAt?.seconds !== "bigint"
    || !Number.isInteger(input.observedAt.nanos)
    || input.observedAt.nanos < 0
    || input.observedAt.nanos > 999_999_999
  ) {
    throw new SnapshotProjectionError("invalid-context");
  }
}
