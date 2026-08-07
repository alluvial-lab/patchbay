import type {
  GatewayCapacityReading,
  GatewayContributionHealth,
  GatewayDrawReport,
  GatewayFingerprintState,
  GatewayModel,
  GatewayPoolFingerprint,
  GatewayStatusContribution,
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

export interface AnonymousPoolContribution {
  readonly subKey: string;
  readonly subKeySource: "synthesized-content-hash";
  readonly subKeyStability: "snapshot-local";
  readonly attribution: "unavailable";
  readonly declaredShare: number;
  readonly health: GatewayContributionHealth;
  readonly telemetryState: "readings" | "no-readings";
  readonly capacityReadings: readonly GatewayCapacityReading[];
  readonly fingerprint: GatewayPoolFingerprint;
}

export type ContributionListing =
  | { readonly status: "reported"; readonly contributions: readonly AnonymousPoolContribution[] }
  | { readonly status: "not-reported"; readonly contributions: readonly [] }
  | { readonly status: "unavailable"; readonly contributions: readonly [] };

export type ProviderStatusTelemetry =
  | {
      readonly status: "reported";
      readonly gatewayOk: boolean;
      readonly anthropicHealth: GatewayContributionHealth | null;
      readonly joinability: "unjoinable-with-pool-rows";
      readonly contributions: readonly GatewayStatusContribution[];
    }
  | { readonly status: "not-reported" | "unavailable"; readonly contributions: readonly [] };

export type ProviderModelCatalog =
  | { readonly status: "reported"; readonly models: readonly GatewayModel[] }
  | { readonly status: "unavailable"; readonly models: readonly [] };

export type ProviderFingerprint =
  | {
      readonly status: "reported";
      readonly probe: "anthropic" | "openai-codex";
      readonly value: GatewayFingerprintState;
    }
  | {
      readonly status: "unknown";
      readonly probe: "anthropic" | "openai-codex" | null;
      readonly reason: "probe-unavailable" | "not-probed";
    };

export interface ProviderPoolPayload {
  readonly identityStrategy: "composite-local";
  readonly gatewayDeploymentKey: string;
  readonly provider: string;
  readonly contributionListing: ContributionListing;
  readonly statusTelemetry: ProviderStatusTelemetry;
  readonly modelCatalog: ProviderModelCatalog;
  readonly fingerprint: ProviderFingerprint;
  readonly limitations: {
    readonly snapshotCompleteness: "partial";
    readonly contributorAttribution: "unavailable";
    readonly contributionIdentity: "snapshot-local-synthesized";
    readonly statusPoolJoin: "unavailable";
    readonly capacityAggregation: "none";
  };
}

export interface ProviderPoolProjection {
  readonly provider: string;
  readonly contributionListing: ContributionListing;
  readonly credentialHealthCounts: {
    readonly fresh: number;
    readonly exhausted: number;
    readonly authBroken: number;
  };
  readonly totalDeclaredShare: number;
  readonly statusTelemetry: ProviderStatusTelemetry;
  readonly modelCatalog: ProviderModelCatalog;
  readonly fingerprint: ProviderFingerprint;
  readonly capacityAggregation: "none";
}

export interface MemberDrawPayload {
  readonly identityStrategy: "composite-local";
  readonly gatewayDeploymentKey: string;
  readonly memberDisplayName: string;
  readonly provider: string;
  readonly reports: readonly GatewayDrawReport[];
  readonly limitations: {
    readonly snapshotCompleteness: "partial";
    readonly stableMemberIdentity: "unavailable";
  };
}

export interface MemberDrawProjection {
  readonly memberDisplayName: string;
  readonly provider: string;
  readonly reports: readonly GatewayDrawReport[];
}
