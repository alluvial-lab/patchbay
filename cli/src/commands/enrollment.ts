import { randomUUID } from "node:crypto";
import { hostname } from "node:os";
import { create } from "@bufbuild/protobuf";
import {
  DeviceIdSchema,
  EndpointIdSchema,
  GenerationSchema,
  PrincipalEnrollmentSchema,
  type PrincipalEnrollment,
} from "@patchbay/contracts";

export interface EnrollmentOptions {
  endpointId?: string;
  deviceId?: string;
}

export function newCliEnrollment(options: EnrollmentOptions = {}): PrincipalEnrollment {
  const endpointId = options.endpointId ?? `patchbay-cli-${randomUUID()}`;
  const deviceId = options.deviceId ?? `cli-${hostname()}`;
  if (!endpointId) throw new Error("CLI endpoint id must not be empty");
  if (!deviceId) throw new Error("CLI device id must not be empty");

  return create(PrincipalEnrollmentSchema, {
    endpointId: create(EndpointIdSchema, { value: endpointId }),
    deviceId: create(DeviceIdSchema, { value: deviceId }),
    endpointGeneration: create(GenerationSchema, { value: 1n }),
  });
}
