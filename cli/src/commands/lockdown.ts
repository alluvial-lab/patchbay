import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  AuthorityDomainIdSchema,
  EnterSecurityLockdownRequestSchema,
  type EnterSecurityLockdownResult,
  ExitSecurityLockdownRequestSchema,
  type ExitSecurityLockdownResult,
} from "@patchbay/contracts";
import type { AdminClient, ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { eventIdView, timestampView } from "../output.js";

const SAFE_REASON = /^[a-z0-9_]{1,64}$/;

export async function lockdownEnterCommand(
  client: Pick<ControlClient, "enterSecurityLockdown">,
  store: CredentialStore,
  authorityDomainId: string,
  options: { reasonCode: string; confirm: string; json: boolean },
  output: CliOutput,
): Promise<number> {
  validateReason(options.reasonCode);
  if (options.confirm !== "LOCKDOWN") throw new Error("lockdown-enter requires --confirm LOCKDOWN");
  await store.readRequired();
  let result: EnterSecurityLockdownResult;
  try {
    result = await client.enterSecurityLockdown(create(EnterSecurityLockdownRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
      reasonCode: options.reasonCode,
    }));
    validateEnterResult(result, authorityDomainId);
  } catch (error) {
    return reportFailure(error, output, "entry");
  }
  try { await store.clear(); } catch {
    output.stderr("lockdown entry succeeded, but local credentials could not be cleared; run patchbay-cli login to reconcile");
    return 1;
  }
  const state = result.lockdown!;
  print({
    kind: "security_lockdown", active: true, reasonCode: state.reasonCode,
    enteredAt: timestampView(state.enteredAt), authorityDomainId,
    lockdownEventId: eventIdView(result.lockdownEventId), alreadyActive: result.alreadyActive,
    affectedRuntimeSessionCount: result.affectedRuntimeSessionCount,
    invalidatedThroughOperatorSessionGeneration: result.invalidatedThroughOperatorSessionGeneration?.value.toString() ?? null,
  }, options.json, output);
  output.stderr("local credentials cleared; run patchbay-cli login for read-only inspection or patchbay-cli lockdown-exit for trusted recovery");
  return 0;
}

export async function lockdownExitCommand(
  client: Pick<AdminClient, "exitSecurityLockdown">,
  authorityDomainId: string,
  options: { reasonCode?: string; json: boolean },
  output: CliOutput,
): Promise<number> {
  if (options.reasonCode !== undefined) validateReason(options.reasonCode);
  let result: ExitSecurityLockdownResult;
  try {
    result = await client.exitSecurityLockdown(create(ExitSecurityLockdownRequestSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
      ...(options.reasonCode ? { reasonCode: options.reasonCode } : {}),
    }));
    validateExitResult(result, authorityDomainId);
  } catch (error) {
    return reportFailure(error, output, "exit");
  }
  print({
    kind: "security_lockdown_exit", active: false, authorityDomainId,
    bootstrapChannel: "loopback_admin", lockdownEventId: eventIdView(result.lockdownEventId),
    alreadyInactive: result.alreadyInactive, enteredEventId: eventIdView(result.lockdown?.enteredEventId),
  }, options.json, output);
  output.stderr("lockdown is inactive; run patchbay-cli login to obtain a fresh operator session");
  return 0;
}

function validateEnterResult(result: EnterSecurityLockdownResult, domain: string): void {
  if (!result.lockdown?.active || !result.lockdown.reasonCode) throw new Error("malformed lockdown entry response: posture is not active");
  validateReason(result.lockdown.reasonCode);
  validateEvent(result.lockdownEventId, domain, "lockdown entry event");
  if (!result.invalidatedThroughOperatorSessionGeneration || result.invalidatedThroughOperatorSessionGeneration.value <= 0n) throw new Error("malformed lockdown entry response: missing generation floor");
}

function validateExitResult(result: ExitSecurityLockdownResult, domain: string): void {
  if (!result.lockdown || result.lockdown.active) throw new Error("malformed lockdown exit response: posture remains active");
  if (!result.alreadyInactive) {
    validateEvent(result.lockdownEventId, domain, "lockdown exit event");
    validateEvent(result.lockdown.enteredEventId, domain, "prior lockdown entry event");
  } else if (result.lockdownEventId) throw new Error("contradictory lockdown exit response: inactive result has an event");
}

function validateEvent(event: { authorityDomainId?: { value: string }; lsn?: { value: bigint }} | undefined, domain: string, label: string): void {
  if (!event?.authorityDomainId?.value || event.authorityDomainId.value !== domain || event.lsn === undefined) throw new Error(`malformed lockdown response: invalid ${label}`);
}
function validateReason(value: string): void {
  if (!SAFE_REASON.test(value)) throw new Error("reason code must be 1..64 lowercase ASCII letters, digits, or underscores");
}
function print(view: Record<string, unknown>, json: boolean, output: CliOutput): void {
  if (json) { output.stdout(JSON.stringify(view)); return; }
  output.stdout(Object.entries(view).map(([key, value]) => `${key}=${typeof value === "object" && value !== null ? JSON.stringify(value) : String(value ?? "-")}`).join(" "));
}
function reportFailure(error: unknown, output: CliOutput, operation: "entry" | "exit"): number {
  const code = ConnectError.from(error, Code.Internal).code;
  if (code === Code.PermissionDenied || code === Code.InvalidArgument || code === Code.FailedPrecondition) {
    output.stderr(`lockdown ${operation} denied or rejected; no confirmed success was received`);
    return 2;
  }
  output.stderr(`lockdown ${operation} did not return confirmed success; reconcile posture via patchbay-cli ${operation === "entry" ? "session-health" : "lockdown-exit"}`);
  return 1;
}
