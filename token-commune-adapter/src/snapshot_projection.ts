import { create } from "@bufbuild/protobuf";
import type { Timestamp } from "@bufbuild/protobuf/wkt";
import {
  AdapterIdSchema,
  AdapterSnapshotSupport,
  GenerationSchema,
  ResourceKindSchema,
  ResourceReportSchema,
  ResourceSnapshotReportSchema,
  ResourceViewReportSchema,
  type ResourceReport,
  type ResourceViewReport,
} from "@patchbay/contracts";
import type {
  GatewayFingerprints,
  GatewayMe,
  GatewayModels,
  GatewayPool,
  GatewayStatus,
} from "./gateway_client.js";
import type { ResourceIdentitySynthesizer } from "./identity.js";
import { ResourceEnvelopeValidationError } from "./resource_envelope.js";
import { TOKEN_COMMUNE_RESOURCES } from "./resource_contract.js";

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
    const views = resourceViewOrder.map(({ kind }) =>
      create(ResourceViewReportSchema, {
        resourceKind: create(ResourceKindSchema, { value: kind }),
        completeness: AdapterSnapshotSupport.PARTIAL,
        mutations: [],
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

const resourceViewOrder: ReadonlyArray<{
  readonly kind: string;
  readonly mutations: ResourceViewReport["mutations"];
}> = Object.values(TOKEN_COMMUNE_RESOURCES).map(({ kind }) => ({ kind, mutations: [] }));

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
