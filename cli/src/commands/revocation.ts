import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  DeviceIdSchema,
  EndpointIdSchema,
  RevokeAllOperatorSessionsRequestSchema,
  RevokeControlSurfaceEndpointRequestSchema,
  RevokeControlSurfacePrincipalRequestSchema,
  type RevokeAllOperatorSessionsResult,
  type RevokeControlSurfaceResult,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CliOutput } from "../main.js";
import type { CredentialStore } from "../credentials.js";
import { eventIdView } from "../output.js";

const DEFAULT_REASON_CODE = "operator_requested";
const REASON_CODE = /^[a-z][a-z0-9_]{0,63}$/;

export async function revokeAllSessionsCommand(
  client: Pick<ControlClient, "revokeAllOperatorSessions">,
  store: CredentialStore,
  options: { reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number> {
  validateReasonCode(options.reasonCode);
  await store.readRequired();

  let result: RevokeAllOperatorSessionsResult;
  try {
    result = await client.revokeAllOperatorSessions(
      create(RevokeAllOperatorSessionsRequestSchema, { reasonCode: options.reasonCode }),
    );
    validateRevokeAllResult(result);
  } catch (error) {
    return reportRevocationFailure(error, output);
  }

  const view = {
    kind: "operator_sessions",
    revokedSessionCount: result.revokedSessionCount,
    generation: result.invalidatedThroughGeneration?.value.toString() ?? null,
    revocationEventId: eventIdView(result.revocationEventId),
  };
  try {
    await store.clear();
  } catch {
    output.stderr("revocation succeeded, but local credentials could not be cleared; run patchbay-cli login to reconcile");
    return 1;
  }
  printRevocation(view, options.json, output);
  output.stderr("local credentials cleared; run patchbay-cli login from a trusted host to re-enter");
  return 0;
}

export async function revokePrincipalCommand(
  client: Pick<ControlClient, "revokeControlSurfacePrincipal">,
  store: CredentialStore,
  options: { principalId: string; reasonCode: string; json: boolean },
  output: CliOutput,
): Promise<number> {
  validateTargetId(options.principalId, "principal id");
  validateReasonCode(options.reasonCode);
  const credentials = await store.readRequired();

  let result: RevokeControlSurfaceResult;
  try {
    result = await client.revokeControlSurfacePrincipal(
      create(RevokeControlSurfacePrincipalRequestSchema, {
        principalId: options.principalId,
        reasonCode: options.reasonCode,
      }),
    );
    validateScopeResult(result);
  } catch (error) {
    return reportRevocationFailure(error, output);
  }

  const selfRevoked = credentials.principal.principalId === options.principalId;
  if (selfRevoked) {
    try {
      await store.clear();
    } catch {
      output.stderr("principal revocation succeeded, but local credentials could not be cleared; run patchbay-cli login to reconcile");
      return 1;
    }
  }
  printRevocation({
    kind: "principal",
    principalId: options.principalId,
    newlyRevoked: result.newlyRevoked,
    revokedPrincipalCount: result.revokedPrincipalCount,
    revokedSessionCount: result.revokedSessionCount,
    revocationEventId: eventIdView(result.revocationEventId),
  }, options.json, output);
  if (selfRevoked) {
    output.stderr("local credentials cleared; re-enter with patchbay-cli login from a distinct unrevoked identity");
  }
  return 0;
}

export interface RevokeEndpointOptions {
  endpointId?: string;
  deviceId?: string;
  reasonCode: string;
  json: boolean;
}

export async function revokeEndpointCommand(
  client: Pick<ControlClient, "revokeControlSurfaceEndpoint">,
  store: CredentialStore,
  options: RevokeEndpointOptions,
  output: CliOutput,
): Promise<number> {
  const target = validateEndpointTarget(options);
  validateReasonCode(options.reasonCode);
  const credentials = await store.readRequired();
  const request = target.kind === "endpoint"
    ? create(RevokeControlSurfaceEndpointRequestSchema, {
        target: { case: "endpointId", value: create(EndpointIdSchema, { value: target.id }) },
        reasonCode: options.reasonCode,
      })
    : create(RevokeControlSurfaceEndpointRequestSchema, {
        target: { case: "deviceId", value: create(DeviceIdSchema, { value: target.id }) },
        reasonCode: options.reasonCode,
      });

  let result: RevokeControlSurfaceResult;
  try {
    result = await client.revokeControlSurfaceEndpoint(request);
    validateScopeResult(result);
  } catch (error) {
    return reportRevocationFailure(error, output);
  }

  const selfRevoked = target.kind === "endpoint"
    ? credentials.principal.endpointId === target.id
    : credentials.principal.deviceId === target.id;
  if (selfRevoked) {
    try {
      await store.clear();
    } catch {
      output.stderr(`${target.kind} revocation succeeded, but local credentials could not be cleared; run patchbay-cli login to reconcile`);
      return 1;
    }
  }
  printRevocation({
    kind: target.kind,
    targetId: target.id,
    newlyRevoked: result.newlyRevoked,
    revokedPrincipalCount: result.revokedPrincipalCount,
    revokedSessionCount: result.revokedSessionCount,
    revocationEventId: eventIdView(result.revocationEventId),
  }, options.json, output);
  if (selfRevoked) {
    output.stderr(`${target.kind} credentials cleared; re-enter with patchbay-cli login from a distinct unrevoked identity`);
  }
  return 0;
}

export function validateReasonCode(reasonCode: string | undefined): asserts reasonCode is string {
  const value = reasonCode ?? DEFAULT_REASON_CODE;
  if (!REASON_CODE.test(value)) {
    throw new Error("reason code must be 1..64 lowercase ASCII letters, digits, or underscores");
  }
}

export function validateEndpointTarget(options: Pick<RevokeEndpointOptions, "endpointId" | "deviceId">):
  { kind: "endpoint" | "device"; id: string } {
  const hasEndpoint = options.endpointId !== undefined;
  const hasDevice = options.deviceId !== undefined;
  if (hasEndpoint === hasDevice) {
    throw new Error("exactly one endpoint or device target is required");
  }
  const id = hasEndpoint ? options.endpointId : options.deviceId;
  validateTargetId(id!, hasEndpoint ? "endpoint id" : "device id");
  return { kind: hasEndpoint ? "endpoint" : "device", id: id! };
}

function validateTargetId(value: string, label: string): void {
  if (!value) throw new Error(`${label} must not be empty`);
  if (value.length > 256) throw new Error(`${label} is too long`);
}

function validateRevokeAllResult(result: RevokeAllOperatorSessionsResult): void {
  if (!result.invalidatedThroughGeneration || result.invalidatedThroughGeneration.value <= 0n) {
    throw new Error("invalid revoke-all response: missing invalidated generation");
  }
  validateEvent(result.revocationEventId);
}

function validateScopeResult(result: RevokeControlSurfaceResult): void {
  validateEvent(result.revocationEventId);
}

function validateEvent(event: RevokeControlSurfaceResult["revocationEventId"]): void {
  if (!event?.authorityDomainId?.value || event.lsn === undefined) {
    throw new Error("invalid revocation response: missing source event identity");
  }
}

function printRevocation(view: Record<string, unknown>, json: boolean, output: CliOutput): void {
  if (json) {
    output.stdout(JSON.stringify(view));
    return;
  }
  output.stdout(Object.entries(view).map(([key, value]) => {
    if (key === "revocationEventId") {
      const event = value as { authorityDomainId: string | null; lsn: string | null } | null;
      return `event=${event?.lsn ?? "-"}`;
    }
    return `${key}=${formatValue(value)}`;
  }).join(" "));
}

function formatValue(value: unknown): string {
  if (typeof value === "object" && value !== null) return JSON.stringify(value);
  return String(value ?? "-");
}

function reportRevocationFailure(error: unknown, output: CliOutput): number {
  const code = ConnectError.from(error, Code.Internal).code;
  if (code === Code.PermissionDenied || code === Code.InvalidArgument || code === Code.NotFound) {
    output.stderr("revocation denied or rejected; no confirmed success was received");
    return 2;
  }
  output.stderr("revocation did not return confirmed success; credentials may already be invalid; run patchbay-cli login to reconcile");
  return 1;
}

export function defaultReasonCode(): string {
  return DEFAULT_REASON_CODE;
}
