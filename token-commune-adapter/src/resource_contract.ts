import type {
  GatewayDrawReport,
  GatewayFingerprintSummary,
  GatewayModel,
  GatewayPoolContribution,
  GatewayStatusSummary,
} from "./gateway_client.js";

export const TOKEN_COMMUNE_RESOURCES = {
  providerPool: {
    kind: "token-commune.provider-pool",
    payloadSchema: "patchbay.token_commune.provider_pool.payload.v1",
    projectionSchema: "patchbay.token_commune.provider_pool.projection.v1",
  },
  memberDraw: {
    kind: "token-commune.member-draw",
    payloadSchema: "patchbay.token_commune.member_draw.payload.v1",
    projectionSchema: "patchbay.token_commune.member_draw.projection.v1",
  },
} as const;

export const TOKEN_COMMUNE_RESOURCE_KINDS = {
  providerPool: TOKEN_COMMUNE_RESOURCES.providerPool.kind,
  memberDraw: TOKEN_COMMUNE_RESOURCES.memberDraw.kind,
} as const;

export const TOKEN_COMMUNE_SCHEMAS = {
  providerPoolPayload: TOKEN_COMMUNE_RESOURCES.providerPool.payloadSchema,
  providerPoolProjection: TOKEN_COMMUNE_RESOURCES.providerPool.projectionSchema,
  memberDrawPayload: TOKEN_COMMUNE_RESOURCES.memberDraw.payloadSchema,
  memberDrawProjection: TOKEN_COMMUNE_RESOURCES.memberDraw.projectionSchema,
} as const;

export type TokenCommuneResourceKind =
  (typeof TOKEN_COMMUNE_RESOURCE_KINDS)[keyof typeof TOKEN_COMMUNE_RESOURCE_KINDS];

export interface ProviderPoolPayload {
  identityStrategy: "composite-local";
  gatewayDeploymentKey: string;
  provider: string;
  contributions: readonly GatewayPoolContribution[];
  models: readonly GatewayModel[];
  fingerprint: GatewayFingerprintSummary;
  sourceStatus: GatewayStatusSummary;
  limitations: {
    snapshotCompleteness: "partial";
    contributorAttribution: "unavailable";
    contributionIdentity: "unjoinable";
  };
}

export interface ProviderPoolProjection {
  provider: string;
  contributionCount: number;
  totalDeclaredShare: number;
  healthCounts: { fresh: number; exhausted: number; authBroken: number };
  anonymousContributions: readonly GatewayPoolContribution[];
  models: readonly GatewayModel[];
  fingerprint: GatewayFingerprintSummary;
}

export interface MemberDrawPayload {
  identityStrategy: "composite-local";
  gatewayDeploymentKey: string;
  memberDisplayName: string;
  provider: string;
  reports: readonly GatewayDrawReport[];
  limitations: {
    snapshotCompleteness: "partial";
    stableMemberIdentity: "unavailable";
  };
}

export interface MemberDrawProjection {
  memberDisplayName: string;
  provider: string;
  reports: readonly GatewayDrawReport[];
  enforcementState: "unknown" | "within-limit" | "exceeded";
}
