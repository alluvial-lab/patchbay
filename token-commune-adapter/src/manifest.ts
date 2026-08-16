import { create } from "@bufbuild/protobuf";
import {
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySchema,
  AdapterDiagnosticReportingCapabilitySchema,
  AdapterReconciliationStrength,
  AdapterSnapshotSupport,
  AdapterTargetCategory,
  AttachmentMethodSchema,
  FailureCode,
  IdempotencyStrength,
  PayloadContentType,
  ReconciliationAction,
  ResourceCapabilitySchema,
  ResourceKindSchema,
  ResourceProjectionContractSchema,
  SchemaDescriptorSchema,
  type AdapterCapability,
} from "@patchbay/contracts";
import { TOKEN_COMMUNE_RESOURCES } from "./resource_contract.js";
import { TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES } from "./core_diagnostics_forwarder.js";

export function tokenCommuneCapabilityManifest(): AdapterCapability {
  return create(AdapterCapabilitySchema, {
    supportedOperationKinds: [],
    supportedTargetSpecShapes: [],
    streamingSupport: false,
    sessionSnapshotSupport: AdapterSnapshotSupport.UNSPECIFIED,
    cancellationSupport: false,
    sessionReplacementSupport: false,
    assurance: create(AdapterAssuranceManifestSchema, {
      contract: {
        case: "v1",
        value: create(AdapterAssuranceManifestV1Schema, {
          deduplicationStrength: IdempotencyStrength.NONE,
          continuationProofSupport: false,
          cursorSupport: false,
          generationFenceSupport: false,
          reconciliationStrength: AdapterReconciliationStrength.NONE,
          unprovenOutcomeAction: ReconciliationAction.NONE,
        }),
      },
    }),
    attachmentMethod: create(AttachmentMethodSchema, {
      kind: "configured-local-material",
      descriptor: new Uint8Array(),
      descriptorContentType: PayloadContentType.BINARY,
    }),
    knownFailureModes: [
      FailureCode.UNSUPPORTED_COMMAND,
      FailureCode.ADAPTER_UNAVAILABLE,
      FailureCode.TRANSPORT_TIMEOUT,
      FailureCode.EXECUTION_FAILED,
    ],
    diagnosticReporting: create(AdapterDiagnosticReportingCapabilitySchema, {
      diagnosticCodes: Object.values(TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES),
    }),
    targetCategories: [AdapterTargetCategory.OPERATIONAL_RESOURCE],
    resourceCapabilities: Object.values(TOKEN_COMMUNE_RESOURCES).map((resource) =>
      create(ResourceCapabilitySchema, {
        resourceKind: create(ResourceKindSchema, { value: resource.kind }),
        snapshotSupport: AdapterSnapshotSupport.PARTIAL,
        projectionContract: create(ResourceProjectionContractSchema, {
          targetCategory: AdapterTargetCategory.OPERATIONAL_RESOURCE,
          payloadSchema: create(SchemaDescriptorSchema, {
            schemaRef: resource.payloadSchema,
            contentType: PayloadContentType.JSON,
          }),
          projectionSchema: create(SchemaDescriptorSchema, {
            schemaRef: resource.projectionSchema,
            contentType: PayloadContentType.JSON,
          }),
        }),
      }),
    ),
  });
}
