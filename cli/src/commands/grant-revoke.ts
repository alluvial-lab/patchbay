import { Code, ConnectError } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import {
  GrantIdSchema,
  AuthorityDomainIdSchema,
  FailureCode,
  GrantRevocationPolicy,
  OperationState,
  RevokeGrantRequestSchema,
  type RevokeGrantResult,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CliOutput } from "../main.js";
import { enumLabel, eventIdView } from "../output.js";

export interface GrantRevokeOptions {
  grantId: string;
  reason?: string;
  json: boolean;
}

export async function grantRevokeCommand(
  client: Pick<ControlClient, "revokeGrant">,
  authorityDomainId: string,
  options: GrantRevokeOptions,
  output: CliOutput,
): Promise<number> {
  if (!options.grantId) throw new Error("grant id must not be empty");
  if (options.grantId.length > 256) throw new Error("grant id is too long");
  const reason = options.reason ?? "operator_requested";
  if (!/^[\x21-\x7e]{1,128}$/.test(reason) || reason.includes("=")) {
    throw new Error("reason must be 1..128 safe ASCII characters");
  }

  let result: RevokeGrantResult;
  try {
    result = await client.revokeGrant(create(RevokeGrantRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
      grantId: create(GrantIdSchema, { value: options.grantId }),
      reason,
    }));
  } catch (error) {
    if (error instanceof ConnectError && error.code === Code.PermissionDenied) {
      output.stderr("grant revocation denied; no grant change was made");
      return 2;
    }
    throw error;
  }

  const view = revokeGrantView(result, options.grantId);
  if (options.json) {
    output.stdout(JSON.stringify(view));
  } else {
    output.stdout([
      `grant=${view.grantId}`,
      `status=${view.alreadyRevoked ? "already_revoked" : view.changed ? "changed" : "unchanged"}`,
      `policy=${view.appliedPolicy}`,
      `event=${view.revocationEventId?.lsn ?? "-"}`,
      `affected_commands=${view.affectedCommandCount}`,
    ].join(" "));
  }
  return 0;
}

export function revokeGrantView(result: RevokeGrantResult, requestedGrantId: string) {
  return {
    grantId: requestedGrantId,
    changed: result.changed,
    alreadyRevoked: result.alreadyRevoked,
    revocationEventId: eventIdView(result.revocationEventId),
    appliedPolicy: enumLabel(GrantRevocationPolicy, result.appliedPolicy),
    affectedCommandCount: result.commandEffects.length,
    commandEffects: result.commandEffects.map((effect) => ({
      commandId: effect.commandId?.value || null,
      fromState: enumLabel(OperationState, effect.fromState),
      toState: enumLabel(OperationState, effect.toState),
      failureCode: enumLabel(FailureCode, effect.failureCode),
    })),
  };
}
