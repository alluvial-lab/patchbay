import { create } from "@bufbuild/protobuf";
import { PayloadContentType, PayloadEnvelopeSchema, type PayloadEnvelope } from "@patchbay/contracts";
import { Ajv2020, type ValidateFunction } from "ajv/dist/2020.js";
import ajvFormatsModule from "ajv-formats";
import memberDrawPayloadSchema from "../schemas/member-draw-payload.schema.json" with { type: "json" };
import memberDrawProjectionSchema from "../schemas/member-draw-projection.schema.json" with { type: "json" };
import providerPoolPayloadSchema from "../schemas/provider-pool-payload.schema.json" with { type: "json" };
import providerPoolProjectionSchema from "../schemas/provider-pool-projection.schema.json" with { type: "json" };
import { TOKEN_COMMUNE_RESOURCES } from "./resource_contract.js";

export type TokenCommuneResourceName = keyof typeof TOKEN_COMMUNE_RESOURCES;
export type EnvelopeRole = "payload" | "projection";

const schemas = [
  ["provider-pool-payload.schema.json", providerPoolPayloadSchema],
  ["provider-pool-projection.schema.json", providerPoolProjectionSchema],
  ["member-draw-payload.schema.json", memberDrawPayloadSchema],
  ["member-draw-projection.schema.json", memberDrawProjectionSchema],
] as const;

const ajv = new Ajv2020({ allErrors: true, strict: true });
ajvFormatsModule.default(ajv);
for (const [key, schema] of schemas) ajv.addSchema(schema, key);

const validators: Record<TokenCommuneResourceName, Record<EnvelopeRole, ValidateFunction>> = {
  providerPool: {
    payload: requiredValidator(TOKEN_COMMUNE_RESOURCES.providerPool.payloadSchema),
    projection: requiredValidator(TOKEN_COMMUNE_RESOURCES.providerPool.projectionSchema),
  },
  memberDraw: {
    payload: requiredValidator(TOKEN_COMMUNE_RESOURCES.memberDraw.payloadSchema),
    projection: requiredValidator(TOKEN_COMMUNE_RESOURCES.memberDraw.projectionSchema),
  },
};

export class ResourceEnvelopeValidationError extends Error {
  readonly name = "ResourceEnvelopeValidationError";
  constructor() {
    super("token-commune resource contract validation failed");
  }
}

export function encodeResourceEnvelope(
  resource: TokenCommuneResourceName,
  role: EnvelopeRole,
  value: unknown,
): PayloadEnvelope {
  if (!validators[resource][role](value)) throw new ResourceEnvelopeValidationError();
  const descriptor = TOKEN_COMMUNE_RESOURCES[resource];
  return create(PayloadEnvelopeSchema, {
    payload: new TextEncoder().encode(JSON.stringify(value)),
    contentType: PayloadContentType.JSON,
    schemaRef: role === "payload" ? descriptor.payloadSchema : descriptor.projectionSchema,
  });
}

function requiredValidator(schemaRef: string): ValidateFunction {
  const validator = ajv.getSchema(schemaRef);
  if (!validator) throw new Error(`missing token-commune schema validator: ${schemaRef}`);
  return validator;
}
