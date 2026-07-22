import {
  FailureCode,
  OperationState,
  SubmissionOutcome,
  type SubmissionResult,
} from "@patchbay/contracts";
import type { CliOutput } from "./main.js";

export interface SubmissionView {
  outcome: string;
  commandId: string | null;
  operationState: string;
  failureCode: string | null;
  diagnosticMessage: string | null;
  acceptedLsn: string | null;
  deduplicated: boolean;
}

export function exitCodeForSubmission(outcome: SubmissionOutcome): number {
  switch (outcome) {
    case SubmissionOutcome.ACCEPTED:
      return 0;
    case SubmissionOutcome.REJECTED:
      return 2;
    case SubmissionOutcome.FAILED:
      return 3;
    case SubmissionOutcome.UNKNOWN:
      return 4;
    case SubmissionOutcome.UNSPECIFIED:
    default:
      return 1;
  }
}

export function printSubmissionResult(
  result: SubmissionResult,
  json: boolean,
  output: CliOutput,
): number {
  const view = submissionView(result);
  if (json) {
    output.stdout(JSON.stringify(view));
  } else {
    const fields = [
      `outcome=${view.outcome}`,
      `command=${view.commandId ?? "-"}`,
      `state=${view.operationState}`,
      `failure=${view.failureCode ?? "-"}`,
      `lsn=${view.acceptedLsn ?? "-"}`,
      `deduplicated=${String(view.deduplicated)}`,
    ];
    output.stdout(fields.join(" "));
    if (view.diagnosticMessage) output.stderr(view.diagnosticMessage);
  }

  if (result.outcome === SubmissionOutcome.UNKNOWN) {
    output.stderr("Submission outcome is UNKNOWN; reconcile via the core's command records.");
  }
  return exitCodeForSubmission(result.outcome);
}

export function submissionView(result: SubmissionResult): SubmissionView {
  return {
    outcome: enumLabel(SubmissionOutcome, result.outcome),
    commandId: result.commandId?.value || null,
    operationState: enumLabel(OperationState, result.operationState),
    failureCode:
      result.failureCode === FailureCode.UNSPECIFIED
        ? null
        : enumLabel(FailureCode, result.failureCode),
    diagnosticMessage: result.diagnosticMessage || null,
    acceptedLsn: result.acceptedLsn?.value.toString() ?? null,
    deduplicated: result.deduplicated,
  };
}

export function enumLabel(
  registry: Record<number, string>,
  value: number,
): string {
  return (registry[value] ?? `UNRECOGNIZED_${value}`).toLowerCase();
}
